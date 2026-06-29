//! 3D 密度场抽象
//!
//! Marching Cubes / Transvoxel 消费 `DensityField` trait：在 3D 空间采样密度值，
//! 等值面 threshold=0.5 处提取三角形网格。
//!
//! ## 多层架构（007-体素设计决策 §1.4）
//!
//! 装饰器链模式 — 高优先级包裹低优先级，`sample()` 按链顺序仲裁：
//!
//! ```text
//! CaveDensity(L4, priority=4)
//!   └── HeightfieldDensity(L0, priority=0)  ← 地基
//! ```
//!
//! 未来扩展到完整 L0-L10 链时，继续在 `DensityStack::with_*()` 方法中挂载新层。

use noise::permutationtable::PermutationTable;

use crate::biome::BiomeClassifier;
use crate::noise_gen::{
    derive_noise_seed, worley_3d_f2f1, WorldNoise, NOISE_DISCRIMINANT_WORLEY_CAVE,
};

// ── 体素材质 ID 常量 (007-体素设计决策 §2) ──────

/// 空气
pub const VOXEL_AIR: u8 = 0;
/// 泥土 (地下 0-5m)
pub const VOXEL_DIRT: u8 = 1;
/// 草皮 (地表)
pub const VOXEL_GRASS: u8 = 2;
/// 沙 (沙漠/河岸/海滩)
pub const VOXEL_SAND: u8 = 3;
/// 碎石 (地下 5-20m)
pub const VOXEL_GRAVEL: u8 = 5;
/// 石头 (地下 20m+)
pub const VOXEL_STONE: u8 = 6;
/// 花岗岩 (山区深层)
pub const VOXEL_GRANITE: u8 = 7;
/// 雪 (雪山/极地)
pub const VOXEL_SNOW: u8 = 11;
/// 冰 (极地/高山)
pub const VOXEL_ICE: u8 = 12;
/// 水
pub const VOXEL_WATER: u8 = 30;

/// 3D 密度场 trait
///
/// 返回值 ∈ [0.0, 1.0]:
/// - 0.0 = 纯空气
/// - 1.0 = 纯固体
/// - 0.5 = 等值面（Marching Cubes 提取边界）
///
/// 多层叠加时，高 priority 层覆盖低 priority 层。
pub trait DensityField: Send + Sync {
    /// 采样 (x, y, z) 处的密度值
    fn sample(&self, x: f64, y: f64, z: f64) -> f32;

    /// 查询 (x, y, z) 处的体素材质 ID
    ///
    /// 返回值: 0=空气, 1=泥土, 2=草皮, 3=沙, 5=碎石, 6=石头,
    /// 7=花岗岩, 11=雪, 12=冰, 30=水 — 详见 007 §2
    fn material_at(&self, x: f64, y: f64, z: f64) -> u8;

    /// 该层的优先级
    ///
    /// 高 priority 覆盖低 priority。
    /// 生成基底 (L0-L6) < 文化覆盖 (L7) < NPC修改 (L8)
    /// < 玩家SDF (L9) < 天气临时 (L10)
    fn priority(&self) -> u8;

    /// 如果本密度场是高度场，返回 (x,z) 处的地表高度（米）。
    ///
    /// 默认 `None`——纯 3D 密度场（洞穴/SDF/矿脉）不需要覆写。
    /// 覆写此方法可让 MC/Transvoxel 使用 2D 高度缓存优化
    /// （O(N²) 噪声调用 vs O(N³)）。
    fn height_at(&self, _x: f64, _z: f64) -> Option<f64> {
        None
    }
}

/// 高度场密度函数
///
/// 把 2D 噪声高度映射为 3D 密度——地表 ±1m 过渡带产生平滑等值面。
/// 不引入新的 3D 噪声——消费已有的 `WorldNoise::sample_height()`。
///
/// 可挂载 `BiomeClassifier` 以确定体素材质 ID。
pub struct HeightfieldDensity {
    noise: WorldNoise,
    /// 地表过渡带半宽（米）——默认 1.0
    half_band: f64,
    /// 群系分类器——用于判定地表材质
    biome_classifier: Option<BiomeClassifier>,
}

impl HeightfieldDensity {
    pub fn new(noise: WorldNoise) -> Self {
        Self {
            noise,
            half_band: 3.0, // 6m 过渡带 — 消除 MC 细胞面拓扑不一致裂缝
            biome_classifier: None,
        }
    }

