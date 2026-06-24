//! ChunkManager — 分块地形加载/卸载管理
//!
//! 纯 Rust，引擎无关。消费 HeightfieldTerrain，
//! 通过 generate_terrain_mesh() 生成每 Chunk 的规则高度场网格。
//!
//! ★ MC 等值面提取已就绪（marching_cubes.rs）——Clipmap LOD 就位后切换。
//!
//! 参见: `WoWorld-Design/开发阶段/世界生成/007-体素设计决策.md` §1.3

use std::collections::HashSet;

use woworld_core::id::ChunkCoord;
use woworld_core::prelude::WorldPos;

use crate::terrain::HeightfieldTerrain;
use crate::terrain_mesh::{generate_terrain_mesh, TerrainMeshData};

// ── 缺省参数 ──────────────────────────
/// Chunk 水平尺寸（米）——高度场阶段 128m，未来 Transvoxel 改为 32m
const CHUNK_SIZE_M: f64 = 128.0;
/// 体素尺寸（米）——MC 等值面提取分辨率
const VERTEX_SPACING: f64 = 2.0;
/// 加载半径（Chunk 数）——7×7 = 49 Chunk 覆盖 ~896m
const GRID_RADIUS: u32 = 3;
/// 每帧最多处理的事件数——防止跨边界时单帧卡顿
const EVENTS_PER_FRAME: usize = 4;

// ── 公开类型 ──────────────────────────

/// Chunk 生命周期事件
///
/// WorldDriver 消费此枚举：Load → 创建 MeshInstance3D，Unload → queue_free。
#[derive(Debug)]
pub enum ChunkEvent {
    Load {
        coord: ChunkCoord,
        mesh: TerrainMeshData,
    },
    Unload {
        coord: ChunkCoord,
    },
}

/// 分块地形管理器
///
/// 每帧 `poll(player_pos)` → 返回需要创建/销毁的 Chunk 列表。
/// 不依赖 Godot——纯 Rust 逻辑，`cargo test` 可独立验证。
pub struct ChunkManager {
    terrain: HeightfieldTerrain,
    loaded: HashSet<ChunkCoord>,
    chunk_size: f64,
    grid_radius: u32,
    spacing: f64,
    vertices_per_edge: u32,
    /// 尚未返回给调用方的事件（分摊到多帧）
    pending_events: Vec<ChunkEvent>,
    events_per_frame: usize,
}

impl ChunkManager {
    /// 创建新管理器
    ///
    /// `terrain` 必须已挂载 clock 和 biome_classifier（如需要）。
    /// 内部从 terrain 的噪声创建 HeightfieldDensity（相同 seed）。
    pub fn new(terrain: HeightfieldTerrain) -> Self {
        let vertices_per_edge = (CHUNK_SIZE_M / VERTEX_SPACING) as u32 + 1;
        Self {
            terrain,
            loaded: HashSet::new(),
            chunk_size: CHUNK_SIZE_M,
            grid_radius: GRID_RADIUS,
            spacing: VERTEX_SPACING,
            vertices_per_edge,
            pending_events: Vec::new(),
            events_per_frame: EVENTS_PER_FRAME,
        }
    }

    // ── 内部方法 ────────────────────────

    /// 以玩家为中心的期望 Chunk 坐标集合
    fn desired_coords(&self, player_pos: WorldPos) -> HashSet<ChunkCoord> {
        let center_cx = (player_pos.x / self.chunk_size).floor() as i64;
        let center_cz = (player_pos.z / self.chunk_size).floor() as i64;
        let r = self.grid_radius as i64;

        let capacity = ((2 * r + 1) * (2 * r + 1)) as usize;
        let mut coords = HashSet::with_capacity(capacity);
        for dx in -r..=r {
            for dz in -r..=r {
                coords.insert(ChunkCoord {
                    x: center_cx + dx,
                    y: 0, // 高度场: 所有 Chunk 在同一水平面
                    z: center_cz + dz,
                });
            }
        }
        coords
    }

    /// ChunkCoord → 世界空间左下角 (origin_x, origin_z)
    fn chunk_origin(&self, coord: ChunkCoord) -> (f64, f64) {
        (
            coord.x as f64 * self.chunk_size,
            coord.z as f64 * self.chunk_size,
        )
    }

    /// 从 HeightfieldTerrain 确定性生成单个 Chunk 网格
    ///
    /// ★ 当前使用高度场规则网格（快、已验证）。
    /// MC 等值面提取已就绪（marching_cubes.rs）——Clipmap LOD 就位后切换。
    fn generate_chunk(&self, coord: ChunkCoord) -> TerrainMeshData {
        let (ox, oz) = self.chunk_origin(coord);
        generate_terrain_mesh(&self.terrain, ox, oz, self.vertices_per_edge, self.spacing)
    }

    // ── 公开 API ────────────────────────

