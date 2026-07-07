//! 关系规范 — 从 CultureCoreParams 的第一层派生 (严格对齐设计文档 004 §4.2)

use super::CultureCoreParams;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResidencePattern { Patrilocal, Matrilocal, #[default] Neolocal, Ambilocal }

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RelationshipNorms {
    pub monogamous_default: bool,
    pub polygyny_acceptance: f32,
    pub polyandry_acceptance: f32,   // 硬上限 0.3
    pub polyamory_acceptance: f32,   // 硬上限 0.4
    pub group_marriage_acceptance: f32, // 硬上限 0.2
    pub arranged_marriage_tendency: f32,
    pub contractual_marriage_acceptance: f32, // 硬上限 0.6
    pub homosexuality_acceptance: f32,
    pub same_sex_marriage_legal: f32,
    pub conjugal_birth_norm: f32,
    pub divorce_acceptance: f32,
    pub max_spouses: Option<u8>,
    pub residence_pattern: ResidencePattern,
}

impl Default for RelationshipNorms {
    fn default() -> Self {
        Self {
            monogamous_default: true, polygyny_acceptance: 0.0, polyandry_acceptance: 0.0,
            polyamory_acceptance: 0.0, group_marriage_acceptance: 0.0,
            arranged_marriage_tendency: 0.5, contractual_marriage_acceptance: 0.0,
            homosexuality_acceptance: 0.5, same_sex_marriage_legal: 0.0,
            conjugal_birth_norm: 0.5, divorce_acceptance: 0.5,
            max_spouses: None, residence_pattern: ResidencePattern::default(),
        }
    }
}

impl RelationshipNorms {
    /// 从 CultureCoreParams 派生完整关系规范（严格对齐 004 §4.2）
    pub fn derive_from(core: &CultureCoreParams) -> Self {
        // hierarchy = power_distance × religiosity（设计文档核心构造）
        let hierarchy = core.power_distance * core.religiosity;

        // polygyny = hierarchy × 0.8 + militarism × 0.2 - individualism × 0.4
        let polygyny_acceptance = (hierarchy * 0.8 + core.militarism * 0.2
            - core.individualism * 0.4).clamp(0.0, 1.0);

        // polyandry = (1-power_distance) × 0.3 + long_term × 0.1 - religiosity × 0.3, cap 0.3
        let polyandry_acceptance = ((1.0 - core.power_distance) * 0.3
            + core.long_term_orientation * 0.1 - core.religiosity * 0.3)
            .clamp(0.0, 1.0).min(0.3);

        // polyamory = individualism × 0.5 + indulgence × 0.3 - religiosity × 0.6 - uncertainty × 0.2, cap 0.4
        let polyamory_acceptance = (core.individualism * 0.5 + core.indulgence * 0.3
            - core.religiosity * 0.6 - core.uncertainty_avoidance * 0.2)
            .clamp(0.0, 1.0).min(0.4);

        // group_marriage = (1-individualism) × 0.3 + (1-power_distance) × 0.2 - religiosity × 0.5, cap 0.2
        let group_marriage_acceptance = ((1.0 - core.individualism) * 0.3
            + (1.0 - core.power_distance) * 0.2 - core.religiosity * 0.5)
            .clamp(0.0, 1.0).min(0.2);

        // monogamous_default = individualism > 0.5 || hierarchy < 0.3
        let monogamous_default = core.individualism > 0.5 || hierarchy < 0.3;

        // arranged_marriage = power_distance × 0.7 + (1-individualism) × 0.3 + uncertainty × 0.2
        let arranged_marriage_tendency = (core.power_distance * 0.7
            + (1.0 - core.individualism) * 0.3 + core.uncertainty_avoidance * 0.2)
            .clamp(0.0, 1.0);

        // contractual_marriage = individualism × 0.4 + long_term × 0.3 - religiosity × 0.5, cap 0.6
        let contractual_marriage_acceptance = (core.individualism * 0.4
            + core.long_term_orientation * 0.3 - core.religiosity * 0.5)
            .clamp(0.0, 1.0).min(0.6);

        // homosexuality_acceptance = individualism × 0.6 + openness × 0.2
        //   - religiosity × 0.5 - uncertainty × 0.3 - power_distance × 0.3
        let homosexuality_acceptance = (core.individualism * 0.6
            + core.openness_to_outsiders * 0.2 - core.religiosity * 0.5
            - core.uncertainty_avoidance * 0.3 - core.power_distance * 0.3)
            .clamp(0.0, 1.0);

        // same_sex_marriage_legal: 3 层连续阈值
        let same_sex_marriage_legal = if homosexuality_acceptance > 0.7 {
            homosexuality_acceptance
        } else if homosexuality_acceptance > 0.4 {
            homosexuality_acceptance * 0.5
        } else { 0.0 };

        // conjugal_birth_norm = religiosity × 0.6 + power_distance × 0.3
        //   + uncertainty × 0.2 - individualism × 0.3
        let conjugal_birth_norm = (core.religiosity * 0.6 + core.power_distance * 0.3
            + core.uncertainty_avoidance * 0.2 - core.individualism * 0.3)
            .clamp(0.0, 1.0);

        // divorce_acceptance = individualism × 0.5 + indulgence × 0.2
        //   - religiosity × 0.6 - uncertainty × 0.3 - long_term × 0.2
        let divorce_acceptance = (core.individualism * 0.5 + core.indulgence * 0.2
            - core.religiosity * 0.6 - core.uncertainty_avoidance * 0.3
            - core.long_term_orientation * 0.2).clamp(0.0, 1.0);

        // max_spouses: 设计文档公式——截断取整
        let max_spouses = if polygyny_acceptance > 0.5 || polyandry_acceptance > 0.3 {
            Some((2.0 + core.power_distance * 6.0) as u8)
        } else { None };

        // residence_pattern: max-score among 4 weighted sums
        let patrilocal_score = core.power_distance * 0.50
            + core.uncertainty_avoidance * 0.25 + (1.0 - core.individualism) * 0.15
            + core.militarism * 0.10;
        let matrilocal_score = (1.0 - core.power_distance) * 0.35
            + (1.0 - core.militarism) * 0.30 + core.individualism * 0.20
            + core.openness_to_outsiders * 0.15;
        let neolocal_score = core.individualism * 0.50
            + core.openness_to_outsiders * 0.20 + (1.0 - core.uncertainty_avoidance) * 0.20
            + core.indulgence * 0.10;
        let ambilocal_score = core.openness_to_outsiders * 0.50
            + (1.0 - core.uncertainty_avoidance) * 0.30 + core.indulgence * 0.20;

        let residence_pattern = {
            let scores = [
                (ResidencePattern::Patrilocal, patrilocal_score),
                (ResidencePattern::Matrilocal, matrilocal_score),
                (ResidencePattern::Neolocal, neolocal_score),
                (ResidencePattern::Ambilocal, ambilocal_score),
            ];
            scores.iter().max_by(|a, b| a.1.total_cmp(&b.1)).map(|(p, _)| *p)
                .unwrap_or(ResidencePattern::Neolocal)
        };

        Self { monogamous_default, polygyny_acceptance, polyandry_acceptance,
            polyamory_acceptance, group_marriage_acceptance, arranged_marriage_tendency,
            contractual_marriage_acceptance, homosexuality_acceptance, same_sex_marriage_legal,
            conjugal_birth_norm, divorce_acceptance, max_spouses, residence_pattern }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_fields_in_range() {
        for seed in 0..100 {
            let core = CultureCoreParams::from_seed(seed);
            let n = RelationshipNorms::derive_from(&core);
            assert!((0.0..=1.0).contains(&n.polygyny_acceptance));
            assert!((0.0..=0.3).contains(&n.polyandry_acceptance));
            assert!((0.0..=0.4).contains(&n.polyamory_acceptance));
            assert!((0.0..=0.2).contains(&n.group_marriage_acceptance));
            assert!((0.0..=1.0).contains(&n.arranged_marriage_tendency));
            assert!((0.0..=0.6).contains(&n.contractual_marriage_acceptance));
            assert!((0.0..=1.0).contains(&n.homosexuality_acceptance));
            assert!((0.0..=1.0).contains(&n.conjugal_birth_norm));
            assert!((0.0..=1.0).contains(&n.divorce_acceptance));
            if let Some(ms) = n.max_spouses { assert!((2..=8).contains(&ms)); }
        }
    }

    #[test]
    fn test_hierarchy_polygyny() {
        let core = CultureCoreParams { power_distance: 1.0, religiosity: 1.0, militarism: 1.0, individualism: 0.0, ..Default::default() };
        let n = RelationshipNorms::derive_from(&core);
        assert!(n.polygyny_acceptance > 0.6);
    }

    #[test]
    fn test_caps_enforced() {
        let core = CultureCoreParams { individualism: 1.0, indulgence: 1.0, religiosity: 0.0,
            uncertainty_avoidance: 0.0, power_distance: 0.0, long_term_orientation: 0.0,
            openness_to_outsiders: 1.0, militarism: 0.0, ..Default::default() };
        let n = RelationshipNorms::derive_from(&core);
        assert!(n.polyandry_acceptance <= 0.3);
        assert!(n.polyamory_acceptance <= 0.4);
        assert!(n.group_marriage_acceptance <= 0.2);
        assert!(n.contractual_marriage_acceptance <= 0.6);
    }

    #[test]
    fn test_individualist_divorce() {
        let core = CultureCoreParams { individualism: 1.0, indulgence: 1.0, religiosity: 0.0,
            uncertainty_avoidance: 0.0, long_term_orientation: 0.0, ..Default::default() };
        assert!(RelationshipNorms::derive_from(&core).divorce_acceptance > 0.6);
    }

    #[test]
    fn test_deterministic() {
        let core = CultureCoreParams::from_seed(42);
        let a = RelationshipNorms::derive_from(&core);
        let b = RelationshipNorms::derive_from(&core);
        assert_eq!(a.monogamous_default, b.monogamous_default);
        assert!((a.polygyny_acceptance - b.polygyny_acceptance).abs() < f32::EPSILON);
    }
}
