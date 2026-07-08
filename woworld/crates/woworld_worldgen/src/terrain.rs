//! HeightfieldTerrain — TerrainQuery trait 实现
//!
//! 基于双层 Perlin 噪声的高度场地形。
//! 不存储体素数据——所有查询通过噪声函数实时计算。
//! 可选集成: WorldClock (昼夜) + BiomeClassifier (群系)

use std::sync::Arc;

use glam::Vec3;
use woworld_core::density::{DensityProvider, DensityStack};
use woworld_core::edit_terrain::EditTerrainSnapshot;
use woworld_core::prelude::*;
use woworld_core::spatial::TerrainQuery;

use crate::biome::BiomeClassifier;
use crate::noise_gen::WorldNoise;
use woworld_core::time::WorldClock;

// ── P2a: 地形基底密度层 ─────────────────

/// 地形基底密度层——P2 噪声驱动的基础地形
///
/// 将 Perlin 高度场包装为 `DensityProvider`——密度场层叠体系的第一个实现。
/// `density_at(pos) = pos.y - noise.sample_height(pos.x, pos.z)`
/// 正值 = 实体（ground），负值 = 空（air/water）。
#[derive(Debug)]
pub struct TerrainBaseDensity {
    noise: Arc<WorldNoise>,
    biome: Option<BiomeClassifier>,
}

impl TerrainBaseDensity {
    pub fn new(noise: Arc<WorldNoise>) -> Self {
        Self { noise, biome: None }
    }

    pub fn with_biomes(mut self, classifier: BiomeClassifier) -> Self {
        self.biome = Some(classifier);
        self
    }
}

impl DensityProvider for TerrainBaseDensity {
    fn density_at(&self, pos: WorldPos) -> f32 {
        let h = self.noise.sample_height(pos.x, pos.z) as f32;
        h - pos.y as f32 // 正值=地下实体，负值=空中
    }
    fn material_at(&self, pos: WorldPos) -> u8 {
        let h = self.noise.sample_height(pos.x, pos.z);
        if let Some(ref classifier) = self.biome {
            if let Some(biome) = classifier.classify(WorldPos {
                x: pos.x,
                y: h,
                z: pos.z,
            }) {
                return biome.surface_material as u8;
            }
        }
        // 计算真实坡度——与 Clipmap 路径一致（Sprint 045 色差修复）
        let normal = self.noise.calc_normal(pos.x, pos.z, 0.5);
        let steepness = (1.0 - normal.y).abs(); // normal.y 已是 f32
        HeightfieldTerrain::material_from_height(h, steepness) as u8
    }
    fn priority(&self) -> u8 {
        0
    }
    fn layer_name(&self) -> &'static str {
        "terrain_base"
    }
}

// ── HeightfieldTerrain ──────────────────

/// 高度场地形——无状态，纯函数式查询
///
/// `clock` 和 `biome_classifier` 为 `Option`——向后兼容，
/// 未设置时退化到当前行为（light=1.0, 高度法选材质）。
///
/// `noise` 为 `Arc<WorldNoise>`——与 `BiomeClassifier` 共享同一噪声实例。
/// `density_stack` 包含至少 1 层 `TerrainBaseDensity`——密度场层叠体系的入口。
#[derive(Clone, Debug)]
pub struct HeightfieldTerrain {
    noise: Arc<WorldNoise>,
    seed: u64,
    density_stack: DensityStack,
    pub clock: Option<WorldClock>,
    pub biome_classifier: Option<BiomeClassifier>,
    /// 地形修改快照（CoW——读者零锁开销）
    ///
    /// 为 None 时表示无修改，所有查询回退到噪声地形。
    /// WorldDriver 每帧从 ECS EditTerrainResource 更新此快照。
    edit_terrain: Option<Arc<EditTerrainSnapshot>>,
}

impl Default for HeightfieldTerrain {
    fn default() -> Self {
        let noise = Arc::new(WorldNoise::new(42));
        let mut density_stack = DensityStack::new();
        density_stack.push(Arc::new(TerrainBaseDensity::new(noise.clone())));
        density_stack.push(crate::cave::CaveDensity::new_arc(42, Default::default()));
        Self {
            noise,
            seed: 42,
            density_stack,
            clock: None,
            biome_classifier: None,
            edit_terrain: None,
        }
    }
}