    pub fn new_with_params(noise: WorldNoise, biome_classifier: Option<BiomeClassifier>) -> Self {
        Self {
            noise,
            half_band: 3.0, // 6m 过渡带 — 消除 MC 细胞面拓扑不一致裂缝
            biome_classifier,
        }
    }

    /// 挂载群系分类器——之后 `material_at()` 群系优先
    pub fn with_biomes(mut self, classifier: BiomeClassifier) -> Self {
        self.biome_classifier = Some(classifier);
        self
    }

    /// 获取噪声引用（供外部消费）
    pub fn noise(&self) -> &WorldNoise {
        &self.noise
    }

    /// 在 (x, z) 采样高度
    pub fn height_at(&self, x: f64, z: f64) -> f64 {
        self.noise.sample_height(x, z)
    }
}

impl DensityField for HeightfieldDensity {
    fn sample(&self, x: f64, y: f64, z: f64) -> f32 {
        let h = self.noise.sample_height(x, z);
        // dist > 0 = 地下, dist < 0 = 空中
        let dist = h - y;
        // smoothstep: 从 -half_band(空气) 到 +half_band(固体)
        let t = (dist + self.half_band) / (2.0 * self.half_band);
        t.clamp(0.0, 1.0) as f32
    }

    fn material_at(&self, x: f64, y: f64, z: f64) -> u8 {
        let h = self.noise.sample_height(x, z);

        // 水柱 (h < 0 时，y 在海床 h 之上、海平面 0 之下)
        if h < 0.0 && y > h && y <= 0.0 {
            return VOXEL_WATER;
        }

        // 空气（地表/海平面以上 > 1m）
        let surface = h.max(0.0);
        if y > surface + 1.0 {
            return VOXEL_AIR;
        }

        // ── 地下深层 ─────────────────
        let depth = h - y; // > 0 = 地下深度

        if depth > 20.0 {
            return VOXEL_STONE;
        }

        if depth > 5.0 {
            return VOXEL_GRAVEL;
        }

        if depth > 1.0 {
            return VOXEL_DIRT;
        }

        // ── 地表层 (|y-h| < 1m) ───────
        // 1. 群系优先
        if let Some(ref classifier) = self.biome_classifier {
            use woworld_core::prelude::WorldPos;
            if let Some(biome) = classifier.classify(WorldPos { x, y, z }) {
                return surface_material_to_voxel_id(biome.surface_material);
            }
        }

        // 2. 高度回退
        if h > 500.0 {
            VOXEL_GRANITE
        } else if h > 200.0 {
            VOXEL_STONE
        } else if (0.0..5.0).contains(&h) {
            VOXEL_SAND
        } else {
            VOXEL_GRASS
        }
    }

    fn priority(&self) -> u8 {
        0 // 生成基底 — 最低优先，被所有修改层覆盖
    }

    fn height_at(&self, x: f64, z: f64) -> Option<f64> {
        Some(self.noise.sample_height(x, z))
    }
}

// ── L4 洞穴密度装饰器 ─────────────────

/// L4 洞穴密度参数
///
/// 全部可序列化——未来可从 TOML 加载。
pub struct CaveParams {
    /// Worley 3D 频率 — 隧道网络密度 (default: 0.04)
    pub frequency: f64,
    /// |F1-F2| 阈值 — 低于此值为洞穴 (default: 0.012 → ~3% 空洞)
    pub threshold: f64,
    /// 陆地洞穴带：地表下方 top 到 bottom (default: 20, 80)
    pub land_top: f64,
    pub land_bottom: f64,
    /// 海底洞穴带：海床下方 top 到 bottom (default: 10, 40)
    pub sea_top: f64,
    pub sea_bottom: f64,
}

impl Default for CaveParams {
    fn default() -> Self {
        Self {
            frequency: 0.04,
            threshold: 0.012,
            land_top: 20.0,
            land_bottom: 80.0,
            sea_top: 10.0,
            sea_bottom: 40.0,
        }
    }
}

