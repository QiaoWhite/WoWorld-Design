//! PlayerInputSystem — 玩家方向输入 → CMoveIntent
//!
//! Movement 域特殊：玩家方向输入**不经** ActionResolver（004 §五）——
//! 由本系统直接把 `InputState.move_direction`（相机相对）转世界空间写入
//! `CMoveIntent.direction`。ActionResolver 只处理离散动作。
//!
//! 仅玩家实体（`With<PlayerComponent>`）且手控 Movement 域（Manual）。
//! 绞杀者：旧 NPC 无 CMoveIntent/PlayerComponent 不受影响。
//!
//! 参见: `WoWorld-Design/.../角色控制器/004-ActionResolver与输入解析.md` §五
//!       `001-角色控制器总纲.md` §四 Phase 0
//!
//! ⚠️ `desired_state`（pace/stance 意图）当前**无消费者**——作为前瞻契约写入。
//!   实际 pace 生效待 MovementModeSystem 扩展读取 desired_state（后续 sprint）。
//!   `direction` 有真实效果：movement_system 立即消费。

use crate::components::movement_state::{CMoveIntent, CMovementState};
use crate::components::player::{ControlModeComponent, PlayerComponent};
use woworld_core::input::{InputAction, InputState};
use woworld_core::movement::{MovementState, Pace, Stance};
use woworld_core::player::ActionDomain;

/// PlayerInputSystem —— InputState.move_direction（相机相对）→ CMoveIntent.direction（世界空间）。
///
/// ★ 007 修复：除了写 `CMoveIntent.desired_state`（前瞻契约），也直接写 `CMovementState.0.pace`。
/// 因为 `movement_system` 只读 `CMovementState`——若无此直写，`desired_state.pace=Sprinting` 永远不生效。
pub fn player_input_system(world: &mut hecs::World, input: &InputState) {
    for (_, (ctrl, intent, move_state)) in world
        .query_mut::<(&ControlModeComponent, &mut CMoveIntent, &mut CMovementState)>()
        .with::<&PlayerComponent>()
    {
        // Auto（或未手控 Movement 域）→ 由 GOAP 驱动移动，玩家不接管。
        // ★ 007: 夺舍期裸玩家设为 Auto，必须显式零化 direction——
        //   否则 CMoveIntent 保留夺舍前最后一帧的非零方向，
        //   movement_system 会让裸玩家实体持续漂移。
        if !ctrl.mode.controls_domain(ActionDomain::Movement) {
            intent.direction = glam::Vec3::ZERO;
            continue;
        }

        // ── 相机相对 → 世界空间 XZ ──
        //   move_direction.x = 左右平移, move_direction.y = 前后
        let right = input.camera_transform.x_axis.truncate();
        let forward = -input.camera_transform.z_axis.truncate(); // -Z 为前
        let mut dir = right * input.move_direction.x + forward * input.move_direction.y;
        dir.y = 0.0; // 投影到 XZ 平面
        intent.direction = dir.normalize_or_zero();
        intent.camera_transform = input.camera_transform;

        // ── pace/stance（★ 007: 直写 CMovementState —— desired_state 仍无消费者）──
        let stance = if input.is_held(InputAction::Crawl) {
            Stance::Prone
        } else if input.is_held(InputAction::Crouch) {
            Stance::Crouching
        } else {
            Stance::Standing
        };
        let pace = if intent.direction.length_squared() < 1e-6 {
            Pace::Still
        } else if input.is_held(InputAction::Sprint) {
            Pace::Sprinting
        } else if input.is_held(InputAction::Walk) {
            Pace::Walking
        } else {
            Pace::Running
        };
        move_state.0.stance = stance;
        move_state.0.pace = pace;
        intent.desired_state = Some(MovementState {
            stance,
            pace,
            special: None,
            exhaustion_cooldown: 0.0,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::player::ControlModeComponent;
    use glam::{Mat4, Vec2, Vec3};
    use woworld_core::player::ControlMode;

    fn spawn(world: &mut hecs::World, mode: ControlMode) -> hecs::Entity {
        world.spawn((
            PlayerComponent::default(),
            ControlModeComponent { mode },
            CMoveIntent::default(),
            CMovementState::default(),
        ))
    }

    #[test]
    fn test_auto_mode_does_not_write_direction() {
        let mut world = hecs::World::new();
        let e = spawn(&mut world, ControlMode::Auto);
        let mut input = InputState::default();
        input.move_direction = Vec2::new(0.0, 1.0);

        player_input_system(&mut world, &input);

        let intent = world.get::<&CMoveIntent>(e).unwrap();
        assert_eq!(intent.direction, Vec3::ZERO); // 未接管
        assert!(intent.desired_state.is_none());
    }

    #[test]
    fn test_manual_forward_maps_to_world_minus_z() {
        // 默认相机（IDENTITY）：前 = -Z
        let mut world = hecs::World::new();
        let e = spawn(&mut world, ControlMode::Manual);
        let mut input = InputState::default();
        input.camera_transform = Mat4::IDENTITY;
        input.move_direction = Vec2::new(0.0, 1.0); // 前

        player_input_system(&mut world, &input);

        let intent = world.get::<&CMoveIntent>(e).unwrap();
        assert!((intent.direction - Vec3::new(0.0, 0.0, -1.0)).length() < 1e-5);
    }

    #[test]
    fn test_manual_strafe_maps_to_world_plus_x() {
        let mut world = hecs::World::new();
        let e = spawn(&mut world, ControlMode::Manual);
        let mut input = InputState::default();
        input.move_direction = Vec2::new(1.0, 0.0); // 右

        player_input_system(&mut world, &input);

        let intent = world.get::<&CMoveIntent>(e).unwrap();
        assert!((intent.direction - Vec3::new(1.0, 0.0, 0.0)).length() < 1e-5);
    }

    #[test]
    fn test_sprint_modifier_sets_pace() {
        let mut world = hecs::World::new();
        let e = spawn(&mut world, ControlMode::Manual);
        let mut input = InputState::default();
        input.move_direction = Vec2::new(0.0, 1.0);
        input.press(InputAction::Sprint); // held

        player_input_system(&mut world, &input);

        let intent = world.get::<&CMoveIntent>(e).unwrap();
        assert_eq!(intent.desired_state.unwrap().pace, Pace::Sprinting);
        assert_eq!(intent.desired_state.unwrap().stance, Stance::Standing);
    }

    #[test]
    fn test_no_move_input_is_still() {
        let mut world = hecs::World::new();
        let e = spawn(&mut world, ControlMode::Manual);
        let input = InputState::default(); // move_direction = 0

        player_input_system(&mut world, &input);

        let intent = world.get::<&CMoveIntent>(e).unwrap();
        assert_eq!(intent.desired_state.unwrap().pace, Pace::Still);
    }

    #[test]
    fn test_crouch_modifier_sets_stance() {
        let mut world = hecs::World::new();
        let e = spawn(&mut world, ControlMode::Manual);
        let mut input = InputState::default();
        input.move_direction = Vec2::new(0.0, 1.0);
        input.press(InputAction::Crouch);

        player_input_system(&mut world, &input);

        let intent = world.get::<&CMoveIntent>(e).unwrap();
        assert_eq!(intent.desired_state.unwrap().stance, Stance::Crouching);
    }
}
