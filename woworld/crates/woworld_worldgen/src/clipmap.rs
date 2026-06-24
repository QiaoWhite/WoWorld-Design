//! ClipmapManager — 多分辨率 LOD 地形管理
//!
//! 4 级 LOD: L0(近·MC体素) → L1 → L2 → L3(远·高度场)。
//! 所有 tile 保持 ~33² 顶点——GPU 负载恒定。
//!
//! 接口兼容现有 WorldDriver（TileEvent ≈ ChunkEvent）。

use std::collections::{HashMap, HashSet};

use woworld_core::prelude::WorldPos;

use crate::marching_cubes::{extract_isosurface, IsoSurfaceParams};
use crate::terrain::HeightfieldTerrain;
use crate::terrain_mesh::{generate_terrain_mesh, TerrainMeshData};

// ── LOD 层级定义 ──────────────────────

/// 一个 LOD 层级的配置
#[derive(Clone, Debug)]
struct LodLevel {
    /// 层级索引 (0=最近)
    index: u8,
    /// 该层覆盖的最小距离（米）
    min_range: f64,
    /// 该层覆盖的最大距离（米）
    max_range: f64,
    /// 单个 tile 的边长（米）
    tile_size: f64,
    /// 网格采样间距（米）——顶点间距
    spacing: f64,
    /// 是否使用 MC 等值面提取
    use_mc: bool,
    /// MC 体素尺寸（仅 use_mc=true 时有效）
    mc_voxel_size: f64,
}

