//! AestheticTaste 审美品味 Component — ECS 铁律合规
//!
//! 参见: `开发文档/05-审美与艺术/004-AestheticTaste详细设计.md`
//!
//! 6 维审美权重 + 3 标量参数从 BigFive + CultureParams 派生。
//! Phase 1: 仅 BigFive 项——culture/beauty_standard/父母品味均用中性占位。

use crate::components::bigfive::BigFive;
use crate::prng::pseudo_random_f32_range;

/// 6 个审美信号维度常量
pub const DIM_FLUENCY: usize = 0;
pub const DIM_NOVELTY: usize = 1;
pub const DIM_COMPLEXITY: usize = 2;
pub const DIM_HARMONY: usize = 3;
pub const DIM_EXPRESSIVENESS: usize = 4;
pub const DIM_VIRTUOSITY: usize = 5;

/// 审美品味——决定 NPC 对美的个体化偏好
///
/// 一次性从 BigFive + 文化参数派生，仅在极端文化冲击或审美事件后微调。
/// Phase 1: 文化参数用中性值 0.5 替代。
#[derive(Debug, Clone, Copy)]
pub struct AestheticTaste {
    /// 6 维审美权重 [0.05, 0.95]——越高越重视该维度
    pub dimension_weights: [f32; 6],
    /// 熟悉度偏好 [-1, 1]: -1=求新，1=恋旧
    pub familiarity_bias: f32,
    /// 审美开放度 [0, 1]: 0=封闭(仅接受本文化)，1=极度开放
    pub aesthetic_openness: f32,
    /// 复杂度容忍度 [0, 1]: 0=极简，1=极繁
    pub complexity_tolerance: f32,
    /// 不可变个人种子 (u32)
    pub personal_seed: u32,
}

impl Default for AestheticTaste {
    fn default() -> Self {
        Self {
            dimension_weights: [0.5; 6],
            familiarity_bias: 0.0,
            aesthetic_openness: 0.5,
            complexity_tolerance: 0.5,
            personal_seed: 0,
        }
    }
}

