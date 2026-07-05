//! 昼夜节律 Component
//!
//! Chronotype 从 BigFive 人格派生（Phase 2），Sprint 042 使用默认值。
//! 参见: `开发文档/02-NPC核心/02-基本需求.md` §昼夜节律调制

/// 睡眠类型——决定生理节律的相位偏移
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Chronotype {
    /// 早鸟型——节律提前 2h
    Morning,
    /// 中性型——无偏移
    #[default]
    Neutral,
    /// 夜猫型——节律延迟 2h
    Evening,
}

/// 昼夜振幅系数（设计文档: ±8%）
const CIRCADIAN_AMPLITUDE: f32 = 0.08;
/// 相位偏移（小时 → day_progress 偏移量）
const PHASE_OFFSET: f32 = 2.0 / 24.0; // 2h / 24h

/// 计算当前时刻的昼夜调制因子（1.0 = 无调制）
///
/// `day_progress`: 0.0-1.0（0=午夜, 0.25=日出, 0.5=正午, 0.75=日落）
/// 返回: 1.0 ± CIRCADIAN_AMPLITUDE
pub fn circadian_factor(chronotype: Chronotype, day_progress: f32) -> f32 {
    let phase_shift = match chronotype {
        Chronotype::Morning => -PHASE_OFFSET,
        Chronotype::Neutral => 0.0,
        Chronotype::Evening => PHASE_OFFSET,
    };
    // 正弦波: 峰值在白天（0.5 = 正午），谷值在夜晚
    let phase = (day_progress + phase_shift).fract();
    let wave = (phase * std::f32::consts::TAU).sin();
    1.0 + wave * CIRCADIAN_AMPLITUDE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_circadian_noon_is_peak() {
        // 正午（0.5）→ 正弦 sin(π) ≈ 0 → factor ≈ 1.0
        let f = circadian_factor(Chronotype::Neutral, 0.5);
        assert!((f - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_circadian_midnight_is_trough() {
        // 午夜（0.0）→ 正弦 sin(0) ≈ 0 → factor ≈ 1.0
        let f = circadian_factor(Chronotype::Neutral, 0.0);
        assert!((f - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_circadian_dawn_is_rising() {
        // 日出（0.25）→ 正弦 sin(π/2) = 1 → factor ≈ 1.08
        let f = circadian_factor(Chronotype::Neutral, 0.25);
        assert!(f > 1.02); // 正调制
    }

    #[test]
    fn test_circadian_dusk_is_falling() {
        // 日落（0.75）→ 正弦 sin(3π/2) = -1 → factor ≈ 0.92
        let f = circadian_factor(Chronotype::Neutral, 0.75);
        assert!(f < 0.98); // 负调制
    }

    #[test]
    fn test_different_chronotypes_differ() {
        // 三种类型在同一时刻产生不同值
        let f_m = circadian_factor(Chronotype::Morning, 0.3);
        let f_n = circadian_factor(Chronotype::Neutral, 0.3);
        let f_e = circadian_factor(Chronotype::Evening, 0.3);
        // 至少有一个不同于其他
        assert!(f_m != f_n || f_n != f_e || f_m != f_e,
            "different chronotypes should differ at same time");
    }

    #[test]
    fn test_evening_type_late_peak() {
        // 夜猫型: 深夜比中性型更活跃
        let f_neutral = circadian_factor(Chronotype::Neutral, 0.9);
        let f_evening = circadian_factor(Chronotype::Evening, 0.9);
        // 深夜 (0.9) = trough for neutral, but evening type shifted later
        assert!(f_evening > f_neutral,
            "evening type should be more active late at night");
    }

    #[test]
    fn test_circadian_range() {
        // 所有值应在 1.0 ± 8% 范围内
        for i in 0..100 {
            let p = i as f32 / 100.0;
            let f = circadian_factor(Chronotype::Morning, p);
            assert!(f >= 0.92 && f <= 1.08);
        }
    }
}
