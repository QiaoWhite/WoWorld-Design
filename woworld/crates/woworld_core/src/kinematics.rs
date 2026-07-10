//! 运动学基础类型 — LocomotionMode stub + 移动/朝向锁 + 物理约束
//!
//! CHG-067-SHIM: LocomotionMode 当前为 2 变体最小 stub。
//! CHG-067 实现后将扩展为三态（Grounded/PhysicsBody/Attached）并添加 ImpulseQueue。
//!
//! 参见: `WoWorld-Design/.../角色控制器/001-角色控制器总纲.md` §三
//!       `WoWorld-Design/Change/CHG-067-物理运动学地基-20260709.md`

use glam::Vec3;

// ── LocomotionMode ──────────────────────────────────────────────
// CHG-067-SHIM: 仅 Grounded 和 PhysicsBody 两态。
// 待 CHG-067 实现后替换为完整三态机（含 Attached + ImpulseQueue）。

/// 运动模式——实体处于何种物理状态。
///
/// Sprint 1 stub: Grounded（地形跟随） | PhysicsBody（COM 积分，未实现）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LocomotionMode {
    /// 着地——地形跟随、坡度检测、加速度/摩擦控制
    #[default]
    Grounded,
    /// 物理体——空中/被击飞/水中。Sprint 1 仅作标记，无 COM 积分
    PhysicsBody,
}

// ── MovementLock ────────────────────────────────────────────────

/// 移动锁——ActionController 写入，MovementSystem 读取。
///
/// 决定移动速度和方向是否被当前动作覆盖。
/// 参见: `002-MovementState与连续移动.md` §十一
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum MovementLock {
    /// 正常——读 MovementState 的速度曲线
    #[default]
    Free,
    /// 减速上限——防御/瞄准中可慢走
    Partial {
        /// 最大允许速度 (m/s)
        speed_cap: f32,
    },
    /// 原地不动——摩擦减速到零
    Full,
    /// 动作接管——闪避/跳跃的强制位移
    Override(Vec3),
}

// ── RotationLock ────────────────────────────────────────────────

/// 朝向锁——ActionController 写入，MovementSystem 读取。
///
/// 决定身体朝向的约束。
/// 参见: `002-MovementState与连续移动.md` §十一
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RotationLock {
    /// 不强制——保持上一帧朝向
    #[default]
    Free,
    /// 面朝摇杆方向
    InputDirection,
    /// 面朝镜头（战斗/防御默认）
    CameraForward,
    /// 面朝锁定目标
    TargetDirection,
    /// 完全锁住
    Locked,
}

// ── PhysicsRequirement ──────────────────────────────────────────

/// 动作的物理约束——决定动作在什么 LocomotionMode 下可执行。
///
/// ActionController 在接受新请求前检查此约束。
/// 参见: `003-ActionController与离散动作.md` §4.1
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub enum PhysicsRequirement {
    /// 必须在地面——Block/Dodge/LightAttack
    Grounded,
    /// 地面或空中——AimBow
    NotInWater,
    /// 任何 LocomotionMode——ChannelSpell/EmergencyDismount
    Any,
    /// 必须在水中——Dive
    InWater,
    /// 不能在被击飞/完全坠落中——大部分自愿动作
    NotAirborne,
}

impl PhysicsRequirement {
    /// 检查当前 LocomotionMode 是否满足此物理约束。
    ///
    /// Sprint 1: LocomotionMode 仅 2 变体，InWater/NotAirborne 暂时宽松。
    /// CHG-067 实现后精细化。
    pub fn is_satisfied_by(&self, loco: LocomotionMode) -> bool {
        match self {
            Self::Grounded => loco == LocomotionMode::Grounded,
            Self::NotInWater => true, // Sprint 1: 无水下检测，始终通过
            Self::Any => true,
            Self::InWater => false, // Sprint 1: 无水下，水中动作不可用
            Self::NotAirborne => loco == LocomotionMode::Grounded,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── LocomotionMode ──

    #[test]
    fn test_locomotion_mode_default_is_grounded() {
        assert_eq!(LocomotionMode::default(), LocomotionMode::Grounded);
    }

    #[test]
    fn test_locomotion_mode_eq() {
        assert_eq!(LocomotionMode::Grounded, LocomotionMode::Grounded);
        assert_ne!(LocomotionMode::Grounded, LocomotionMode::PhysicsBody);
    }

    // ── MovementLock ──

    #[test]
    fn test_movement_lock_default_is_free() {
        assert_eq!(MovementLock::default(), MovementLock::Free);
    }

    #[test]
    fn test_movement_lock_partial_holds_speed() {
        let lock = MovementLock::Partial { speed_cap: 1.0 };
        assert_eq!(lock, MovementLock::Partial { speed_cap: 1.0 });
    }

    #[test]
    fn test_movement_lock_override_holds_vector() {
        let v = Vec3::new(3.0, 0.0, 0.0);
        let lock = MovementLock::Override(v);
        assert_eq!(lock, MovementLock::Override(v));
    }

    // ── RotationLock ──

    #[test]
    fn test_rotation_lock_default_is_free() {
        assert_eq!(RotationLock::default(), RotationLock::Free);
    }

    #[test]
    fn test_rotation_lock_all_variants_distinct() {
        let variants = [
            RotationLock::Free,
            RotationLock::InputDirection,
            RotationLock::CameraForward,
            RotationLock::TargetDirection,
            RotationLock::Locked,
        ];
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i], variants[j]);
            }
        }
    }

    // ── PhysicsRequirement ──

    #[test]
    fn test_physics_req_grounded() {
        assert!(PhysicsRequirement::Grounded.is_satisfied_by(LocomotionMode::Grounded));
        assert!(!PhysicsRequirement::Grounded.is_satisfied_by(LocomotionMode::PhysicsBody));
    }

    #[test]
    fn test_physics_req_not_in_water_always_passes_sprint1() {
        assert!(PhysicsRequirement::NotInWater.is_satisfied_by(LocomotionMode::Grounded));
        assert!(PhysicsRequirement::NotInWater.is_satisfied_by(LocomotionMode::PhysicsBody));
    }

    #[test]
    fn test_physics_req_any_always_passes() {
        assert!(PhysicsRequirement::Any.is_satisfied_by(LocomotionMode::Grounded));
        assert!(PhysicsRequirement::Any.is_satisfied_by(LocomotionMode::PhysicsBody));
    }

    #[test]
    fn test_physics_req_in_water_always_fails_sprint1() {
        assert!(!PhysicsRequirement::InWater.is_satisfied_by(LocomotionMode::Grounded));
        assert!(!PhysicsRequirement::InWater.is_satisfied_by(LocomotionMode::PhysicsBody));
    }

    #[test]
    fn test_physics_req_not_airborne() {
        assert!(PhysicsRequirement::NotAirborne.is_satisfied_by(LocomotionMode::Grounded));
        assert!(!PhysicsRequirement::NotAirborne.is_satisfied_by(LocomotionMode::PhysicsBody));
    }
}
