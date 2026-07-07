//! 生育规范 — 从 CultureCoreParams 的第一层派生
//!
//! 消费者: 生命系统（生育率、婚姻年龄）、世界生成（P8 人口模拟）
//!
//! 参见: `WoWorld-Design/Happy Game/开发阶段/文化系统/004-文化审美与物质.md` §4

use super::CultureCoreParams;

/// 生育与婚育文化规范
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FertilityNorms {
    /// 理想家庭规模 (最小, 最大), 子女数
    pub ideal_family_size: (u32, u32),
    /// 非婚生子女污名 [0,1]
    pub illegitimacy_stigma: f32,
    /// 子女性别偏好: -1=极度偏好女性, 0=无偏好, 1=极度偏好男性
    pub sex_preference: f32,
    /// 婚姻压力: 社会对适龄未婚者的催婚强度 [0,1]
    pub marriage_pressure: f32,
}

impl Default for FertilityNorms {
    fn default() -> Self {
        Self {
            ideal_family_size: (2, 4),
            illegitimacy_stigma: 0.5,
            sex_preference: 0.0,
            marriage_pressure: 0.5,
        }
    }
}

impl FertilityNorms {
    /// 从 CultureCoreParams 派生生育规范
    pub fn derive_from(core: &CultureCoreParams) -> Self {
        // ideal_family_size: 乘法公式（设计文档 004 §4）
        let base = if core.individualism < 0.4 { 4.0 } else { 2.0 };
        let adjusted = base * (1.0 - core.long_term_orientation * 0.5) * (1.0 + core.militarism * 0.5);
        let mid = adjusted.clamp(1.0, 8.0) as u32;
        let ideal_family_size = (mid.max(1) - 1, mid + 1);

        // illegitimacy_stigma = religiosity×0.5 + uncertainty×0.3 + power_distance×0.2
        let illegitimacy_stigma = (core.religiosity * 0.5
            + core.uncertainty_avoidance * 0.3
            + core.power_distance * 0.2)
            .clamp(0.0, 1.0);

        // sex_preference — Phase 1: 固定 0.0
        // 设计文档未指定完整公式；后续阶段从历史/经济/文化组合派生
        let sex_preference = 0.0;

        // marriage_pressure = religiosity×0.35 + power_distance×0.25
        //   + (1-individualism)×0.25 + uncertainty×0.15
        let marriage_pressure = (core.religiosity * 0.35
            + core.power_distance * 0.25
            + (1.0 - core.individualism) * 0.25
            + core.uncertainty_avoidance * 0.15)
            .clamp(0.0, 1.0);

        Self {
            ideal_family_size,
            illegitimacy_stigma,
            sex_preference,
            marriage_pressure,
        }
    }

    /// 年化生育率——每名育龄女性每年预期活产数
    ///
    /// 简化为 avg_children / 25_reproductive_years, 钳制在 [0.02, 0.50].
    pub fn annual_fertility_rate(&self) -> f32 {
        let avg_children = (self.ideal_family_size.0 + self.ideal_family_size.1) as f32 / 2.0;
        (avg_children / 25.0).clamp(0.02, 0.50)
    }

