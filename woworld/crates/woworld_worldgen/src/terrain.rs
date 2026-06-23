//! HeightfieldTerrain — TerrainQuery trait 实现
//!
//! 基于双层 Perlin 噪声的高度场地形。
//! 不存储体素数据——所有查询通过噪声函数实时计算。

use glam::Vec3;
use woworld_core::prelude::*;
use woworld_core::spatial::TerrainQuery;

use crate::noise_gen::WorldNoise;

/// 高度场地形——无状态，纯函数式查询
#[derive(Clone, Debug)]
pub struct HeightfieldTerrain {
    noise: WorldNoise,
}

impl HeightfieldTerrain {
    pub fn new(seed: u32) -> Self {
        Self {
            noise: WorldNoise::new(seed),
        }
    }

    pub fn with_noise(noise: WorldNoise) -> Self {
        Self { noise }
    }

    /// 在 (x, z) 采样高度，有限差分计算法线
    fn calc_normal(&self, x: f64, z: f64, eps: f64) -> Vec3 {
        let h_center = self.noise.sample_height(x, z);
        let h_right = self.noise.sample_height(x + eps, z);
        let h_forward = self.noise.sample_height(x, z + eps);

        // 梯度: (-dh/dx, 1.0, -dh/dz) 归一化
        let dx = (h_center - h_right) / eps;
        let dz = (h_center - h_forward) / eps;
        glam::vec3(-dx as f32, 1.0, -dz as f32).normalize()
    }

    /// 根据高度和坡度选择地表材质
    fn material_from_height(h: f64, steepness: f32) -> SurfaceMaterial {
        if h < 0.0 {
            // 水下
            if h < -10.0 {
                SurfaceMaterial::Sand // 海床
            } else {
                SurfaceMaterial::Water // 浅水
            }
        } else if h < 10.0 {
            SurfaceMaterial::Sand // 海滩
        } else if steepness > 0.7 {
            SurfaceMaterial::Stone // 陡坡
        } else if h > 500.0 {
            SurfaceMaterial::Rock // 高山
        } else if h > 200.0 {
            SurfaceMaterial::Gravel // 丘陵
        } else {
            SurfaceMaterial::Grass // 平原
        }
    }
}

impl TerrainQuery for HeightfieldTerrain {
    fn height_at(&self, pos: WorldPos) -> f32 {
        self.noise.sample_height(pos.x, pos.z) as f32
    }

    fn normal_at(&self, pos: WorldPos) -> Vec3 {
        self.calc_normal(pos.x, pos.z, 0.5)
    }

    fn terrain_raycast(
        &self,
        origin: WorldPos,
        direction: Vec3,
        max_dist: f32,
    ) -> Option<TerrainHit> {
        let step = 0.5;
        let steps = (max_dist / step) as usize;
        let dir_norm = direction.normalize();

        for i in 1..=steps {
            let t = i as f32 * step;
            let pos = WorldPos {
                x: origin.x + dir_norm.x as f64 * t as f64,
                y: origin.y + dir_norm.y as f64 * t as f64,
                z: origin.z + dir_norm.z as f64 * t as f64,
            };
            if self.density_at(pos) > 0.5 {
                return Some(TerrainHit {
                    point: pos,
                    normal: self.normal_at(pos),
                    material: self.surface_material_at(pos),
                    distance: t,
                });
            }
        }
        None
    }

    fn density_at(&self, pos: WorldPos) -> f32 {
        let terrain_h = self.noise.sample_height(pos.x, pos.z);
        if pos.y < terrain_h {
            1.0
        } else {
            0.0
        }
    }

    fn is_walkable(&self, pos: WorldPos) -> bool {
        let h = self.noise.sample_height(pos.x, pos.z);
        let on_surface = (pos.y - h).abs() < 1.0;
        if !on_surface {
            return false;
        }
        // 检查坡度
        let normal = self.calc_normal(pos.x, pos.z, 0.5);
        let steepness = (1.0 - normal.y).abs(); // dot(normal, up) 的补
        steepness < 0.7 // cos(45°) ≈ 0.707
    }

    fn surface_material_at(&self, pos: WorldPos) -> SurfaceMaterial {
        let h = self.noise.sample_height(pos.x, pos.z);
        let normal = self.calc_normal(pos.x, pos.z, 0.5);
        let steepness = (1.0 - normal.y).abs();
        Self::material_from_height(h, steepness)
    }

    fn medium_at(&self, pos: WorldPos) -> Medium {
        let h = self.noise.sample_height(pos.x, pos.z);
        if h < 0.0 && pos.y < 0.0 && pos.y > h {
            Medium::Water
        } else {
            Medium::Air
        }
    }

    fn light_level_at(&self, _pos: WorldPos) -> f32 {
        1.0 // 无昼夜系统前全亮
    }

    fn sample_horizon(&self, pos: WorldPos, directions: &[Vec3]) -> Vec<f32> {
        directions
            .iter()
            .map(|dir| {
                // 沿方向步进，查是否命中地形
                let mut blocked = 0.0;
                for i in 1..=40 {
                    let t = i as f32 * 2.5; // 2.5m 步长，最远 100m
                    let sp = WorldPos {
                        x: pos.x + dir.x as f64 * t as f64,
                        y: pos.y + dir.y as f64 * t as f64,
                        z: pos.z + dir.z as f64 * t as f64,
                    };
                    if self.density_at(sp) > 0.5 {
                        blocked = 1.0;
                        break;
                    }
                }
                blocked
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_density_below_terrain() {
        let terrain = HeightfieldTerrain::new(42);
        let h = terrain.height_at(WorldPos {
            x: 100.0,
            y: 0.0,
            z: 100.0,
        });
        // 地下任意点密度应为 1.0
        let d = terrain.density_at(WorldPos {
            x: 100.0,
            y: (h - 10.0) as f64,
            z: 100.0,
        });
        assert!(d > 0.5, "underground density should be solid");
    }

    #[test]
    fn test_density_above_terrain() {
        let terrain = HeightfieldTerrain::new(42);
        let h = terrain.height_at(WorldPos {
            x: 100.0,
            y: 0.0,
            z: 100.0,
        });
        // 空中密度应为 0.0
        let d = terrain.density_at(WorldPos {
            x: 100.0,
            y: (h + 100.0) as f64,
            z: 100.0,
        });
        assert!(d < 0.5, "above-ground density should be air");
    }

    #[test]
    fn test_normal_mostly_up() {
        let terrain = HeightfieldTerrain::new(42);
        // 在原点附近采样（应该是比较平坦的区域）
        let n = terrain.normal_at(WorldPos {
            x: 500.0,
            y: 0.0,
            z: 500.0,
        });
        // y 分量应为正（朝上）
        assert!(n.y > 0.0);
        // 大概率接近 (0, 1, 0)
    }

    #[test]
    fn test_raycast_hits_ground() {
        let terrain = HeightfieldTerrain::new(42);
        let origin = WorldPos {
            x: 100.0,
            y: 500.0,
            z: 100.0,
        };
        // 向下射——必然命中地面
        let hit = terrain.terrain_raycast(origin, -Vec3::Y, 1000.0);
        assert!(hit.is_some(), "downward ray should hit ground");
    }

    #[test]
    fn test_medium_water() {
        let terrain = HeightfieldTerrain::new(42);
        // 找一个海洋区域（噪声值低）
        // 直接测试水下位置
        let medium = terrain.medium_at(WorldPos {
            x: 0.0,
            y: -50.0,
            z: 5000.0,
        });
        // 可能是 Water 或 Air（取决于在该点是海还是陆）
        // 只验证不 panic
        let _ = medium;
    }
}