/// L4 地质特征密度装饰器 — 洞穴/隧道
///
/// 装饰器模式：包裹基底密度场，在洞穴深度带内覆写密度。
/// 洞穴带外及非洞穴区域全部委托给 base。
///
/// ## 深度带
///
/// - 陆地 (height_at > 0): `[h - land_bottom, h - land_top]`
/// - 海底 (height_at ≤ 0): `[h - sea_bottom, h - sea_top]`
///
/// 硬切分带（无过渡），海岸线处隧道可能突然终止——真实洞穴也因地质变化终止。
///
/// ## 水下洞穴
///
/// 海底洞穴与陆地洞穴行为完全相同——密度设为 0.0（固体中的空洞）。
/// 洞穴口若触及海床，水体自然填充空洞——这是正确行为，非 bug。
pub struct CaveDensity {
    base: Box<dyn DensityField>,
    perm: PermutationTable,
    params: CaveParams,
}

impl CaveDensity {
    /// 创建洞穴密度装饰器
    ///
    /// `master_seed` 是世界级种子——洞穴 seed 通过 `derive_noise_seed()` 独立派生。
    pub fn new(master_seed: u64, params: CaveParams, base: Box<dyn DensityField>) -> Self {
        let cave_seed =
            derive_noise_seed(master_seed, NOISE_DISCRIMINANT_WORLEY_CAVE);
        Self {
            base,
            perm: PermutationTable::new(cave_seed),
            params,
        }
    }

    /// 返回 (x,z) 处的表面高度（委托 base）
    fn surface_at(&self, x: f64, z: f64) -> f64 {
        self.base.height_at(x, z).unwrap_or(0.0)
    }

    /// 该 (x,z) 处使用的深度带 → (top, bottom) 相对地表距离
    fn depth_band(&self, x: f64, z: f64) -> (f64, f64) {
        let h = self.surface_at(x, z);
        if h > 0.0 {
            (self.params.land_top, self.params.land_bottom)
        } else {
            (self.params.sea_top, self.params.sea_bottom)
        }
    }

    /// y 是否在洞穴深度带内
    fn in_band(&self, x: f64, y: f64, z: f64) -> bool {
        let h = self.surface_at(x, z);
        let (top, bottom) = self.depth_band(x, z);
        y >= h - bottom && y <= h - top
    }

    /// 3D Worley |F2-F1| 洞穴判定
    fn is_cave(&self, x: f64, y: f64, z: f64) -> bool {
        worley_3d_f2f1(&self.perm, self.params.frequency, [x, y, z])
            < self.params.threshold
    }
}

impl DensityField for CaveDensity {
    fn sample(&self, x: f64, y: f64, z: f64) -> f32 {
        if self.in_band(x, y, z) && self.is_cave(x, y, z) {
            0.0
        } else {
            self.base.sample(x, y, z)
        }
    }

    fn material_at(&self, x: f64, y: f64, z: f64) -> u8 {
        // 洞穴不创造材质，只移除密度——洞壁是什么石头就是什么石头
        self.base.material_at(x, y, z)
    }

    fn priority(&self) -> u8 {
        4 // L4 地质特征 — 覆盖 L0-L3，被 L5+ 覆盖
    }

    fn height_at(&self, x: f64, z: f64) -> Option<f64> {
        // 洞穴不改变地表高度
        self.base.height_at(x, z)
    }
}

// ── DensityStack — 多层密度容器 ────────

/// 多层密度场 builder
///
/// 装饰器链容器 — 高优先级包裹低优先级。
/// `DensityStack::new(base).with_cave_layer(seed, params).as_density()` 返回链顶。
///
/// ```text
/// DensityStack
///   └── CaveDensity(L4)
///         └── HeightfieldDensity(L0)
/// ```
pub struct DensityStack {
    top: Box<dyn DensityField>,
}

impl DensityStack {
    /// 用基底密度场创建栈
    ///
    /// `base` 必须是 `DensityField + 'static`（满足 `Box<dyn DensityField>` 的生命周期）
    pub fn new(base: impl DensityField + 'static) -> Self {
        Self {
            top: Box::new(base),
        }
    }

    /// 挂载洞穴密度装饰器
    ///
    /// 包裹当前栈顶 → 新栈顶 = `CaveDensity { base: self.top }`
    pub fn with_cave_layer(mut self, master_seed: u64, params: CaveParams) -> Self {
        let cave = CaveDensity::new(master_seed, params, self.top);
        self.top = Box::new(cave);
        self
    }

    /// 返回栈顶的 trait object 引用（供 Transvoxel 消费）
    pub fn as_density(&self) -> &dyn DensityField {
        &*self.top
    }
}

