//! ActionSystem — ActionController 的 ECS 包装
//!
//! 遍历所有带 CActionRequestBuf 的实体 → 调用 action_controller_tick → 写回 CActiveAction + CMovementControl。
//!
//! 参见: `WoWorld-Design/.../角色控制器/003-ActionController与离散动作.md`

use smallvec::SmallVec;
use woworld_core::kinematics::{MovementLock, RotationLock};

use crate::components::action_state::{CActionRequestBuf, CActiveAction, CPendingFollowUp};
use crate::components::movement_state::{CMoveIntent, CMovementControl};
use crate::components::vitals::Vitals;
use crate::entity_id::entity_id_from_hecs;
use crate::events::EventChannel;
use crate::resources::action_instance_counter::ActionInstanceCounter;
use crate::resources::action_registry::ActionRegistry;
use woworld_core::action::{
    action_priority, ActionDef, ActionFailureReason, ActionKind, ActionLifecycleEvent,
    ActiveAction, InterruptSource, OverchargeBehavior, ReleaseBehavior, ResourceType, SustainPhase,
};
use woworld_core::kinematics::LocomotionMode;
use woworld_core::spatial::TerrainQuery;
use woworld_core::types::WorldPos;

use super::action_controller::dispatch_release;

/// ECS System——每帧驱动 ActionController。
///
/// Query: `(CActiveAction, CActionRequestBuf, CMovementControl, Position,
///          Option<Vitals>, Option<CPendingFollowUp>)`.
///
/// ★ Sprint-065: 除离散动作阶段推进外，本 wrapper 承载持续/充能动作运行时——
/// sustain drain（消耗 Vitals）、SustainPhase 迁移、释放分发（dispatch_release）
/// 与充能子动作的帧间接续（CPendingFollowUp）。绞杀者隔离：无 Vitals/CPendingFollowUp
/// 的实体（旧 NPC）走 Option 缺省路径，离散动作零回归。
pub fn action_system(
    world: &mut hecs::World,
    dt: f32,
    registry: &ActionRegistry,
    counter: &mut ActionInstanceCounter,
    events: &mut EventChannel<ActionLifecycleEvent>,
    terrain: &dyn TerrainQuery,
) {
    for (
        entity,
        (active_action, request_buf, move_control, pos, vitals_opt, mut follow_opt, move_intent),
    ) in world.query_mut::<(
        &mut CActiveAction,
        &mut CActionRequestBuf,
        &mut CMovementControl,
        &crate::components::transform::Position,
        Option<&mut Vitals>,
        Option<&mut CPendingFollowUp>,
        Option<&CMoveIntent>,
    )>() {
        let eid = entity_id_from_hecs(entity);

        // 计算 LocomotionMode
        let loco = compute_locomotion(pos.0, terrain);

        // ── 0. 注入上一帧的充能子动作（CPendingFollowUp → 请求缓冲）──
        //   一帧间隙：上一帧 dispatch_release 写入 follow-up，本帧灌入请求缓冲，
        //   由下方 controller_tick 以 source=ChargedAction 接受（006 §五）。
        if let Some(fu) = follow_opt.as_mut() {
            if let Some(req) = fu.0.take() {
                request_buf.0.push(req);
            }
        }

        // ── 1. 推进当前动作的阶段计时器 ──
        if let Some(active) = active_action.0.as_mut() {
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
                            // 持续/充能动作——无限 Active，等待释放
                            f32::MAX
                        };
                        if active.elapsed >= total_active && active_s > 0.0 {
                            let from = active.phase;
                            active.phase = woworld_core::action::ActionPhase::Recovery;
                            // 设置取消窗口
                            active.cancel_window_open = def.cancel_window_ms > 0;
                            events.send(woworld_core::action::ActionLifecycleEvent::PhaseChanged {
                                instance: active.instance,
                                entity: eid,
                                from,
                                to: active.phase,
                            });
                        } else if active_s == 0.0 {
                            // ★ 006 §〇：持续/充能动作 Active 中取消窗口**始终开放**
                            //   （block 可被 dodge/parry/light/heavy 取消，aim_bow 可被 dodge 取消）。
                            active.cancel_window_open = def.cancel_window_ms > 0;
                        }
                    }
                    woworld_core::action::ActionPhase::Recovery => {
                        // ★ 取消窗口计时：在 Recovery 最后 cancel_window_ms 内开放
                        let recovery_elapsed = active.elapsed - windup_s - active_s;
                        let cancel_window_s = def.cancel_window_ms as f32 / 1000.0;
                        active.cancel_window_open = cancel_window_s > 0.0
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

        // ── 1.5. interrupt_on_move（A3）：移动输入打断标记该旗标的动作（如 interact）──
        if let Some(active) = active_action.0.as_mut() {
            if let Some(def) = registry.get(active.action_id) {
                let moving = move_intent
                    .map(|mi| mi.direction.length_squared() > 1e-6)
                    .unwrap_or(false);
                if def.interrupt_on_move && moving {
                    let instance = active.instance;
                    let action_id = active.action_id;
                    let total = (def.windup_ms + def.active_ms + def.recovery_ms) as f32 / 1000.0;
                    let progress = if total > 0.0 {
                        (active.elapsed / total).min(1.0)
                    } else {
                        0.5
                    };
                    active_action.0 = None;
                    events.send(ActionLifecycleEvent::Interrupted {
                        instance,
                        entity: eid,
                        action_id,
                        by: InterruptSource::MoveInput,
                        progress,
                    });
                    move_control.movement_lock = MovementLock::Free;
                    move_control.rotation_lock = RotationLock::Free;
                    request_buf.0.clear();
                    continue;
                }
            }
        }

        // ── 2. 持续/充能动作运行时——sustain drain + SustainPhase + 释放 ──
        if let Some(active) = active_action.0.as_mut() {
            if let Some(def) = registry.get(active.action_id) {
                let sustained = def.active_ms == 0
                    && matches!(def.kind, ActionKind::Continuous | ActionKind::Charge);
                if sustained && active.phase == woworld_core::action::ActionPhase::Active {
                    // 消耗资源 + 迁移 SustainPhase + 判定强制结束（vitals_opt 仅此处消费，移动传入）
                    let outcome = update_sustain(active, def, vitals_opt, dt);

                    // 主动松键（priority=RELEASE 的同 action_id 请求）
                    let voluntary = request_buf.0.iter().any(|r| {
                        r.action_id == active.action_id && r.priority == action_priority::RELEASE
                    });

                    let instance = active.instance;
                    let action_id = active.action_id;

                    let ended = match outcome {
                        // 强制结束（资源耗尽 / 过久 / overcharge AutoRelease）→ 按 ReleaseBehavior 分发
                        SustainOutcome::ForcedRelease(reason) => {
                            let (evs, fu_req) = dispatch_release(active, def, eid, reason);
                            events.send_all(evs);
                            if let Some(req) = fu_req {
                                if let Some(fu) = follow_opt.as_mut() {
                                    fu.0 = Some(req);
                                }
                            }
                            true
                        }
                        // overcharge ForceCancel——动作失败，无子动作
                        SustainOutcome::ForcedCancel => {
                            events.send(ActionLifecycleEvent::Failed {
                                instance,
                                entity: eid,
                                action_id,
                                // 充能过久属"自身状态失效"（手抖/力竭），非外部上下文失效
                                reason: ActionFailureReason::SelfStateInvalidated,
                            });
                            true
                        }
                        // 未强制结束但玩家松键 → 正常释放分发
                        SustainOutcome::Continue if voluntary => {
                            let (evs, fu_req) = dispatch_release(active, def, eid, None);
                            events.send_all(evs);
                            if let Some(req) = fu_req {
                                if let Some(fu) = follow_opt.as_mut() {
                                    fu.0 = Some(req);
                                }
                            }
                            true
                        }
                        SustainOutcome::Continue => false,
                    };

                    if ended {
                        active_action.0 = None;
                        move_control.movement_lock = MovementLock::Free;
                        move_control.rotation_lock = RotationLock::Free;
                        request_buf.0.clear();
                        continue;
                    }
                }
            }
        }

        // ── 3. 调用仲裁核心（cancel + accept）──
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

/// 持续/充能动作每帧的运行时结果。
enum SustainOutcome {
    /// 继续维持（未强制结束）
    Continue,
    /// 强制结束——按 ReleaseBehavior 分发（`Some` = Interrupted 原因，`None` = 正常完成/AutoRelease 触发）
    ForcedRelease(Option<InterruptSource>),
    /// overcharge ForceCancel——动作失败，无子动作
    ForcedCancel,
}

/// 消耗资源 + 迁移 SustainPhase + 判定强制结束。
///
/// SustainPhase 依 Active 内时长 `t`（= elapsed - windup）迁移：
/// `t < overextend` → Normal；`overextend ≤ t < critical` → Overextended（消耗 ×multiplier）；
/// `t ≥ critical` → Critical。强制结束条件：资源耗尽（VitalDepleted）、Continuous 过久、
/// Charge overcharge（AutoRelease 触发 / ForceCancel 失败；Penalize 继续）。
fn update_sustain(
    active: &mut ActiveAction,
    def: &ActionDef,
    vitals: Option<&mut Vitals>,
    dt: f32,
) -> SustainOutcome {
    let drain = match def.sustain_drain {
        Some(d) => d,
        None => return SustainOutcome::Continue, // 无消耗配置——纯持续动作
    };

    let windup_s = def.windup_ms as f32 / 1000.0;
    let t = (active.elapsed - windup_s).max(0.0);
    let overext = def.overextend_threshold_secs.unwrap_or(f32::MAX);
    let critical = def.critical_threshold_secs.unwrap_or(f32::MAX);

    // ── SustainPhase 迁移 + 消耗乘数 ──
    let multiplier = if t >= critical {
        active.sustain_phase = SustainPhase::Critical {
            forced_release_in: 0.0,
        };
        drain.overextend_multiplier
    } else if t >= overext {
        active.sustain_phase = SustainPhase::Overextended {
            penalty: drain.overextend_multiplier,
        };
        drain.overextend_multiplier
    } else {
        active.sustain_phase = SustainPhase::Normal;
        1.0
    };
    active.resource_drain_rate = drain.rate_per_sec * multiplier;

    // ── 消耗资源 ──
    let amount = active.resource_drain_rate * dt;
    let depleted = vitals
        .map(|v| drain_vitals(v, drain.resource, amount))
        .unwrap_or(false);
    if depleted {
        return SustainOutcome::ForcedRelease(Some(InterruptSource::VitalDepleted(drain.resource)));
    }

    // ── overcharge / 过久强制结束 ──
    match &def.release_behavior {
        Some(ReleaseBehavior::Charged {
            stages,
            on_overcharge,
        }) => {
            let max_threshold_ms = stages.iter().map(|s| s.threshold_ms).max().unwrap_or(0);
            match on_overcharge {
                // 充能到顶自动释放（触发最高阶梯）
                OverchargeBehavior::AutoRelease => {
                    if (t * 1000.0) as u32 >= max_threshold_ms {
                        return SustainOutcome::ForcedRelease(None);
                    }
                }
                // 超过 critical 强制取消
                OverchargeBehavior::ForceCancel => {
                    if t >= critical {
                        return SustainOutcome::ForcedCancel;
                    }
                }
                // 允许继续充能（精度惩罚由战斗侧处理，不在此建模）
                OverchargeBehavior::Penalize { .. } => {}
            }
        }
        // 持续动作（Complete/Trigger）——过久强制正常结束
        _ => {
            if t >= critical {
                return SustainOutcome::ForcedRelease(None);
            }
        }
    }

    SustainOutcome::Continue
}

/// 从 Vitals 扣减指定资源，返回是否耗尽（≤0）。
///
/// ⚠️ 魔法冻结——Vitals 无 mana 字段。Mana-sustain 动作暂不消耗（不会因 mana 耗尽
///    强制结束）。魔法解冻或 Vitals 补 mana 字段后接线。
fn drain_vitals(v: &mut Vitals, resource: ResourceType, amount: f32) -> bool {
    match resource {
        ResourceType::Stamina => {
            v.stamina = (v.stamina - amount).max(0.0);
            v.stamina <= 0.0
        }
        ResourceType::Health => {
            v.hp = (v.hp - amount).max(0.0);
            v.hp <= 0.0
        }
        ResourceType::Mana => false,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::transform::Position;
    use glam::Vec3;
    use woworld_core::action::{
        ActionId, ActionInstanceId, ActionParams, ActionPhase, ActionRequest, ActionSource,
        ChargeStage, CommitmentLevel, MovementLockDef, RotationLockDef, SustainDrain,
    };
    use woworld_core::kinematics::PhysicsRequirement;
    use woworld_core::material::{Medium, SurfaceMaterial};
    use woworld_core::types::TerrainHit;

    struct FlatGround;
    impl TerrainQuery for FlatGround {
        fn height_at(&self, _p: WorldPos) -> f32 {
            0.0
        }
        fn normal_at(&self, _p: WorldPos) -> Vec3 {
            Vec3::Y
        }
        fn terrain_raycast(&self, _o: WorldPos, _d: Vec3, _m: f32) -> Option<TerrainHit> {
            None
        }
        fn density_at(&self, _p: WorldPos) -> f32 {
            0.0
        }
        fn is_walkable(&self, _p: WorldPos) -> bool {
            true
        }
        fn surface_material_at(&self, _p: WorldPos) -> SurfaceMaterial {
            SurfaceMaterial::Grass
        }
        fn medium_at(&self, _p: WorldPos) -> Medium {
            Medium::Air
        }
        fn light_level_at(&self, _p: WorldPos) -> f32 {
            1.0
        }
        fn sample_horizon(&self, _p: WorldPos, _d: &[Vec3]) -> Vec<f32> {
            vec![]
        }
    }

    const BLOCK_ID: ActionId = ActionId(500);
    const AIM_ID: ActionId = ActionId(600);
    const AIMED_SHOT_ID: ActionId = ActionId(602);

    /// block——Continuous + Stamina 消耗 + Complete 释放。
    fn block_def() -> ActionDef {
        ActionDef {
            name: "block".into(),
            category: "Combat".into(),
            kind: ActionKind::Continuous,
            priority: 20,
            commitment: CommitmentLevel::Soft,
            windup_ms: 50,
            active_ms: 0,
            recovery_ms: 100,
            cancel_set: vec![],
            cancel_window_ms: 999,
            bufferable: false,
            buffer_window_ms: 0,
            physics_req: PhysicsRequirement::Grounded,
            movement_lock: MovementLockDef::Partial,
            rotation_lock: RotationLockDef::Free,
            interrupt_on_move: false,
            sustain_drain: Some(SustainDrain {
                resource: ResourceType::Stamina,
                rate_per_sec: 3.0,
                overextend_multiplier: 2.0,
            }),
            release_behavior: Some(ReleaseBehavior::Complete),
            overextend_threshold_secs: Some(8.0),
            critical_threshold_secs: Some(12.0),
        }
    }

    /// aim_bow——Charge + 三阶梯 + Penalize。
    fn aim_def() -> ActionDef {
        ActionDef {
            name: "aim_bow".into(),
            category: "Combat".into(),
            kind: ActionKind::Charge,
            priority: 12,
            commitment: CommitmentLevel::Hard,
            windup_ms: 200,
            active_ms: 0,
            recovery_ms: 0,
            cancel_set: vec![],
            cancel_window_ms: 999,
            bufferable: false,
            buffer_window_ms: 0,
            physics_req: PhysicsRequirement::Grounded,
            movement_lock: MovementLockDef::Partial,
            rotation_lock: RotationLockDef::Free,
            interrupt_on_move: false,
            sustain_drain: Some(SustainDrain {
                resource: ResourceType::Stamina,
                rate_per_sec: 5.0,
                overextend_multiplier: 1.5,
            }),
            release_behavior: Some(ReleaseBehavior::Charged {
                stages: vec![
                    ChargeStage {
                        threshold_ms: 0,
                        action_id: ActionId(601),
                        power_multiplier: 0.5,
                    },
                    ChargeStage {
                        threshold_ms: 400,
                        action_id: AIMED_SHOT_ID,
                        power_multiplier: 1.0,
                    },
                ],
                on_overcharge: OverchargeBehavior::Penalize {
                    accuracy_loss_per_sec: 0.15,
                },
            }),
            overextend_threshold_secs: Some(2.0),
            critical_threshold_secs: Some(4.0),
        }
    }

    /// aimed_shot——离散子动作（供 follow-up 注入后被接受）。
    fn aimed_shot_def() -> ActionDef {
        let mut d = block_def();
        d.name = "aimed_shot".into();
        d.kind = ActionKind::Discrete;
        d.windup_ms = 0;
        d.active_ms = 50;
        d.recovery_ms = 0;
        d.sustain_drain = None;
        d.release_behavior = None;
        d.overextend_threshold_secs = None;
        d.critical_threshold_secs = None;
        d
    }

    fn registry_with(defs: &[(ActionId, ActionDef)]) -> ActionRegistry {
        let mut r = ActionRegistry::new();
        for (id, def) in defs {
            r.register(*id, def.clone());
        }
        r
    }

    fn active(id: ActionId, elapsed: f32) -> CActiveAction {
        CActiveAction(Some(ActiveAction {
            instance: ActionInstanceId(0),
            action_id: id,
            phase: ActionPhase::Active,
            commitment: CommitmentLevel::Soft,
            elapsed,
            cancel_window_open: false,
            resource_drain_rate: 0.0,
            sustain_phase: SustainPhase::Normal,
        }))
    }

    fn release_req(id: ActionId) -> CActionRequestBuf {
        let mut buf = CActionRequestBuf::default();
        buf.0.push(ActionRequest {
            action_id: id,
            priority: action_priority::RELEASE,
            source: ActionSource::Player,
            params: ActionParams::default(),
        });
        buf
    }

    fn run_once(
        world: &mut hecs::World,
        registry: &ActionRegistry,
        events: &mut EventChannel<ActionLifecycleEvent>,
    ) {
        let mut counter = ActionInstanceCounter::new();
        events.begin_frame();
        action_system(world, 0.016, registry, &mut counter, events, &FlatGround);
        events.mid_phase_flush();
    }

    #[test]
    fn test_sustain_drains_stamina() {
        let registry = registry_with(&[(BLOCK_ID, block_def())]);
        let mut world = hecs::World::new();
        let e = world.spawn((
            active(BLOCK_ID, 1.0),
            CActionRequestBuf::default(),
            CMovementControl::default(),
            Position(Vec3::ZERO),
            Vitals::default(),
            CPendingFollowUp::default(),
        ));
        let mut events = EventChannel::new();
        run_once(&mut world, &registry, &mut events);

        let v = world.get::<&Vitals>(e).unwrap();
        assert!(
            v.stamina < 100.0,
            "block 应消耗 stamina，实得 {}",
            v.stamina
        );
        // 仍在防御（未强制结束）
        assert!(world.get::<&CActiveAction>(e).unwrap().0.is_some());
    }

    #[test]
    fn test_voluntary_release_completes() {
        let registry = registry_with(&[(BLOCK_ID, block_def())]);
        let mut world = hecs::World::new();
        let e = world.spawn((
            active(BLOCK_ID, 1.0),
            release_req(BLOCK_ID),
            CMovementControl::default(),
            Position(Vec3::ZERO),
            Vitals::default(),
            CPendingFollowUp::default(),
        ));
        let mut events = EventChannel::new();
        run_once(&mut world, &registry, &mut events);

        assert!(
            world.get::<&CActiveAction>(e).unwrap().0.is_none(),
            "松键后应结束防御"
        );
        assert!(
            events
                .read()
                .iter()
                .any(|ev| matches!(ev, ActionLifecycleEvent::Completed { .. })),
            "应发出 Completed 事件"
        );
    }

    #[test]
    fn test_forced_release_on_stamina_depletion() {
        let registry = registry_with(&[(BLOCK_ID, block_def())]);
        let mut world = hecs::World::new();
        let e = world.spawn((
            active(BLOCK_ID, 1.0),
            CActionRequestBuf::default(),
            CMovementControl::default(),
            Position(Vec3::ZERO),
            Vitals {
                stamina: 0.02, // < 一帧消耗 3.0*0.016=0.048
                ..Default::default()
            },
            CPendingFollowUp::default(),
        ));
        let mut events = EventChannel::new();
        run_once(&mut world, &registry, &mut events);

        assert!(
            world.get::<&CActiveAction>(e).unwrap().0.is_none(),
            "体力耗尽应强制结束"
        );
        assert!(
            events.read().iter().any(|ev| matches!(
                ev,
                ActionLifecycleEvent::Interrupted {
                    by: InterruptSource::VitalDepleted(ResourceType::Stamina),
                    ..
                }
            )),
            "应发出 Interrupted{{VitalDepleted}} 事件"
        );
    }

    #[test]
    fn test_overextended_uses_multiplier() {
        let registry = registry_with(&[(BLOCK_ID, block_def())]);
        let mut world = hecs::World::new();
        // elapsed=9.0 > overextend(8.0)——进入 Overextended，消耗 ×2
        let e = world.spawn((
            active(BLOCK_ID, 9.0),
            CActionRequestBuf::default(),
            CMovementControl::default(),
            Position(Vec3::ZERO),
            Vitals::default(),
            CPendingFollowUp::default(),
        ));
        let mut events = EventChannel::new();
        run_once(&mut world, &registry, &mut events);

        let a = world.get::<&CActiveAction>(e).unwrap();
        let act = a.0.as_ref().unwrap();
        assert!(
            matches!(act.sustain_phase, SustainPhase::Overextended { .. }),
            "应进入 Overextended 阶段"
        );
        // drain_rate = 3.0 × 2.0(overextend) = 6.0
        assert!((act.resource_drain_rate - 6.0).abs() < 1e-4);
    }

    #[test]
    fn test_continuous_critical_forces_complete() {
        let registry = registry_with(&[(BLOCK_ID, block_def())]);
        let mut world = hecs::World::new();
        // elapsed=13.0 > critical(12.0)——Continuous 过久强制结束
        let e = world.spawn((
            active(BLOCK_ID, 13.0),
            CActionRequestBuf::default(),
            CMovementControl::default(),
            Position(Vec3::ZERO),
            Vitals::default(),
            CPendingFollowUp::default(),
        ));
        let mut events = EventChannel::new();
        run_once(&mut world, &registry, &mut events);

        assert!(
            world.get::<&CActiveAction>(e).unwrap().0.is_none(),
            "过久应强制结束"
        );
        // Continuous Complete 强制结束 → Completed（forced=None）
        assert!(events
            .read()
            .iter()
            .any(|ev| matches!(ev, ActionLifecycleEvent::Completed { .. })));
    }

    #[test]
    fn test_charge_release_injects_followup_next_frame() {
        let registry = registry_with(&[(AIM_ID, aim_def()), (AIMED_SHOT_ID, aimed_shot_def())]);
        let mut world = hecs::World::new();
        // elapsed=0.7：charge_ms=(0.7+dt-0.2)*1000 ≈ 516 → 命中 400ms 阶梯 = AIMED_SHOT
        let e = world.spawn((
            active(AIM_ID, 0.7),
            release_req(AIM_ID),
            CMovementControl::default(),
            Position(Vec3::ZERO),
            Vitals::default(),
            CPendingFollowUp::default(),
        ));
        let mut events = EventChannel::new();

        // 帧 N：松键 → ChargeTrigger + 写入 CPendingFollowUp，主动作结束
        run_once(&mut world, &registry, &mut events);
        assert!(
            world.get::<&CActiveAction>(e).unwrap().0.is_none(),
            "充能动作释放后应结束"
        );
        assert!(
            events
                .read()
                .iter()
                .any(|ev| matches!(ev, ActionLifecycleEvent::ChargeTrigger { .. })),
            "应发出 ChargeTrigger 事件"
        );
        assert!(
            world.get::<&CPendingFollowUp>(e).unwrap().0.is_some(),
            "应写入待接续子动作"
        );

        // 帧 N+1：follow-up 注入请求缓冲 → controller_tick 接受子动作
        run_once(&mut world, &registry, &mut events);
        let a = world.get::<&CActiveAction>(e).unwrap();
        assert_eq!(
            a.0.as_ref().map(|x| x.action_id),
            Some(AIMED_SHOT_ID),
            "下一帧应接受 aimed_shot 子动作"
        );
        // follow-up 已消费清空
        assert!(world.get::<&CPendingFollowUp>(e).unwrap().0.is_none());
    }

    /// interact——离散 + interrupt_on_move=true。
    fn interact_def() -> ActionDef {
        let mut d = aimed_shot_def();
        d.name = "interact".into();
        d.active_ms = 100;
        d.interrupt_on_move = true;
        d
    }

    #[test]
    fn test_interrupt_on_move_cancels_interact() {
        let registry = registry_with(&[(ActionId(700), interact_def())]);
        let mut world = hecs::World::new();
        let e = world.spawn((
            active(ActionId(700), 0.02),
            CActionRequestBuf::default(),
            CMovementControl::default(),
            Position(Vec3::ZERO),
            Vitals::default(),
            CPendingFollowUp::default(),
            CMoveIntent {
                direction: Vec3::new(1.0, 0.0, 0.0),
                ..Default::default()
            },
        ));
        let mut events = EventChannel::new();
        run_once(&mut world, &registry, &mut events);

        assert!(
            world.get::<&CActiveAction>(e).unwrap().0.is_none(),
            "移动输入应打断 interact"
        );
        assert!(
            events.read().iter().any(|ev| matches!(
                ev,
                ActionLifecycleEvent::Interrupted {
                    by: InterruptSource::MoveInput,
                    ..
                }
            )),
            "应发出 Interrupted{{MoveInput}} 事件"
        );
    }

    #[test]
    fn test_interrupt_on_move_ignored_when_stationary() {
        let registry = registry_with(&[(ActionId(700), interact_def())]);
        let mut world = hecs::World::new();
        let e = world.spawn((
            active(ActionId(700), 0.02),
            CActionRequestBuf::default(),
            CMovementControl::default(),
            Position(Vec3::ZERO),
            Vitals::default(),
            CPendingFollowUp::default(),
            CMoveIntent::default(), // 零移动方向
        ));
        let mut events = EventChannel::new();
        run_once(&mut world, &registry, &mut events);

        assert!(
            world.get::<&CActiveAction>(e).unwrap().0.is_some(),
            "静止时 interact 不应被打断"
        );
    }

    #[test]
    fn test_held_block_cancellable_by_dodge() {
        // ★ 006 §〇：持续动作 Active 中取消窗口始终开放——dodge 可取消握持中的 block
        let block_id = ActionId::from_key("block");
        let dodge_id = ActionId::from_key("dodge");
        let mut block = block_def();
        block.cancel_set = vec!["dodge".into()];
        let mut dodge = aimed_shot_def(); // 复用离散模板
        dodge.name = "dodge".into();
        let registry = registry_with(&[(block_id, block), (dodge_id, dodge)]);

        let mut world = hecs::World::new();
        let mut buf = CActionRequestBuf::default();
        buf.0.push(ActionRequest {
            action_id: dodge_id,
            priority: 25,
            source: ActionSource::Player,
            params: ActionParams::default(),
        });
        let e = world.spawn((
            active(block_id, 1.0), // Active，握持 1s
            buf,
            CMovementControl::default(),
            Position(Vec3::ZERO),
            Vitals::default(),
            CPendingFollowUp::default(),
        ));
        let mut events = EventChannel::new();
        run_once(&mut world, &registry, &mut events);

        // block 被 dodge 取消 → dodge 同帧接管为当前动作
        let a = world.get::<&CActiveAction>(e).unwrap();
        assert_eq!(
            a.0.as_ref().map(|x| x.action_id),
            Some(dodge_id),
            "dodge 应取消 block 并接管"
        );
        assert!(events
            .read()
            .iter()
            .any(|ev| matches!(ev, ActionLifecycleEvent::Interrupted { .. })));
    }

    /// aim_bow 变体：overcharge = AutoRelease（充能到顶自动释放）。
    fn charge_auto_def() -> ActionDef {
        let mut d = aim_def();
        if let Some(ReleaseBehavior::Charged { on_overcharge, .. }) = d.release_behavior.as_mut() {
            *on_overcharge = OverchargeBehavior::AutoRelease;
        }
        d
    }

    /// aim_bow 变体：overcharge = ForceCancel（超 critical 强制失败）。
    fn charge_forcecancel_def() -> ActionDef {
        let mut d = aim_def();
        if let Some(ReleaseBehavior::Charged { on_overcharge, .. }) = d.release_behavior.as_mut() {
            *on_overcharge = OverchargeBehavior::ForceCancel;
        }
        d
    }

    #[test]
    fn test_overcharge_autorelease_fires_top_stage() {
        // ★ 006：AutoRelease——充能到顶（≥最高阶梯 400ms）无需松键自动触发子动作
        let registry = registry_with(&[
            (AIM_ID, charge_auto_def()),
            (AIMED_SHOT_ID, aimed_shot_def()),
        ]);
        let mut world = hecs::World::new();
        // elapsed=0.7 → t=(0.716-0.2)*1000≈516 ≥ 400ms 最高阶梯 → AutoRelease
        let e = world.spawn((
            active(AIM_ID, 0.7),
            CActionRequestBuf::default(), // 无 RELEASE 请求
            CMovementControl::default(),
            Position(Vec3::ZERO),
            Vitals::default(),
            CPendingFollowUp::default(),
        ));
        let mut events = EventChannel::new();
        run_once(&mut world, &registry, &mut events);

        assert!(
            world.get::<&CActiveAction>(e).unwrap().0.is_none(),
            "AutoRelease 应自动结束充能"
        );
        assert!(events
            .read()
            .iter()
            .any(|ev| matches!(ev, ActionLifecycleEvent::ChargeTrigger { .. })));
        assert!(
            world.get::<&CPendingFollowUp>(e).unwrap().0.is_some(),
            "AutoRelease 应写入子动作 follow-up"
        );
    }

    #[test]
    fn test_overcharge_forcecancel_fails_no_followup() {
        // ★ 006：ForceCancel——超 critical(4.0s) 强制失败，无子动作
        let registry = registry_with(&[(AIM_ID, charge_forcecancel_def())]);
        let mut world = hecs::World::new();
        // elapsed=4.5 → t=(4.516-0.2)≈4.316 ≥ critical 4.0 → ForceCancel
        let e = world.spawn((
            active(AIM_ID, 4.5),
            CActionRequestBuf::default(),
            CMovementControl::default(),
            Position(Vec3::ZERO),
            Vitals::default(),
            CPendingFollowUp::default(),
        ));
        let mut events = EventChannel::new();
        run_once(&mut world, &registry, &mut events);

        assert!(
            world.get::<&CActiveAction>(e).unwrap().0.is_none(),
            "ForceCancel 应结束充能"
        );
        assert!(events
            .read()
            .iter()
            .any(|ev| matches!(ev, ActionLifecycleEvent::Failed { .. })));
        assert!(
            world.get::<&CPendingFollowUp>(e).unwrap().0.is_none(),
            "ForceCancel 无子动作"
        );
    }
}
