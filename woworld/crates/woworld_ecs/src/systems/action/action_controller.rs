//! ActionController — 承诺中断系统核心算法
//!
//! ~40 行仲裁逻辑 + 阶段推进。无内部状态——状态在 CActiveAction Component 中。
//! 复杂度在 TOML 数据（ActionRegistry）中，不在代码中。
//!
//! 参见: `WoWorld-Design/.../角色控制器/003-ActionController与离散动作.md` §二

use smallvec::SmallVec;
use woworld_core::action::{
    action_priority, ActionKind, ActionLifecycleEvent, ActionPhase, ActionRequest, ActionSource,
    ActiveAction, InterruptSource, SustainPhase,
};
use woworld_core::kinematics::LocomotionMode;

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

    // ── 1. 处理释放信号（priority=RELEASE 的同 action_id 请求）──
    if let Some(active) = current.as_ref() {
        let should_release = requests
            .iter()
            .any(|r| r.action_id == active.action_id && r.priority == action_priority::RELEASE);
        if should_release {
            let instance = active.instance;
            let action_id = active.action_id;
            let total_ms = (active.elapsed * 1000.0) as u32;
            *current = None;
            events.push(ActionLifecycleEvent::Completed {
                instance,
                entity, // 由外层 ECS wrapper 填充
                action_id,
                total_duration_ms: total_ms,
            });
            return events;
        }
    }

    // ── 2. 检查取消请求 ──
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
                // Sprint 1: 仅处理 Discrete 动作
                if def.kind == ActionKind::Discrete {
                    let instance = counter.next_id();
                    let started_at = 0.0; // 由外层 ECS wrapper 填充时间
                    *current = Some(ActiveAction {
                        instance,
                        action_id: req.action_id,
                        phase: ActionPhase::Windup,
                        commitment: def.commitment,
                        elapsed: 0.0,
                        cancel_window_open: false,
                        resource_drain_rate: 0.0,
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
}