/// SurfaceMaterial → 体素材质 ID 映射
///
/// 从 woworld_core 21 变体映射到 007 §2 定义的 u8 体素材质 ID。
/// 仅用于 HeightfieldDensity 的地表层材质判定。
fn surface_material_to_voxel_id(mat: woworld_core::material::SurfaceMaterial) -> u8 {
    use woworld_core::material::SurfaceMaterial::*;
    match mat {
        Water => VOXEL_WATER,
        Sand => VOXEL_SAND,
        Grass => VOXEL_GRASS,
        Rock => VOXEL_GRANITE,
        Stone => VOXEL_STONE,
        Gravel => VOXEL_GRAVEL,
        Snow => VOXEL_SNOW,
        Ice => VOXEL_ICE,
        Mud => VOXEL_DIRT,
        _ => VOXEL_DIRT, // 默认泥土
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_density() -> HeightfieldDensity {
        HeightfieldDensity::new(WorldNoise::new(42))
    }

    #[test]
    fn test_material_at_air_above_surface() {
        let d = make_density();
        // 选陆地坐标
        let h = d.height_at(500.0, 500.0);
        let surface = h.max(0.0);
        let mat = d.material_at(500.0, surface + 50.0, 500.0);
        assert_eq!(mat, VOXEL_AIR, "above surface should be air");
    }

    #[test]
    fn test_material_at_underground() {
        let d = make_density();
        // 选一个大概率在陆地上的坐标（沿海岸附近）
        let h = d.height_at(500.0, 500.0);
        if h > 10.0 {
            // 地下 30m → 应当是非空气
            let mat = d.material_at(500.0, h - 30.0, 500.0);
            assert_ne!(mat, VOXEL_AIR, "underground should not be air");
            assert!(
                mat != VOXEL_WATER,
                "underground should not be water, got {}",
                mat
            );
        }
        // 若 h <= 10m（罕见：该点是海洋），跳过
    }

    #[test]
    fn test_material_at_deterministic() {
        let d1 = make_density();
        let d2 = make_density();
        for x in (0..50).step_by(10) {
            for z in (0..50).step_by(10) {
                let m1 = d1.material_at(x as f64, 0.0, z as f64);
                let m2 = d2.material_at(x as f64, 0.0, z as f64);
                assert_eq!(m1, m2, "material_at should be deterministic");
            }
        }
    }

    #[test]
    fn test_priority_is_minimum() {
        let d = make_density();
        assert_eq!(
            d.priority(),
            0,
            "generative base should have lowest priority"
        );
    }

    #[test]
    fn test_sample_deterministic() {
        let d1 = make_density();
        let d2 = make_density();
        for x in (0..50).step_by(10) {
            for z in (0..50).step_by(10) {
                let s1 = d1.sample(x as f64, 10.0, z as f64);
                let s2 = d2.sample(x as f64, 10.0, z as f64);
                assert_eq!(s1, s2, "sample should be deterministic");
            }
        }
    }

    // ── CaveDensity 测试 ─────────────────

    fn make_cave() -> CaveDensity {
        let base = HeightfieldDensity::new(WorldNoise::new(42));
        CaveDensity::new(42, CaveParams::default(), Box::new(base))
    }

    #[test]
    fn test_cave_sample_delegates_above_band() {
        let c = make_cave();
        // 陆地上某坐标，y 远在地表之上 → 委托 base
        let h = c.height_at(500.0, 500.0).unwrap();
        let sky = h + 100.0;
        let cave_val = c.sample(500.0, sky, 500.0);
        let base_val = HeightfieldDensity::new(WorldNoise::new(42)).sample(500.0, sky, 500.0);
        assert_eq!(cave_val, base_val, "above cave band must delegate to base");
    }

    #[test]
    fn test_cave_sample_delegates_below_band() {
        let c = make_cave();
        let h = c.height_at(500.0, 500.0).unwrap();
        // 远在洞穴带之下（地表下 200m）
        let deep = h - 200.0;
        let cave_val = c.sample(500.0, deep, 500.0);
        let base_val = HeightfieldDensity::new(WorldNoise::new(42)).sample(500.0, deep, 500.0);
        assert_eq!(
            cave_val, base_val,
            "below cave band must delegate to base"
        );
    }

    #[test]
    fn test_cave_material_delegates_to_base() {
        let c = make_cave();
        let h = c.height_at(500.0, 500.0).unwrap();
        // 洞穴深度带内的任意点 — material_at 仍委托 base
        let mat = c.material_at(500.0, h - 30.0, 500.0);
        let base_mat =
            HeightfieldDensity::new(WorldNoise::new(42)).material_at(500.0, h - 30.0, 500.0);
        assert_eq!(mat, base_mat, "CaveDensity must delegate material_at");
    }

    #[test]
    fn test_cave_height_delegates_to_base() {
        let c = make_cave();
        let h = c.height_at(500.0, 500.0);
        assert!(h.is_some(), "cave height_at must delegate to base");
    }

    #[test]
    fn test_cave_priority_is_4() {
        let c = make_cave();
        assert_eq!(c.priority(), 4);
    }

    #[test]
    fn test_cave_sea_band_differs_from_land() {
        // 构造一个人工场景：基底高度 < 0（海底），确认使用海底深度带
        //
        // 使用 `CaveDensity.sample()` 在海底下方 30m —
        // 这在海底带 [h-40, h-10] 内 → 触发 Worley → 返回有效密度
        let base = HeightfieldDensity::new(WorldNoise::new(42));
        let cave = CaveDensity::new(42, CaveParams::default(), Box::new(base));

        // 寻找一个已知海洋坐标（height ≤ 0）
        let mut found_sea = false;
        for x in (0..2000).step_by(100) {
            for z in (0..2000).step_by(100) {
                if let Some(h) = cave.height_at(x as f64, z as f64) {
                    if h <= 0.0 {
                        // 海底下方 30m → 应在海底带 [h-40, h-10] 内
                        let v = cave.sample(x as f64, h - 30.0, z as f64);
                        assert!(v >= 0.0 && v <= 1.0, "sea cave density must be valid");
                        found_sea = true;
                        break;
                    }
                }
            }
            if found_sea {
                break;
            }
        }
        assert!(found_sea, "should find at least one sea point with seed 42");
    }

    // ── DensityStack 测试 ────────────────

    #[test]
    fn test_density_stack_outer_wins() {
        let base = HeightfieldDensity::new(WorldNoise::new(42));
        let stack = DensityStack::new(base)
            .with_cave_layer(42, CaveParams::default());
        let d = stack.as_density();
        // 栈顶是 CaveDensity → priority = 4
        assert_eq!(d.priority(), 4, "top of stack must be CaveDensity");
        // 栈顶通过 height_at 委托 base → height 可用
        assert!(
            d.height_at(500.0, 500.0).is_some(),
            "stack must delegate height_at to base"
        );
    }

    #[test]
    fn test_cave_sample_deterministic() {
        let make = || {
            let base = HeightfieldDensity::new(WorldNoise::new(42));
            let c = CaveDensity::new(42, CaveParams::default(), Box::new(base));
            // 在洞穴带内采样
            let h = c.height_at(500.0, 500.0).unwrap();
            (c.sample(500.0, h - 30.0, 500.0), c.sample(500.0, h - 50.0, 500.0))
        };
        let (a1, a2) = make();
        let (b1, b2) = make();
        assert_eq!(a1, b1, "cave sample must be deterministic");
        assert_eq!(a2, b2, "cave sample must be deterministic");
    }

    #[test]
    fn test_cave_sample_in_band_zero_when_cave() {
        // 遍历大量点——洞穴带内至少有一些点返回 0.0（有洞穴生成）
        let base = HeightfieldDensity::new(WorldNoise::new(42));
        let cave = CaveDensity::new(42, CaveParams::default(), Box::new(base));
        let h = cave.height_at(500.0, 500.0).unwrap();

        let mut zero_count = 0;
        let total = 100;
        for i in 0..total {
            let x = 500.0 + (i as f64) * 1.5;
            let z = 500.0 + (i as f64) * 0.7;
            let v = cave.sample(x, h - 40.0, z);
            if v == 0.0 {
                zero_count += 1;
            }
        }
        // 默认参数下应当有少量但非零的洞穴
        assert!(
            zero_count > 0,
            "with default params some cave voids should exist"
        );
        assert!(
            zero_count < 50,
            "cave voids should not dominate ({}/{})",
            zero_count,
            total
        );
    }
}
