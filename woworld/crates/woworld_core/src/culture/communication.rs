//! 沟通规范 — 从 CultureCoreParams 的第一层派生
//!
//! 拥有者: 文化系统（CHG-024 从语言表达模块移交所有权）
//! 消费者: 语言表达（对话文本选择、敬语生成）、NPC（眼神行为、社交距离）
//!
//! 参见: `WoWorld-Design/Happy Game/开发阶段/文化系统/003-文化沟通规范.md`

use super::CultureCoreParams;

// ── EyeContactNorm ─────────────────────────────────────

/// 眼神接触文化规范
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EyeContactNorm {
    /// 直视 (开放文化)
    Direct,
    /// 回避 (等级/保守文化)
    Averted,
    /// 地位依赖 (高权力距离)
    #[default]
    StatusBased,
}

// ── HonorificSystem ────────────────────────────────────

/// 敬语系统 — 语言表达模块生成敬语形式的依据
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct HonorificSystem {
    /// 是否有语法化敬语（如日语敬语/韩语阶称）
    pub has_grammatical_honorifics: bool,
    /// 年龄触发敬语
    pub age_based: bool,
    /// 社会地位触发敬语
    pub status_based: bool,
    /// 亲密关系是否覆盖敬语要求
    pub intimacy_overrides: bool,
}

// ── TouchNorms ─────────────────────────────────────────

/// 身体接触规范
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchNorms {
    /// 握手接受度
    pub handshake: f32,
    /// 拥抱接受度
    pub hug: f32,
    /// 异性接触接受度
    pub opposite_gender: f32,
    /// 公共场合身体接触接受度
    pub public: f32,
    /// 支配性接触（拍肩/推搡等）接受度
    pub dominance: f32,
}

impl Default for TouchNorms {
    fn default() -> Self {
        Self {
            handshake: 0.5,
            hug: 0.5,
            opposite_gender: 0.5,
            public: 0.5,
            dominance: 0.5,
        }
    }
}

// ── CommunicationNorms ─────────────────────────────────

/// 沟通规范 — 完整的 8 字段派生结构
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CommunicationNorms {
    /// 打断容忍度: 低→轮流发言严格, 高→重叠发言常见
    pub interruption_tolerance: f32,
    /// 眼神接触规范
    pub eye_contact_norm: EyeContactNorm,
    /// 个人空间半径 (m), [0.3, 1.5]
    pub personal_space_radius_m: f32,
    /// 直接性: 低→委婉/暗示, 高→直白/高效
    pub directness: f32,
    /// 沉默容忍度: 低→沉默尴尬需填补, 高→沉默是舒适/尊重
    pub silence_tolerance: f32,
    /// 情绪表达性: 低→内敛, 高→外放
    pub emotional_expressiveness: f32,
    /// 敬语系统
    pub honorifics: HonorificSystem,
    /// 身体接触规范
    pub touch_norms: TouchNorms,
}

impl Default for CommunicationNorms {
    fn default() -> Self {
        Self {
            interruption_tolerance: 0.5,
            eye_contact_norm: EyeContactNorm::default(),
            personal_space_radius_m: 0.9,
            directness: 0.5,
            silence_tolerance: 0.5,
            emotional_expressiveness: 0.5,
            honorifics: HonorificSystem::default(),
            touch_norms: TouchNorms::default(),
        }
    }
}

impl CommunicationNorms {
    /// 从 CultureCoreParams 确定性派生沟通规范
    ///
    /// 所有公式来自设计文档 003 §3。
    pub fn derive_from(core: &CultureCoreParams) -> Self {
        // interruption_tolerance = competition×0.40 + individualism×0.30 + (1-power_distance)×0.30
        let interruption_tolerance = (core.competition_orientation * 0.40
            + core.individualism * 0.30
            + (1.0 - core.power_distance) * 0.30)
            .clamp(0.0, 1.0);

        // eye_contact_norm: power_distance > 0.6 => StatusBased
        //                   openness > 0.5 => Direct
        //                   else => Averted
        let eye_contact_norm = if core.power_distance > 0.6 {
            EyeContactNorm::StatusBased
        } else if core.openness_to_outsiders > 0.5 {
            EyeContactNorm::Direct
        } else {
            EyeContactNorm::Averted
        };

        // personal_space_radius_m: raw ∈ [0,1] → mapped to [0.3, 1.5]
        let raw_space = core.individualism * 0.40
            + (1.0 - core.indulgence) * 0.35
            + (1.0 - core.power_distance) * 0.25;
        let personal_space_radius_m = 0.3 + raw_space * 1.2;

        // directness = individualism×0.45 + (1-power_distance)×0.30 + (1-uncertainty)×0.25
        let directness = (core.individualism * 0.45
            + (1.0 - core.power_distance) * 0.30
            + (1.0 - core.uncertainty_avoidance) * 0.25)
            .clamp(0.0, 1.0);

        // silence_tolerance = (1-uncertainty)×0.40 + long_term×0.35 + (1-individualism)×0.25
        let silence_tolerance = ((1.0 - core.uncertainty_avoidance) * 0.40
            + core.long_term_orientation * 0.35
            + (1.0 - core.individualism) * 0.25)
            .clamp(0.0, 1.0);

        // emotional_expressiveness = indulgence×0.50 + individualism×0.25 + artistry×0.25
        let emotional_expressiveness = (core.indulgence * 0.50
            + core.individualism * 0.25
            + core.artistry * 0.25)
            .clamp(0.0, 1.0);

        // honorifics
        let honorifics = HonorificSystem {
            has_grammatical_honorifics: core.power_distance > 0.4,
            age_based: core.power_distance > 0.3 || core.long_term_orientation > 0.5,
            status_based: core.power_distance > 0.25,
            intimacy_overrides: core.individualism > 0.3,
        };

        // touch_norms: base touch_acceptance from 4 params
        let touch_acceptance = ((1.0 - core.individualism) * 0.40
            + core.indulgence * 0.30
            + core.openness_to_outsiders * 0.20
            + (1.0 - core.power_distance) * 0.10)
            .clamp(0.0, 1.0);

        let touch_norms = TouchNorms {
            handshake: (touch_acceptance * 1.2).clamp(0.0, 1.0),
            hug: ((touch_acceptance - 0.3) * 1.5).clamp(0.0, 1.0),
            opposite_gender: if core.power_distance > 0.7 { 0.1 }
                else { touch_acceptance.clamp(0.0, 1.0) },
            public: touch_acceptance.clamp(0.0, 1.0),
            dominance: if core.power_distance > 0.5 { 0.6 } else { 0.2 },
        };

        Self {
            interruption_tolerance,
            eye_contact_norm,
            personal_space_radius_m,
            directness,
            silence_tolerance,
            emotional_expressiveness,
            honorifics,
            touch_norms,
        }
    }
}

