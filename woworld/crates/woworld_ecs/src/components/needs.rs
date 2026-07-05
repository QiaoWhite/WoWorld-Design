//! NPC 基本需求 Component — ECS 铁律合规
//!
//! 参见: `开发文档/02-NPC核心/02-基本需求.md` §需求分类（4 类 × 7 维度）

/// 基本生理与心理需求——7 维驱动力，0=满足 → 1=极度缺乏
///
/// | 类别 | 维度 | 衰减方式 |
/// |------|------|---------|
/// | 缺失驱动 | hunger, thirst, fatigue, safety | 随时间累积 |
/// | 心理驱动 | social | 独处积累, 交互恢复 |
/// | 积累驱动 | element_balance | 瓶颈模型 |
/// | 周期驱动 | libido | 周期型衰减 |
#[derive(Debug, Clone, Copy)]
pub struct Needs {
    pub hunger: f32,
    pub thirst: f32,
    pub fatigue: f32,
    pub safety: f32,
    pub social: f32,
    pub element_balance: f32, // 瓶颈模型——最缺的元素决定 urgency
    pub libido: f32,          // 周期驱动
}

impl Default for Needs {
    fn default() -> Self {
        Self {
            hunger: 0.0, thirst: 0.0, fatigue: 0.0,
            safety: 0.0, social: 0.0, element_balance: 0.0, libido: 0.0,
        }
    }
}

/// 需求敏感度——从 BigFive 人格派生（Sprint 042 使用默认值，Phase 2 接入 BigFive）
/// Sprint 053: 扩展 7→9 维（+esteem +competence，v2.0 进阶需求系统）
#[derive(Debug, Clone, Copy)]
pub struct NeedSensitivity {
    pub hunger_sens: f32,
    pub thirst_sens: f32,
    pub fatigue_sens: f32,
    pub safety_sens: f32,
    pub social_sens: f32,
    pub element_sens: f32,
    pub libido_sens: f32,
    /// v2.0 进阶需求: 尊重认可敏感度
    pub esteem_sens: f32,
    /// v2.0 进阶需求: 胜任挫折敏感度
    pub competence_sens: f32,
}

impl Default for NeedSensitivity {
    fn default() -> Self {
        Self {
            hunger_sens: 1.0, thirst_sens: 1.0, fatigue_sens: 1.0,
            safety_sens: 1.0, social_sens: 1.0, element_sens: 1.0, libido_sens: 1.0,
            esteem_sens: 1.0, competence_sens: 1.0,
        }
    }
}

/// 欲望种类——对应 Needs 的 7 个维度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesireKind {
    Eat,
    Drink,
    Rest,
    SeekSafety,
    Socialize,
    BalanceElements,
    ExpressLibido,
}

/// 当前欲望——由 NeedEvaluation 写入，GoalResolution 消费
#[derive(Debug, Clone, Copy)]
pub struct Desire {
    pub kind: DesireKind,
    /// 紧急性 0→1，>0.8 触发目标选择
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
    fn test_needs_default_all_zero() {
        let n = Needs::default();
        assert_eq!(n.hunger, 0.0);
        assert_eq!(n.social, 0.0);
        assert_eq!(n.libido, 0.0);
    }

    #[test]
    fn test_needs_has_seven_dimensions() {
        // 验证 7 维结构（编译期保证 + 运行时烟雾测试）
        let n = Needs::default();
        let _ = (
            n.hunger, n.thirst, n.fatigue, n.safety,
            n.social, n.element_balance, n.libido,
        );
    }

    #[test]
    fn test_sensitivity_has_nine_dimensions() {
        let s = NeedSensitivity::default();
        let _ = (
            s.hunger_sens, s.thirst_sens, s.fatigue_sens, s.safety_sens,
            s.social_sens, s.element_sens, s.libido_sens,
            s.esteem_sens, s.competence_sens,
        );
    }

    #[test]
    fn test_desire_kind_count() {
        // 7 维 Needs → 7 种 DesireKind
        let kinds = [
            DesireKind::Eat, DesireKind::Drink, DesireKind::Rest,
            DesireKind::SeekSafety, DesireKind::Socialize,
            DesireKind::BalanceElements, DesireKind::ExpressLibido,
        ];
        assert_eq!(kinds.len(), 7);
    }
}