    /// 典型初婚年龄 (女, 男)
    ///
    /// base: 女=18, 男=20. 延迟: long_term_orientation×6. 提前: power_distance×3.
    /// 钳制: 女 [14,30], 男 [16,35].
    pub fn marriage_age_typical(core: &CultureCoreParams) -> (u8, u8) {
        let delay = core.long_term_orientation * 6.0;
        let early = core.power_distance * 3.0;
        let net = delay - early;

        let female = (18.0 + net).round().clamp(14.0, 30.0) as u8;
        let male = (20.0 + net).round().clamp(16.0, 35.0) as u8;
        (female, male)
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
            let norms = FertilityNorms::derive_from(&core);
            // 设计文档公式: (mid-1, mid+1) — 下界可为 0
            assert!(norms.ideal_family_size.1 >= 1 && norms.ideal_family_size.1 <= 9);
            assert!((0.0..=1.0).contains(&norms.illegitimacy_stigma));
            assert!((-1.0..=1.0).contains(&norms.sex_preference));
            assert!((0.0..=1.0).contains(&norms.marriage_pressure));
        }
    }

    #[test]
    fn test_individualist_small_family() {
        let core = CultureCoreParams {
            individualism: 0.9,
            long_term_orientation: 0.5,
            militarism: 0.0,
            ..CultureCoreParams::default()
        };
        let norms = FertilityNorms::derive_from(&core);
        // base (1,3), adjustment = 1-0.5*0.5+0*0.5=0.75, so (0.75, 2.25) → (1,2)
        assert!(norms.ideal_family_size.1 <= 3);
    }

    #[test]
    fn test_collectivist_large_family() {
        let core = CultureCoreParams {
            individualism: 0.1,
            long_term_orientation: 0.0,
            militarism: 1.0,
            ..CultureCoreParams::default()
        };
        let norms = FertilityNorms::derive_from(&core);
        // base (2,4), adjustment = 1-0+0.5=1.5, so (3,6)
        assert!(norms.ideal_family_size.1 >= 4);
    }

    #[test]
    fn test_religious_high_stigma() {
        let core = CultureCoreParams {
            religiosity: 1.0,
            uncertainty_avoidance: 1.0,
            power_distance: 1.0,
            ..CultureCoreParams::default()
        };
        let norms = FertilityNorms::derive_from(&core);
        assert!(norms.illegitimacy_stigma > 0.8);
    }

    #[test]
    fn test_annual_fertility_rate_in_range() {
        for seed in 0..100 {
            let core = CultureCoreParams::from_seed(seed);
            let norms = FertilityNorms::derive_from(&core);
            let rate = norms.annual_fertility_rate();
            assert!((0.02..=0.50).contains(&rate),
                "seed {seed}: rate {rate} not in [0.02, 0.50]");
        }
    }

    #[test]
    fn test_annual_fertility_rate_large_family() {
        let norms = FertilityNorms {
            ideal_family_size: (6, 8),
            ..FertilityNorms::default()
        };
        let rate = norms.annual_fertility_rate();
        assert!(rate > 0.20, "rate {rate} should be >0.20 for 6-8 children");
    }

    #[test]
    fn test_marriage_age_in_range() {
        for seed in 0..100 {
            let core = CultureCoreParams::from_seed(seed);
            let (f, m) = FertilityNorms::marriage_age_typical(&core);
            assert!((14..=30).contains(&f), "seed {seed}: female={f} not in [14,30]");
            assert!((16..=35).contains(&m), "seed {seed}: male={m} not in [16,35]");
        }
    }

    #[test]
    fn test_marriage_age_long_term_delays() {
        let core = CultureCoreParams {
            long_term_orientation: 1.0,
            power_distance: 0.0,
            ..CultureCoreParams::default()
        };
        let (f, m) = FertilityNorms::marriage_age_typical(&core);
        assert!(f >= 22);
        assert!(m >= 24);
    }

    #[test]
    fn test_marriage_age_power_distance_early() {
        let core = CultureCoreParams {
            long_term_orientation: 0.0,
            power_distance: 1.0,
            ..CultureCoreParams::default()
        };
        let (f, m) = FertilityNorms::marriage_age_typical(&core);
        assert!(f <= 17);
        assert!(m <= 19);
    }

    #[test]
    fn test_deterministic() {
        let core = CultureCoreParams::from_seed(42);
        let a = FertilityNorms::derive_from(&core);
        let b = FertilityNorms::derive_from(&core);
        assert_eq!(a.ideal_family_size, b.ideal_family_size);
        assert!((a.illegitimacy_stigma - b.illegitimacy_stigma).abs() < f32::EPSILON);
    }
}
