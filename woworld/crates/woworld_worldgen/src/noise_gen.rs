//! 双层 Perlin 噪声地形生成器
//!
//! 三层噪声叠加：
//! - continent (~100km 波长): 海陆分布
//! - detail (~100m 波长): 地形起伏
//! - mountain (~500m 波长): 山脊

use std::sync::Arc;

use glam::Vec3;
use noise::permutationtable::{NoiseHasher, PermutationTable};
use noise::{NoiseFn, Perlin};
use serde::Deserialize;

/// 64-bit mixing hash (splitmix64 finalizer)
fn mix64(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}

/// 噪声参数——可调
#[derive(Clone, Debug, Deserialize)]
pub struct NoiseParams {
    pub continent_scale: f64,  // 默认 0.00001 (100km 波长)
    pub detail_scale: f64,     // 默认 0.01 (100m 波长)
    pub mountain_scale: f64,   // 默认 0.002 (500m 波长)
    pub sea_threshold: f64,    // 默认 0.3 (海:陆≈7:3)
    pub height_amplitude: f64, // 默认 350.0 (最高~700m, v3.0 spec)
    pub sea_depth: f64,        // 默认 400.0 (最深-400m)
    pub climate_scale: f64,    // 默认 0.0005 (~2km 波长, 温度/降水噪声)
}

impl NoiseParams {
    /// 从 TOML 字符串加载（编译期嵌入——零运行时 I/O）
    pub fn from_toml_str(toml_str: &str) -> Result<Self, String> {
        #[derive(Deserialize)]
        struct NoiseParamsToml {
            noise: NoiseParams,
        }
        toml::from_str::<NoiseParamsToml>(toml_str)
            .map(|t| t.noise)
            .map_err(|e| format!("Failed to parse noise_params.toml: {}", e))
    }
}

impl Default for NoiseParams {
    fn default() -> Self {
        Self {
            continent_scale: 0.00001,
            detail_scale: 0.01,
            mountain_scale: 0.002,
            sea_threshold: 0.3,
            height_amplitude: 350.0,
            sea_depth: 400.0,
            climate_scale: 0.0005,
        }
    }
}

/// 世界噪声生成器
#[derive(Clone, Debug)]
pub struct WorldNoise {
    continent: Perlin,
    detail: Perlin,
    mountain: Perlin,
    temperature: Perlin,
    precipitation: Perlin,
    pub params: NoiseParams,
}

impl WorldNoise {
    /// 使用种子创建——不同种子 → 不同世界
    pub fn new(seed: u64) -> Self {
        let seeds = derive_perlin_seeds(seed);
        Self {
            continent: Perlin::new(seeds[0]),
            detail: Perlin::new(seeds[1]),
            mountain: Perlin::new(seeds[2]),
            temperature: Perlin::new(seeds[3]),
            precipitation: Perlin::new(seeds[4]),
            params: NoiseParams::default(),
        }
    }

    pub fn with_params(seed: u64, params: NoiseParams) -> Arc<Self> {
        let seeds = derive_perlin_seeds(seed);
        Arc::new(Self {
            continent: Perlin::new(seeds[0]),
            detail: Perlin::new(seeds[1]),
            mountain: Perlin::new(seeds[2]),
            temperature: Perlin::new(seeds[3]),
            precipitation: Perlin::new(seeds[4]),
            params,
        })
    }