impl HeightfieldTerrain {
    pub fn new(seed: u32) -> Self {
        let s = seed as u64;
        let noise = Arc::new(WorldNoise::new(s));
        let mut density_stack = DensityStack::new();
        density_stack.push(Arc::new(TerrainBaseDensity::new(noise.clone())));
        density_stack.push(crate::cave::CaveDensity::new_arc(s, Default::default()));
        Self {
            noise,
            seed: s,
            density_stack,
            clock: None,
            biome_classifier: None,
            edit_terrain: None,
        }
    }

    pub fn with_noise(noise: Arc<WorldNoise>, seed: u64) -> Self {
        let mut density_stack = DensityStack::new();
        density_stack.push(Arc::new(TerrainBaseDensity::new(noise.clone())));
        density_stack.push(crate::cave::CaveDensity::new_arc(seed, Default::default()));
        Self {
            noise,
            seed,
            density_stack,
            clock: None,
            biome_classifier: None,
            edit_terrain: None,
        }
    }

    /// 获取世界种子（用于派生密度层独立种子）
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// 获取噪声参数（用于重建等 seed 的 DensityField）
    pub fn noise_params(&self) -> crate::noise_gen::NoiseParams {
        self.noise.params.clone()
    }

    /// 获取噪声引用（用于构造 DensityField）
    pub fn noise(&self) -> &WorldNoise {
        &self.noise
    }

    /// 获取密度栈引用（用于 Transvoxel 等值面提取）
    pub fn density_stack(&self) -> &DensityStack {
        &self.density_stack
    }

    /// 获取噪声 Arc 克隆（用于 rayon 后台线程共享）
    pub fn noise_arc(&self) -> Arc<WorldNoise> {
        self.noise.clone()
    }

    /// 底层地形高度（内部——2D 高度场权威入口）
    ///
    /// 不走 `DensityStack`（3D 密度场 ≠ 2D 高度场）。
    /// `sample_vertex()` / `calc_normal()` / `surface_material_at()` 等
    /// 所有 2D 高度查询统一走此方法。
    #[inline]
    fn terrain_height(&self, x: f64, z: f64) -> f64 {
        self.noise.sample_height(x, z)
    }

    /// 合并查询：一次采样得高度、法线、材质（消除冗余噪声调用）
    ///
    /// `height_at` + `normal_at` + `surface_material_at` 各自独立调用
    /// `sample_height` — 每顶点最高 10 次噪声调用。本方法共享结果，降至约 4 次。
    pub fn sample_vertex(&self, x: f64, z: f64) -> (f32, Vec3, SurfaceMaterial) {
        let h = self.actual_height_at(x, z);
        let normal = self.normal_at(WorldPos { x, y: h as f64, z });
        let mat = self.actual_material_at_with_height(x, z, h as f64);
        (h, normal, mat)
    }

    /// 挂载昼夜时钟——之后 `light_level_at()` 返回实际值
    pub fn with_clock(mut self, clock: WorldClock) -> Self {
        self.clock = Some(clock);
        self
    }

    /// 挂载群系分类器——之后 `surface_material_at()` 群系优先
    ///
    /// ★ 同时重建 DensityStack 中的 TerrainBaseDensity 以携带群系引用——
    ///   确保 DensityStack::material_at() fallback 路径也能返回正确的 biome 材质。
    pub fn with_biomes(mut self, classifier: BiomeClassifier) -> Self {
        // 重建 DensityStack：TerrainBaseDensity 需携带 biome 引用
        let mut new_stack = DensityStack::new();
        new_stack.push(Arc::new(
            TerrainBaseDensity::new(self.noise.clone()).with_biomes(classifier.clone()),
        ));
        new_stack.push(crate::cave::CaveDensity::new_arc(
            self.seed,
            Default::default(),
        ));
        self.density_stack = new_stack;
        self.biome_classifier = Some(classifier);
        self
    }

    /// 设置地形修改快照——WorldDriver 每帧调用以同步 ECS 修改到地形查询
    pub fn set_edit_terrain(&mut self, snapshot: Option<Arc<EditTerrainSnapshot>>) {
        self.edit_terrain = snapshot;
    }

    /// 获取当前地形修改快照引用
    pub fn edit_terrain(&self) -> Option<&Arc<EditTerrainSnapshot>> {
        self.edit_terrain.as_ref()
    }

    /// 构建 EditDensityLayer（用于传入 VoxelChunk rayon job 的 DensityStack）
    ///
    /// 返回 Clone 成本极低（内部 Arc 引用计数 +1）。
    /// voxel_size 应与 Transvoxel 提取的体素尺寸一致（LOD 0 = 0.5m）。
    pub fn edit_density_layer(
        &self,
        voxel_size: f64,
    ) -> Option<woworld_core::edit_terrain::EditDensityLayer> {
        self.edit_terrain.as_ref().map(|et| {
            woworld_core::edit_terrain::EditDensityLayer::new(et.density.clone(), voxel_size)
        })
    }

