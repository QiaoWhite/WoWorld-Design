//! NPC 目标 Component
//!
//! GoalResolutionSystem 将 Desire 转换为具体 Goal。
//! Goal 被行动 System（Sprint 042+）消费——选择具体原子动作。

use glam::Vec3;

/// 目标类型——"NPC 想做什么"（对应 7 维 DesireKind）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoalType {
    Idle,
    FindFood,
    FindWater,
    FindRest,
    FindSafePlace,
    FindSocialContact,
    BalanceElements,
    ExpressLibido,
}

/// 当前活跃目标
#[derive(Debug, Clone, Copy)]
pub struct Goal {
    pub goal_type: GoalType,
    /// 紧急性 0→1（从 Desire 透传）
    pub urgency: f32,
    /// 目标位置——Phase 2 由知识 System 填充
    pub target_pos: Option<Vec3>,
}

impl Default for Goal {
    fn default() -> Self {
        Self {
            goal_type: GoalType::Idle,
            urgency: 0.0,
            target_pos: None,
        }
    }
}

/// ★ V3a: movement 到达后插入，harvest_on_arrival_system 消费后移除。
///
/// 将"到达"和"采集"解耦——movement 只负责移动，harvest 负责采集。
/// 设计对应：`PathFollowingSystem → ActionRequest::MovementArrived`（010-NPC移动行为）。
#[derive(Debug, Clone, Copy)]
pub struct ArrivedAtTarget {
    pub goal_type: GoalType,
    pub target_pos: Vec3,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_goal_default_is_idle() {
        let g = Goal::default();
        assert_eq!(g.goal_type, GoalType::Idle);
        assert_eq!(g.urgency, 0.0);
    }
}
