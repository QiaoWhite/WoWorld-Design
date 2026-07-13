//! PAD 三维情感 Component — ECS 铁律合规
//!
//! 参见: `开发文档/01-NPC核心/NPC活人感开发文档ver2.0.md` §2.1
//!
//! Pleasure-Arousal-Dominance (PAD) 是情感引擎的底层轴。Phase 1 仅包含三轴——
//! 8 种基本情绪、33 种复合情绪、Mood 心境层均为 Phase 2+。

use crate::components::bigfive::BigFive;
use serde::{Deserialize, Serialize};

/// PAD 三维情感状态——所有情绪表达的低维基底
///
/// - **pleasure** (-1..1): 不悦 → 愉悦
/// - **arousal**  (0..1): 平静 → 高度激活
/// - **control**  (-1..1): 无助 → 掌控
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Emotion {
    pub pleasure: f32,
    pub arousal: f32,
    pub control: f32,
}

impl Default for Emotion {
    fn default() -> Self {
        Self {
            pleasure: 0.0,
            arousal: 0.5,
            control: 0.0,
        }
    }
}

impl Emotion {
    /// 从 BigFive 计算 PAD 情感基线——人格决定的情感稳态
    ///
    /// 返回 `(baseline_pleasure, baseline_arousal, baseline_control)`。
    /// 情感漂移系统持续将当前状态拉向此基线。
    ///
    /// 公式来源: `drift_to_baseline()` §2.1.5
    pub fn baseline_from_bigfive(b: &BigFive) -> (f32, f32, f32) {
        let bp = 0.2 - b.neuroticism * 0.4; // N=0→0.2, N=1→-0.2
        let ba = 0.3 + b.extraversion * 0.3; // E=0→0.3, E=1→0.6
        let bc = b.conscientiousness * 0.6 - 0.1; // C=0→-0.1, C=1→0.5
        (bp, ba, bc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emotion_default_neutral() {
        let e = Emotion::default();
        assert!((e.pleasure - 0.0).abs() < f32::EPSILON);
        assert!((e.arousal - 0.5).abs() < f32::EPSILON);
        assert!((e.control - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_baseline_neurotic_low_pleasure() {
        let b = BigFive {
            neuroticism: 1.0,
            ..BigFive::default()
        };
        let (bp, _, _) = Emotion::baseline_from_bigfive(&b);
        assert!((bp + 0.2).abs() < 0.01); // ≈ -0.2
    }

    #[test]
    fn test_baseline_stable_high_pleasure() {
        let b = BigFive {
            neuroticism: 0.0,
            ..BigFive::default()
        };
        let (bp, _, _) = Emotion::baseline_from_bigfive(&b);
        assert!((bp - 0.2).abs() < 0.01); // ≈ 0.2
    }

    #[test]
    fn test_baseline_extravert_high_arousal() {
        let b = BigFive {
            extraversion: 1.0,
            ..BigFive::default()
        };
        let (_, ba, _) = Emotion::baseline_from_bigfive(&b);
        assert!((ba - 0.6).abs() < 0.01);
    }

    #[test]
    fn test_baseline_introvert_low_arousal() {
        let b = BigFive {
            extraversion: 0.0,
            ..BigFive::default()
        };
        let (_, ba, _) = Emotion::baseline_from_bigfive(&b);
        assert!((ba - 0.3).abs() < 0.01);
    }

    #[test]
    fn test_baseline_conscientious_high_control() {
        let b = BigFive {
            conscientiousness: 1.0,
            ..BigFive::default()
        };
        let (_, _, bc) = Emotion::baseline_from_bigfive(&b);
        assert!((bc - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_baseline_casual_low_control() {
        let b = BigFive {
            conscientiousness: 0.0,
            ..BigFive::default()
        };
        let (_, _, bc) = Emotion::baseline_from_bigfive(&b);
        assert!((bc + 0.1).abs() < 0.01); // ≈ -0.1
    }
}