    /// 每帧调用：返回需要创建/销毁的 Chunk 事件列表（最多 `events_per_frame` 个）
    ///
    /// 首次调用产出全部 49 个 Chunk，后续跨边界时把 14 个事件分摊到 ~4 帧。
    /// 内部维护 pending 队列——`loaded` 在事件**入队时**更新，避免重复生成。
    pub fn poll(&mut self, player_pos: WorldPos) -> Vec<ChunkEvent> {
        // 1. 先消费上轮遗留的 pending 事件
        if !self.pending_events.is_empty() {
            let n = self.events_per_frame.min(self.pending_events.len());
            return self.pending_events.drain(..n).collect();
        }

        // 2. 无遗留 → 计算完整 diff
        let desired = self.desired_coords(player_pos);
        let mut events = Vec::new();

        // Unload
        let to_remove: Vec<ChunkCoord> =
            self.loaded.difference(&desired).copied().collect();
        for coord in &to_remove {
            self.loaded.remove(coord);
            events.push(ChunkEvent::Unload { coord: *coord });
        }

        // Load
        for coord in &desired {
            if !self.loaded.contains(coord) {
                self.loaded.insert(*coord);
                let mesh = self.generate_chunk(*coord);
                events.push(ChunkEvent::Load {
                    coord: *coord,
                    mesh,
                });
            }
        }

        // 3. 放入 pending 队列，返回前 N 个
        let n = self.events_per_frame.min(events.len());
        let result: Vec<_> = events.drain(..n).collect();
        self.pending_events = events; // 剩余事件留到后续帧

        result
    }

    /// 当前已加载的 Chunk 数（调试用）
    #[allow(dead_code)]
    pub fn loaded_count(&self) -> usize {
        self.loaded.len()
    }
}

// ── 测试 ──────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manager() -> ChunkManager {
        let terrain = HeightfieldTerrain::new(42);
        ChunkManager::new(terrain)
    }

    #[test]
    fn test_chunk_origin_zero() {
        let mgr = make_manager();
        let coord = ChunkCoord { x: 0, y: 0, z: 0 };
        let (ox, oz) = mgr.chunk_origin(coord);
        assert!((ox - 0.0).abs() < 0.01);
        assert!((oz - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_chunk_origin_offset() {
        let mgr = make_manager();
        let coord = ChunkCoord { x: 3, y: 0, z: -2 };
        let (ox, oz) = mgr.chunk_origin(coord);
        assert!((ox - 384.0).abs() < 0.01); // 3 × 128
        assert!((oz - (-256.0)).abs() < 0.01); // -2 × 128
    }

    #[test]
    fn test_desired_coords_grid_size() {
        let mgr = make_manager();
        let coords = mgr.desired_coords(WorldPos::default());
        // 7×7 = 49
        assert_eq!(coords.len(), 49);
    }

    #[test]
    fn test_desired_coords_centered() {
        let mgr = make_manager();
        // 玩家在 (200, 0, 300) → center chunk = (1, 0, 2)
        let coords = mgr.desired_coords(WorldPos {
            x: 200.0,
            y: 0.0,
            z: 300.0,
        });
        // 应该包含 center chunk
        assert!(coords.contains(&ChunkCoord { x: 1, y: 0, z: 2 }));
        // 应该有边缘: center ± 3
        assert!(coords.contains(&ChunkCoord { x: -2, y: 0, z: 2 })); // 左边缘
        assert!(coords.contains(&ChunkCoord { x: 4, y: 0, z: 2 })); // 右边缘
        // 不应该超过 radius
        assert!(!coords.contains(&ChunkCoord { x: 5, y: 0, z: 2 }));
    }

    #[test]
    fn test_poll_rate_limits_per_frame() {
        let mut mgr = make_manager();
        // 第一次调用：最多 4 个事件
        let events = mgr.poll(WorldPos::default());
        assert!(events.len() <= 4, "first poll capped at 4, got {}", events.len());
        // loaded 已包含全部 49（入队时标记）
        assert_eq!(mgr.loaded_count(), 49);
    }

    #[test]
    fn test_poll_drains_all_over_multiple_frames() {
        let mut mgr = make_manager();
        let mut total = 0;
        // 模拟多帧消费
        for _ in 0..20 {
            let events = mgr.poll(WorldPos::default());
            total += events.len();
            if events.is_empty() {
                break;
            }
        }
        // 最终拿到全部 49 个事件
        assert_eq!(total, 49);
    }

    #[test]
    fn test_poll_no_change_after_drain() {
        let mut mgr = make_manager();
        // 消费全部事件
        loop {
            let events = mgr.poll(WorldPos::default());
            if events.is_empty() {
                break;
            }
        }
        // 再 poll 一次——无变化
        let events = mgr.poll(WorldPos::default());
        assert!(events.is_empty());
    }

    #[test]
    fn test_generated_mesh_is_valid() {
        let mgr = make_manager();
        let mesh = mgr.generate_chunk(ChunkCoord { x: 0, y: 0, z: 0 });
        // MC 输出可变——但应有合理数量的顶点
        assert!(
            mesh.vertices.len() > 100,
            "should have at least 100 vertices, got {}",
            mesh.vertices.len()
        );
        assert_eq!(mesh.vertices.len(), mesh.normals.len());
        assert_eq!(mesh.vertices.len(), mesh.colors.len());
        // 索引数是 3 的倍数
        assert!(mesh.indices.len() % 3 == 0);
    }

    #[test]
    fn test_deterministic_chunk() {
        let t1 = HeightfieldTerrain::new(99);
        let t2 = HeightfieldTerrain::new(99);
        let mgr1 = ChunkManager::new(t1);
        let mgr2 = ChunkManager::new(t2);
        let m1 = mgr1.generate_chunk(ChunkCoord { x: 2, y: 0, z: -3 });
        let m2 = mgr2.generate_chunk(ChunkCoord { x: 2, y: 0, z: -3 });
        // 同 seed + 同 coord → 完全相同的顶点
        for (a, b) in m1.vertices.iter().zip(m2.vertices.iter()) {
            assert!((a.x - b.x).abs() < 0.001);
            assert!((a.y - b.y).abs() < 0.001);
            assert!((a.z - b.z).abs() < 0.001);
        }
    }
}
