//! 审美标准 — 从 CultureCoreParams 的第一层派生
//!
//! 拥有者: 文化系统（CHG-024 从 NPC 模块移交所有权）
//! 消费者: NPC（吸引力判断、装扮行为）
//!
//! Phase 1 简化: 省略 EliteAppearance 参数（NPC 模块尚未提供）。
//! aesthetic_confidence 从核心参数纯派生，skin_tone_ideal 设为 None。
//!
//! 参见: `WoWorld-Design/Happy Game/开发阶段/文化系统/004-文化审美与物质.md` §2

use super::CultureCoreParams;

// ── BuildType ──────────────────────────────────────────

/// 理想体型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuildType {
    Lean,
    Athletic,
    Robust,
    Plump,
    #[default]
    Unspecified,
}

// ── ScarStance ─────────────────────────────────────────

/// 疤痕文化态度
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ScarStance {
    /// 疤痕是荣誉（战士文化）
    Honor,
    /// 中性
    #[default]
    Neutral,
    /// 疤痕是污名
    Stigma,
}

// ── CulturalBeautyStandard ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CulturalBeautyStandard {
    /// 理想体型
    pub ideal_build: BuildType,
    /// 身高偏好: -1=极矮, 0=中性, 1=极高
    pub height_preference: f32,
    /// 肤色理想 (Phase 1: None——等 EliteAppearance)
    pub skin_tone_ideal: Option<()>,  // placeholder: 等 NPC 模块提供 SkinToneRange
    /// 仪容打理重要性 [0,1]
    pub grooming_importance: f32,
    /// 疤痕态度
    pub scar_stance: ScarStance,
    /// 理想年龄范围 (min, max)
    pub ideal_age_range: (f32, f32),
    /// 外貌在择偶中的权重 [0,1]
    pub appearance_weight_in_mating: f32,
    /// 审美自信（文化自我评价）[0,1]
    pub aesthetic_confidence: f32,
}

impl Default for CulturalBeautyStandard {
    fn default() -> Self {
        Self {
            ideal_build: BuildType::default(),
            height_preference: 0.0,
            skin_tone_ideal: None,
            grooming_importance: 0.5,
            scar_stance: ScarStance::default(),
            ideal_age_range: (18.0, 35.0),
            appearance_weight_in_mating: 0.5,
            aesthetic_confidence: 0.5,
        }
    }
}

impl CulturalBeautyStandard {
    /// 从 CultureCoreParams 派生审美标准
    ///
    /// Phase 1: 省略 elite_appearance 参数。
    /// 完整版 Phase 2+ 将接受上流阶层的实际外貌作为模仿基准。
    pub fn derive_from(core: &CultureCoreParams) -> Self {
        // ── ideal_build ──
        let ideal_build = if core.militarism > 0.6 {
            BuildType::Athletic
        } else if core.indulgence > 0.6 && core.militarism < 0.4 {
            BuildType::Plump
        } else if core.artistry > 0.6 && core.individualism > 0.5 {
            BuildType::Lean
        } else if core.power_distance > 0.7 {
            BuildType::Robust
        } else {
            BuildType::Unspecified
        };

        // ── height_preference ──
        // power_distance 推向上, individualism 偏好个体差异
        let height_preference = (core.power_distance * 0.7
            + core.competition_orientation * 0.3
            - 0.5)
            .clamp(-1.0, 1.0);

        // ── grooming_importance ──
        let grooming_importance = (core.artistry * 0.40
            + core.indulgence * 0.25
            + core.power_distance * 0.20
            + core.competition_orientation * 0.15)
            .clamp(0.0, 1.0);

        // ── scar_stance ──
        let scar_stance = if core.militarism > 0.5 {
            ScarStance::Honor
        } else {
            ScarStance::Neutral
        };

        // ── ideal_age_range ──
        // base: 18-35. power_distance 倾向于更年轻, long_term 延迟
        let min_age = (18.0 - core.power_distance * 5.0 + core.long_term_orientation * 3.0)
            .clamp(14.0, 28.0);
        let max_age = (35.0 - core.power_distance * 8.0 + core.long_term_orientation * 5.0)
            .clamp(25.0, 50.0);
        let ideal_age_range = (min_age, max_age.max(min_age + 5.0));

        // ── appearance_weight_in_mating ──
        let appearance_weight_in_mating = (core.indulgence * 0.40
            + core.competition_orientation * 0.30
            + core.artistry * 0.30)
            .clamp(0.0, 1.0);

        // ── aesthetic_confidence ──
        // Phase 1: 核心参数纯派生（无精英基准）
        let aesthetic_confidence = (core.artistry * 0.35
            + core.power_distance * 0.25
            + core.long_term_orientation * 0.20
            + core.uncertainty_avoidance * 0.20)
            .clamp(0.0, 1.0);

        Self {
            ideal_build,
            height_preference,
            skin_tone_ideal: None,
            grooming_importance,
            scar_stance,
            ideal_age_range,
            appearance_weight_in_mating,
            aesthetic_confidence,
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
            let beauty = CulturalBeautyStandard::derive_from(&core);
            assert!((-1.0..=1.0).contains(&beauty.height_preference));
            assert!((0.0..=1.0).contains(&beauty.grooming_importance));
            assert!((0.0..=1.0).contains(&beauty.appearance_weight_in_mating));
            assert!((0.0..=1.0).contains(&beauty.aesthetic_confidence));
            assert!(beauty.ideal_age_range.0 >= 14.0);
            assert!(beauty.ideal_age_range.1 <= 50.0);
            assert!(beauty.ideal_age_range.1 >= beauty.ideal_age_range.0 + 5.0);
        }
    }

