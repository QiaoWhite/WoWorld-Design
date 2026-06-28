//! VegetationNoise — 3 层 Perlin 噪声为植被生成提供连续参数场
//!
//! 复用 `hash_stage_seed` 语义：从世界种子确定性派生植被噪声种子。
//! 架构镜像 `WorldNoise`——独立的 Perlin 实例，独立的种子链。

use noise::{NoiseFn, Perlin};
use woworld_worldgen::hash_stage_seed;

/// 植被噪声层（3 Perlin 实例，确定性种子派生）
#[derive(Clone, Debug)]
pub struct VegetationNoise {
    /// 群落密度噪声（~50m 波长）
    density: Perlin,
    /// 演替阶段噪声（~100m 波长）
    succession: Perlin,
    /// 物种混合噪声（~30m 波长）
    species_mix: Perlin,
}

impl VegetationNoise {
    /// 从世界种子构造植被噪声
    ///
    /// 种子链: `world_seed → "P2.25" → 分派 3 个独立 Perlin seed`
    pub fn new(world_seed: u64) -> Self {
        let stage_seed = hash_stage_seed(world_seed, "P2.25");
        // 分派 3 个子 seed（增量偏移确保独立性）
        Self {
            density: Perlin::new((stage_seed.wrapping_add(0)) as u32),
            succession: Perlin::new((stage_seed.wrapping_add(1)) as u32),
            species_mix: Perlin::new((stage_seed.wrapping_add(2)) as u32),
        }
    }

    /// 群落密度值 [0, 1] — 控制单位面积的植被覆盖度
    pub fn sample_density(&self, x: f64, z: f64) -> f64 {
        let v = self.density.get([x / 50.0, z / 50.0]);
        (v + 1.0) * 0.5 // [-1, 1] → [0, 1]
    }

    /// 演替阶段 [0, 1] — 0=先锋期, 1=老熟林
    pub fn sample_succession(&self, x: f64, z: f64) -> f64 {
        let v = self.succession.get([x / 100.0, z / 100.0]);
        (v + 1.0) * 0.5
    }

    /// 物种混合噪声 [0, 1] — 控制伴生种配比
    pub fn sample_species_mix(&self, x: f64, z: f64) -> f64 {
        let v = self.species_mix.get([x / 30.0, z / 30.0]);
        (v + 1.0) * 0.5
    }

    /// 为指定区块派生独立的 Perlin 种子
    ///
    /// 保证同一区块永远产生相同的植被噪声，与加载顺序无关。
    pub fn for_chunk(&self, _stage_seed: u64, _cx: i64, _cz: i64) -> Self {
        // 当前 MVP 复用全局噪声实例；完整 P2.25 管线中每 VMC 独立种子
        self.clone()
    }
}

// ── 测试 ──────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic() {
        let a = VegetationNoise::new(42);
        let b = VegetationNoise::new(42);
        assert!((a.sample_density(100.0, 200.0) - b.sample_density(100.0, 200.0)).abs() < 1e-9);
        assert!(
            (a.sample_succession(100.0, 200.0) - b.sample_succession(100.0, 200.0)).abs() < 1e-9
        );
    }

    #[test]
    fn test_different_seeds_different_worlds() {
        let a = VegetationNoise::new(42);
        let b = VegetationNoise::new(99);
        // 远离原点的位置——避免 Perlin (0,0) 特殊点
        let diff_density = (a.sample_density(73.5, 128.3) - b.sample_density(73.5, 128.3)).abs();
        let diff_succession =
            (a.sample_succession(73.5, 128.3) - b.sample_succession(73.5, 128.3)).abs();
        assert!(diff_density > 0.0 || diff_succession > 0.0);
    }

    #[test]
    fn test_output_in_range() {
        let vn = VegetationNoise::new(42);
        for x in (0..100).step_by(20) {
            for z in (0..100).step_by(20) {
                let d = vn.sample_density(x as f64, z as f64);
                assert!(d >= 0.0 && d <= 1.0, "density {} out of range", d);
                let s = vn.sample_succession(x as f64, z as f64);
                assert!(s >= 0.0 && s <= 1.0, "succession {} out of range", s);
            }
        }
    }
}
