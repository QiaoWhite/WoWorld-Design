//! BigFive 大五人格 Component — ECS 铁律合规
//!
//! 参见: `开发文档/01-NPC核心/01-NPC活人感.md` §1.1.2
//!
//! BigFive 是 NPC 所有个性化的根——NeedSensitivity、Chronotype、CognitiveStyle、
//! 情感基线全部从这 5 个维度派生。人格极少变动（仅极端创伤或长期环境冲突时微调 ≤0.05）。

use crate::components::circadian::Chronotype;
use crate::components::needs::NeedSensitivity;
use crate::prng::pseudo_random_f32_salted;

/// 大五人格 (OCEAN)——NPC 个性化根基
///
/// 每个维度 0.0-1.0，中点 0.5。至少一个维度偏离中点 >0.25（可感知差异）。
#[derive(Debug, Clone, Copy)]
pub struct BigFive {
    /// 开放性: 高→好奇/尝试新事物; 低→保守/偏好熟悉
    pub openness: f32,
    /// 尽责性: 高→自律/守时/计划性强; 低→随性/易分心
    pub conscientiousness: f32,
    /// 外向性: 高→主动社交/群体愉悦; 低→回避大型社交
    pub extraversion: f32,
    /// 宜人性: 高→信任/合作/易感染; 低→竞争/怀疑/交易激进
    pub agreeableness: f32,
    /// 神经质: 高→负面事件敏感/恢复慢; 低→情绪稳定/恢复快
    pub neuroticism: f32,
}

impl Default for BigFive {
    fn default() -> Self {
        Self {
            openness: 0.5,
            conscientiousness: 0.5,
            extraversion: 0.5,
            agreeableness: 0.5,
            neuroticism: 0.5,
        }
    }
}

impl BigFive {
    /// 确保至少一个维度偏离中点 >0.25，保证可感知的人格差异。
    pub fn has_prominent_dimension(&self) -> bool {
        (self.openness - 0.5).abs() > 0.25
            || (self.conscientiousness - 0.5).abs() > 0.25
            || (self.extraversion - 0.5).abs() > 0.25
            || (self.agreeableness - 0.5).abs() > 0.25
            || (self.neuroticism - 0.5).abs() > 0.25
    }

    /// 从 BigFive 派生需求敏感度（一次性计算，仅在人格极端冲击后重算）
    ///
    /// 公式来源: `NPC活人感开发文档ver2.0.md` §1.1.2a
    /// safety_sens 无 BigFive 公式——保持 1.0（设计文档未定义）
    /// Sprint 053: +esteem +competence (v2.0 进阶需求系统)
    pub fn derive_sensitivity(&self) -> NeedSensitivity {
        NeedSensitivity {
            hunger_sens: 0.5 + self.neuroticism * 0.5,
            thirst_sens: 0.5 + self.conscientiousness * 0.25 + self.neuroticism * 0.25,
            fatigue_sens: 0.5 + (1.0 - self.extraversion) * 0.3,
            safety_sens: 1.0, // 设计文档无对应公式，保持默认
            social_sens: 0.2 + self.extraversion * 0.8,
            element_sens: 0.4 + self.conscientiousness * 0.45 + self.neuroticism * 0.15,
            libido_sens: 0.3 + self.extraversion * 0.4 + self.openness * 0.3,
            // v2.0 进阶需求 (NPC活人感开发文档ver2.0.md lines 264-265)
            esteem_sens: (0.2 + self.extraversion * 0.5 + (1.0 - self.agreeableness) * 0.3)
                .clamp(0.2, 1.0),
            competence_sens: (0.2 + self.conscientiousness * 0.5 + self.neuroticism * 0.3)
                .clamp(0.2, 1.0),
        }
    }

