//! ActionController — 承诺中断系统核心算法
//!
//! ~40 行仲裁逻辑 + 阶段推进。无内部状态——状态在 CActiveAction Component 中。
//! 复杂度在 TOML 数据（ActionRegistry）中，不在代码中。
//!
//! 参见: `WoWorld-Design/.../角色控制器/003-ActionController与离散动作.md` §二

use smallvec::SmallVec;
use woworld_core::action::{
    action_priority, ActionDef, ActionFailureReason, ActionId, ActionInstanceId,
    ActionLifecycleEvent, ActionParams, ActionPhase, ActionRequest, ActionSource, ActiveAction,
    ChargeStage, InterruptSource, ReleaseBehavior, SustainPhase,
};
use woworld_core::kinematics::LocomotionMode;
use woworld_core::types::EntityId;

use crate::resources::action_instance_counter::ActionInstanceCounter;
use crate::resources::action_registry::ActionRegistry;

/// 承诺中断仲裁核心——纯函数，无内部状态。
///
/// # 参数
/// - `current`: CActiveAction 中的当前动作（None = 空闲）
/// - `requests`: CActionRequestBuf 中的请求列表
/// - `registry`: 动作定义注册表
/// - `loco`: 当前 LocomotionMode
/// - `counter`: 动作实例 ID 生成器
///
/// # 返回
/// 本帧产生的生命周期事件。
pub fn action_controller_tick(
    current: &mut Option<ActiveAction>,
    requests: &[ActionRequest],
    registry: &ActionRegistry,
    loco: LocomotionMode,
    counter: &mut ActionInstanceCounter,
    entity: woworld_core::types::EntityId,
) -> SmallVec<[ActionLifecycleEvent; 4]> {
    let mut events = SmallVec::new();

    // ── 1. 检查取消请求 ──
    //   释放（voluntary / forced）已由 action_system（wrapper）在调用本函数前处理——
    //   wrapper 独占 Vitals（强制释放）与 CPendingFollowUp（子动作接续），本纯函数只
    //   负责仲裁（cancel + accept）。参见 006 §五 / dispatch_release。
    if let Some(active) = current.as_ref() {
        for req in requests {
            if req.action_id == active.action_id {
                continue;
            }
            if can_interrupt(active, req, registry) {
                let instance = active.instance;
                let action_id = active.action_id;
                let by = match req.source {
                    ActionSource::System if req.priority >= action_priority::STAGGER_THRESHOLD => {
                        InterruptSource::Staggered
                    }
                    ActionSource::Instinct => InterruptSource::DodgeCancel,
                    _ => InterruptSource::HigherPriorityAction(req.action_id),
                };
                let progress = if let Some(def) = registry.get(active.action_id) {
                    let total = (def.windup_ms + def.active_ms + def.recovery_ms) as f32 / 1000.0;
                    if total > 0.0 {
                        (active.elapsed / total).min(1.0)
                    } else {
                        0.5
                    }
                } else {
                    0.5
                };
                *current = None;
                events.push(ActionLifecycleEvent::Interrupted {
                    instance,
                    entity,
                    action_id,
                    by,
                    progress,
                });
                break;
            }
        }
    }

    // ── 3. 接受新请求（最高优先级）──
    if current.is_none() {
        let best = requests
            .iter()
            .filter(|r| r.priority > 0) // 非释放信号
            .filter(|r| {
                registry
                    .get(r.action_id)
                    .is_some_and(|def| def.physics_req.is_satisfied_by(loco))
            })
            .max_by_key(|r| (r.priority, r.source as u8));

        if let Some(req) = best {
            if let Some(def) = registry.get(req.action_id) {
                // ★ Sprint-065: Continuous/Charge 与 Discrete 走同一接受路径——
                //   差异在运行时（无限 Active / sustain drain / 释放分发），由
                //   action_system 承载。此处只负责"提交动作，进入 Windup"。
                let instance = counter.next_id();
                let started_at = 0.0; // 由外层 ECS wrapper 填充时间
                                      // 持续/充能动作从 sustain_drain 取初始消耗速率（Discrete 为 0）
                let resource_drain_rate = def.sustain_drain.map(|d| d.rate_per_sec).unwrap_or(0.0);
                *current = Some(ActiveAction {
                    instance,
                    action_id: req.action_id,
                    phase: ActionPhase::Windup,
                    commitment: def.commitment,
                    elapsed: 0.0,
                    cancel_window_open: false,
                    resource_drain_rate,
                    sustain_phase: SustainPhase::Normal,
                });
                events.push(ActionLifecycleEvent::Started {
                    instance,
                    entity,
                    action_id: req.action_id,
                    params: req.params,
                    started_at,
                });
            }
        }
    }

    events
}