impl AestheticTaste {
    /// 从 BigFive 派生审美品味（含 Phase 1 占位项）
    ///
    /// `seed`: 用于确定性 RNG jitter（通常从 NPC 创建种子派生）
    ///
    /// Phase 1: culture_params 用 0.5 中性占位，beauty_standard 用 0.5 占位，
    /// 父母品味无 → 跳过 rebellion。
    pub fn derive_from_bigfive(b: &BigFive, seed: u64) -> Self {
        let mut weights = [0.5; 6]; // 基线 0.5

        // ── BigFive → 维度权重 (004- §2a) ──
        // openness: NOVELTY +0.25, EXPRESSIVENESS +0.15, COMPLEXITY +0.10
        weights[DIM_NOVELTY] += b.openness * 0.25;
        weights[DIM_EXPRESSIVENESS] += b.openness * 0.15;
        weights[DIM_COMPLEXITY] += b.openness * 0.10;

        // conscientiousness: VIRTUOSITY +0.20, FLUENCY +0.10
        weights[DIM_VIRTUOSITY] += b.conscientiousness * 0.20;
        weights[DIM_FLUENCY] += b.conscientiousness * 0.10;

        // agreeableness: HARMONY +0.20
        weights[DIM_HARMONY] += b.agreeableness * 0.20;

        // extraversion: EXPRESSIVENESS +0.15
        weights[DIM_EXPRESSIVENESS] += b.extraversion * 0.15;

        // neuroticism: FLUENCY -0.15
        weights[DIM_FLUENCY] -= b.neuroticism * 0.15;

        // Phase 1: culture 项用 0.5 中性占位
        // artistry→EXPRESSIVENESS +0.30 → 0.5*0.30=0.15
        // uncertainty_avoidance→FLUENCY +0.20 → 0.5*0.20=0.10
        // individualism→NOVELTY +0.15 → 0.5*0.15=0.075
        // power_distance→VIRTUOSITY +0.25 → 0.5*0.25=0.125
        // indulgence→HARMONY +0.20 → 0.5*0.20=0.10
        weights[DIM_EXPRESSIVENESS] += 0.15;
        weights[DIM_FLUENCY] += 0.10;
        weights[DIM_NOVELTY] += 0.075;
        weights[DIM_VIRTUOSITY] += 0.125;
        weights[DIM_HARMONY] += 0.10;

        // personal jitter ±0.12（从 seed 确定）
        for (i, w) in weights.iter_mut().enumerate() {
            let jitter = pseudo_random_f32_range(seed, i as u64, -0.12, 0.12);
            *w = (*w + jitter).clamp(0.05, 0.95);
        }

        // ── 标量参数 ──
        // familiarity_bias = culture.uncertainty_avoidance×0.6 + (-O)×0.4
        // Phase 1: culture.ua = 0.5
        let familiarity_bias = (0.5 * 0.6 - b.openness * 0.4
            + pseudo_random_f32_range(seed, 6, -0.10, 0.10))
        .clamp(-1.0, 1.0);

        // aesthetic_openness = culture.openness_to_outsiders×0.40 + O×0.40
        //                      + (1-beauty_standard.confidence)×0.20
        // Phase 1: culture openness=0.5, beauty confidence=0.5
        let aesthetic_openness = (0.5 * 0.40
            + b.openness * 0.40
            + (1.0 - 0.5) * 0.20
            + pseudo_random_f32_range(seed, 7, -0.08, 0.08))
        .clamp(0.0, 1.0);

        // complexity_tolerance = O×0.60 + culture.artistry×0.40
        // Phase 1: culture.artistry = 0.5
        let complexity_tolerance =
            (b.openness * 0.60 + 0.5 * 0.40 + pseudo_random_f32_range(seed, 8, -0.10, 0.10))
                .clamp(0.0, 1.0);

        Self {
            dimension_weights: weights,
            familiarity_bias,
            aesthetic_openness,
            complexity_tolerance,
            personal_seed: seed as u32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_in_range() {
        let t = AestheticTaste::default();
        assert!((-1.0..=1.0).contains(&t.familiarity_bias));
        assert!((0.0..=1.0).contains(&t.aesthetic_openness));
        assert!((0.0..=1.0).contains(&t.complexity_tolerance));
    }

    #[test]
    fn test_openness_drives_novelty() {
        let high = BigFive {
            openness: 1.0,
            ..BigFive::default()
        };
        let low = BigFive {
            openness: 0.0,
            ..BigFive::default()
        };
        let t_high = AestheticTaste::derive_from_bigfive(&high, 42);
        let t_low = AestheticTaste::derive_from_bigfive(&low, 42);
        assert!(t_high.dimension_weights[DIM_NOVELTY] > t_low.dimension_weights[DIM_NOVELTY]);
    }

    #[test]
    fn test_neurotic_reduces_fluency() {
        let high_n = BigFive {
            neuroticism: 1.0,
            ..BigFive::default()
        };
        let low_n = BigFive {
            neuroticism: 0.0,
            ..BigFive::default()
        };
        let t_high = AestheticTaste::derive_from_bigfive(&high_n, 42);
        let t_low = AestheticTaste::derive_from_bigfive(&low_n, 42);
        assert!(t_high.dimension_weights[DIM_FLUENCY] < t_low.dimension_weights[DIM_FLUENCY]);
    }

    #[test]
    fn test_agreeable_values_harmony() {
        let b = BigFive {
            agreeableness: 1.0,
            ..BigFive::default()
        };
        let t = AestheticTaste::derive_from_bigfive(&b, 1);
        assert!(t.dimension_weights[DIM_HARMONY] > 0.55);
    }

    #[test]
    fn test_conscientious_values_virtuosity() {
        let b = BigFive {
            conscientiousness: 1.0,
            ..BigFive::default()
        };
        let t = AestheticTaste::derive_from_bigfive(&b, 2);
        assert!(t.dimension_weights[DIM_VIRTUOSITY] > 0.55);
    }

    #[test]
    fn test_familiarity_bias_range() {
        for seed in 0..20 {
            let b = BigFive::from_seed(seed);
            let t = AestheticTaste::derive_from_bigfive(&b, seed);
            assert!(
                (-1.0..=1.0).contains(&t.familiarity_bias),
                "seed {seed}: bias out of range"
            );
        }
    }

    #[test]
    fn test_aesthetic_openness_high() {
        let b = BigFive {
            openness: 1.0,
            ..BigFive::default()
        };
        let t = AestheticTaste::derive_from_bigfive(&b, 0);
        assert!(
            t.aesthetic_openness > 0.55,
            "high O → high openness, got {}",
            t.aesthetic_openness
        );
    }

    #[test]
    fn test_complexity_tolerance_range() {
        for seed in 0..20 {
            let b = BigFive::from_seed(seed);
            let t = AestheticTaste::derive_from_bigfive(&b, seed);
            assert!((0.0..=1.0).contains(&t.complexity_tolerance));
        }
    }

    #[test]
    fn test_dimension_weights_clamped() {
        for seed in 0..30 {
            let b = BigFive::from_seed(seed);
            let t = AestheticTaste::derive_from_bigfive(&b, seed);
            for (i, &w) in t.dimension_weights.iter().enumerate() {
                assert!(w >= 0.05, "seed {seed} dim {i}: {w} < 0.05");
                assert!(w <= 0.95, "seed {seed} dim {i}: {w} > 0.95");
            }
        }
    }

    #[test]
    fn test_personal_seed_deterministic() {
        let a = AestheticTaste::derive_from_bigfive(&BigFive::default(), 12345);
        let b = AestheticTaste::derive_from_bigfive(&BigFive::default(), 12345);
        assert_eq!(a.personal_seed, b.personal_seed);
        for i in 0..6 {
            assert!((a.dimension_weights[i] - b.dimension_weights[i]).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn test_dimension_constants_unique() {
        // 验证常量互不相同
        let dims = [
            DIM_FLUENCY,
            DIM_NOVELTY,
            DIM_COMPLEXITY,
            DIM_HARMONY,
            DIM_EXPRESSIVENESS,
            DIM_VIRTUOSITY,
        ];
        for i in 0..dims.len() {
            for j in (i + 1)..dims.len() {
                assert_ne!(dims[i], dims[j]);
            }
        }
    }
}
