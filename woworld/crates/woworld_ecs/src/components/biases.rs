//! CognitiveBiases 认知偏误 Component — ECS 铁律合规
//!
//! 参见: `开发文档/06-认知与智慧系统/002-CognitiveStyle与认知偏误详细设计.md` §3
//!
//! 7 种认知偏误从 CognitiveStyle + BigFive + Emotion 纯函数派生。
//! Phase 1: availability_heuristic 的 cognitive_load 项用 0.5 占位（需 CognitiveTide）。

use crate::components::bigfive::BigFive;
use crate::components::cognitive::CognitiveStyle;
use crate::components::emotion::Emotion;

/// 7 种认知偏误——每个 0-1，越高越强
///
/// 每决策周期从上游 Component 重新计算（纯函数，无额外存储开销）。
/// Phase 2: 接入 CognitiveTide → availability_heuristic 完整公式。
#[derive(Debug, Clone, Copy)]
pub struct CognitiveBiases {
    /// 确认偏误——只寻找支持已有信念的信息
    pub confirmation_bias: f32,
    /// 负面偏误——负面信息权重远大于正面
    pub negativity_bias: f32,
    /// 近因权重——最近事件压倒历史趋势（0.1=完全反思，0.8=完全冲动）
    pub recency_weight: f32,
    /// 自利偏误——将成功归因于自己，失败归因于外部
    pub self_serving_bias: f32,
    /// 可得性启发——容易想到的事就认为更可能发生
    pub availability_heuristic: f32,
    /// 认知失调容忍度——能同时持有矛盾信念（低=需要一致性）
    pub dissonance_tolerance: f32,
    /// 反刍倾向——反复咀嚼负面经历
    pub rumination_tendency: f32,
}

impl Default for CognitiveBiases {
    fn default() -> Self {
        Self {
            confirmation_bias: 0.0,
            negativity_bias: 0.0,
            recency_weight: 0.0,
            self_serving_bias: 0.0,
            availability_heuristic: 0.0,
            dissonance_tolerance: 0.0,
            rumination_tendency: 0.0,
        }
    }
}