/// 三规则仲裁——决定是否可以用 `req` 打断 `current`。
///
/// CommitmentLevel 门控:
/// - Soft: 规则 1 (cancel_window) + 规则 2 (Instinct) + 规则 3 (System) 全部可用
/// - Hard: Active 阶段仅规则 3 (System interrupt) 可通过；Windup/Recovery 阶段规则 1+2 可用
/// - Locked: 任何规则均不通过
fn can_interrupt(current: &ActiveAction, req: &ActionRequest, registry: &ActionRegistry) -> bool {
    use woworld_core::action::{ActionPhase, CommitmentLevel};

    // ★ A6: Death/濒死等紧急系统中断（priority≥EMERGENCY）穿透一切承诺，含 Locked——
    //   否则濒死实体会永久卡死在锁定动作里（005 §二 InterruptSource::Death/Dying）。
    if req.source == ActionSource::System && req.priority >= action_priority::EMERGENCY {
        return true;
    }

    // Locked: 除 EMERGENCY 外不可打断
    if current.commitment >= CommitmentLevel::Locked {
        return false;
    }

    // Hard + Active: 仅系统中断可通过
    if current.commitment >= CommitmentLevel::Hard && current.phase == ActionPhase::Active {
        return req.source == ActionSource::System
            && req.priority >= action_priority::STAGGER_THRESHOLD;
    }

    // 规则 3: 系统中断（硬直/击飞/死亡）始终通过
    if req.source == ActionSource::System && req.priority >= action_priority::STAGGER_THRESHOLD {
        return true;
    }

    // 规则 2: 本能层始终通过
    if req.source == ActionSource::Instinct {
        return true;
    }

    // 规则 1: 取消窗口 + cancel_set（玩家/GOAP 主动取消）
    //   ★ 按 ActionId 比对（registry 预解析的 cancel_set_ids），而非中文显示名——
    //     修复 cancel_set 存 key、却拿 def.name 比对导致规则1 永不触发的 bug。
    if current.cancel_window_open
        && registry
            .cancel_set_ids(current.action_id)
            .contains(&req.action_id)
    {
        return true;
    }

    false
}

// ── Sprint-065: 持续/充能动作释放分发 ──────────────────────────────

