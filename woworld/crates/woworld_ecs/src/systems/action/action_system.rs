//! ActionSystem — ActionController 的 ECS 包装
//!
//! 遍历所有带 CActionRequestBuf 的实体 → 调用 action_controller_tick → 写回 CActiveAction + CMovementControl。
//!
//! 参见: `WoWorld-Design/.../角色控制器/003-ActionController与离散动作.md`

use smallvec::SmallVec;
use woworld_core::kinematics::{MovementLock, RotationLock};

use crate::components::action_state::{CActionRequestBuf, CActiveAction};
use crate::components::movement_state::CMovementControl;
use crate::entity_id::entity_id_from_hecs;
use crate::events::EventChannel;
use crate::resources::action_instance_counter::ActionInstanceCounter;
use crate::resources::action_registry::ActionRegistry;
use woworld_core::action::ActionLifecycleEvent;
use woworld_core::kinematics::LocomotionMode;
use woworld_core::spatial::TerrainQuery;
use woworld_core::types::WorldPos;

/// ECS System——每帧驱动 ActionController。
///
/// Query: `(CActiveAction, CActionRequestBuf, CMovementControl, Position)`.
pub fn action_system(
    world: &mut hecs::World,
    dt: f32,
    registry: &ActionRegistry,
    counter: &mut ActionInstanceCounter,
    events: &mut EventChannel<ActionLifecycleEvent>,
    terrain: &dyn TerrainQuery,
) {
    for (entity, (active_action, request_buf, move_control, pos)) in world.query_mut::<(
        &mut CActiveAction,
        &mut CActionRequestBuf,
        &mut CMovementControl,
        &crate::components::transform::Position,
    )>() {
        let eid = entity_id_from_hecs(entity);

        // 计算 LocomotionMode
        let loco = compute_locomotion(pos.0, terrain);

        // ── 推进当前动作的阶段计时器 ──
        if let Some(ref mut active) = active_action.0 {
            active.elapsed += dt;

            if let Some(def) = registry.get(active.action_id) {
                let windup_s = def.windup_ms as f32 / 1000.0;
                let active_s = def.active_ms as f32 / 1000.0;
                let recovery_s = def.recovery_ms as f32 / 1000.0;

                match active.phase {
                    woworld_core::action::ActionPhase::Windup => {
                        // ★ 修复：零前摇动作（dodge/interact windup_ms=0）须能离开 Windup。
                        //   原守卫 `&& windup_s > 0.0` 会使其永久卡死。
                        if active.elapsed >= windup_s {
                            let from = active.phase;
                            active.phase = woworld_core::action::ActionPhase::Active;
                            events.send(woworld_core::action::ActionLifecycleEvent::PhaseChanged {
                                instance: active.instance,
                                entity: eid,
                                from,
                                to: active.phase,
                            });
                        }
                    }
                    woworld_core::action::ActionPhase::Active => {
                        let total_active = if active_s > 0.0 {
                            windup_s + active_s
                        } else {
                            // 持续动作——无限 Active，等待释放
                            f32::MAX
                        };
                        if active.elapsed >= total_active && active_s > 0.0 {
                            let from = active.phase;
                            active.phase = woworld_core::action::ActionPhase::Recovery;
                            // 设置取消窗口
                            active.cancel_window_open =
                                def.cancel_window_ms > 0;
                            events.send(woworld_core::action::ActionLifecycleEvent::PhaseChanged {
                                instance: active.instance,
                                entity: eid,
                                from,
                                to: active.phase,
                            });
                        }
                    }
                    woworld_core::action::ActionPhase::Recovery => {
                        // ★ 取消窗口计时：在 Recovery 最后 cancel_window_ms 内开放
                        let recovery_elapsed = active.elapsed - windup_s - active_s;
                        let cancel_window_s = def.cancel_window_ms as f32 / 1000.0;
                        active.cancel_window_open =
                            cancel_window_s > 0.0
                                && recovery_elapsed >= (recovery_s - cancel_window_s).max(0.0);

                        if active.elapsed >= windup_s + active_s + recovery_s {
                            let instance = active.instance;
                            let action_id = active.action_id;
                            let total_ms = (active.elapsed * 1000.0) as u32;
                            active_action.0 = None;
                            events.send(woworld_core::action::ActionLifecycleEvent::Completed {
                                instance,
                                entity: eid,
                                action_id,
                                total_duration_ms: total_ms,
                            });
                            // 恢复移动锁
                            move_control.movement_lock = MovementLock::Free;
                            request_buf.0.clear();
                            continue;
                        }
                    }
                }

                // 更新移动/朝向锁
                move_control.movement_lock = movement_lock_from_def(def.movement_lock);
                move_control.rotation_lock = rotation_lock_from_def(def.rotation_lock);
            }
        }

        // ── 调用仲裁核心 ──
        let requests: SmallVec<[_; 4]> = request_buf.0.iter().cloned().collect();
        let new_events = super::action_controller::action_controller_tick(
            &mut active_action.0,
            &requests,
            registry,
            loco,
            counter,
            eid,
        );

        // 发送事件
        events.send_all(new_events);

        // 清空请求缓冲（已消费）
        request_buf.0.clear();

        // 更新移动锁（新动作可能改变了锁；空闲/被打断则复位 Free）
        if let Some(ref active) = active_action.0 {
            if let Some(def) = registry.get(active.action_id) {
                move_control.movement_lock = movement_lock_from_def(def.movement_lock);
                move_control.rotation_lock = rotation_lock_from_def(def.rotation_lock);
            }
        } else {
            // ★ 修复移动锁泄漏：动作被打断/释放后 active=None，
            //   锁必须复位——否则实体被上一动作的 Full 锁冻结无法移动。
            move_control.movement_lock = MovementLock::Free;
            move_control.rotation_lock = RotationLock::Free;
        }
    }
}

fn rotation_lock_from_def(def: woworld_core::action::RotationLockDef) -> RotationLock {
    use woworld_core::action::RotationLockDef as D;
    match def {
        D::Free => RotationLock::Free,
        D::InputDirection => RotationLock::InputDirection,
        D::CameraForward => RotationLock::CameraForward,
        D::TargetDirection => RotationLock::TargetDirection,
        D::Locked => RotationLock::Locked,
    }
}

fn movement_lock_from_def(def: woworld_core::action::MovementLockDef) -> MovementLock {
    match def {
        woworld_core::action::MovementLockDef::Free => MovementLock::Free,
        woworld_core::action::MovementLockDef::Partial => {
            MovementLock::Partial { speed_cap: 1.0 } // Sprint 1: 默认慢走速度
        }
        woworld_core::action::MovementLockDef::Full => MovementLock::Full,
        woworld_core::action::MovementLockDef::Override => {
            MovementLock::Override(glam::Vec3::ZERO) // Sprint 1: 由具体动作系统覆写
        }
    }
}

fn compute_locomotion(pos: glam::Vec3, terrain: &dyn TerrainQuery) -> LocomotionMode {
    let wp = WorldPos {
        x: pos.x as f64,
        y: pos.y as f64,
        z: pos.z as f64,
    };
    if terrain.is_walkable(wp) {
        LocomotionMode::Grounded
    } else {
        LocomotionMode::PhysicsBody
    }
}
