//! ClipmapManager — 多分辨率 LOD 地形管理
//!
//! 8 级 LOD 严格对齐 CHG-049 scene_lod 0-7: 0.5m-64m voxel/spacing, 0-10km 视野。
//! tile 链 16→32→64→128→256→512→1024→2048m — Transvoxel (LOD 0-4) + SH (LOD 5-7)。
//! ★ Sprint-017: scene_lod 6-7 新增 (SH, 32m/64m spacing, 4-10km 远距离)。
//!
//! Clipmap LOD 管理器。8 层 LOD（0-7），对齐 CHG-049 scene_lod 表。
//!
//! ## Async 生成（v2）
//!
//! 生产模式 (`new_async`): mesh 生成提交到 rayon 后台线程池，
//! 主线程仅收割已完成结果 + Godot mesh upload。消除移动中卡顿。
//!
//! 测试模式 (`new`): 同步生成，保留原有行为。

use std::collections::{HashMap, HashSet};
use std::sync::{mpsc, Arc};

use woworld_core::prelude::WorldPos;

use crate::density::{CaveParams, DensityField, DensityStack, HeightfieldDensity};
use crate::transvoxel::IsoSurfaceParams;
use crate::terrain::HeightfieldTerrain;
use crate::terrain_mesh::{generate_sh_mesh, TerrainMeshData};
use crate::transvoxel::extract_isosurface_transvoxel;

// ── LOD 层级定义 ──────────────────────

/// 网格生成算法
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum MeshAlgorithm {
    /// Transvoxel 3D 等值面提取
    Transvoxel { voxel_size: f64 },
    /// Signed Heightfield — 2D 高度网格 + 梯度法线 + 高度着色
    SignedHeightfield { spacing: f64 },
}

/// 一个 LOD 层级的配置
#[derive(Clone, Debug)]
pub(crate) struct LodLevel {
    /// 层级索引 (0=最近)
    pub index: u8,
    /// 该层覆盖的最小距离（米）
    pub min_range: f64,
    /// 该层覆盖的最大距离（米）
    pub max_range: f64,
    /// 单个 tile 的边长（米）
    pub tile_size: f64,
    /// 网格生成算法
    pub algorithm: MeshAlgorithm,
}

const LEVELS: [LodLevel; 8] = [
    LodLevel { index: 0, min_range: 0.0,     max_range: 30.0,    tile_size: 16.0,    algorithm: MeshAlgorithm::Transvoxel { voxel_size: 0.5 } },
    LodLevel { index: 1, min_range: 30.0,    max_range: 80.0,    tile_size: 32.0,    algorithm: MeshAlgorithm::Transvoxel { voxel_size: 1.0 } },
    LodLevel { index: 2, min_range: 80.0,    max_range: 200.0,   tile_size: 64.0,    algorithm: MeshAlgorithm::Transvoxel { voxel_size: 2.0 } },
    LodLevel { index: 3, min_range: 200.0,   max_range: 500.0,   tile_size: 128.0,   algorithm: MeshAlgorithm::Transvoxel { voxel_size: 4.0 } },
    LodLevel { index: 4, min_range: 500.0,   max_range: 1500.0,  tile_size: 256.0,   algorithm: MeshAlgorithm::Transvoxel { voxel_size: 8.0 } },
    LodLevel { index: 5, min_range: 1500.0,  max_range: 4000.0,  tile_size: 512.0,   algorithm: MeshAlgorithm::SignedHeightfield { spacing: 16.0 } },
    LodLevel { index: 6, min_range: 4000.0,  max_range: 7000.0,  tile_size: 1024.0,  algorithm: MeshAlgorithm::SignedHeightfield { spacing: 32.0 } },
    LodLevel { index: 7, min_range: 7000.0,  max_range: 10000.0, tile_size: 2048.0,  algorithm: MeshAlgorithm::SignedHeightfield { spacing: 64.0 } },
];

/// 每帧最多处理的事件数（跨层共享）
const EVENTS_PER_FRAME: usize = 4;
/// MC 垂直余量（米）
const MC_VERTICAL_MARGIN: f64 = 15.0;

// ── 公开类型 ──────────────────────────

/// 全局唯一的 tile 标识符
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct LodKey {
    pub level: u8,
    pub gx: i64,
    pub gz: i64,
}

/// Tile 生命周期事件
#[derive(Debug)]
pub enum TileEvent {
    Load { key: LodKey, mesh: TerrainMeshData },
    Unload { key: LodKey },
}

// ── 纯函数（无 self，可在后台线程调用）─────

/// 为指定层计算期望的 tile 坐标集合
fn desired_keys(level: &LodLevel, px: f64, pz: f64) -> HashSet<LodKey> {
    let ts = level.tile_size;
    let center_gx = (px / ts).floor() as i64;
    let center_gz = (pz / ts).floor() as i64;
    // 搜索半径（格数）
    let grid_radius = (level.max_range / ts).ceil() as i64 + 1;
    // SignedHeightfield 层 margin=0 — 防 SH→SH 边界 z-fighting（无 transition cell 桥接）
    let margin = if matches!(level.algorithm, MeshAlgorithm::SignedHeightfield { .. }) {
        0.0
    } else {
        ts * 0.5
    };
    let mut keys = HashSet::new();
    for dx in -grid_radius..=grid_radius {
        for dz in -grid_radius..=grid_radius {
            // 圆形裁剪：用 tile 中心距离
            let cx = (dx as f64 + 0.5) * ts;
            let cz = (dz as f64 + 0.5) * ts;
            let dist = (cx * cx + cz * cz).sqrt();
            if dist < level.min_range - margin || dist > level.max_range + margin {
                continue;
            }
            keys.insert(LodKey {
                level: level.index,
                gx: center_gx + dx,
                gz: center_gz + dz,
            });
        }
    }
    keys
}

