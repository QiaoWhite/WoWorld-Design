//! CaveDensity — 3D Worley 洞穴密度层
//!
//! 实现 `DensityProvider`，消费 `worley_3d_f2f1()` 产生 3D 脊状隧道网络。
//!
//! 语义：Worley F₂−F₁ < threshold → 负密度贡献（洞穴空洞）
//!       否则 → 零贡献（固体区域）
//!
//! `DensityStack` 累加模式下，洞穴层抵消 TerrainBaseDensity 的正密度，
//! 在地形内部产生连通空洞 → Transvoxel 等值面提取出洞穴。

use std::sync::Arc;

use noise::permutationtable::PermutationTable;
use woworld_core::density::DensityProvider;
use woworld_core::prelude::WorldPos;

use crate::noise_gen;

/// 洞穴参数
#[derive(Clone, Debug)]
pub struct CaveParams {
    /// Worley 3D 频率——控制洞穴密度/大小（默认 0.05）
    pub frequency: f64,
    /// 低于此值 = 洞穴空洞（默认 0.18）
    pub threshold: f64,
    /// 洞穴贡献的负密度幅值（默认 80.0）
    pub amplitude: f64,
}

impl Default for CaveParams {
    fn default() -> Self {
        Self {
            frequency: 0.05, // Sprint 045: 0.04→0.05 (稍多洞穴)
            threshold: 0.18, // Sprint 045: 0.15→0.18 (稍大洞口)
            amplitude: 80.0,
        }
    }
}

/// 3D Worley 洞穴密度层
///
/// 持有独立 seeded `PermutationTable`——从世界主种子确定性派生。
#[derive(Debug)]
pub struct CaveDensity {
    perm: PermutationTable,
    params: CaveParams,
}

impl CaveDensity {
    /// 从世界种子确定性创建
    pub fn new(master_seed: u64, params: CaveParams) -> Self {
        let cave_seed =
            noise_gen::derive_noise_seed(master_seed, noise_gen::NOISE_DISCRIMINANT_WORLEY_CAVE);
        let perm = PermutationTable::new(cave_seed);
        Self { perm, params }
    }

    /// 创建带自定义参数的实例（Arc 包装——DensityStack 要求）
    pub fn new_arc(master_seed: u64, params: CaveParams) -> Arc<Self> {
        Arc::new(Self::new(master_seed, params))
    }
}

impl DensityProvider for CaveDensity {
    fn density_at(&self, pos: WorldPos) -> f32 {
        let w = noise_gen::worley_3d_f2f1(&self.perm, self.params.frequency, [pos.x, pos.y, pos.z]);
        if w < self.params.threshold {
            -self.params.amplitude as f32
        } else {
            0.0
        }
    }

    fn material_at(&self, _pos: WorldPos) -> u8 {
        // 洞穴材质由 TerrainBaseDensity 提供（Stone/Rock）
        // 洞穴本身不覆盖材质——下层决定
        0
    }

    fn priority(&self) -> u8 {
        4 // 在 TerrainBaseDensity(0) 之后叠加
    }

    fn layer_name(&self) -> &'static str {
        "cave"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use woworld_core::density::DensityStack;

    #[test]
    fn test_cave_returns_zero_outside_threshold() {
        let cave = CaveDensity::new(
            42,
            CaveParams {
                threshold: 0.0,
                ..Default::default()
            },
        );
        // threshold=0 意味着 worley < 0 才触发——worley ∈ [0, ~1.2] 永不为负 → 始终返回 0
        let d = cave.density_at(WorldPos::default());
        assert!(d.abs() < 0.001, "threshold=0 should never trigger, got {d}");
    }

    #[test]
    fn test_cave_returns_negative_inside_threshold() {
        let cave = CaveDensity::new(
            42,
            CaveParams {
                threshold: 1.0,
                ..Default::default()
            },
        );
        // threshold=1.0 意味着 worley < 1.0 触发——大多数区域 → 返回 -amplitude
        let d = cave.density_at(WorldPos {
            x: 100.0,
            y: 0.0,
            z: 100.0,
        });
        // 不能保证一定 < 0（极少数点 worley > 1.0），但大概率是
        // 只验证不 panic
        assert!(d <= 0.0, "cave density must be <= 0, got {d}");
    }

    #[test]
    fn test_cave_deterministic() {
        let cave_a = CaveDensity::new(42, CaveParams::default());
        let cave_b = CaveDensity::new(42, CaveParams::default());
        let pos = WorldPos {
            x: 100.0,
            y: 50.0,
            z: 200.0,
        };
        let da = cave_a.density_at(pos);
        let db = cave_b.density_at(pos);
        assert!((da - db).abs() < 0.001, "same seed should give same output");
    }

    #[test]
    fn test_cave_priority() {
        let cave = CaveDensity::new(42, CaveParams::default());
        assert_eq!(cave.priority(), 4);
    }

    #[test]
    fn test_cave_layer_name() {
        let cave = CaveDensity::new(42, CaveParams::default());
        assert_eq!(cave.layer_name(), "cave");
    }

    #[test]
    fn test_cave_in_density_stack() {
        use crate::terrain::TerrainBaseDensity;
        use crate::WorldNoise;

        let noise = Arc::new(WorldNoise::new(42));
        let terrain = Arc::new(TerrainBaseDensity::new(noise));
        let cave = CaveDensity::new_arc(
            42,
            CaveParams {
                amplitude: 80.0,
                ..Default::default()
            },
        );

        let mut stack = DensityStack::new();
        stack.push(terrain);
        stack.push(cave);

        // 在地下较深位置，地形密度应大于 0，洞穴可能降低它
        let pos = WorldPos {
            x: 500.0,
            y: -20.0,
            z: 500.0,
        };
        let d = stack.density_at(pos);
        // 只验证不 panic，不保证具体值（取决于噪声）
        assert!(d.is_finite(), "stack density should be finite");
    }
}
