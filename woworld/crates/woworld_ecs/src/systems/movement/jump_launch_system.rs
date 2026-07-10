//! JumpLaunchSystem — 跳跃起跳：动作激活瞬间注入垂直速度 + 置腾空态
//!
//! 在 `action_system`（推进动作相位）之后、`movement_system`（积分）之前运行。
//! 边沿检测：active 动作为 jump、进入 Active 相位、且当前非腾空（`special` 为 None）→ 起跳。
//! 一旦 `special` 变 Airborne，条件不再满足，天然一次性——无需额外 flag。
//!
//! 绞杀者：仅作用于带 CActiveAction + CMovementState + Velocity 的角色控制器实体。
//!
//! 参见: `WoWorld-Design/.../角色控制器/003-ActionController与离散动作.md`（jump 动作）,
//!       `002-MovementState与连续移动.md`（AirState）, `001-角色控制器总纲.md` §四

use woworld_core::action::ActionPhase;
use woworld_core::movement::{AirState, JumpHeight, SpecialMode};

use crate::components::action_state::CActiveAction;
use crate::components::movement_state::{CMovementRecovery, CMovementState};
use crate::components::transform::Velocity;
use crate::resources::action_registry::ActionRegistry;
use crate::resources::movement_profile_registry::MovementProfileRegistry;

/// 跳跃起跳系统——jump 动作进入 Active 时注入 `jump_speed` 垂直速度 + 置 Airborne。
///
/// ★ 002 §二：腾空进入前把当前自愿状态 push 到恢复栈——落地
/// `movement_mode_system.pop_compatible` 才能恢复起跳前的 pace（否则弹空栈得
/// `{Standing, Still}` → `max_speed=0` → 落地后无法移动）。
pub fn jump_launch_system(world: &mut hecs::World, profiles: &MovementProfileRegistry) {
    let jump_id = ActionRegistry::id_of("jump");
    let jump_speed = profiles.default_profile().jump_speed;

    for (_, (active, ms, recovery, vel)) in world.query_mut::<(
        &CActiveAction,
        &mut CMovementState,
        &mut CMovementRecovery,
        &mut Velocity,
    )>() {
        let Some(a) = &active.0 else { continue };
        if a.action_id == jump_id && a.phase == ActionPhase::Active && ms.0.special.is_none() {
            // 起跳前保存自愿地面状态——落地由恢复栈弹回（保住 pace，002 §二）
            recovery.0.push_if_voluntary(ms.0);
            vel.0.y = jump_speed;
            ms.0.special = Some(SpecialMode::Airborne(AirState::Jumping {
                control_ratio: 0.7, // 002 §四：跳跃空中可控转向 0.7
                height: JumpHeight::Normal,
            }));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::action_state::CActiveAction;
    use crate::components::movement_state::CMovementState;
    use crate::components::transform::Velocity;
    use glam::Vec3;
    use woworld_core::action::{
        ActionInstanceId, ActionPhase, ActiveAction, CommitmentLevel, SustainPhase,
    };
    use woworld_core::movement::MovementState;

    fn active_jump(phase: ActionPhase) -> CActiveAction {
        CActiveAction(Some(ActiveAction {
            instance: ActionInstanceId(0),
            action_id: ActionRegistry::id_of("jump"),
            phase,
            commitment: CommitmentLevel::Hard,
            elapsed: 0.1,
            cancel_window_open: false,
            resource_drain_rate: 0.0,
            sustain_phase: SustainPhase::Normal,
        }))
    }

    #[test]
    fn test_jump_launch_sets_velocity_and_airborne() {
        let mut world = hecs::World::new();
        let profiles = MovementProfileRegistry::new();
        let e = world.spawn((
            active_jump(ActionPhase::Active),
            CMovementState(MovementState::default()),
            CMovementRecovery::default(),
            Velocity(Vec3::ZERO),
        ));

        jump_launch_system(&mut world, &profiles);

        let vel = world.get::<&Velocity>(e).unwrap();
        let ms = world.get::<&CMovementState>(e).unwrap();
        assert!(vel.0.y > 0.0, "jump should impart upward velocity");
        assert!(matches!(ms.0.special, Some(SpecialMode::Airborne(_))));
    }

    #[test]
    fn test_jump_launch_pushes_recovery_to_preserve_pace() {
        use woworld_core::movement::{Pace, Stance};
        let mut world = hecs::World::new();
        let profiles = MovementProfileRegistry::new();
        // 起跳前在跑（Running）——落地应能恢复，故必须入恢复栈
        let ms = MovementState {
            stance: Stance::Standing,
            pace: Pace::Running,
            ..Default::default()
        };
        let e = world.spawn((
            active_jump(ActionPhase::Active),
            CMovementState(ms),
            CMovementRecovery::default(),
            Velocity(Vec3::ZERO),
        ));

        jump_launch_system(&mut world, &profiles);

        // 恢复栈弹回应为起跳前的 Running（而非空栈默认 Still）
        let mut rec = world.get::<&mut CMovementRecovery>(e).unwrap();
        let popped = rec.0.pop_compatible(
            woworld_core::kinematics::LocomotionMode::Grounded,
            woworld_core::material::Medium::Air,
        );
        assert_eq!(popped.pace, Pace::Running, "落地应恢复起跳前 pace");
    }

    #[test]
    fn test_no_relaunch_when_already_airborne() {
        let mut world = hecs::World::new();
        let profiles = MovementProfileRegistry::new();
        let mut ms = MovementState::default();
        ms.special = Some(SpecialMode::Airborne(AirState::Falling {
            coyote_time_remaining: 0.0,
        }));
        let e = world.spawn((
            active_jump(ActionPhase::Active),
            CMovementState(ms),
            CMovementRecovery::default(),
            Velocity(Vec3::new(0.0, -3.0, 0.0)),
        ));

        jump_launch_system(&mut world, &profiles);

        // 已腾空——不重新起跳，垂直速度不被覆盖
        let vel = world.get::<&Velocity>(e).unwrap();
        assert_eq!(vel.0.y, -3.0);
    }

    #[test]
    fn test_no_launch_in_windup() {
        let mut world = hecs::World::new();
        let profiles = MovementProfileRegistry::new();
        let e = world.spawn((
            active_jump(ActionPhase::Windup),
            CMovementState(MovementState::default()),
            CMovementRecovery::default(),
            Velocity(Vec3::ZERO),
        ));

        jump_launch_system(&mut world, &profiles);

        let vel = world.get::<&Velocity>(e).unwrap();
        assert_eq!(vel.0.y, 0.0, "no launch during windup");
    }
}