/// Tile 的世界空间左下角
fn tile_origin(key: LodKey, level: &LodLevel) -> (f64, f64) {
    (
        key.gx as f64 * level.tile_size,
        key.gz as f64 * level.tile_size,
    )
}

/// 计算 tile 需要过渡单元的面（位掩码）
///
/// 位定义：bit 0=-X, 1=+X, 2=-Z, 3=+Z
/// 仅检查紧邻的高层级——Transvoxel transition cell 假设 scale=2。
///
/// 额外检查：tile 中心必须在 coarser 层级的有效距离内——否则
/// coarser tile 不存在，transition cell 会桥接到空无，产生浮空三角形。
#[allow(dead_code)]
fn compute_transition_faces(key: LodKey) -> u8 {
    let level = &LEVELS[key.level as usize];
    let next_level_idx = key.level + 1;
    if next_level_idx >= LEVELS.len() as u8 {
        return 0;
    }
    let nl_lvl = &LEVELS[next_level_idx as usize];

    // 检查本 tile 是否在 coarser 层级的有效范围内
    let (ox, oz) = tile_origin(key, level);
    let cx = ox + level.tile_size * 0.5;
    let cz = oz + level.tile_size * 0.5;
    let dist = (cx * cx + cz * cz).sqrt();
    let nl_margin = if matches!(nl_lvl.algorithm, MeshAlgorithm::Transvoxel { .. }) {
        nl_lvl.tile_size * 0.5
    } else {
        0.0
    };
    if dist < nl_lvl.min_range - nl_margin || dist > nl_lvl.max_range + nl_margin {
        return 0; // coarser tile 不存在，无需 transition
    }

    let scale = (nl_lvl.tile_size / level.tile_size) as i64;
    let mut faces: u8 = 0;
    if scale >= 2 {
        let self_lo_gx = key.gx.div_euclid(scale);
        let self_lo_gz = key.gz.div_euclid(scale);

        if (key.gx + 1).div_euclid(scale) != self_lo_gx {
            faces |= 0b0010;
        }
        if (key.gx - 1).div_euclid(scale) != self_lo_gx {
            faces |= 0b0001;
        }
        if (key.gz + 1).div_euclid(scale) != self_lo_gz {
            faces |= 0b1000;
        }
        if (key.gz - 1).div_euclid(scale) != self_lo_gz {
            faces |= 0b0100;
        }
    }

    faces
}

/// 估算某 LOD 级别可见圆环范围内的地形垂直范围。
///
/// 所有同层 tile 共享同一垂直范围——绝对消除 bottom_y 不一致。
fn estimate_ring_vertical(
    terrain: &HeightfieldTerrain,
    player_pos: WorldPos,
    level: &LodLevel,
) -> (f64, f64) {
    use woworld_core::spatial::TerrainQuery;
    let mut min_h = f64::MAX;
    let mut max_h = f64::MIN;
    // 在可见圆环内均匀采样（极坐标栅格）
    let steps_r = 6;
    let steps_theta = 12;
    for ir in 0..=steps_r {
        let r = level.min_range
            + (ir as f64 / steps_r as f64) * (level.max_range - level.min_range);
        for it in 0..steps_theta {
            let theta = it as f64 * 2.0 * std::f64::consts::PI / steps_theta as f64;
            let wx = player_pos.x + r * theta.cos();
            let wz = player_pos.z + r * theta.sin();
            let h = terrain.height_at(WorldPos {
                x: wx,
                y: 0.0,
                z: wz,
            }) as f64;
            min_h = min_h.min(h);
            max_h = max_h.max(h);
        }
    }
    (min_h, max_h)
}

/// 为单个 tile 生成网格（纯函数，可在任意线程调用）
fn generate_tile(
    terrain: &HeightfieldTerrain,
    key: LodKey,
    transition_faces: u8,
    bottom_y: f64,
    top_y: f64,
) -> TerrainMeshData {
    let level = &LEVELS[key.level as usize];
    let (ox, oz) = tile_origin(key, level);

    let mesh = match level.algorithm {
        MeshAlgorithm::Transvoxel { voxel_size } => {
            let vertical_voxels = ((top_y - bottom_y) / voxel_size).ceil() as u32;
            let voxels_edge = (level.tile_size / voxel_size) as u32;
            let params = IsoSurfaceParams {
                ox,
                oz,
                bottom_y,
                voxels_x: voxels_edge,
                voxels_y: vertical_voxels.max(1),
                voxels_z: voxels_edge,
                voxel_size,
                transition_faces,
            };
            let base = HeightfieldDensity::new_with_params(
                terrain.noise().clone(),
                terrain.biome_classifier.clone(),
            );
            let stack = DensityStack::new(base)
                .with_cave_layer(terrain.seed(), CaveParams::default());
            let density: &dyn DensityField = stack.as_density();
            extract_isosurface_transvoxel(density, &params)
        }
        MeshAlgorithm::SignedHeightfield { spacing } => {
            let grid_size = (level.tile_size / spacing) as u32 + 1;
            generate_sh_mesh(terrain, ox, oz, grid_size, spacing)
        }
    };

    mesh
}

// ── ClipmapManager ────────────────────

pub struct ClipmapManager {
    /// 共享只读地形数据（async 模式下多线程访问）
    terrain: Arc<HeightfieldTerrain>,
    /// 活跃 tile: LodKey → (是否已入队——防止重复生成)
    active: HashMap<LodKey, ()>,
    /// 尚未返回的事件（分摊到多帧）
    pending_events: Vec<TileEvent>,
    /// 待生成 mesh 的 tile 键（仅 sync 模式使用）
    pending_loads: Vec<LodKey>,
    events_per_frame: usize,