    /// 实际表面高度——优先查 EditHeightfield，回退到噪声地形
    ///
    /// ★ 核心修复：原 `height_at()` 走裸噪声，修改不可见。
    /// 此方法先查 EditHeightfield（CoW 快照，零锁），未命中回退。
    #[inline]
    pub fn actual_height_at(&self, x: f64, z: f64) -> f32 {
        if let Some(ref et) = self.edit_terrain {
            if let Some(h) = et.heightfield.height_at(x, z) {
                return h;
            }
        }
        self.terrain_height(x, z) as f32
    }

    /// 实际表面材质——优先 EditDensity → 群系分类 → DensityStack → 高度回退
    ///
    /// ★ 群系分类必须在 DensityStack 之前——DensityStack::material_at() 的 fallback
    ///   使用 TerrainBaseDensity（无 biome），永远返回 Some(…)，会使群系路径成为死代码。
    ///   修复：群系优先于 DensityStack，Clipmap 路径获得正确的 biome 颜色。
    #[inline]
    pub fn actual_material_at(&self, pos: WorldPos) -> SurfaceMaterial {
        self.actual_material_at_with_height(pos.x, pos.z, self.terrain_height(pos.x, pos.z))
    }

    /// `actual_material_at` 的内部实现——接受预计算的地形高度以避免冗余噪声调用。
    ///
    /// `sample_vertex()` 调用此方法以复用已计算的 `actual_height_at` 结果。
    #[inline]
    fn actual_material_at_with_height(&self, x: f64, z: f64, h: f64) -> SurfaceMaterial {
        // 1. 优先检查 EditDensity 材质覆盖（CoW 快照，零锁）
        if let Some(ref et) = self.edit_terrain {
            let voxel = glam::IVec3::new(
                (x / 0.5).floor() as i32,
                (h / 0.5).floor() as i32,
                (z / 0.5).floor() as i32,
            );
            if let Some(mat_id) = et.density.material_at(voxel) {
                return Self::material_id_to_surface(mat_id);
            }
        }
        // 2. 水下：3 层高度法（与旧 surface_material_at 行为一致）
        if h < -100.0 {
            return SurfaceMaterial::Stone; // 深海
        } else if h < -10.0 {
            return SurfaceMaterial::Gravel; // 大陆架
        } else if h < 0.0 {
            return SurfaceMaterial::Sand; // 浅海床
        }
        // 3. 陆地：群系分类优先
        if let Some(ref classifier) = self.biome_classifier {
            if let Some(biome) = classifier.classify(WorldPos { x, y: h, z }) {
                return biome.surface_material;
            }
        }
        // 4. 回退：DensityStack 材质组合（含 CaveDensity 等——地下/编辑区域）
        let pos = WorldPos { x, y: h, z };
        if let Some(mat_id) = self.density_stack.material_at(pos) {
            return Self::material_id_to_surface(mat_id);
        }
        // 5. 最终回退：高度+坡度法
        let normal = self.calc_normal(x, z, 0.5);
        let steepness = (1.0 - normal.y).abs();
        Self::material_from_height(h, steepness)
    }

    /// u8 材质 ID → SurfaceMaterial 枚举
    fn material_id_to_surface(mat_id: u8) -> SurfaceMaterial {
        use woworld_core::material::SurfaceMaterial;
        match mat_id {
            0 => SurfaceMaterial::Grass,
            1 => SurfaceMaterial::Sand,
            2 => SurfaceMaterial::Rock,
            3 => SurfaceMaterial::Stone,
            4 => SurfaceMaterial::Wood,
            5 => SurfaceMaterial::Metal,
            6 => SurfaceMaterial::Water,
            7 => SurfaceMaterial::Ice,
            8 => SurfaceMaterial::Mud,
            9 => SurfaceMaterial::Snow,
            10 => SurfaceMaterial::Gravel,
            11 => SurfaceMaterial::Clay,
            12 => SurfaceMaterial::Moss,
            13 => SurfaceMaterial::LeafLitter,
            14 => SurfaceMaterial::Cobblestone,
            15 => SurfaceMaterial::Marble,
            16 => SurfaceMaterial::Glass,
            17 => SurfaceMaterial::Fabric,
            18 => SurfaceMaterial::Thatch,
            19 => SurfaceMaterial::Bone,
            20 => SurfaceMaterial::Flesh,
            _ => SurfaceMaterial::Stone,
        }
    }

