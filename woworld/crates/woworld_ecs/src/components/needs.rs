//! NPC 基本需求 Component — ECS 铁律合规
//!
//! 参见: `开发文档/02-NPC核心/02-基本需求.md`

/// 基本生理需求——3 维驱动力，0=满足 → 1=极度缺乏
#[derive(Debug, Clone, Copy)]
pub struct Needs {
    pub hunger: f32,
    pub thirst: f32,
    pub fatigue: f32,
}

impl Default for Needs {
    fn default() -> Self {
        Self { hunger: 0.0, thirst: 0.0, fatigue: 0.0 }
    }
}

/// 需求敏感度——从 BigFive 人格派生，Sprint 040 使用默认值
#[derive(Debug, Clone, Copy)]
pub struct NeedSensitivity {
    pub hunger_sens: f32,
    pub thirst_sens: f32,
    pub fatigue_sens: f32,
}

impl Default for NeedSensitivity {
    fn default() -> Self {
        Self { hunger_sens: 1.0, thirst_sens: 1.0, fatigue_sens: 1.0 }
    }
}

/// 欲望种类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesireKind {
    Eat,
    Drink,
    Rest,
}

/// 当前欲望——由 NeedEvaluation 写入，GoalResolution 消费
#[derive(Debug, Clone, Copy)]
pub struct Desire {
    pub kind: DesireKind,
    /// 紧急性 0→1，>0.8 触发 GOAP 目标选择
    pub urgency: f32,
}

impl Default for Desire {
    fn default() -> Self {
        Self { kind: DesireKind::Eat, urgency: 0.0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_needs_default_is_zero() {
        let n = Needs::default();
        assert_eq!(n.hunger, 0.0);
        assert_eq!(n.thirst, 0.0);
        assert_eq!(n.fatigue, 0.0);
    }

    #[test]
    fn test_sensitivity_default_is_one() {
        let s = NeedSensitivity::default();
        assert_eq!(s.hunger_sens, 1.0);
    }
}