/// 释放/强制结束一个持续或充能动作，按 `ReleaseBehavior` 分发。
///
/// 由 `action_system`（wrapper）调用——wrapper 独占 `Vitals`（判定强制释放）与
/// `CPendingFollowUp`（写入子动作）。本函数是纯逻辑，返回 (生命周期事件, 子动作请求)。
///
/// - `forced = None`：玩家/NPC 主动松键（正常释放）。
/// - `forced = Some(src)`：资源耗尽/过久等强制结束（如 `VitalDepleted`）。
///
/// follow-up 由 wrapper 写入 `CPendingFollowUp`，下一帧注入 `CActionRequestBuf`（006 §五）。
pub fn dispatch_release(
    active: &ActiveAction,
    def: &ActionDef,
    entity: EntityId,
    forced: Option<InterruptSource>,
) -> (SmallVec<[ActionLifecycleEvent; 4]>, Option<ActionRequest>) {
    let mut events = SmallVec::new();
    let instance = active.instance;
    let action_id = active.action_id;
    let total_ms = (active.elapsed * 1000.0) as u32;
    let windup_s = def.windup_ms as f32 / 1000.0;
    // 充能时长——从进入 Active 起算（elapsed 扣除 windup）
    let charge_ms = ((active.elapsed - windup_s).max(0.0) * 1000.0) as u32;

    let behavior = def
        .release_behavior
        .as_ref()
        .unwrap_or(&ReleaseBehavior::Complete);

    match behavior {
        // 纯结束——释放即完成
        ReleaseBehavior::Complete => {
            push_end_event(&mut events, instance, entity, action_id, total_ms, forced);
            (events, None)
        }
        // 释放触发固定子动作
        ReleaseBehavior::Trigger { action_id: sub } => {
            push_end_event(&mut events, instance, entity, action_id, total_ms, forced);
            (events, Some(follow_up_request(*sub, 1.0)))
        }
        // 根据充能时长选择子动作
        ReleaseBehavior::Charged { stages, .. } => match select_charge_stage(stages, charge_ms) {
            Some(stage) => {
                events.push(ActionLifecycleEvent::ChargeTrigger {
                    instance,
                    entity,
                    charge_ms,
                    stage,
                });
                (
                    events,
                    Some(follow_up_request(stage.action_id, stage.power_multiplier)),
                )
            }
            None => {
                // 未达最低阶梯（松得太早）——动作失败，无子动作
                events.push(ActionLifecycleEvent::Failed {
                    instance,
                    entity,
                    action_id,
                    reason: ActionFailureReason::ContextInvalidated,
                });
                (events, None)
            }
        },
    }
}

/// 结束事件——正常释放发 `Completed`，强制结束发 `Interrupted`。
fn push_end_event(
    events: &mut SmallVec<[ActionLifecycleEvent; 4]>,
    instance: ActionInstanceId,
    entity: EntityId,
    action_id: ActionId,
    total_ms: u32,
    forced: Option<InterruptSource>,
) {
    match forced {
        Some(by) => events.push(ActionLifecycleEvent::Interrupted {
            instance,
            entity,
            action_id,
            by,
            progress: 1.0,
        }),
        None => events.push(ActionLifecycleEvent::Completed {
            instance,
            entity,
            action_id,
            total_duration_ms: total_ms,
        }),
    }
}

/// 构造充能子动作请求——`power_multiplier` 定点编码入 `ActionParams.data`（×1000）。
/// 战斗侧读回 `data as f32 / 1000.0` = 威力乘数。
fn follow_up_request(action_id: ActionId, power_multiplier: f32) -> ActionRequest {
    ActionRequest {
        action_id,
        priority: action_priority::CHARGE_TRIGGER,
        source: ActionSource::ChargedAction,
        params: ActionParams {
            target: None,
            position: None,
            data: (power_multiplier * 1000.0) as u32,
        },
    }
}

/// 选择满足充能时长的最高阶梯——`threshold_ms` 升序中取 `charge_ms ≥ threshold` 的最高者。
///
/// 返回 `None` 表示未达最低阶梯（松键太早）。
pub fn select_charge_stage(stages: &[ChargeStage], charge_ms: u32) -> Option<ChargeStage> {
    stages
        .iter()
        .filter(|s| charge_ms >= s.threshold_ms)
        .max_by_key(|s| s.threshold_ms)
        .copied()
}

#[cfg(test)]
mod tests {
    use super::*;
    use woworld_core::action::{ActionId, ActionInstanceId, ActionParams, CommitmentLevel};

    fn locked_action() -> ActiveAction {
        ActiveAction {
            instance: ActionInstanceId(0),
            action_id: ActionId(1),
            phase: ActionPhase::Active,
            commitment: CommitmentLevel::Locked,
            elapsed: 0.1,
            cancel_window_open: false,
            resource_drain_rate: 0.0,
            sustain_phase: SustainPhase::Normal,
        }
    }