    /// 在 (x, z) 采样高度，有限差分计算法线
    fn calc_normal(&self, x: f64, z: f64, eps: f64) -> Vec3 {
        let h_center = self.terrain_height(x, z);
        let h_right = self.terrain_height(x + eps, z);
        let h_forward = self.terrain_height(x, z + eps);

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
        self.actual_height_at(pos.x, pos.z)
    }

    fn normal_at(&self, pos: WorldPos) -> Vec3 {
        // 用实际表面高度计算法线（含修改）
        let eps = 0.5;
        let h_center = self.actual_height_at(pos.x, pos.z) as f64;
        let h_right = self.actual_height_at(pos.x + eps, pos.z) as f64;
        let h_forward = self.actual_height_at(pos.x, pos.z + eps) as f64;
        let dx = (h_center - h_right) / eps;
        let dz = (h_center - h_forward) / eps;
        glam::vec3(-dx as f32, 1.0, -dz as f32).normalize()
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
        let base = self.density_stack.density_at(pos);
        // 叠加 EditDensity 的密度增量（CoW 快照，零锁）
        if let Some(ref et) = self.edit_terrain {
            let voxel_size = 0.5; // LOD 0 体素尺寸
            let voxel = glam::IVec3::new(
                (pos.x / voxel_size).floor() as i32,
                (pos.y / voxel_size).floor() as i32,
                (pos.z / voxel_size).floor() as i32,
            );
            if let Some(delta) = et.density.density_delta_at(voxel) {
                return base + delta;
            }
        }
        base
    }

    fn is_walkable(&self, pos: WorldPos) -> bool {
        let h = self.actual_height_at(pos.x, pos.z) as f64;
        let on_surface = (pos.y - h).abs() < 1.0;
        if !on_surface {
            return false;
        }
        // 检查坡度——用实际表面法线
        let normal = self.normal_at(pos);
        let steepness = (1.0 - normal.y).abs();
        steepness < 0.7 // cos(45°) ≈ 0.707
    }

    fn surface_material_at(&self, pos: WorldPos) -> SurfaceMaterial {
        self.actual_material_at(pos)
    }

    fn medium_at(&self, pos: WorldPos) -> Medium {
        let h = self.actual_height_at(pos.x, pos.z) as f64;
        if h < 0.0 && pos.y < 0.0 && pos.y > h {
            Medium::Water
        } else {
            Medium::Air
        }
    }

    fn light_level_at(&self, _pos: WorldPos) -> f32 {
        self.clock
            .as_ref()
            .map(|c| c.current.light_level)
            .unwrap_or(1.0) // 无时钟 → 全亮
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
        // 地下 200m 深度——远超洞穴层 80m 幅值，地形密度应主导
        let d = terrain.density_at(WorldPos {
            x: 100.0,
            y: (h - 200.0) as f64,
            z: 100.0,
        });
        assert!(d > 0.5, "underground density should be solid (got {d})");
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

    // ── 群系集成测试 ─────────────────

    #[test]
    fn test_surface_material_without_biome_falls_back() {
        // 无群系 → 高度法
        let terrain = HeightfieldTerrain::new(42);
        let mat = terrain.surface_material_at(WorldPos {
            x: 500.0,
            y: 0.0,
            z: 500.0,
        });
        // 只要不 panic 且返回有效材质
        let _ = mat;
    }

    #[test]
    fn test_light_level_without_clock_is_full() {
        let terrain = HeightfieldTerrain::new(42);
        let light = terrain.light_level_at(WorldPos::default());
        assert!((light - 1.0).abs() < 0.01, "without clock should be 1.0");
    }

    #[test]
    fn test_light_level_with_clock_varies() {
        // 正午 = 亮
        let mut noon_clock = WorldClock::new(60.0);
        noon_clock.set_time(0.5);
        let noon_terrain = HeightfieldTerrain::new(42).with_clock(noon_clock);
        let noon_light = noon_terrain.light_level_at(WorldPos::default());
        assert!(
            noon_light > 0.9,
            "noon should be bright, got {}",
            noon_light
        );

        // 午夜 = 暗
        let mut mid_clock = WorldClock::new(60.0);
        mid_clock.set_time(0.0);
        let mid_terrain = HeightfieldTerrain::new(42).with_clock(mid_clock);
        let mid_light = mid_terrain.light_level_at(WorldPos::default());
        assert!(
            mid_light < 0.1,
            "midnight should be dark, got {}",
            mid_light
        );
    }
}
