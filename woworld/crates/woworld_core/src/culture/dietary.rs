//! 饮食偏好 — 从 CultureCoreParams 的第一层派生
//!
//! 消费者: 经济系统（消费偏好）、NPC（食物选择）
//!
//! 参见: `WoWorld-Design/Happy Game/开发阶段/文化系统/004-文化审美与物质.md` §3

use super::CultureCoreParams;

// ── StapleType ─────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StapleType {
    GrainBread,
    GrainRice,
    GrainPorridge,
    Tuber,
    Pastoral,
    Maritime,
    #[default]
    Mixed,
}

// ── MeatRole ───────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MeatRole {
    Central,
    #[default]
    Regular,
    Occasional,
    Taboo,
}

// ── AlcoholStance ──────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlcoholStance {
    #[default]
    Accepted,
    Ritualistic,
    Discouraged,
    Forbidden,
}

// ── DiningStyle ────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DiningStyle {
    /// 集体共食（大盘共享）
    CommunalShared,
    /// 个人分食（单独盘装）
    IndividualPlated,
    /// 从公共容器分食
    #[default]
    CommunalFromCommon,
}

// ── DietaryBasePreferences ─────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DietaryBasePreferences {
    pub staple: StapleType,
    pub meat_role: MeatRole,
    pub alcohol_stance: AlcoholStance,
    pub dining_style: DiningStyle,
    /// 香料偏好, [0,1]
    pub spice_preference: f32,
}

impl Default for DietaryBasePreferences {
    fn default() -> Self {
        Self {
            staple: StapleType::default(),
            meat_role: MeatRole::default(),
            alcohol_stance: AlcoholStance::default(),
            dining_style: DiningStyle::default(),
            spice_preference: 0.5,
        }
    }
}

impl DietaryBasePreferences {
    /// 从 CultureCoreParams 和环境标志派生饮食偏好
    pub fn derive_from(core: &CultureCoreParams, is_grassland: bool, is_tropical: bool) -> Self {
        // ── staple ──
        // 气候主导，文化调节
        let staple = if is_grassland {
            StapleType::Pastoral
        } else if is_tropical && core.long_term_orientation > 0.5 {
            StapleType::GrainRice
        } else if core.long_term_orientation > 0.6 {
            StapleType::GrainBread
        } else if core.individualism < 0.3 {
            StapleType::GrainPorridge // 集体主义→粥（慢炖共享）
        } else if core.uncertainty_avoidance < 0.3 {
            StapleType::Mixed
        } else {
            StapleType::GrainBread
        };

        // ── meat_role ──
        let meat_role = if is_grassland {
            MeatRole::Central
        } else if core.religiosity > 0.7 {
            MeatRole::Occasional
        } else {
            MeatRole::Regular
        };

        // ── alcohol_stance ──
        let alcohol_stance = if core.religiosity > 0.7 && core.indulgence < 0.3 {
            AlcoholStance::Discouraged
        } else if core.indulgence > 0.6 {
            AlcoholStance::Accepted
        } else if core.long_term_orientation > 0.7 {
            AlcoholStance::Ritualistic
        } else {
            AlcoholStance::Accepted
        };

        // ── dining_style ──
        let dining_style = if (1.0 - core.individualism) > 0.6 {
            DiningStyle::CommunalShared
        } else if core.individualism > 0.6 {
            DiningStyle::IndividualPlated
        } else {
            DiningStyle::CommunalFromCommon
        };

        // ── spice_preference ──
        let base_spice = if is_tropical { 0.7 } else { 0.3 };
        let spice_preference = (base_spice
            + core.indulgence * 0.3
            + core.openness_to_outsiders * 0.1
            + core.artistry * 0.1)
            .clamp(0.0, 1.0);

        Self {
            staple,
            meat_role,
            alcohol_stance,
            dining_style,
            spice_preference,
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
            for (grassland, tropical) in
                &[(false, false), (true, false), (false, true), (true, true)]
            {
                let diet = DietaryBasePreferences::derive_from(&core, *grassland, *tropical);
                assert!((0.0..=1.0).contains(&diet.spice_preference));
            }
        }
    }

    #[test]
    fn test_grassland_pastoral() {
        let core = CultureCoreParams::default();
        let diet = DietaryBasePreferences::derive_from(&core, true, false);
        assert_eq!(diet.staple, StapleType::Pastoral);
        assert_eq!(diet.meat_role, MeatRole::Central);
    }

    #[test]
    fn test_religious_occasional_meat() {
        let core = CultureCoreParams {
            religiosity: 0.9,
            ..CultureCoreParams::default()
        };
        let diet = DietaryBasePreferences::derive_from(&core, false, false);
        assert_eq!(diet.meat_role, MeatRole::Occasional);
    }

    #[test]
    fn test_high_indulgence_accepted_alcohol() {
        let core = CultureCoreParams {
            indulgence: 0.8,
            religiosity: 0.2,
            ..CultureCoreParams::default()
        };
        let diet = DietaryBasePreferences::derive_from(&core, false, false);
        assert_eq!(diet.alcohol_stance, AlcoholStance::Accepted);
    }

    #[test]
    fn test_religious_restrained_discouraged_alcohol() {
        let core = CultureCoreParams {
            religiosity: 0.9,
            indulgence: 0.1,
            ..CultureCoreParams::default()
        };
        let diet = DietaryBasePreferences::derive_from(&core, false, false);
        assert_eq!(diet.alcohol_stance, AlcoholStance::Discouraged);
    }

    #[test]
    fn test_collectivist_communal_dining() {
        let core = CultureCoreParams {
            individualism: 0.1,
            ..CultureCoreParams::default()
        };
        let diet = DietaryBasePreferences::derive_from(&core, false, false);
        assert_eq!(diet.dining_style, DiningStyle::CommunalShared);
    }

    #[test]
    fn test_individualist_plated_dining() {
        let core = CultureCoreParams {
            individualism: 0.9,
            ..CultureCoreParams::default()
        };
        let diet = DietaryBasePreferences::derive_from(&core, false, false);
        assert_eq!(diet.dining_style, DiningStyle::IndividualPlated);
    }

    #[test]
    fn test_tropical_high_spice() {
        let core = CultureCoreParams::default();
        let diet = DietaryBasePreferences::derive_from(&core, false, true);
        assert!(diet.spice_preference > 0.7);
    }

    #[test]
    fn test_temperate_low_spice() {
        let core = CultureCoreParams {
            indulgence: 0.0,
            openness_to_outsiders: 0.0,
            artistry: 0.0,
            ..CultureCoreParams::default()
        };
        let diet = DietaryBasePreferences::derive_from(&core, false, false);
        assert!((diet.spice_preference - 0.3).abs() < 0.01);
    }

    #[test]
    fn test_deterministic() {
        let core = CultureCoreParams::from_seed(42);
        let a = DietaryBasePreferences::derive_from(&core, true, false);
        let b = DietaryBasePreferences::derive_from(&core, true, false);
        assert_eq!(a.staple, b.staple);
        assert_eq!(a.meat_role, b.meat_role);
    }
}