    /// 采样 (x, z) 处的地形高度（米）
    ///
    /// 算法:
    /// 1. 大陆噪声 → continent_value ∈ [-1, 1]
    /// 2. continent_value > sea_threshold → 陆地，否则 → 海洋
    /// 3. 陆地: 叠加 detail + mountain 噪声 → 高度
    /// 4. 海洋: 负高度（海床）
    pub fn sample_height(&self, x: f64, z: f64) -> f64 {
        let p = &self.params;
        // 无理数相位偏移——确保原点不在 Perlin 整数格点（Perlin(0,0)≡0 对所有 seed）
        // φ⁻¹ (黄金比例倒数) 和 1−φ⁻¹ 确保任何缩放比下都不回到格点
        const PHI_INV: f64 = 0.6180339887498949;
        let continent_val = self.continent.get([
            x * p.continent_scale + PHI_INV,
            z * p.continent_scale + (1.0 - PHI_INV),
        ]);

        if continent_val > p.sea_threshold {
            // 陆地——叠层
            let land_factor = (continent_val - p.sea_threshold) / (1.0 - p.sea_threshold); // 0→1 越接近大陆中心越高

            let detail_val = self.detail.get([x * p.detail_scale, z * p.detail_scale]);

            let mountain_val = self
                .mountain
                .get([x * p.mountain_scale, z * p.mountain_scale]);

            // mountain 在山脊区域（detail > 0.3）更突出
            let mountain_factor = if detail_val > 0.3 {
                (detail_val - 0.3) / 0.7
            } else {
                0.0
            };

            let base_height = land_factor * 280.0; // 海岸→内陆基准上升 (~280m 内陆平原)
            let detail_height = land_factor * detail_val * p.height_amplitude * 0.6;
            let mountain_height =
                land_factor * mountain_val * mountain_factor * p.height_amplitude * 0.4;

            base_height + detail_height + mountain_height
        } else {
            // 海洋——海床深度（非线性：近岸浅，远岸快速加深）
            let sea_factor = (p.sea_threshold - continent_val) / (p.sea_threshold + 1.0); // 0→1
            let ocean_depth = sea_factor.powf(0.4) * p.sea_depth;

            // 海床细部起伏——detail noise 加权，近岸更明显
            let ocean_detail = self
                .detail
                .get([x * p.detail_scale * 0.5, z * p.detail_scale * 0.5]);
            -ocean_depth + ocean_detail * sea_factor * 50.0
        }
    }

    /// 采样 (x, z) 处的归一化温度 (0.0 = 极寒, 1.0 = 酷热)
    pub fn sample_temperature(&self, x: f64, z: f64) -> f64 {
        let raw = self
            .temperature
            .get([x * self.params.climate_scale, z * self.params.climate_scale]);
        // Perlin ∈ [-1, 1] → [0, 1]
        (raw + 1.0) * 0.5
    }

    /// 采样 (x, z) 处的归一化降水 (0.0 = 极旱, 1.0 = 极湿)
    pub fn sample_precipitation(&self, x: f64, z: f64) -> f64 {
        let raw = self
            .precipitation
            .get([x * self.params.climate_scale, z * self.params.climate_scale]);
        (raw + 1.0) * 0.5
    }

    /// 有限差分计算 (x, z) 处的表面法线
    ///
    /// 使用 `sample_height` 的对称有限差分，eps 为步长 (m)。
    pub fn calc_normal(&self, x: f64, z: f64, eps: f64) -> Vec3 {
        let h_center = self.sample_height(x, z);
        let h_right = self.sample_height(x + eps, z);
        let h_forward = self.sample_height(x, z + eps);

        let dx = (h_center - h_right) / eps;
        let dz = (h_center - h_forward) / eps;

        // 法线 = normalize(-dh/dx, 1.0, -dh/dz)
        let n = Vec3::new(-dx as f32, 1.0, -dz as f32);
        n.normalize()
    }
}

/// 从 u64 seed 派生 5 个独立 u32 seed 给 Perlin 实例
///
/// 使用 mix64 链式派生：每个 seed 从前一个 mix64 输出截取低 32 位。
/// noise::Perlin::new() 仅接受 u32，5 个实例各需独立 seed。
fn derive_perlin_seeds(seed: u64) -> [u32; 5] {
    let mut s = seed;
    let mut out = [0u32; 5];
    for item in &mut out {
        s = mix64(s);
        *item = s as u32;
    }
    out
}

// ── 噪声判别符 ──────────────────────

/// 噪声类型判别符：Worley 洞穴
///
/// 用于 `derive_noise_seed()` — 确保洞穴噪声种子与 Perlin 系列完全正交。
pub const NOISE_DISCRIMINANT_WORLEY_CAVE: u64 = 10;

/// 带判别符的种子派生
///
/// `derive_noise_seed(master_seed, discriminant) -> u32`
///
/// 从主世界 seed 确定性派生独立噪声 seed。
/// 判别符确保不同噪声类型获得独立种子空间，永不碰撞。
///
/// # 确定性
/// 同 seed + 同 discriminant → 永远同输出。
pub fn derive_noise_seed(master_seed: u64, discriminant: u64) -> u32 {
    mix64(master_seed ^ discriminant) as u32
}

// ── Worley 3D |F₂ − F₁| ─────────────────