    /// 从 BigFive 派生昼夜类型
    ///
    /// 尽责性是早起的最强预测因子，外向性微弱预测晚睡。
    /// score = C × 0.6 - E × 0.35
    /// score > 0.2 → Morning, < -0.2 → Evening, else Neutral
    pub fn derive_chronotype(&self) -> Chronotype {
        let score = self.conscientiousness * 0.6 - self.extraversion * 0.35;
        if score > 0.2 {
            Chronotype::Morning
        } else if score < -0.2 {
            Chronotype::Evening
        } else {
            Chronotype::Neutral
        }
    }

    /// 从种子确定性生成 BigFive（Phase 1: 均匀分布）
    ///
    /// 相同 seed 始终产生相同人格。确保至少一个维度突出（偏离 >0.25）。
    pub fn from_seed(seed: u64) -> Self {
        let o = pseudo_random_f32_salted(seed, 0);
        let c = pseudo_random_f32_salted(seed, 1);
        let e = pseudo_random_f32_salted(seed, 2);
        let a = pseudo_random_f32_salted(seed, 3);
        let n = pseudo_random_f32_salted(seed, 4);

        let mut bf = Self {
            openness: o,
            conscientiousness: c,
            extraversion: e,
            agreeableness: a,
            neuroticism: n,
        };

        // 强制至少一个维度突出
        if !bf.has_prominent_dimension() {
            // 找到偏离最大的维度，向外推至偏离 ≥ 0.25
            let devs: [(usize, f32); 5] = [
                (0, (o - 0.5).abs()),
                (1, (c - 0.5).abs()),
                (2, (e - 0.5).abs()),
                (3, (a - 0.5).abs()),
                (4, (n - 0.5).abs()),
            ];
            let (max_idx, _) = devs
                .iter()
                .max_by(|(_, d1), (_, d2)| d1.total_cmp(d2))
                .unwrap();
            // 向远离 0.5 的方向推至 0.26
            let target = if [o, c, e, a, n][*max_idx] >= 0.5 { 0.76 } else { 0.24 };
            match max_idx {
                0 => bf.openness = target,
                1 => bf.conscientiousness = target,
                2 => bf.extraversion = target,
                3 => bf.agreeableness = target,
                4 => bf.neuroticism = target,
                _ => unreachable!(),
            }
        }

        bf
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_midpoint() {
        let b = BigFive::default();
        assert!((b.openness - 0.5).abs() < f32::EPSILON);
        assert!((b.conscientiousness - 0.5).abs() < f32::EPSILON);
        assert!((b.extraversion - 0.5).abs() < f32::EPSILON);
        assert!((b.agreeableness - 0.5).abs() < f32::EPSILON);
        assert!((b.neuroticism - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_prominent_dimension_false_at_midpoint() {
        assert!(!BigFive::default().has_prominent_dimension());
    }

    #[test]
    fn test_prominent_dimension_true() {
        let b = BigFive {
            openness: 0.8, // 偏离 0.3 > 0.25
            ..BigFive::default()
        };
        assert!(b.has_prominent_dimension());

        let b2 = BigFive {
            neuroticism: 0.2, // 偏离 0.3 > 0.25
            ..BigFive::default()
        };
        assert!(b2.has_prominent_dimension());
    }

    // ── derive_sensitivity ──────────────────

    #[test]
    fn test_derive_sensitivity_all_in_range() {
        let b = BigFive::from_seed(42);
        let s = b.derive_sensitivity();
        // 所有敏感度应在 [0.2, 1.0]
        assert!((0.2..=1.0).contains(&s.hunger_sens));
        assert!((0.2..=1.0).contains(&s.thirst_sens));
        assert!((0.2..=1.0).contains(&s.fatigue_sens));
        assert!((0.2..=1.0).contains(&s.safety_sens));
        assert!((0.2..=1.0).contains(&s.social_sens));
        assert!((0.2..=1.0).contains(&s.element_sens));
        assert!((0.2..=1.0).contains(&s.libido_sens));
    }

    #[test]
    fn test_derive_sensitivity_neuroticism_high() {
        let b = BigFive {
            neuroticism: 1.0,
            ..BigFive::default()
        };
        let s = b.derive_sensitivity();
        // hunger = 0.5 + N×0.5 → 1.0
        assert!((s.hunger_sens - 1.0).abs() < 0.01);
        // thirst = 0.5 + C×0.25 + N×0.25 → 0.5 + 0.125 + 0.25 = 0.875
        assert!((s.thirst_sens - 0.875).abs() < 0.01);
    }

    #[test]
    fn test_derive_sensitivity_extraversion_low() {
        let b = BigFive {
            extraversion: 0.0,
            ..BigFive::default()
        };
        let s = b.derive_sensitivity();
        // social = 0.2 + E×0.8 → 0.2
        assert!((s.social_sens - 0.2).abs() < 0.01);
        // fatigue = 0.5 + (1-E)×0.3 → 0.8
        assert!((s.fatigue_sens - 0.8).abs() < 0.01);
    }

    #[test]
    fn test_derive_sensitivity_extraversion_high() {
        let b = BigFive {
            extraversion: 1.0,
            ..BigFive::default()
        };
        let s = b.derive_sensitivity();
        // social = 0.2 + 1.0×0.8 → 1.0
        assert!((s.social_sens - 1.0).abs() < 0.01);
        // libido = 0.3 + E×0.4 + O×0.3 → 0.3 + 0.4 + 0.15 = 0.85
        assert!((s.libido_sens - 0.85).abs() < 0.01);
    }

    #[test]
    fn test_safety_sens_stays_default() {
        let b = BigFive::from_seed(99);
        let s = b.derive_sensitivity();
        assert!((s.safety_sens - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_derive_esteem_sensitivity() {
        // E=1, A=0 → 0.2 + 1.0×0.5 + 1.0×0.3 = 1.0
        let b = BigFive { extraversion: 1.0, agreeableness: 0.0, ..BigFive::default() };
        let s = b.derive_sensitivity();
        assert!((s.esteem_sens - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_derive_competence_sensitivity() {
        // C=1, N=0 → 0.2 + 1.0×0.5 + 0×0.3 = 0.7
        let b = BigFive { conscientiousness: 1.0, neuroticism: 0.0, ..BigFive::default() };
        let s = b.derive_sensitivity();
        assert!((s.competence_sens - 0.7).abs() < 0.01);
    }

    #[test]
    fn test_esteem_clamped_min() {
        // E=0, A=1 → 0.2 + 0×0.5 + 0×0.3 = 0.2（下限）
        let b = BigFive { extraversion: 0.0, agreeableness: 1.0, ..BigFive::default() };
        let s = b.derive_sensitivity();
        assert!((s.esteem_sens - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_competence_clamped_min() {
        // C=0, N=0 → 0.2 + 0×0.5 + 0×0.3 = 0.2（下限）
        let b = BigFive { conscientiousness: 0.0, neuroticism: 0.0, ..BigFive::default() };
        let s = b.derive_sensitivity();
        assert!((s.competence_sens - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_derive_sensitivity_backward_compat() {
        // 前 7 字段值必须与 Sprint 043 完全一致
        // BigFive::default() = all 0.5
        let b = BigFive::default();
        let s = b.derive_sensitivity();
        // hunger = 0.5 + 0.5*0.5 = 0.75
        assert!((s.hunger_sens - 0.75).abs() < 0.01);
        // social = 0.2 + 0.5*0.8 = 0.6
        assert!((s.social_sens - 0.6).abs() < 0.01);
        // libido = 0.3 + 0.5*0.4 + 0.5*0.3 = 0.65
        assert!((s.libido_sens - 0.65).abs() < 0.01);
        // element = 0.4 + 0.5*0.45 + 0.5*0.15 = 0.7
        assert!((s.element_sens - 0.7).abs() < 0.01);
    }

    // ── derive_chronotype ──────────────────

    #[test]
    fn test_derive_chronotype_morning() {
        let b = BigFive {
            conscientiousness: 1.0,
            extraversion: 0.0,
            ..BigFive::default()
        };
        assert_eq!(b.derive_chronotype(), Chronotype::Morning);
    }

    #[test]
    fn test_derive_chronotype_evening() {
        let b = BigFive {
            conscientiousness: 0.0,
            extraversion: 1.0,
            ..BigFive::default()
        };
        assert_eq!(b.derive_chronotype(), Chronotype::Evening);
    }

    #[test]
    fn test_derive_chronotype_neutral() {
        // 全中点: score = 0.5×0.6 - 0.5×0.35 = 0.3 - 0.175 = 0.125
        assert_eq!(BigFive::default().derive_chronotype(), Chronotype::Neutral);
    }

    // ── from_seed ──────────────────

    #[test]
    fn test_from_seed_deterministic() {
        let a = BigFive::from_seed(12345);
        let b = BigFive::from_seed(12345);
        assert!((a.openness - b.openness).abs() < f32::EPSILON);
        assert!((a.conscientiousness - b.conscientiousness).abs() < f32::EPSILON);
        assert!((a.extraversion - b.extraversion).abs() < f32::EPSILON);
        assert!((a.agreeableness - b.agreeableness).abs() < f32::EPSILON);
        assert!((a.neuroticism - b.neuroticism).abs() < f32::EPSILON);
    }

    #[test]
    fn test_from_seed_has_prominent() {
        for seed in 0..100 {
            let b = BigFive::from_seed(seed);
            assert!(
                b.has_prominent_dimension(),
                "seed {seed} must produce prominent dimension"
            );
        }
    }

    #[test]
    fn test_from_seed_diverse() {
        let a = BigFive::from_seed(0);
        let b = BigFive::from_seed(1);
        let c = BigFive::from_seed(2);
        // 三个种子至少有两个不同的人格
        let same_ab = (a.openness - b.openness).abs() < f32::EPSILON
            && (a.conscientiousness - b.conscientiousness).abs() < f32::EPSILON;
        let same_bc = (b.openness - c.openness).abs() < f32::EPSILON
            && (b.conscientiousness - c.conscientiousness).abs() < f32::EPSILON;
        assert!(!same_ab || !same_bc, "different seeds should produce diverse results");
    }

    #[test]
    fn test_from_seed_values_in_range() {
        for seed in 0..50 {
            let b = BigFive::from_seed(seed);
            assert!((0.0..=1.0).contains(&b.openness));
            assert!((0.0..=1.0).contains(&b.conscientiousness));
            assert!((0.0..=1.0).contains(&b.extraversion));
            assert!((0.0..=1.0).contains(&b.agreeableness));
            assert!((0.0..=1.0).contains(&b.neuroticism));
        }
    }

    // ── 端到端: BigFive → NeedSensitivity → urgency ──

    #[test]
    fn test_different_bigfive_different_urgency() {
        // 高神经质 NPC → 高 hunger 敏感度 → 相同 hunger 值产生更高 urgency
        let anxious = BigFive {
            neuroticism: 1.0,
            ..BigFive::default()
        };
        let stable = BigFive {
            neuroticism: 0.0,
            ..BigFive::default()
        };

        let s_anxious = anxious.derive_sensitivity();
        let s_stable = stable.derive_sensitivity();

        // hunger_sens: anxious=1.0, stable=0.5
        assert!(s_anxious.hunger_sens > s_stable.hunger_sens);

        // 相同 hunger=0.6 → urgency 不同
        // urgency = value * sensitivity (baseline=0)
        let u_anxious = 0.6 * s_anxious.hunger_sens;
        let u_stable = 0.6 * s_stable.hunger_sens;
        assert!(u_anxious > u_stable,
            "anxious NPC should feel hungrier: {u_anxious} vs {u_stable}");
    }

    #[test]
    fn test_different_bigfive_different_chronotype() {
        let morning = BigFive {
            conscientiousness: 1.0,
            extraversion: 0.0,
            ..BigFive::default()
        };
        let evening = BigFive {
            conscientiousness: 0.0,
            extraversion: 1.0,
            ..BigFive::default()
        };
        assert_ne!(morning.derive_chronotype(), evening.derive_chronotype());
    }
}
