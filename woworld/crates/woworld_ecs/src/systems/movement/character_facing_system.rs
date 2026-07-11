//! CharacterFacingSystem — RotationLock → Rotation (smooth damp)
//!
//! 读取 CMovementControl.rotation_lock + CMoveIntent → 写入 Rotation。
//! 插在 action_system flush 之后、jump_launch 之前（Block A0，terrain_chunk.rs）。
//! 仅操作带 PlayerComponent 的实体（玩家与夺舍 NPC）。
//!
//! 参见: 玩家系统 007 §六

use glam::{Quat, Vec3};
use woworld_core::camera::smooth_damp_quat;
use woworld_core::kinematics::RotationLock;

use crate::components::movement_state::{CMoveIntent, CMovementControl};
use crate::components::player::PlayerComponent;
use crate::components::transform::Rotation;

/// 转向平滑时间 (s) — 匹配 `input_feel.toml` 的 `turn_smooth_time = 0.1`
const TURN_SMOOTH_TIME: f32 = 0.1;
/// 最大转向速率 (rad/s) — 约 720°/s，防止大型 delta 瞬时翻面
const TURN_RATE_RAD_S: f32 = 12.5664;

/// 按 rotation_lock 解析目标朝向，smooth_damp_quat 写 Rotation。
///
/// - `InputDirection` / `Free`(有输入) → CMoveIntent.direction (XZ 归一化)
/// - `CameraForward` → camera_transform -Z 投影 XZ
/// - `TargetDirection` → None (MVP stub，预留)
/// - `Locked` / `Free`(零输入) → None（保持当前 Rotation）
pub fn character_facing_system(world: &mut hecs::World, dt: f32) {
    for (_, (ctrl, intent, rot)) in world
        .query_mut::<(&CMovementControl, &CMoveIntent, &mut Rotation)>()
        .with::<&PlayerComponent>()
    {
        let lock = ctrl.rotation_lock;

        let target_dir: Option<Vec3> = match lock {
            RotationLock::Free => {
                // 玩家休息态 Free → InputDirection（有方向输入时）
                if intent.direction.length_squared() > 1e-6 {
                    Some(intent.direction)
                } else {
                    None
                }
            }
            RotationLock::InputDirection => {
                if intent.direction.length_squared() > 1e-6 {
                    Some(intent.direction)
                } else {
                    None
                }
            }
            RotationLock::CameraForward => {
                // 面朝镜头方向（-Z）投影到 XZ
                let cf = -intent.camera_transform.z_axis.truncate();
                let cf_xz = Vec3::new(cf.x, 0.0, cf.z);
                if cf_xz.length_squared() > 1e-6 {
                    Some(cf_xz.normalize())
                } else {
                    None
                }
            }
            RotationLock::TargetDirection => {
                // MVP stub: 无锁定目标
                None
            }
            RotationLock::Locked => None,
        };

        if let Some(dir) = target_dir {
            let target_quat = Quat::from_rotation_arc(Vec3::NEG_Z, dir.normalize_or_zero());
            let new_rot = smooth_damp_quat(
                rot.0,
                target_quat,
                TURN_SMOOTH_TIME,
                dt,
                Some(TURN_RATE_RAD_S),
            );
            rot.0 = new_rot;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::movement_state::{CMoveIntent, CMovementControl};
    use crate::components::player::PlayerComponent;
    use crate::components::transform::{Position, Rotation};
    use glam::{Mat4, Quat, Vec3};
    use woworld_core::kinematics::RotationLock;

    fn test_player_world() -> hecs::World {
        let mut world = hecs::World::new();
        world.spawn((
            PlayerComponent::default(),
            CMovementControl::default(),
            CMoveIntent::default(),
            Rotation(Quat::IDENTITY),
            Position(Vec3::ZERO),
        ));
        world
    }

    #[test]
    fn test_free_with_input_faces_direction() {
        let mut world = test_player_world();
        // Set input direction (world +X), Free rotation lock
        for (_, (ctrl, intent)) in world
            .query_mut::<(&mut CMovementControl, &mut CMoveIntent)>()
            .with::<&PlayerComponent>()
        {
            ctrl.rotation_lock = RotationLock::Free;
            intent.direction = Vec3::X; // normalized +X
        }
        character_facing_system(&mut world, 1.0 / 60.0);
        for (_, rot) in world.query_mut::<&Rotation>().with::<&PlayerComponent>() {
            let angle = rot.0.angle_between(Quat::IDENTITY);
            // First frame should rotate toward +X (from NEG_Z)
            assert!(angle > 0.0, "should rotate on first frame, got angle={angle}");
        }
    }

    #[test]
    fn test_free_no_input_keeps_rotation() {
        let mut world = test_player_world();
        // Zero direction → no rotation change
        for (_, ctrl) in world
            .query_mut::<&mut CMovementControl>()
            .with::<&PlayerComponent>()
        {
            ctrl.rotation_lock = RotationLock::Free;
        }
        character_facing_system(&mut world, 1.0 / 60.0);
        for (_, rot) in world.query_mut::<&Rotation>().with::<&PlayerComponent>() {
            let angle = rot.0.angle_between(Quat::IDENTITY);
            assert!(angle < 0.001, "should stay at identity, got angle={angle}");
        }
    }

    #[test]
    fn test_input_direction_faces_move_dir() {
        let mut world = test_player_world();
        for (_, (ctrl, intent)) in world
            .query_mut::<(&mut CMovementControl, &mut CMoveIntent)>()
            .with::<&PlayerComponent>()
        {
            ctrl.rotation_lock = RotationLock::InputDirection;
            intent.direction = Vec3::NEG_X; // world -X
        }
        character_facing_system(&mut world, 1.0 / 60.0);
        for (_, rot) in world.query_mut::<&Rotation>().with::<&PlayerComponent>() {
            // Should point toward -X: up vector is world Y, facing = -X
            let facing = rot.0 * Vec3::NEG_Z; // NEG_Z = forward in Godot
            assert!(facing.x < 0.0, "should face -X, got {:?}", facing);
            assert!(facing.y.abs() < 0.5); // roughly horizontal
        }
    }

    #[test]
    fn test_camera_forward_faces_camera() {
        let mut world = test_player_world();
        for (_, (ctrl, intent)) in world
            .query_mut::<(&mut CMovementControl, &mut CMoveIntent)>()
            .with::<&PlayerComponent>()
        {
            ctrl.rotation_lock = RotationLock::CameraForward;
            // 构造一个相机看向世界 +X 的 Godot Basis：
            //   forward = +X world = 相机 local -Z
            //   ∴ 相机 local +Z = -X world (backward)
            //   right = camera local +X, 由 cross(up, +Z) 得 = world -Z
            // Godot Transform3D columns: col0=right, col1=up, col2=-forward
            //   col0 = (0, 0, -1) = world -Z = right
            //   col1 = (0, 1, 0)  = world +Y = up
            //   col2 = (-1, 0, 0) = world -X = -forward = back
            let cam_xform = Mat4::from_cols(
                glam::Vec4::new(0.0, 0.0, -1.0, 0.0),   // col0 = right (local +X)
                glam::Vec4::new(0.0, 1.0, 0.0, 0.0),     // col1 = up    (local +Y)
                glam::Vec4::new(-1.0, 0.0, 0.0, 0.0),    // col2 = back  (local +Z, = -forward)
                glam::Vec4::W,
            );
            intent.camera_transform = cam_xform;
        }
        character_facing_system(&mut world, 1.0 / 60.0);
        for (_, rot) in world.query_mut::<&Rotation>().with::<&PlayerComponent>() {
            let facing = rot.0 * Vec3::NEG_Z;
            // CameraForward → face same direction as camera (-Z of camera matrix projects to XZ)
            // Camera faces +X world → character should face +X
            assert!(facing.x > 0.0, "should face +X (camera direction), got {:?}", facing);
        }
    }

    #[test]
    fn test_locked_prevents_rotation() {
        let mut world = test_player_world();
        for (_, (ctrl, intent)) in world
            .query_mut::<(&mut CMovementControl, &mut CMoveIntent)>()
            .with::<&PlayerComponent>()
        {
            ctrl.rotation_lock = RotationLock::Locked;
            intent.direction = Vec3::X;
        }
        character_facing_system(&mut world, 1.0 / 60.0);
        for (_, rot) in world.query_mut::<&Rotation>().with::<&PlayerComponent>() {
            let angle = rot.0.angle_between(Quat::IDENTITY);
            assert!(angle < 0.001, "Locked should prevent rotation, got angle={angle}");
        }
    }

    #[test]
    fn test_smooth_damp_converges_over_multiple_frames() {
        let mut world = test_player_world();
        // Set up: current facing IDENTITY, want to face +X
        for (_, (ctrl, intent)) in world
            .query_mut::<(&mut CMovementControl, &mut CMoveIntent)>()
            .with::<&PlayerComponent>()
        {
            ctrl.rotation_lock = RotationLock::InputDirection;
            intent.direction = Vec3::X;
        }
        let dt = 1.0 / 60.0;
        let mut angles: Vec<f32> = Vec::new();
        for _ in 0..60 {
            character_facing_system(&mut world, dt);
            for (_, rot) in world.query_mut::<&Rotation>().with::<&PlayerComponent>() {
                let target = Quat::from_rotation_arc(Vec3::NEG_Z, Vec3::X);
                angles.push(rot.0.angle_between(target));
            }
        }
        // Angles should monotonically decrease (converging to target)
        for w in angles.windows(2) {
            assert!(w[1] <= w[0] + 0.001, "angles should not increase: {:?}", w);
        }
        // Final angle should be close to 0
        let final_angle = *angles.last().unwrap();
        assert!(final_angle < 0.05, "final_angle={final_angle}");
    }
}
