//! 移动相关 ECS Component — CMovementState, CMovementRecovery, CMoveIntent, CMovementControl
//!
//! 纯数据，零方法（铁律 1）。无堆数据内联（铁律 2）。
//!
//! 参见: `WoWorld-Design/.../角色控制器/002-MovementState与连续移动.md` §十二

use glam::{Mat4, Vec3};
use woworld_core::movement::{MovementRecoveryStack, MovementState};
use woworld_core::kinematics::{MovementLock, RotationLock};

/// 当前移动状态——MovementModeSystem 写入，MovementSystem 消费。
#[derive(Debug, Clone, Copy)]
#[derive(Default)]
pub struct CMovementState(pub MovementState);


/// 上一帧的移动状态——用于检测状态变化（was_grounded → now_falling 等）。
#[derive(Debug, Clone, Copy)]
#[derive(Default)]
pub struct CPrevMovementState(pub MovementState);


/// 介质变迁恢复栈——MovementModeSystem 管理。
#[derive(Debug, Clone, Copy, Default)]
pub struct CMovementRecovery(pub MovementRecoveryStack);

/// 移动意图——ActionResolver（玩家）或 GOAP（NPC）写入。
///
/// `camera_transform` 用于将输入方向从相机空间转到世界空间。
/// Sprint 1: NPC 不写此 Component（旧管线），仅测试实体使用。
#[derive(Debug, Clone, Copy)]
pub struct CMoveIntent {
    /// 归一化移动方向（世界空间 XZ 平面）
    pub direction: Vec3,
    /// 相机变换矩阵（用于方向解算）
    pub camera_transform: Mat4,
    /// 期望的移动状态（可选——None 表示保持当前）
    pub desired_state: Option<MovementState>,
}

impl Default for CMoveIntent {
    fn default() -> Self {
        Self {
            direction: Vec3::ZERO,
            camera_transform: Mat4::IDENTITY,
            desired_state: None,
        }
    }
}

/// 移动/朝向控制锁——ActionController 写入，MovementSystem 读取。
#[derive(Debug, Clone, Copy)]
pub struct CMovementControl {
    pub movement_lock: MovementLock,
    pub rotation_lock: RotationLock,
}

impl Default for CMovementControl {
    fn default() -> Self {
        Self {
            movement_lock: MovementLock::Free,
            rotation_lock: RotationLock::Free,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmovement_state_default() {
        let c = CMovementState::default();
        assert_eq!(c.0.stance, woworld_core::movement::Stance::Standing);
    }

    #[test]
    fn test_cprev_movement_state_default() {
        let c = CPrevMovementState::default();
        assert_eq!(c.0.pace, woworld_core::movement::Pace::Still);
    }

    #[test]
    fn test_cmovement_intent_default_zero_direction() {
        let c = CMoveIntent::default();
        assert_eq!(c.direction, Vec3::ZERO);
        assert!(c.desired_state.is_none());
    }

    #[test]
    fn test_cmovement_control_default_free() {
        let c = CMovementControl::default();
        assert_eq!(c.movement_lock, MovementLock::Free);
        assert_eq!(c.rotation_lock, RotationLock::Free);
    }

    #[test]
    fn test_cmovement_intent_with_direction() {
        let c = CMoveIntent {
            direction: Vec3::new(1.0, 0.0, 0.0),
            camera_transform: Mat4::IDENTITY,
            desired_state: Some(MovementState::default()),
        };
        assert!((c.direction.x - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cmovement_control_full_lock() {
        let c = CMovementControl {
            movement_lock: MovementLock::Full,
            rotation_lock: RotationLock::TargetDirection,
        };
        assert_eq!(c.movement_lock, MovementLock::Full);
    }
}