    fn sys_req(priority: u8) -> ActionRequest {
        ActionRequest {
            action_id: ActionId(2),
            priority,
            source: ActionSource::System,
            params: ActionParams::default(),
        }
    }

    #[test]
    fn test_a6_emergency_interrupts_locked() {
        // ★ A6: EMERGENCY（死亡）系统中断穿透 Locked 承诺
        let reg = ActionRegistry::new();
        assert!(can_interrupt(
            &locked_action(),
            &sys_req(action_priority::EMERGENCY),
            &reg
        ));
    }

    #[test]
    fn test_stagger_does_not_interrupt_locked() {
        // 普通 STAGGER 系统中断不穿透 Locked
        let reg = ActionRegistry::new();
        assert!(!can_interrupt(
            &locked_action(),
            &sys_req(action_priority::STAGGER_THRESHOLD),
            &reg
        ));
    }

    // ── Sprint-065: Continuous/Charge 接受路径 ──

    use woworld_core::action::{
        ActionDef, ActionKind, MovementLockDef, ReleaseBehavior, ResourceType, RotationLockDef,
        SustainDrain,
    };
    use woworld_core::kinematics::PhysicsRequirement;
    use woworld_core::types::EntityId;

    /// 一个持续动作定义（block 风格）——Continuous + sustain_drain + Complete 释放。
    fn continuous_block_def() -> ActionDef {
        ActionDef {
            name: "block".into(),
            category: "Combat".into(),
            kind: ActionKind::Continuous,
            priority: 20,
            commitment: CommitmentLevel::Soft,
            windup_ms: 50,
            active_ms: 0, // 无限 Active
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

    #[test]
    fn test_continuous_action_accepted_enters_windup() {
        let mut reg = ActionRegistry::new();
        let id = ActionId(100);
        reg.register(id, continuous_block_def());

        let mut current = None;
        let mut counter = ActionInstanceCounter::new();
        let reqs = [ActionRequest {
            action_id: id,
            priority: 20,
            source: ActionSource::Player,
            params: ActionParams::default(),
        }];
        let events = action_controller_tick(
            &mut current,
            &reqs,
            &reg,
            LocomotionMode::Grounded,
            &mut counter,
            EntityId(1),
        );

        // ★ 解除 Discrete 门后，Continuous 动作被接受
        let active = current.as_ref().expect("Continuous 动作应被接受");
        assert_eq!(active.phase, ActionPhase::Windup);
        // resource_drain_rate 从 sustain_drain 初始化
        assert!((active.resource_drain_rate - 3.0).abs() < 1e-6);
        assert!(matches!(
            events.first(),
            Some(ActionLifecycleEvent::Started { .. })
        ));
    }

    /// 一个充能动作定义（aim_bow 风格）——Charge + 三阶梯 + Penalize overcharge。
    fn charge_aim_def() -> ActionDef {
        use woworld_core::action::{ChargeStage, OverchargeBehavior};
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
            physics_req: PhysicsRequirement::NotInWater,
            movement_lock: MovementLockDef::Partial,
            rotation_lock: RotationLockDef::Free,
            bufferable: false,
            buffer_window_ms: 0,
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
                        action_id: ActionId(201),
                        power_multiplier: 0.5,
                    },
                    ChargeStage {
                        threshold_ms: 400,
                        action_id: ActionId(202),
                        power_multiplier: 1.0,
                    },
                    ChargeStage {
                        threshold_ms: 800,
                        action_id: ActionId(203),
                        power_multiplier: 1.3,
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

    fn active_of(action_id: ActionId, elapsed: f32) -> ActiveAction {
        ActiveAction {
            instance: ActionInstanceId(0),
            action_id,
            phase: ActionPhase::Active,
            commitment: CommitmentLevel::Soft,
            elapsed,
            cancel_window_open: true,
            resource_drain_rate: 3.0,
            sustain_phase: SustainPhase::Normal,
        }
    }

    #[test]
    fn test_dispatch_release_complete_voluntary_completes() {
        // ReleaseBehavior::Complete + 主动松键 → Completed，无 follow-up
        let def = continuous_block_def();
        let (events, follow) =
            super::dispatch_release(&active_of(ActionId(100), 2.0), &def, EntityId(1), None);
        assert!(follow.is_none());
        assert!(matches!(
            events.first(),
            Some(ActionLifecycleEvent::Completed { .. })
        ));
    }

    #[test]
    fn test_dispatch_release_forced_is_interrupted() {
        // 强制结束（资源耗尽）→ Interrupted{VitalDepleted}
        let def = continuous_block_def();
        let (events, _) = super::dispatch_release(
            &active_of(ActionId(100), 8.0),
            &def,
            EntityId(1),
            Some(InterruptSource::VitalDepleted(ResourceType::Stamina)),
        );
        assert!(matches!(
            events.first(),
            Some(ActionLifecycleEvent::Interrupted {
                by: InterruptSource::VitalDepleted(ResourceType::Stamina),
                ..
            })
        ));
    }

    #[test]
    fn test_select_charge_stage_picks_highest_satisfied() {
        use woworld_core::action::ChargeStage;
        let stages = [
            ChargeStage {
                threshold_ms: 0,
                action_id: ActionId(1),
                power_multiplier: 0.5,
            },
            ChargeStage {
                threshold_ms: 400,
                action_id: ActionId(2),
                power_multiplier: 1.0,
            },
            ChargeStage {
                threshold_ms: 800,
                action_id: ActionId(3),
                power_multiplier: 1.3,
            },
        ];
        assert_eq!(
            super::select_charge_stage(&stages, 0).unwrap().action_id,
            ActionId(1)
        );
        assert_eq!(
            super::select_charge_stage(&stages, 500).unwrap().action_id,
            ActionId(2)
        );
        assert_eq!(
            super::select_charge_stage(&stages, 900).unwrap().action_id,
            ActionId(3)
        );
    }

    #[test]
    fn test_select_charge_stage_below_min_returns_none() {
        use woworld_core::action::ChargeStage;
        let stages = [ChargeStage {
            threshold_ms: 200,
            action_id: ActionId(1),
            power_multiplier: 0.5,
        }];
        assert!(super::select_charge_stage(&stages, 100).is_none());
    }

    #[test]
    fn test_dispatch_release_charged_emits_trigger_and_followup() {
        // aim_bow 充能 500ms（windup 200ms + 300ms 充能）→ 命中 aimed_shot(400ms 阶梯? 否)
        //   elapsed=0.7s → charge_ms=(0.7-0.2)*1000=500 → 命中 400ms 阶梯 = ActionId(202)
        let def = charge_aim_def();
        let (events, follow) =
            super::dispatch_release(&active_of(ActionId(200), 0.7), &def, EntityId(1), None);
        assert!(matches!(
            events.first(),
            Some(ActionLifecycleEvent::ChargeTrigger { .. })
        ));
        let f = follow.expect("charged 释放应产出 follow-up 子动作");
        assert_eq!(f.action_id, ActionId(202));
        assert_eq!(f.source, ActionSource::ChargedAction);
        assert_eq!(f.priority, action_priority::CHARGE_TRIGGER);
    }

    #[test]
    fn test_dispatch_release_trigger_completes_and_followups() {
        // ReleaseBehavior::Trigger——松键完成 + 触发固定子动作
        let mut def = continuous_block_def();
        def.release_behavior = Some(ReleaseBehavior::Trigger {
            action_id: ActionId(888),
        });
        let (events, follow) =
            super::dispatch_release(&active_of(ActionId(100), 2.0), &def, EntityId(1), None);
        assert!(matches!(
            events.first(),
            Some(ActionLifecycleEvent::Completed { .. })
        ));
        let f = follow.expect("Trigger 释放应产出固定子动作");
        assert_eq!(f.action_id, ActionId(888));
        assert_eq!(f.source, ActionSource::ChargedAction);
    }
}