// ── 测试 ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_all_fields_in_range() {
        for seed in 0..100 {
            let core = CultureCoreParams::from_seed(seed);
            let norms = CommunicationNorms::derive_from(&core);
            assert!((0.0..=1.0).contains(&norms.interruption_tolerance));
            assert!((0.3..=1.5).contains(&norms.personal_space_radius_m));
            assert!((0.0..=1.0).contains(&norms.directness));
            assert!((0.0..=1.0).contains(&norms.silence_tolerance));
            assert!((0.0..=1.0).contains(&norms.emotional_expressiveness));
            assert!((0.0..=1.0).contains(&norms.touch_norms.handshake));
            assert!((0.0..=1.0).contains(&norms.touch_norms.hug));
            assert!((0.0..=1.0).contains(&norms.touch_norms.opposite_gender));
            assert!((0.0..=1.0).contains(&norms.touch_norms.public));
            assert!((0.0..=1.0).contains(&norms.touch_norms.dominance));
        }
    }

    #[test]
    fn test_high_power_distance_status_based_eye_contact() {
        let core = CultureCoreParams {
            power_distance: 0.8,
            ..CultureCoreParams::default()
        };
        let norms = CommunicationNorms::derive_from(&core);
        assert_eq!(norms.eye_contact_norm, EyeContactNorm::StatusBased);
    }

    #[test]
    fn test_high_openness_direct_eye_contact() {
        let core = CultureCoreParams {
            power_distance: 0.3,
            openness_to_outsiders: 0.8,
            ..CultureCoreParams::default()
        };
        let norms = CommunicationNorms::derive_from(&core);
        assert_eq!(norms.eye_contact_norm, EyeContactNorm::Direct);
    }

    #[test]
    fn test_low_openness_averted_eye_contact() {
        let core = CultureCoreParams {
            power_distance: 0.3,
            openness_to_outsiders: 0.2,
            ..CultureCoreParams::default()
        };
        let norms = CommunicationNorms::derive_from(&core);
        assert_eq!(norms.eye_contact_norm, EyeContactNorm::Averted);
    }

    #[test]
    fn test_high_individualism_large_personal_space() {
        let core = CultureCoreParams {
            individualism: 1.0,
            indulgence: 0.0,
            power_distance: 0.0,
            ..CultureCoreParams::default()
        };
        let norms = CommunicationNorms::derive_from(&core);
        assert!(norms.personal_space_radius_m > 1.2);
    }

    #[test]
    fn test_collectivist_small_personal_space() {
        let core = CultureCoreParams {
            individualism: 0.0,
            indulgence: 1.0,
            power_distance: 1.0,
            ..CultureCoreParams::default()
        };
        let norms = CommunicationNorms::derive_from(&core);
        assert!(norms.personal_space_radius_m < 0.7);
    }

    #[test]
    fn test_honorifics_high_power_distance() {
        let core = CultureCoreParams {
            power_distance: 0.9,
            long_term_orientation: 0.3,
            individualism: 0.2,
            ..CultureCoreParams::default()
        };
        let norms = CommunicationNorms::derive_from(&core);
        assert!(norms.honorifics.has_grammatical_honorifics);
        assert!(norms.honorifics.age_based);
        assert!(norms.honorifics.status_based);
        assert!(!norms.honorifics.intimacy_overrides);
    }

    #[test]
    fn test_honorifics_egalitarian_individualist() {
        let core = CultureCoreParams {
            power_distance: 0.1,
            long_term_orientation: 0.1,
            individualism: 0.9,
            ..CultureCoreParams::default()
        };
        let norms = CommunicationNorms::derive_from(&core);
        assert!(!norms.honorifics.has_grammatical_honorifics);
        assert!(!norms.honorifics.age_based);
        assert!(!norms.honorifics.status_based); // power_distance 0.1 < 0.25
        assert!(norms.honorifics.intimacy_overrides);
    }

    #[test]
    fn test_deterministic() {
        let core = CultureCoreParams::from_seed(42);
        let a = CommunicationNorms::derive_from(&core);
        let b = CommunicationNorms::derive_from(&core);
        assert!((a.interruption_tolerance - b.interruption_tolerance).abs() < f32::EPSILON);
        assert_eq!(a.eye_contact_norm, b.eye_contact_norm);
    }

    #[test]
    fn test_high_indulgence_expressive() {
        let core = CultureCoreParams {
            indulgence: 1.0,
            individualism: 0.8,
            artistry: 1.0,
            ..CultureCoreParams::default()
        };
        let norms = CommunicationNorms::derive_from(&core);
        assert!(norms.emotional_expressiveness > 0.8);
    }
}
