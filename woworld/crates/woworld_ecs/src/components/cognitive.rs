//! CognitiveStyle 认知风格 Component — ECS 铁律合规
//!
//! 参见: `开发文档/06-认知与智慧系统/002-CognitiveStyle与认知偏误详细设计.md` §1
//!
//! 4 维认知风格从 BigFive 派生。Phase 1 仅包含 BigFive 公式项——
//! wisdom/mental_age/life_event_count 依赖生命周期和历史系统（未实现），用中点占位。

use crate::components::bigfive::BigFive;

/// 四维认知风格——决定 NPC 如何感知、思考、决策
///
/// 所有维度 0.0-1.0。从 BigFive 一次性派生，仅在极端人格冲击后重算。
#[derive(Debug, Clone, Copy)]
pub struct CognitiveStyle {
    /// 分析←0.0→直觉: 高=逻辑推理，低=感觉判断
    pub analytic_intuitive: f32,
    /// 反思←0.0→冲动: 高=深思熟虑，低=立即反应
    pub reflective_impulsive: f32,
    /// 具象←0.0→抽象: 高=概念模式，低=具体事物
    pub abstract_concrete: f32,
    /// 顽固←0.0→灵活: 高=愿意调整信念，低=信念难以改变
    pub rigid_flexible: f32,
}

impl Default for CognitiveStyle {
    fn default() -> Self {
        Self {
            analytic_intuitive: 0.5,
            reflective_impulsive: 0.5,
            abstract_concrete: 0.5,
            rigid_flexible: 0.5,
        }
    }
}

impl CognitiveStyle {
    /// 从 BigFive 派生认知风格（含 Phase 1 占位项）
    ///
    /// `abstract_concrete` 和 `rigid_flexible` 各含 ~50% 非 BigFive 项
    /// （wisdom、mental_age、life_event_count），Phase 1 用中点 0.5 替代。
    /// Phase 2 接入生命周期系统后将改为完整公式。
    pub fn derive_from_bigfive(b: &BigFive) -> Self {
        // §1.2a: analytic_intuitive = C×0.7 + (1-O)×0.3  [纯 BigFive]
        let ai = (b.conscientiousness * 0.7 + (1.0 - b.openness) * 0.3).clamp(0.0, 1.0);

        // §1.2b: reflective_impulsive = (1-N)×0.5 + O×0.3 + (1-E)×0.2  [纯 BigFive]
        let ri = ((1.0 - b.neuroticism) * 0.5
            + b.openness * 0.3
            + (1.0 - b.extraversion) * 0.2)
            .clamp(0.0, 1.0);

        // §1.2c: abstract_concrete = O×0.5 + wisdom×0.3 + sigmoid(mental_age-0.3)×0.2
        // Phase 1: wisdom=0.5, sigmoid(mental_age-0.3)=0.5 (中点占位)
        let ac = (b.openness * 0.5 + 0.5 * 0.3 + 0.5 * 0.2).clamp(0.0, 1.0);

        // §1.2d: rigid_flexible = O×0.6 + (1-C)×0.2 + sigmoid(life_events×0.01)×0.2
        // Phase 1: sigmoid(life_events×0.01)=0.5 (中点占位)
        let rf =
            (b.openness * 0.6 + (1.0 - b.conscientiousness) * 0.2 + 0.5 * 0.2).clamp(0.0, 1.0);

        Self {
            analytic_intuitive: ai,
            reflective_impulsive: ri,
            abstract_concrete: ac,
            rigid_flexible: rf,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_midpoint() {
        let cs = CognitiveStyle::default();
        assert!((cs.analytic_intuitive - 0.5).abs() < f32::EPSILON);
        assert!((cs.reflective_impulsive - 0.5).abs() < f32::EPSILON);
        assert!((cs.abstract_concrete - 0.5).abs() < f32::EPSILON);
        assert!((cs.rigid_flexible - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_analytic_conscientious_high() {
        // C=1, O=0 → ai = 1.0×0.7 + 1.0×0.3 = 1.0（纯分析）
        let b = BigFive { conscientiousness: 1.0, openness: 0.0, ..BigFive::default() };
        let cs = CognitiveStyle::derive_from_bigfive(&b);
        assert!((cs.analytic_intuitive - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_analytic_open_intuitive() {
        // C=0, O=1 → ai = 0×0.7 + 0×0.3 = 0.0（纯直觉）
        let b = BigFive { conscientiousness: 0.0, openness: 1.0, ..BigFive::default() };
        let cs = CognitiveStyle::derive_from_bigfive(&b);
        assert!((cs.analytic_intuitive - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_reflective_stable_introvert() {
        // N=0, O=1, E=0 → ri = 1.0×0.5 + 1.0×0.3 + 1.0×0.2 = 1.0（纯反思）
        let b = BigFive {
            neuroticism: 0.0,
            openness: 1.0,
            extraversion: 0.0,
            ..BigFive::default()
        };
        let cs = CognitiveStyle::derive_from_bigfive(&b);
        assert!((cs.reflective_impulsive - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_reflective_neurotic_extravert() {
        // N=1, O=0, E=1 → ri = 0×0.5 + 0×0.3 + 0×0.2 = 0.0（纯冲动）
        let b = BigFive {
            neuroticism: 1.0,
            openness: 0.0,
            extraversion: 1.0,
            ..BigFive::default()
        };
        let cs = CognitiveStyle::derive_from_bigfive(&b);
        assert!((cs.reflective_impulsive - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_abstract_open_high() {
        // O=1 → ac = 1.0×0.5 + 0.5×0.3 + 0.5×0.2 = 0.75
        let b = BigFive { openness: 1.0, ..BigFive::default() };
        let cs = CognitiveStyle::derive_from_bigfive(&b);
        assert!(cs.abstract_concrete > 0.65, "high openness → abstract, got {}", cs.abstract_concrete);
    }

    #[test]
    fn test_rigid_open_flexible() {
        // O=1, C=0 → rf = 1.0×0.6 + 1.0×0.2 + 0.5×0.2 = 0.9
        let b = BigFive { openness: 1.0, conscientiousness: 0.0, ..BigFive::default() };
        let cs = CognitiveStyle::derive_from_bigfive(&b);
        assert!(cs.rigid_flexible > 0.8, "high O low C → flexible, got {}", cs.rigid_flexible);
    }

    #[test]
    fn test_rigid_conservative_rigid() {
        // O=0, C=1 → rf = 0×0.6 + 0×0.2 + 0.5×0.2 = 0.1
        let b = BigFive { openness: 0.0, conscientiousness: 1.0, ..BigFive::default() };
        let cs = CognitiveStyle::derive_from_bigfive(&b);
        assert!(cs.rigid_flexible < 0.2, "low O high C → rigid, got {}", cs.rigid_flexible);
    }

    #[test]
    fn test_all_dimensions_in_range() {
        // 随机采样 20 个种子验证范围
        for seed in 0..20 {
            let b = BigFive::from_seed(seed);
            let cs = CognitiveStyle::derive_from_bigfive(&b);
            assert!((0.0..=1.0).contains(&cs.analytic_intuitive), "seed {seed}: ai out of range");
            assert!((0.0..=1.0).contains(&cs.reflective_impulsive), "seed {seed}: ri out of range");
            assert!((0.0..=1.0).contains(&cs.abstract_concrete), "seed {seed}: ac out of range");
            assert!((0.0..=1.0).contains(&cs.rigid_flexible), "seed {seed}: rf out of range");
        }
    }
}
