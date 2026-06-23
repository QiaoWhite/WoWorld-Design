//! DDA 射线可见性查询 — VisibilityQuery trait 实现
//!
//! 使用 3D Digital Differential Analyzer 算法步进检测射线-地形交点。
//! 消费 TerrainQuery trait（泛型参数），不耦合具体地形实现。

use woworld_core::prelude::*;
use woworld_core::spatial::{TerrainQuery, VisibilityQuery};

/// 密度阈值——density > 此值视为固体
const SOLID_DENSITY_THRESHOLD: f32 = 0.5;

/// DDA 射线可见性查询器
///
/// 泛型参数 `T: TerrainQuery` 允许注入任意地形实现。
pub struct DdaVisibility<T: TerrainQuery> {
    terrain: T,
    step_size: f32,
}

impl<T: TerrainQuery> DdaVisibility<T> {
    /// 创建新的 DDA 查询器
    /// - `terrain`: 地形查询实现
    /// - `step_size`: 射线步进尺寸（米），默认 0.5m（体素尺寸）
    pub fn new(terrain: T, step_size: f32) -> Self {
        Self { terrain, step_size }
    }
}

impl<T: TerrainQuery> VisibilityQuery for DdaVisibility<T> {
    fn line_of_sight(&self, from: WorldPos, to: WorldPos) -> bool {
        self.line_of_sight_hit(from, to).is_none()
    }

    fn line_of_sight_hit(
        &self,
        from: WorldPos,
        to: WorldPos,
    ) -> Option<(WorldPos, SurfaceMaterial)> {
        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let dz = to.z - from.z;
        let dist = ((dx * dx + dy * dy + dz * dz) as f32).sqrt();

        if dist < self.step_size {
            return None;
        }

        let steps = (dist / self.step_size) as usize;
        let inv_steps = 1.0 / steps as f64;

        for i in 1..=steps {
            let t = i as f64 * inv_steps;
            let pos = WorldPos {
                x: from.x + dx * t,
                y: from.y + dy * t,
                z: from.z + dz * t,
            };

            let density = self.terrain.density_at(pos);
            if density > SOLID_DENSITY_THRESHOLD {
                let material = self.terrain.surface_material_at(pos);
                return Some((pos, material));
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// 用于测试的 mock 地形——密度场由 HashMap 定义
    struct MockTerrain {
        densities: HashMap<(i32, i32, i32), f32>,
    }

    impl MockTerrain {
        fn new() -> Self {
            Self {
                densities: HashMap::new(),
            }
        }

        fn set_solid(&mut self, x: i32, y: i32, z: i32) {
            // 将体素坐标转为 WorldPos 对应的密度
            // density_at 按 0.5m 精度查询，这里简化处理
            self.densities.insert((x, y, z), 1.0);
        }

        fn pos_key(pos: WorldPos) -> (i32, i32, i32) {
            (
                (pos.x * 2.0).round() as i32, // 0.5m → 整数索引
                (pos.y * 2.0).round() as i32,
                (pos.z * 2.0).round() as i32,
            )
        }
    }

    impl TerrainQuery for MockTerrain {
        fn height_at(&self, _pos: WorldPos) -> f32 {
            0.0
        }
        fn normal_at(&self, _pos: WorldPos) -> Vec3 {
            Vec3::Y
        }
        fn terrain_raycast(
            &self,
            _origin: WorldPos,
            _direction: Vec3,
            _max_dist: f32,
        ) -> Option<TerrainHit> {
            None
        }
        fn density_at(&self, pos: WorldPos) -> f32 {
            *self.densities.get(&Self::pos_key(pos)).unwrap_or(&0.0)
        }
        fn is_walkable(&self, _pos: WorldPos) -> bool {
            true
        }
        fn surface_material_at(&self, _pos: WorldPos) -> SurfaceMaterial {
            SurfaceMaterial::Stone
        }
        fn medium_at(&self, _pos: WorldPos) -> Medium {
            Medium::Air
        }
        fn light_level_at(&self, _pos: WorldPos) -> f32 {
            0.0
        }
        fn sample_horizon(&self, _pos: WorldPos, directions: &[Vec3]) -> Vec<f32> {
            vec![1.0; directions.len()]
        }
    }

    #[test]
    fn test_los_clear() {
        let terrain = MockTerrain::new();
        let vis = DdaVisibility::new(terrain, 0.5);

        let from = WorldPos {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        let to = WorldPos {
            x: 10.0,
            y: 1.0,
            z: 0.0,
        };
        assert!(vis.line_of_sight(from, to));
    }

    #[test]
    fn test_los_blocked() {
        let mut terrain = MockTerrain::new();
        // 在 (5.0, 1.0, 0.0) 放置固体块
        terrain.set_solid(10, 2, 0); // 0.5m 精度 → 10=5.0m

        let vis = DdaVisibility::new(terrain, 0.5);

        let from = WorldPos {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        let to = WorldPos {
            x: 10.0,
            y: 1.0,
            z: 0.0,
        };
        assert!(!vis.line_of_sight(from, to));
    }

    #[test]
    fn test_los_hit_returns_material() {
        let mut terrain = MockTerrain::new();
        terrain.set_solid(10, 2, 0); // 5.0m 处固体

        let vis = DdaVisibility::new(terrain, 0.5);

        let from = WorldPos {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        let to = WorldPos {
            x: 10.0,
            y: 1.0,
            z: 0.0,
        };
        let hit = vis.line_of_sight_hit(from, to);
        assert!(hit.is_some());
        let (hit_pos, material) = hit.unwrap();
        assert_eq!(material, SurfaceMaterial::Stone);
        // 命中点应在 5.0m 附近
        assert!((hit_pos.x - 5.0).abs() < 0.5);
    }

    #[test]
    fn test_los_short_distance() {
        let terrain = MockTerrain::new();
        let vis = DdaVisibility::new(terrain, 0.5);

        // 距离小于步长
        let from = WorldPos {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        };
        let to = WorldPos {
            x: 0.1,
            y: 1.0,
            z: 0.0,
        };
        assert!(vis.line_of_sight_hit(from, to).is_none());
    }
}