/// get_vec3 — 3D Worley 种子点偏移查找表
///
/// 从 `noise` crate (0.9.0) `core::worley::get_vec3` 复制。
/// 将哈希索引映射到单位细胞内的确定性子点位置。
/// 纯数学表 — 永久跨平台一致。
#[rustfmt::skip]
#[inline]
fn get_vec3(index: usize) -> [f64; 3] {
    let length = ((index & 0xE0) >> 5) as f64 * 0.5 / 7.0;
    let diag = length * std::f64::consts::FRAC_1_SQRT_2;

    match index % 18 {
        0  => [   diag,    diag,     0.0],
        1  => [   diag,   -diag,     0.0],
        2  => [  -diag,    diag,     0.0],
        3  => [  -diag,   -diag,     0.0],
        4  => [   diag,     0.0,    diag],
        5  => [   diag,     0.0,   -diag],
        6  => [  -diag,     0.0,    diag],
        7  => [  -diag,     0.0,   -diag],
        8  => [    0.0,    diag,    diag],
        9  => [    0.0,    diag,   -diag],
        10 => [    0.0,   -diag,    diag],
        11 => [    0.0,   -diag,   -diag],
        12 => [ length,     0.0,     0.0],
        13 => [    0.0,  length,     0.0],
        14 => [    0.0,     0.0,  length],
        15 => [-length,     0.0,     0.0],
        16 => [    0.0, -length,     0.0],
        17 => [    0.0,     0.0, -length],
        _ => unreachable!(),
    }
}