const LEVELS: [LodLevel; 4] = [
    LodLevel { index: 0, min_range: 0.0,   max_range: 128.0, tile_size: 32.0,  spacing: 0.5,  use_mc: true,  mc_voxel_size: 1.0 },
    LodLevel { index: 1, min_range: 128.0, max_range: 256.0, tile_size: 64.0,  spacing: 2.0,  use_mc: false, mc_voxel_size: 0.0 },
    LodLevel { index: 2, min_range: 256.0, max_range: 512.0, tile_size: 128.0, spacing: 4.0,  use_mc: false, mc_voxel_size: 0.0 },
    LodLevel { index: 3, min_range: 512.0, max_range: 1024.0,tile_size: 256.0, spacing: 8.0,  use_mc: false, mc_voxel_size: 0.0 },
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

/// Tile 生命周期事件（与 ChunkEvent 兼容）
#[derive(Debug)]
pub enum TileEvent {
    Load { key: LodKey, mesh: TerrainMeshData },
    Unload { key: LodKey },
}

// ── ClipmapManager ────────────────────

pub struct ClipmapManager {
    terrain: HeightfieldTerrain,
    /// 活跃 tile: LodKey → (是否已入队——防止重复生成)
    active: HashMap<LodKey, ()>,
    /// 尚未返回的事件（分摊到多帧）
    pending_events: Vec<TileEvent>,
    events_per_frame: usize,
}

impl ClipmapManager {
    pub fn new(terrain: HeightfieldTerrain) -> Self {
        Self {
            terrain,
            active: HashMap::new(),
            pending_events: Vec::new(),
            events_per_frame: EVENTS_PER_FRAME,
        }
    }

    // ── 内部方法 ────────────────────────

    /// 为指定层计算期望的 tile 坐标集合
    fn desired_keys(level: &LodLevel, px: f64, pz: f64) -> HashSet<LodKey> {
        let ts = level.tile_size;
        let center_gx = (px / ts).floor() as i64;
        let center_gz = (pz / ts).floor() as i64;
        // 搜索半径（格数）
        let grid_radius = (level.max_range / ts).ceil() as i64 + 1;
        let mut keys = HashSet::new();
        for dx in -grid_radius..=grid_radius {
            for dz in -grid_radius..=grid_radius {
                // 圆形裁剪：用 tile 中心距离
                let cx = (dx as f64 + 0.5) * ts;
                let cz = (dz as f64 + 0.5) * ts;
                let dist = (cx * cx + cz * cz).sqrt();
                if dist < level.min_range - ts * 0.5 || dist > level.max_range + ts * 0.5 {
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
        (key.gx as f64 * level.tile_size, key.gz as f64 * level.tile_size)
    }

    /// 估计 tile 的垂直范围（用于 MC）
    fn estimate_vertical(&self, ox: f64, oz: f64, size: f64) -> (f64, f64) {
        use woworld_core::spatial::TerrainQuery;
        let mut min_h = f64::MAX;
        let mut max_h = f64::MIN;
        for i in 0..=4 {
            let wx = ox + i as f64 * size / 4.0;
            for j in 0..=4 {
                let wz = oz + j as f64 * size / 4.0;
                let h = self.terrain.height_at(WorldPos { x: wx, y: 0.0, z: wz }) as f64;
                min_h = min_h.min(h);
                max_h = max_h.max(h);
            }
        }
        ((min_h - MC_VERTICAL_MARGIN).max(-250.0), max_h + MC_VERTICAL_MARGIN)
    }

    /// 为单个 tile 生成网格
    fn generate_tile(&self, key: LodKey) -> TerrainMeshData {
        let level = &LEVELS[key.level as usize];
        let (ox, oz) = Self::tile_origin(key, level);

        if level.use_mc {
            let (bottom_y, top_y) = self.estimate_vertical(ox, oz, level.tile_size);
            let vertical_voxels = ((top_y - bottom_y) / level.mc_voxel_size).ceil() as u32;
            let voxels_edge = (level.tile_size / level.mc_voxel_size) as u32;
            let params = IsoSurfaceParams {
                ox, oz, bottom_y,
                voxels_x: voxels_edge,
                voxels_y: vertical_voxels.max(1),
                voxels_z: voxels_edge,
                voxel_size: level.mc_voxel_size,
            };
            extract_isosurface(&self.terrain, &params)
        } else {
            let verts = (level.tile_size / level.spacing) as u32 + 1;
            generate_terrain_mesh(&self.terrain, ox, oz, verts, level.spacing)
        }
    }

    // ── 公开 API ────────────────────────

    /// 每帧调用：返回需要创建/销毁的 tile 事件（最多 N 个）
    pub fn poll(&mut self, player_pos: WorldPos) -> Vec<TileEvent> {
        // 1. 先消费遗留
        if !self.pending_events.is_empty() {
            let n = self.events_per_frame.min(self.pending_events.len());
            return self.pending_events.drain(..n).collect();
        }

        // 2. 计算所有层的 diff
        let mut events = Vec::new();

        for level in &LEVELS {
            let desired = Self::desired_keys(level, player_pos.x, player_pos.z);

            // Unload: active 中但不再需要
            let to_remove: Vec<LodKey> = self
                .active
                .keys()
                .filter(|k| k.level == level.index && !desired.contains(k))
                .copied()
                .collect();
            for key in &to_remove {
                self.active.remove(key);
                events.push(TileEvent::Unload { key: *key });
            }

            // Load: 需要但不在 active
            for key in &desired {
                if !self.active.contains_key(key) {
                    self.active.insert(*key, ());
                    let mesh = self.generate_tile(*key);
                    events.push(TileEvent::Load { key: *key, mesh });
                }
            }
        }

        // 3. 速率限制
        let n = self.events_per_frame.min(events.len());
        let result: Vec<_> = events.drain(..n).collect();
        self.pending_events = events;
        result
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
        let keys = ClipmapManager::desired_keys(&LEVELS[0], 0.0, 0.0);
        // L0 覆盖 0-128m，32m tiles → 应该有 ~20-30 个 tile
        assert!(keys.len() >= 40, "L0 should have at least 40 tiles, got {}", keys.len());
        assert!(keys.len() <= 60, "L0 should have at most 60 tiles, got {}", keys.len());
        // 所有 key 都是 level 0
        for k in &keys {
            assert_eq!(k.level, 0);
        }
    }

    #[test]
    fn test_desired_keys_l3() {
        let keys = ClipmapManager::desired_keys(&LEVELS[3], 0.0, 0.0);
        // L3 覆盖 512-1024m，256m tiles
        assert!(keys.len() >= 8, "L3 should have at least 8 tiles, got {}", keys.len());
        for k in &keys {
            assert_eq!(k.level, 3);
        }
    }

    #[test]
    fn test_tile_origin() {
        let key = LodKey { level: 0, gx: 3, gz: -2 };
        let (ox, oz) = ClipmapManager::tile_origin(key, &LEVELS[0]);
        assert!((ox - 96.0).abs() < 0.01); // 3 × 32
        assert!((oz - (-64.0)).abs() < 0.01); // -2 × 32
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
        for _ in 0..100 {
            let events = mgr.poll(WorldPos::default());
            total += events.len();
            if events.is_empty() { break; }
        }
        // 全部消耗后应有 64 左右的 tile
        assert!(total > 70, "should have >70 tiles total, got {}", total);
    }

    #[test]
    fn test_poll_no_change_after_drain() {
        let mut mgr = make_manager();
        loop {
            if mgr.poll(WorldPos::default()).is_empty() { break; }
        }
        assert!(mgr.poll(WorldPos::default()).is_empty());
    }
}