impl CognitiveBiases {
    /// 从 CognitiveStyle + BigFive + Emotion 派生 7 种偏误
    ///
    /// Phase 1: `cognitive_load` = 0.5（CognitiveTide 占位）
    pub fn derive(style: &CognitiveStyle, personality: &BigFive, emotion: &Emotion) -> Self {
        // ── confirmation_bias ──
        // ((1-rigid_flexible)×0.6 + (1-analytic_intuitive)×0.4) × (1+|pleasure|×0.3)
        let cb = ((1.0 - style.rigid_flexible) * 0.6 + (1.0 - style.analytic_intuitive) * 0.4)
            * (1.0 + emotion.pleasure.abs() * 0.3);
        let confirmation_bias = cb.clamp(0.0, 1.0);

        // ── negativity_bias ──
        // ((1-analytic_intuitive)×0.4 + neuroticism×0.6) × (1+|min(pleasure,0)|×0.5)
        let nb = ((1.0 - style.analytic_intuitive) * 0.4 + personality.neuroticism * 0.6)
            * (1.0 + emotion.pleasure.min(0.0).abs() * 0.5);
        let negativity_bias = nb.clamp(0.0, 1.0);

        // ── recency_weight ──
        // (1-reflective_impulsive)×0.7 + 0.1 → [0.1, 0.8]
        let recency_weight = ((1.0 - style.reflective_impulsive) * 0.7 + 0.1).clamp(0.0, 1.0);

        // ── self_serving_bias ──
        // (1-rigid_flexible)×0.5 + (1-agreeableness)×0.5
        let ssb = (1.0 - style.rigid_flexible) * 0.5 + (1.0 - personality.agreeableness) * 0.5;
        let self_serving_bias = ssb.clamp(0.0, 1.0);

        // ── availability_heuristic ──
        // (1-reflective_impulsive)×0.5 + cognitive_load×0.3 + extraversion×0.2
        // Phase 1: cognitive_load = 0.5
        let ah =
            (1.0 - style.reflective_impulsive) * 0.5 + 0.5 * 0.3 + personality.extraversion * 0.2;
        let availability_heuristic = ah.clamp(0.0, 1.0);

        // ── dissonance_tolerance ──
        // rigid_flexible×0.5 + (1-neuroticism)×0.5
        let dt = style.rigid_flexible * 0.5 + (1.0 - personality.neuroticism) * 0.5;
        let dissonance_tolerance = dt.clamp(0.0, 1.0);

        // ── rumination_tendency ──
        // (1-reflective_impulsive)×0.4 + neuroticism×0.6
        let rt = (1.0 - style.reflective_impulsive) * 0.4 + personality.neuroticism * 0.6;
        let rumination_tendency = rt.clamp(0.0, 1.0);

        Self {
            confirmation_bias,
            negativity_bias,
            recency_weight,
            self_serving_bias,
            availability_heuristic,
            dissonance_tolerance,
            rumination_tendency,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_style() -> CognitiveStyle {
        CognitiveStyle::derive_from_bigfive(&BigFive::default())
    }

    #[test]
    fn test_neurotic_high_negativity() {
        let b = BigFive {
            neuroticism: 1.0,
            ..BigFive::default()
        };
        let style = CognitiveStyle::derive_from_bigfive(&b);
        let biases = CognitiveBiases::derive(&style, &b, &Emotion::default());
        assert!(biases.negativity_bias > 0.5, "high N → high negativity");
    }

    #[test]
    fn test_stable_low_negativity() {
        let b = BigFive {
            neuroticism: 0.0,
            ..BigFive::default()
        };
        let style = CognitiveStyle::derive_from_bigfive(&b);
        let biases = CognitiveBiases::derive(&style, &b, &Emotion::default());
        assert!(biases.negativity_bias < 0.4, "low N → low negativity");
    }

    #[test]
    fn test_reflective_low_recency() {
        // Fully reflective (ri=1.0): recency = (1-1)×0.7 + 0.1 = 0.1
        let b = BigFive {
            neuroticism: 0.0,
            openness: 1.0,
            extraversion: 0.0,
            ..BigFive::default()
        };
        let style = CognitiveStyle::derive_from_bigfive(&b);
        let biases = CognitiveBiases::derive(&style, &b, &Emotion::default());
        assert!(biases.recency_weight < 0.2, "reflective → low recency");
    }

    #[test]
    fn test_impulsive_high_recency() {
        // Fully impulsive (ri=0.0): recency = (1-0)×0.7 + 0.1 = 0.8
        let b = BigFive {
            neuroticism: 1.0,
            openness: 0.0,
            extraversion: 1.0,
            ..BigFive::default()
        };
        let style = CognitiveStyle::derive_from_bigfive(&b);
        let biases = CognitiveBiases::derive(&style, &b, &Emotion::default());
        assert!(biases.recency_weight > 0.6, "impulsive → high recency");
    }

    #[test]
    fn test_agreeable_low_self_serving() {
        let b = BigFive {
            agreeableness: 1.0,
            ..BigFive::default()
        };
        let mut style = test_style();
        style.rigid_flexible = 1.0;
        let biases = CognitiveBiases::derive(&style, &b, &Emotion::default());
        assert!(
            biases.self_serving_bias < 0.3,
            "agreeable+flexible → low self-serving"
        );
    }

    #[test]
    fn test_flexible_high_dissonance_tolerance() {
        let b = BigFive {
            neuroticism: 0.0,
            ..BigFive::default()
        };
        let mut style = test_style();
        style.rigid_flexible = 1.0;
        let biases = CognitiveBiases::derive(&style, &b, &Emotion::default());
        assert!(biases.dissonance_tolerance > 0.7);
    }

    #[test]
    fn test_neurotic_high_rumination() {
        let b = BigFive {
            neuroticism: 1.0,
            ..BigFive::default()
        };
        let style = CognitiveStyle::derive_from_bigfive(&b);
        let biases = CognitiveBiases::derive(&style, &b, &Emotion::default());
        assert!(biases.rumination_tendency > 0.5);
    }

    #[test]
    fn test_all_in_range() {
        for seed in 0..20 {
            let b = BigFive::from_seed(seed);
            let style = CognitiveStyle::derive_from_bigfive(&b);
            let biases = CognitiveBiases::derive(&style, &b, &Emotion::default());

            assert!((0.0..=1.0).contains(&biases.confirmation_bias));
            assert!((0.0..=1.0).contains(&biases.negativity_bias));
            assert!((0.0..=1.0).contains(&biases.recency_weight));
            assert!((0.0..=1.0).contains(&biases.self_serving_bias));
            assert!((0.0..=1.0).contains(&biases.availability_heuristic));
            assert!((0.0..=1.0).contains(&biases.dissonance_tolerance));
            assert!((0.0..=1.0).contains(&biases.rumination_tendency));
        }
    }

    #[test]
    fn test_default_all_zero() {
        let cb = CognitiveBiases::default();
        assert_eq!(cb.confirmation_bias, 0.0);
        assert_eq!(cb.negativity_bias, 0.0);
        assert_eq!(cb.recency_weight, 0.0);
        assert_eq!(cb.self_serving_bias, 0.0);
        assert_eq!(cb.availability_heuristic, 0.0);
        assert_eq!(cb.dissonance_tolerance, 0.0);
        assert_eq!(cb.rumination_tendency, 0.0);
    }

    #[test]
    fn test_sadness_amplifies_negativity() {
        let b = BigFive {
            neuroticism: 0.5,
            ..BigFive::default()
        };
        let style = test_style();
        let happy = CognitiveBiases::derive(
            &style,
            &b,
            &Emotion {
                pleasure: 0.5,
                ..Emotion::default()
            },
        );
        let sad = CognitiveBiases::derive(
            &style,
            &b,
            &Emotion {
                pleasure: -0.5,
                ..Emotion::default()
            },
        );
        assert!(
            sad.negativity_bias > happy.negativity_bias,
            "sadness amplifies negativity bias"
        );
    }
}