    #[test]
    fn test_militarist_athletic_honor_scars() {
        let core = CultureCoreParams {
            militarism: 0.8,
            ..CultureCoreParams::default()
        };
        let beauty = CulturalBeautyStandard::derive_from(&core);
        assert_eq!(beauty.ideal_build, BuildType::Athletic);
        assert_eq!(beauty.scar_stance, ScarStance::Honor);
    }

    #[test]
    fn test_indulgent_plump() {
        let core = CultureCoreParams {
            indulgence: 0.8,
            militarism: 0.1,
            ..CultureCoreParams::default()
        };
        let beauty = CulturalBeautyStandard::derive_from(&core);
        assert_eq!(beauty.ideal_build, BuildType::Plump);
    }

    #[test]
    fn test_artistic_individualist_lean() {
        let core = CultureCoreParams {
            artistry: 0.8,
            individualism: 0.7,
            ..CultureCoreParams::default()
        };
        let beauty = CulturalBeautyStandard::derive_from(&core);
        assert_eq!(beauty.ideal_build, BuildType::Lean);
    }

    #[test]
    fn test_hierarchy_robust() {
        let core = CultureCoreParams {
            power_distance: 0.9,
            militarism: 0.3,
            ..CultureCoreParams::default()
        };
        let beauty = CulturalBeautyStandard::derive_from(&core);
        assert_eq!(beauty.ideal_build, BuildType::Robust);
    }

    #[test]
    fn test_high_artistry_grooming() {
        let core = CultureCoreParams {
            artistry: 1.0,
            indulgence: 1.0,
            power_distance: 1.0,
            competition_orientation: 1.0,
            ..CultureCoreParams::default()
        };
        let beauty = CulturalBeautyStandard::derive_from(&core);
        assert!(beauty.grooming_importance > 0.8);
    }

    #[test]
    fn test_power_distance_height() {
        let high = CultureCoreParams {
            power_distance: 1.0,
            competition_orientation: 1.0,
            ..CultureCoreParams::default()
        };
        let low = CultureCoreParams {
            power_distance: 0.0,
            competition_orientation: 0.0,
            ..CultureCoreParams::default()
        };
        let h = CulturalBeautyStandard::derive_from(&high);
        let l = CulturalBeautyStandard::derive_from(&low);
        assert!(h.height_preference > l.height_preference);
    }

    #[test]
    fn test_peaceful_neutral_scars() {
        let core = CultureCoreParams {
            militarism: 0.2,
            ..CultureCoreParams::default()
        };
        let beauty = CulturalBeautyStandard::derive_from(&core);
        assert_eq!(beauty.scar_stance, ScarStance::Neutral);
    }

    #[test]
    fn test_skin_tone_none_phase1() {
        let core = CultureCoreParams::from_seed(42);
        let beauty = CulturalBeautyStandard::derive_from(&core);
        assert!(beauty.skin_tone_ideal.is_none());
    }

    #[test]
    fn test_deterministic() {
        let core = CultureCoreParams::from_seed(42);
        let a = CulturalBeautyStandard::derive_from(&core);
        let b = CulturalBeautyStandard::derive_from(&core);
        assert_eq!(a.ideal_build, b.ideal_build);
        assert!((a.height_preference - b.height_preference).abs() < f32::EPSILON);
    }
}