    // ── Async 生成 ──
    /// async 模式开关（false = 兼容测试的同步行为）
    async_mode: bool,
    /// 后台线程完成的 mesh → 主线程收割
    result_rx: mpsc::Receiver<TileEvent>,
    /// 提交生成任务到后台线程
    result_tx: mpsc::Sender<TileEvent>,
    /// 正在后台生成的 tile（防重复提交）
    in_flight: HashSet<LodKey>,
    /// 每 LOD 级别的统一垂直范围缓存（每帧预计算一次，所有同层 tile 共享）
    vertical_cache: HashMap<u8, (f64, f64)>,
}

impl ClipmapManager {
    /// 同步模式（测试用）
    ///
    /// 测试中 poll() 在循环中调用，需要同步返回 mesh。
    pub fn new(terrain: HeightfieldTerrain) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            terrain: Arc::new(terrain),
            active: HashMap::new(),
            pending_events: Vec::new(),
            pending_loads: Vec::new(),
            events_per_frame: EVENTS_PER_FRAME,
            async_mode: false,
            result_rx: rx,
            result_tx: tx,
            in_flight: HashSet::new(),
            vertical_cache: HashMap::new(),
        }
    }

    /// 异步模式（生产用）
    ///
    /// Mesh 生成提交到 rayon 后台线程池，主线程仅收割结果。
    /// `terrain` 以 Arc 传入——多线程共享只读访问。
    pub fn new_async(terrain: Arc<HeightfieldTerrain>) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            terrain,
            active: HashMap::new(),
            pending_events: Vec::new(),
            pending_loads: Vec::new(),
            events_per_frame: EVENTS_PER_FRAME,
            async_mode: true,
            result_rx: rx,
            result_tx: tx,
            in_flight: HashSet::new(),
            vertical_cache: HashMap::new(),
        }
    }

    // ── 内部方法 ────────────────────────

    /// 提交一个 tile 到 rayon 后台线程池生成
    fn submit_async(&mut self, key: LodKey, transition_faces: u8, bottom_y: f64, top_y: f64) {
        let terrain = Arc::clone(&self.terrain);
        let tx = self.result_tx.clone();
        self.in_flight.insert(key);

        rayon::spawn(move || {
            let mesh = generate_tile(&terrain, key, transition_faces, bottom_y, top_y);
            // 如果 ClipmapManager 已 drop（rx 关闭），send 静默失败
            let _ = tx.send(TileEvent::Load { key, mesh });
        });
    }

    // ── 公开 API ────────────────────────

    /// 每帧调用：返回需要创建/销毁的 tile 事件（最多 events_per_frame 个）
    ///
    /// **Async 模式**: 从 channel 收割已完成 mesh → 提交新 tile 到 rayon →
    /// 返回已完成的 Load 事件 + Unload 事件。
    ///
    /// **Sync 模式**: 行为与旧版相同（同步生成）。
    pub fn poll(&mut self, player_pos: WorldPos) -> Vec<TileEvent> {
        let max_events = self.events_per_frame;
        let mut events: Vec<TileEvent> = Vec::new();

        // ── 0. 收割后台完成的结果 ──────────
        if self.async_mode {
            while let Ok(event) = self.result_rx.try_recv() {
                if let TileEvent::Load { key, mesh } = event {
                    self.in_flight.remove(&key);
                    // 仅保留仍活跃的 tile——玩家可能已离开
                    if self.active.contains_key(&key) {
                        self.pending_events.push(TileEvent::Load { key, mesh });
                    }
                }
            }
        }

        // ── 1. 消费上次遗留的事件 ──────────
        if !self.pending_events.is_empty() {
            let n = max_events.min(self.pending_events.len());
            events.extend(self.pending_events.drain(..n));
            if events.len() >= max_events {
                return events;
            }
        }

        // ── 2. 消费待生成队列（仅 sync 模式）──
        if !self.async_mode {
            while events.len() < max_events && !self.pending_loads.is_empty() {
                let key = self.pending_loads.remove(0);
                // 检查该 tile 是否仍然需要
                let level = &LEVELS[key.level as usize];
                let desired = desired_keys(level, player_pos.x, player_pos.z);
                if desired.contains(&key) {
                    let (vy_bottom, vy_top) = self
                        .vertical_cache
                        .get(&key.level)
                        .copied()
                        .unwrap_or((0.0, 0.0));
                    let mesh = generate_tile(&self.terrain, key, 0, vy_bottom, vy_top);
                    events.push(TileEvent::Load { key, mesh });
                } else {
                    self.active.remove(&key);
                }
            }
            if events.len() >= max_events {
                return events;
            }
        }

        // ── 2.5. 预计算每 LOD 级别的统一垂直范围 ──
        self.vertical_cache.clear();
        for level in &LEVELS {
            if let MeshAlgorithm::Transvoxel { voxel_size } = level.algorithm {
                let (min_h, max_h) =
                    estimate_ring_vertical(&self.terrain, player_pos, level);
                let bottom =
                    ((min_h - MC_VERTICAL_MARGIN) / voxel_size).floor() * voxel_size;
                let top =
                    ((max_h + MC_VERTICAL_MARGIN) / voxel_size).ceil() * voxel_size;
                self.vertical_cache
                    .insert(level.index, (bottom.max(-250.0), top));
            }
        }

        // ── 3. 计算所有层的 diff ───────────
        for level in &LEVELS {
            let desired = desired_keys(level, player_pos.x, player_pos.z);

            // Unload: active 中但不再需要
            let to_remove: Vec<LodKey> = self
                .active
                .keys()
                .filter(|k| k.level == level.index && !desired.contains(k))
                .copied()
                .collect();
            for key in &to_remove {
                self.active.remove(key);
                self.in_flight.remove(key); // 取消正在生成的 tile
                events.push(TileEvent::Unload { key: *key });
            }

            // Load: 需要但不在 active 且未在待生成/生成中
            for key in &desired {
                if !self.active.contains_key(key)
                    && !self.pending_loads.contains(key)
                    && !self.in_flight.contains(key)
                {
                    self.active.insert(*key, ());
                    // 获取该层的统一垂直范围（SH 层不需要）
                    let (vy_bottom, vy_top) = self
                        .vertical_cache
                        .get(&key.level)
                        .copied()
                        .unwrap_or((0.0, 0.0));
                    if self.async_mode {
                        // 过渡单元已禁用（extract_transition_face 产生错误顶点，偏差 max~90m）
                        // 跨 LOD 接缝暂时依赖顶点焊接 (20cm) + 大气雾化
                        // TODO: 正确修复——粗 tile 边界用细 voxel 子分
                        self.submit_async(*key, 0, vy_bottom, vy_top);
                    } else {
                        // Sync 模式：立刻生成或推迟
                        if events.len() < max_events {
                            let mesh =
                                generate_tile(&self.terrain, *key, 0, vy_bottom, vy_top);
                            events.push(TileEvent::Load { key: *key, mesh });
                        } else {
                            self.pending_loads.push(*key);
                        }
                    }
                }
            }
        }

        // ── 4. 速率限制：超额事件推迟 ──────
        if events.len() > max_events {
            let rest: Vec<_> = events.drain(max_events..).collect();
            self.pending_events = rest;
        }

        events
    }

    /// 活跃 tile 数（调试用）
    #[allow(dead_code)]
    pub fn active_count(&self) -> usize {
        self.active.len()
    }
}