/// 3D Worley |F₂ − F₁| 脊状噪声
///
/// 返回最近两个特征点距离的差值 ∈ [0, ~1.2]。
///
/// 低值 → 查询点在两个特征点的等距面附近 → 脊状连通隧道网络。
/// 高值 → 查询点靠近单个特征点 → 固体区域。
///
/// 算法：遍历 3×3×3 邻域（27 个细胞）→ 对每个细胞计算其种子点距离 →
/// 追踪最短 (F₁) 和次短 (F₂) → 返回 F₂ − F₁。
///
/// 基于 `noise::core::worley::worley_3d` 算法结构，扩展为双距离追踪。
pub fn worley_3d_f2f1(hasher: &PermutationTable, frequency: f64, point: [f64; 3]) -> f64 {
    let [px, py, pz] = [
        point[0] * frequency,
        point[1] * frequency,
        point[2] * frequency,
    ];

    let cx = px.floor() as isize;
    let cy = py.floor() as isize;
    let cz = pz.floor() as isize;

    let mut f1 = f64::MAX;
    let mut f2 = f64::MAX;

    for dx in -1..=1 {
        for dy in -1..=1 {
            for dz in -1..=1 {
                let cell = [cx + dx, cy + dy, cz + dz];
                let index = hasher.hash(&cell);
                let [ox, oy, oz] = get_vec3(index);
                let sx = cell[0] as f64 + ox;
                let sy = cell[1] as f64 + oy;
                let sz = cell[2] as f64 + oz;

                let dist = ((px - sx).powi(2) + (py - sy).powi(2) + (pz - sz).powi(2)).sqrt();

                if dist < f1 {
                    f2 = f1;
                    f1 = dist;
                } else if dist < f2 {
                    f2 = dist;
                }
            }
        }
    }

    f2 - f1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic() {
        let n1 = WorldNoise::new(42);
        let n2 = WorldNoise::new(42);
        for x in (0..100).step_by(10) {
            for z in (0..100).step_by(10) {
                assert_eq!(
                    n1.sample_height(x as f64, z as f64),
                    n2.sample_height(x as f64, z as f64)
                );
            }
        }
    }

    #[test]
    fn test_different_seeds_different_worlds() {
        let n1 = WorldNoise::new(42);
        let n2 = WorldNoise::new(99);
        // 至少有一半的点不同（统计概率极高）
        let mut diff = 0;
        for x in 0..50 {
            for z in 0..50 {
                let h1 = n1.sample_height(x as f64 * 10.0, z as f64 * 10.0);
                let h2 = n2.sample_height(x as f64 * 10.0, z as f64 * 10.0);
                if (h1 - h2).abs() > 0.01 {
                    diff += 1;
                }
            }
        }
        assert!(diff > 1250); // >50%
    }

    #[test]
    fn test_height_range() {
        let n = WorldNoise::new(123);
        let mut min_h = f64::MAX;
        let mut max_h = f64::MIN;
        for x in 0..100 {
            for z in 0..100 {
                let h = n.sample_height(x as f64 * 50.0, z as f64 * 50.0);
                min_h = min_h.min(h);
                max_h = max_h.max(h);
            }
        }
        // 高度范围合理（新参数：最高 ~630m, 最深 ~-400m）
        assert!(min_h >= -450.0, "min too low: {}", min_h);
        assert!(max_h <= 800.0, "max too high: {}", max_h);
    }

    #[test]
    fn test_temperature_range() {
        let n = WorldNoise::new(42);
        for x in 0..100 {
            for z in 0..100 {
                let t = n.sample_temperature(x as f64 * 50.0, z as f64 * 50.0);
                assert!(t >= 0.0 && t <= 1.0, "temperature out of [0,1]: {}", t);
            }
        }
    }

    #[test]
    fn test_precipitation_range() {
        let n = WorldNoise::new(42);
        for x in 0..100 {
            for z in 0..100 {
                let p = n.sample_precipitation(x as f64 * 50.0, z as f64 * 50.0);
                assert!(p >= 0.0 && p <= 1.0, "precipitation out of [0,1]: {}", p);
            }
        }
    }

    #[test]
    fn test_climate_deterministic() {
        let n1 = WorldNoise::new(42);
        let n2 = WorldNoise::new(42);
        for x in (0..100).step_by(10) {
            for z in (0..100).step_by(10) {
                assert_eq!(
                    n1.sample_temperature(x as f64, z as f64),
                    n2.sample_temperature(x as f64, z as f64)
                );
                assert_eq!(
                    n1.sample_precipitation(x as f64, z as f64),
                    n2.sample_precipitation(x as f64, z as f64)
                );
            }
        }
    }

    // ── u64 seed 测试 ─────────────────

    #[test]
    fn test_seed_u64_deterministic() {
        let n1 = WorldNoise::new(u64::MAX);
        let n2 = WorldNoise::new(u64::MAX);
        for x in (0..100).step_by(10) {
            for z in (0..100).step_by(10) {
                assert_eq!(
                    n1.sample_height(x as f64, z as f64),
                    n2.sample_height(x as f64, z as f64)
                );
            }
        }
    }

    #[test]
    fn test_seed_u64_different_worlds() {
        let n1 = WorldNoise::new(42u64);
        let n2 = WorldNoise::new(99u64);
        let mut diff = 0;
        for x in 0..50 {
            for z in 0..50 {
                let h1 = n1.sample_height(x as f64 * 10.0, z as f64 * 10.0);
                let h2 = n2.sample_height(x as f64 * 10.0, z as f64 * 10.0);
                if (h1 - h2).abs() > 0.01 {
                    diff += 1;
                }
            }
        }
        assert!(diff > 1250); // >50%
    }

    #[test]
    fn test_derive_perlin_seeds_deterministic() {
        let a = derive_perlin_seeds(42);
        let b = derive_perlin_seeds(42);
        assert_eq!(a, b);
    }

    // ── Worley 3D F2-F1 测试 ──

    #[test]
    fn test_worley_f2f1_deterministic() {
        let hasher = PermutationTable::new(42);
        let a = worley_3d_f2f1(&hasher, 0.04, [100.0, -50.0, 200.0]);
        let b = worley_3d_f2f1(&hasher, 0.04, [100.0, -50.0, 200.0]);
        assert_eq!(a, b, "same input must return same output");
    }

    #[test]
    fn test_worley_f2f1_non_negative() {
        let hasher = PermutationTable::new(42);
        for i in 0..100 {
            let x = (i as f64) * 3.7;
            let y = -50.0;
            let z = (i as f64) * 5.1;
            let v = worley_3d_f2f1(&hasher, 0.04, [x, y, z]);
            assert!(v >= 0.0, "|F2-F1| must be non-negative, got {}", v);
        }
    }

    #[test]
    fn test_worley_f2f1_different_seed() {
        let h1 = PermutationTable::new(42);
        let h2 = PermutationTable::new(99);
        let mut diff = 0;
        for i in 0..100 {
            let x = (i as f64) * 3.7;
            let y = -50.0;
            let z = (i as f64) * 5.1;
            let a = worley_3d_f2f1(&h1, 0.04, [x, y, z]);
            let b = worley_3d_f2f1(&h2, 0.04, [x, y, z]);
            if (a - b).abs() > 1e-10 {
                diff += 1;
            }
        }
        assert!(diff > 50, "different seeds should produce different output");
    }

    #[test]
    fn test_derive_noise_seed_deterministic() {
        assert_eq!(derive_noise_seed(42, 10), derive_noise_seed(42, 10));
        assert_eq!(
            derive_noise_seed(u64::MAX, 0),
            derive_noise_seed(u64::MAX, 0)
        );
    }

    #[test]
    fn test_derive_noise_seed_different_discriminant() {
        let a = derive_noise_seed(42, 0);
        let b = derive_noise_seed(42, 10);
        assert_ne!(a, b, "different discriminants must yield different seeds");
    }
}