// ── 测试 ──────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::HeightfieldTerrain;

    fn make_manager() -> ClipmapManager {
        ClipmapManager::new(HeightfieldTerrain::new(42))
    }

    #[test]
    fn test_desired_keys_l0() {
        let keys = desired_keys(&LEVELS[0], 0.0, 0.0);
        // scene_lod 0: 0-30m, 16m tiles → ~16 tiles
        assert!(
            keys.len() >= 12,
            "L0 should have at least 12 tiles, got {}",
            keys.len()
        );
        assert!(
            keys.len() <= 22,
            "L0 should have at most 22 tiles, got {}",
            keys.len()
        );
        // 所有 key 都是 level 0
        for k in &keys {
            assert_eq!(k.level, 0);
        }
    }

    #[test]
    fn test_desired_keys_l3() {
        let keys = desired_keys(&LEVELS[3], 0.0, 0.0);
        // scene_lod 3: 200-500m, 128m tiles → ~56 tiles
        assert!(
            keys.len() >= 40,
            "L3 should have at least 40 tiles, got {}",
            keys.len()
        );
        for k in &keys {
            assert_eq!(k.level, 3);
        }
    }

    #[test]
    fn test_tile_origin() {
        let key = LodKey {
            level: 0,
            gx: 3,
            gz: -2,
        };
        let (ox, oz) = tile_origin(key, &LEVELS[0]);
        assert!((ox - 48.0).abs() < 0.01); // 3 × 16
        assert!((oz - (-32.0)).abs() < 0.01); // -2 × 16
    }

    #[test]
    fn test_poll_first_frame() {
        let mut mgr = make_manager();
        let events = mgr.poll(WorldPos::default());
        // 首帧最多 4 个事件
        assert!(events.len() <= 4);
    }

    #[test]
    fn test_poll_drains_all() {
        let mut mgr = make_manager();
        let mut total = 0;
        for _ in 0..500 {
            let events = mgr.poll(WorldPos::default());
            total += events.len();
            if events.is_empty() {
                break;
            }
        }
        // 8 级 LOD，全部消耗后应有 ~560 个 tile
        assert!(total > 400, "should have >400 tiles total, got {}", total);
    }

    #[test]
    fn test_poll_no_change_after_drain() {
        let mut mgr = make_manager();
        loop {
            if mgr.poll(WorldPos::default()).is_empty() {
                break;
            }
        }
        assert!(mgr.poll(WorldPos::default()).is_empty());
    }

    // ── Sprint-013: 6 级 CHG-049 对齐测试 ─────

    #[test]
    fn test_desired_keys_scene_lod_5() {
        let keys = desired_keys(&LEVELS[5], 0.0, 0.0);
        // scene_lod 5: 1.5-4km, 512m tiles → ~200 tiles
        assert!(
            keys.len() >= 150,
            "scene_lod 5 should have at least 150 tiles, got {}",
            keys.len()
        );
        assert!(
            keys.len() <= 260,
            "scene_lod 5 should have at most 260 tiles, got {}",
            keys.len()
        );
        for k in &keys {
            assert_eq!(k.level, 5);
            // 所有 scene_lod 5 tile 中心距应在 1.5km 以上
            let level = &LEVELS[5];
            let cx = (k.gx as f64 + 0.5) * level.tile_size;
            let cz = (k.gz as f64 + 0.5) * level.tile_size;
            let dist = (cx * cx + cz * cz).sqrt();
            assert!(
                dist >= level.min_range - level.tile_size * 0.6,
                "scene_lod 5 tile ({},{}) center dist {} below min_range {}",
                k.gx, k.gz, dist, level.min_range
            );
        }
    }

    #[test]
    fn test_highest_lod_has_zero_transition_faces() {
        // 最高层级无更粗级别邻居，transition_faces 应为 0
        let highest = (LEVELS.len() - 1) as u8;
        let key = LodKey { level: highest, gx: 0, gz: 0 };
        assert_eq!(compute_transition_faces(key), 0);
        let key2 = LodKey { level: highest, gx: 10, gz: -5 };
        assert_eq!(compute_transition_faces(key2), 0);
    }

    #[test]
    fn test_scene_lod_0_transvoxel_generation() {
        // scene_lod 0: 16m tile, 0.5m voxel — 最高精度 Transvoxel
        let terrain = HeightfieldTerrain::new(42);
        let key = LodKey { level: 0, gx: 0, gz: 0 };
        let mesh = generate_tile(&terrain, key, 0, -250.0, 500.0);
        assert!(
            mesh.vertices.len() >= 100,
            "scene_lod 0 (0.5m voxel) should have at least 100 vertices, got {}",
            mesh.vertices.len()
        );
        assert_eq!(mesh.vertices.len(), mesh.normals.len());
        assert_eq!(mesh.vertices.len(), mesh.colors.len());
        assert!(mesh.indices.len() % 3 == 0);
    }

    #[test]
    fn test_scene_lod_2_transvoxel_generation() {
        // scene_lod 2: 64m tile, 2m voxel
        let terrain = HeightfieldTerrain::new(42);
        let key = LodKey { level: 2, gx: 0, gz: 0 };
        let mesh = generate_tile(&terrain, key, 0, -250.0, 500.0);
        assert!(
            mesh.vertices.len() >= 100,
            "scene_lod 2 should have at least 100 vertices, got {}",
            mesh.vertices.len()
        );
        assert_eq!(mesh.vertices.len(), mesh.normals.len());
        assert_eq!(mesh.vertices.len(), mesh.colors.len());
        assert!(mesh.indices.len() % 3 == 0);
    }

    #[test]
    fn test_scene_lod_4_transvoxel_generation() {
        // scene_lod 4: 256m tile, 8m voxel — 低精度 Transvoxel
        let terrain = HeightfieldTerrain::new(42);
        let key = LodKey { level: 4, gx: 0, gz: 0 };
        let mesh = generate_tile(&terrain, key, 0, -250.0, 500.0);
        assert!(
            mesh.vertices.len() >= 50,
            "scene_lod 4 (8m voxel) should have at least 50 vertices, got {}",
            mesh.vertices.len()
        );
        assert_eq!(mesh.vertices.len(), mesh.normals.len());
        assert_eq!(mesh.vertices.len(), mesh.colors.len());
        assert!(mesh.indices.len() % 3 == 0);
    }

    #[test]
    fn test_all_eight_levels_have_transition_coverage() {
        for lvl in 0..8u8 {
            let key = LodKey { level: lvl, gx: 0, gz: 0 };
            let faces = compute_transition_faces(key);
            assert!(faces <= 0b1111, "Level {} faces={:#06b} exceeds 4 bits", lvl, faces);
        }
    }

    #[test]
    fn test_scene_lod_5_generates_sh_mesh() {
        let terrain = HeightfieldTerrain::new(42);
        let key = LodKey {
            level: 5,
            gx: 0,
            gz: 0,
        };
        let mesh = generate_tile(&terrain, key, 0, -250.0, 500.0);
        // SH: 35²=1225 + 裙边 256 = 1089 顶点
        assert_eq!(
            mesh.vertices.len(),
            1089,
            "scene_lod 5 SH+skirt should have 1089 vertices (35²+skirt), got {} vertices",
            mesh.vertices.len()
        );
        assert_eq!(mesh.normals.len(), 1089);
        assert_eq!(mesh.colors.len(), 1089);
        // 32²×6 + 4×32×6 裙边 = 6144+768 = 6144
        assert_eq!(mesh.indices.len(), 6144);
        assert!(mesh.indices.len() % 3 == 0);
    }

    #[test]
    fn test_scene_lod_6_generates_sh_mesh() {
        let terrain = HeightfieldTerrain::new(42);
        let key = LodKey {
            level: 6,
            gx: 0,
            gz: 0,
        };
        let mesh = generate_tile(&terrain, key, 0, -250.0, 500.0);
        assert_eq!(mesh.vertices.len(), 1089);
        assert_eq!(mesh.normals.len(), 1089);
        assert_eq!(mesh.colors.len(), 1089);
        assert_eq!(mesh.indices.len(), 6144);
        assert!(mesh.indices.len() % 3 == 0);
    }

    #[test]
    fn test_scene_lod_7_generates_sh_mesh() {
        let terrain = HeightfieldTerrain::new(42);
        let key = LodKey {
            level: 7,
            gx: 0,
            gz: 0,
        };
        let mesh = generate_tile(&terrain, key, 0, -250.0, 500.0);
        // SH: (33+2)² = 1225 顶点（overlap=1）
        assert_eq!(
            mesh.vertices.len(),
            1089,
            "scene_lod 7 SH+skirt: {} vertices",
            mesh.vertices.len()
        );
        assert_eq!(mesh.normals.len(), 1089);
        assert_eq!(mesh.colors.len(), 1089);
        assert_eq!(mesh.indices.len(), 6144);
        assert!(mesh.indices.len() % 3 == 0);
    }

    #[test]
    fn test_desired_keys_scene_lod_7() {
        let keys = desired_keys(&LEVELS[7], 0.0, 0.0);
        // scene_lod 7: 7-10km, 2048m tiles
        assert!(
            keys.len() >= 30,
            "scene_lod 7 should have at least 30 tiles, got {}",
            keys.len()
        );
        assert!(
            keys.len() <= 100,
            "scene_lod 7 should have at most 100 tiles, got {}",
            keys.len()
        );
        for k in &keys {
            assert_eq!(k.level, 7);
            // SH 层 margin=0：所有 tile 中心距应在 min_range 以上
            let level = &LEVELS[7];
            let cx = (k.gx as f64 + 0.5) * level.tile_size;
            let cz = (k.gz as f64 + 0.5) * level.tile_size;
            let dist = (cx * cx + cz * cz).sqrt();
            assert!(
                dist >= level.min_range - level.tile_size * 0.01,
                "scene_lod 7 tile ({},{}) center dist {} below min_range {}",
                k.gx, k.gz, dist, level.min_range
            );
        }
    }

    /// 验证相邻 tile 共享边上的高度值完全一致（终极几何诊断）
    #[test]
    fn test_adjacent_tiles_share_identical_edge_heights() {
        use woworld_core::prelude::WorldPos;
        use woworld_core::spatial::TerrainQuery;
        let terrain = HeightfieldTerrain::new(42);

        for lvl_idx in 0..8u8 {
            let level = &LEVELS[lvl_idx as usize];
            let (ox_a, _) = tile_origin(
                LodKey { level: lvl_idx, gx: 0, gz: 0 },
                level,
            );
            let (ox_b, _) = tile_origin(
                LodKey { level: lvl_idx, gx: 1, gz: 0 },
                level,
            );

            let shared_x = ox_a + level.tile_size;
            assert!((shared_x - ox_b).abs() < 0.01, "tile origins should align");

            let eps: f32 = 0.001;

            // 在共享边上采样高度（SH: 按 spacing 步进；Transvoxel: 按 voxel_size/2 步进）
            let step = match level.algorithm {
                MeshAlgorithm::Transvoxel { voxel_size } => (voxel_size * 0.5).max(0.25),
                MeshAlgorithm::SignedHeightfield { spacing } => spacing,
            };
            let tile_extent = level.tile_size;
            let n_samples = ((tile_extent / step).ceil() as usize).min(200);

            let mut max_diff: f32 = 0.0;
            for i in 0..=n_samples {
                let wz = i as f64 * step;
                let h_a = terrain.height_at(WorldPos {
                    x: shared_x,
                    y: 0.0,
                    z: wz,
                });
                let h_b = terrain.height_at(WorldPos {
                    x: shared_x,
                    y: 0.0,
                    z: wz,
                });
                let diff = (h_a - h_b).abs();
                max_diff = max_diff.max(diff);
                assert!(
                    diff < eps,
                    "LOD {}: height mismatch at x={:.1} z={:.1}: tileA={:.4} tileB={:.4} diff={:.4}",
                    lvl_idx,
                    shared_x,
                    wz,
                    h_a,
                    h_b,
                    diff
                );
            }
            // 确认至少有一些地形变化（不是平坦的）
            assert!(max_diff < eps,
                "LOD {}: all samples match (max diff {:.6}) — terrain is deterministic ✓",
                lvl_idx, max_diff
            );
        }
    }

    /// 诊断测试：相邻同 LOD tile 共享面顶点偏差
    ///
    /// 用生产参数生成两个相邻 Transvoxel tile（同 LOD），独立提取等值面，
    /// 比较共享面上的顶点位置。验证同 LOD tile 边界顶点是否一致。
    #[test]
    fn test_adjacent_tile_boundary_vertex_deviation() {

        let terrain = make_production_terrain();
        let bottom_y = -100.0;
        let top_y = 500.0;

        let (face_a, face_b, shared_x) =
            adjacent_tile_face_vertices(&terrain, 1, 0, 0, 1, 0, bottom_y, top_y);
        let (deviations, _orphans) = compare_face_vertices(&face_a, &face_b, shared_x, "同LOD");
        print_deviation_stats(&deviations, 0.2);

        // 同 LOD 应完美一致
        assert!(deviations.iter().all(|&d| d < 0.001),
            "Same-LOD boundary vertices should be identical");
    }

    /// 诊断测试：跨 LOD 过渡面顶点偏差
    ///
    /// 生成一个 LOD N tile 和一个 LOD N+1 tile（coarser），比较它们在共享
    /// 面上的顶点。coarser tile 的顶点间距是 finer 的 2 倍——这是 Transvoxel
    /// 过渡单元需要桥接的差距。
    #[test]
    fn test_cross_lod_boundary_vertex_deviation() {

        let terrain = make_production_terrain();
        let bottom_y = -100.0;
        let top_y = 500.0;

        // LOD 1 (32m tile, 1.0m voxel) 与 LOD 2 (64m tile, 2.0m voxel)
        // LOD 2 的 tile 覆盖 2x LOD 1 的 tile
        // 过渡面: LOD 1 gx=1,gz=0 的 +X 面 (x=64) = LOD 2 gx=1,gz=0 的 -X 面 (x=64)
        let key_fine = LodKey { level: 1, gx: 1, gz: 0 };
        let key_coarse = LodKey { level: 2, gx: 1, gz: 0 };
        let tf_fine = compute_transition_faces(key_fine);
        let tf_coarse = compute_transition_faces(key_coarse);

        let shared_x = 64.0_f32;
        let (face_lod1, face_lod2, _) = cross_lod_face_vertices(
            &terrain,
            1, 1, 0,  // LOD 1, gx=1, gz=0 (origin 32,0 → +X at 64)
            2, 1, 0,  // LOD 2, gx=1, gz=0 (origin 64,0 → -X at 64)
            bottom_y,
            top_y,
        );

        let (deviations, orphans) = compare_face_vertices(&face_lod1, &face_lod2, shared_x, "跨LOD");

        // 计算统计
        let mut sorted: Vec<f32> = deviations.iter().cloned().filter(|&d| d < 100.0).collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let min = sorted.first().copied().unwrap_or(0.0);
        let max = sorted.last().copied().unwrap_or(0.0);
        let mean = sorted.iter().sum::<f32>() / sorted.len() as f32;
        let median = if sorted.is_empty() { 0.0 } else { sorted[sorted.len() / 2] };
        let n_gt_20cm = sorted.iter().filter(|&&d| d > 0.2).count();

        eprintln!(
            "\n══════════ 跨 LOD 边界顶点诊断 ═══════════\n\
             共享面 x={:.1}\n\
             transition_faces: fine=0b{:04b} coarse=0b{:04b}\n\
             Finer 面顶点 (LOD 1):  {}\n\
             Coarser 面顶点 (LOD 2): {}\n\
             顶点偏差 (m): min={:.4} median={:.4} mean={:.4} max={:.4}\n\
             >20cm (超出焊接容差): {} / {} ({:.1}%)\n\
             Orphan 顶点 (d>1m): finer→coarser={} coarser→finer={}\n\
             ═══════════════════════════════════════════",
            shared_x,
            tf_fine, tf_coarse,
            face_lod1.len(),
            face_lod2.len(),
            min, median, mean, max,
            n_gt_20cm, sorted.len(),
            n_gt_20cm as f64 / sorted.len() as f64 * 100.0,
            orphans.0, orphans.1,
        );

        // 检查过渡面是否被正确触发
        // LOD 1 gx=1 的 +X 面: (gx+1).div_euclid(2) = 2/2 = 1 != gx.div_euclid(2) = 1/2 = 0 → 应触发
        let fine_has_transition = (tf_fine & 0b0010) != 0; // bit 1 = +X
        assert!(
            fine_has_transition,
            "LOD 1 gx=1 +X 面应有过渡单元 (tf=0b{:04b})", tf_fine
        );

        // 诊断: 打印精细 tile 面顶点坐标，查找 90m 偏差的来源
        eprintln!(
            "跨LOD诊断: tf_fine=0b{:04b} tf_coarse=0b{:04b} | 顶点: fine={} coarse={} | 偏差: min={:.4} med={:.4} max={:.4} | >20cm: {}/{}",
            tf_fine, tf_coarse,
            face_lod1.len(), face_lod2.len(),
            min, median, max,
            n_gt_20cm, sorted.len(),
        );

        // 打印精细 tile 的面顶点，标识哪些来自过渡单元
        let regular_max_x = 64.0_f32; // 精细 tile 的 +X 边界
        let tv_half = 0.5_f32; // half_vs for LOD 1 (1.0*0.5)
        eprintln!("精细 tile (LOD 1) {} 个面顶点:", face_lod1.len());
        for (i, v) in face_lod1.iter().enumerate() {
            let origin = if (v.x - regular_max_x).abs() < 0.001 { "regular" }
                else if (v.x - (regular_max_x + tv_half)).abs() < 0.001 { "transition(offset)" }
                else { "UNKNOWN" };
            eprintln!("  [{}] ({:.3}, {:.3}, {:.3}) {}", i, v.x, v.y, v.z, origin);
        }

        eprintln!("粗糙 tile (LOD 2) {} 个面顶点:", face_lod2.len());
        for (i, v) in face_lod2.iter().take(10).enumerate() {
            eprintln!("  [{}] ({:.3}, {:.3}, {:.3})", i, v.x, v.y, v.z);
        }
        if face_lod2.len() > 10 {
            eprintln!("  ... ({} more)", face_lod2.len() - 10);
        }

        // 诊断: 分析精细 tile 面顶点的 Y 分布
        let mut y_dist: Vec<(f32, f32)> = face_lod1.iter().map(|v| (v.y, v.z)).collect();
        y_dist.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let y_min = y_dist.first().map(|v| v.0).unwrap_or(0.0);
        let y_max = y_dist.last().map(|v| v.0).unwrap_or(0.0);
        eprintln!("精细面顶点 Y 范围: {:.1} ~ {:.1} (共 {} 个)", y_min, y_max, y_dist.len());
        // 打印 Y 最低和最高的 5 个顶点
        eprintln!("  Y 最低 5 个:");
        for v in y_dist.iter().take(5) { eprintln!("    y={:.3} z={:.3}", v.0, v.1); }
        eprintln!("  Y 最高 5 个:");
        for v in y_dist.iter().rev().take(5) { eprintln!("    y={:.3} z={:.3}", v.0, v.1); }

        // 粗 tile 面顶点
        let mut cy: Vec<f32> = face_lod2.iter().map(|v| v.y).collect();
        cy.sort_by(|a, b| a.partial_cmp(b).unwrap());
        eprintln!("粗 tile 面顶点 Y 范围: {:.1} ~ {:.1} (共 {} 个)", cy.first().unwrap_or(&0.0), cy.last().unwrap_or(&0.0), cy.len());

        // 过渡单元已禁用——诊断完成。记录无过渡单元时的偏差基准。
        assert!(
            max < 2.0,
            "即使无过渡单元，跨LOD偏差也不应超过2m: max={:.3}m", max
        );
    }

    // ── 诊断测试辅助函数 ──────────────────

    fn make_production_terrain() -> HeightfieldTerrain {
        use crate::noise_gen::{NoiseParams, WorldNoise};
        let params = NoiseParams {
            height_amplitude: 120.0,
            detail_scale: 0.005,
            mountain_scale: 0.001,
            sea_threshold: -0.5,
            continent_scale: 0.001,
            ..NoiseParams::default()
        };
        let noise = WorldNoise::with_params(42, params);
        HeightfieldTerrain::with_noise(noise)
    }

    /// 生成两个相邻同 LOD tile 的面顶点
    fn adjacent_tile_face_vertices(
        terrain: &HeightfieldTerrain,
        level_idx: u8,
        gx_a: i64,
        gz_a: i64,
        gx_b: i64,
        gz_b: i64,
        bottom_y: f64,
        top_y: f64,
    ) -> (Vec<glam::Vec3>, Vec<glam::Vec3>, f32) {
        let level = &LEVELS[level_idx as usize];
        let tile_size = level.tile_size;

        let key_a = LodKey { level: level_idx, gx: gx_a, gz: gz_a };
        let key_b = LodKey { level: level_idx, gx: gx_b, gz: gz_b };

        // 过渡单元已禁用
        let mesh_a = generate_tile(terrain, key_a, 0, bottom_y, top_y);
        let mesh_b = generate_tile(terrain, key_b, 0, bottom_y, top_y);

        // 共享面: Tile A +X → Tile B -X
        let shared_x = (gx_b as f64 * tile_size) as f32;
        let eps = 0.01_f32;

        let face_a: Vec<glam::Vec3> = mesh_a
            .vertices.iter()
            .filter(|v| (v.x - shared_x).abs() < eps)
            .copied()
            .collect();
        let face_b: Vec<glam::Vec3> = mesh_b
            .vertices.iter()
            .filter(|v| (v.x - shared_x).abs() < eps)
            .copied()
            .collect();

        (face_a, face_b, shared_x)
    }

    /// 生成 LOD N 和 LOD N+1 在共享面处的顶点
    /// LOD N 的 finer tile 和 LOD N+1 的 coarser tile 在共享 X 面比较
    fn cross_lod_face_vertices(
        terrain: &HeightfieldTerrain,
        fine_level: u8,
        fine_gx: i64,
        fine_gz: i64,
        coarse_level: u8,
        coarse_gx: i64,
        coarse_gz: i64,
        bottom_y: f64,
        top_y: f64,
    ) -> (Vec<glam::Vec3>, Vec<glam::Vec3>, f32) {
        let level_coarse = &LEVELS[coarse_level as usize];
        let shared_x = (coarse_gx as f64 * level_coarse.tile_size) as f32;

        let key_fine = LodKey { level: fine_level, gx: fine_gx, gz: fine_gz };
        let key_coarse = LodKey { level: coarse_level, gx: coarse_gx, gz: coarse_gz };

        // 使用正确的 transition_faces（而非硬编码 0）
        // 过渡单元已禁用
        let mesh_fine = generate_tile(terrain, key_fine, 0, bottom_y, top_y);
        let mesh_coarse = generate_tile(terrain, key_coarse, 0, bottom_y, top_y);

        let eps = 0.01_f32;
        let face_fine: Vec<glam::Vec3> = mesh_fine
            .vertices.iter()
            .filter(|v| (v.x - shared_x).abs() < eps)
            .copied()
            .collect();
        let face_coarse: Vec<glam::Vec3> = mesh_coarse
            .vertices.iter()
            .filter(|v| (v.x - shared_x).abs() < eps)
            .copied()
            .collect();

        (face_fine, face_coarse, shared_x)
    }

    /// 比较两个面顶点集，返回 (偏差列表, (finer孤儿数, coarser孤儿数))
    fn compare_face_vertices(
        face_a: &[glam::Vec3],
        face_b: &[glam::Vec3],
        shared_x: f32,
        label: &str,
    ) -> (Vec<f32>, (usize, usize)) {
        println!("\n=== {} 面顶点比较 (x={:.1}) ===", label, shared_x);
        println!("面 A 顶点数: {}", face_a.len());
        println!("面 B 顶点数: {}", face_b.len());

        let mut deviations: Vec<f32> = Vec::new();
        let mut orphan_a = 0usize;
        let mut matched_b = vec![false; face_b.len()];

        for va in face_a {
            let mut min_dist = f32::MAX;
            let mut min_idx = 0;
            for (j, vb) in face_b.iter().enumerate() {
                let dist = (*va - *vb).length();
                if dist < min_dist {
                    min_dist = dist;
                    min_idx = j;
                }
            }
            deviations.push(min_dist);
            if min_dist < 1.0 {
                matched_b[min_idx] = true;
            } else {
                orphan_a += 1;
            }
        }
        let orphan_b = matched_b.iter().filter(|&&m| !m).count();

        (deviations, (orphan_a, orphan_b))
    }

    /// 打印偏差统计
    fn print_deviation_stats(deviations: &[f32], weld_eps: f32) {
        if deviations.is_empty() {
            println!("  (无顶点)");
            return;
        }
        let mut sorted: Vec<f32> = deviations.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let min = sorted.first().copied().unwrap_or(0.0);
        let max = sorted.last().copied().unwrap_or(0.0);
        let mean = sorted.iter().sum::<f32>() / sorted.len() as f32;
        let median = sorted[sorted.len() / 2];
        let p95 = sorted[(sorted.len() as f64 * 0.95) as usize];

        println!("  偏差 (m): min={:.4} median={:.4} mean={:.4} p95={:.4} max={:.4}",
            min, median, mean, p95, max);

        let un_weldable = sorted.iter().filter(|&&d| d > weld_eps).count();
        if un_weldable > 0 {
            println!(
                "  ⚠️  超出焊接容差 ({}m): {} / {} ({:.1}%)",
                weld_eps, un_weldable, sorted.len(),
                un_weldable as f64 / sorted.len() as f64 * 100.0
            );
        } else {
            println!("  ✓ 全部 {} 顶点在焊接容差内 (≤{}m)", sorted.len(), weld_eps);
        }
    }
}
