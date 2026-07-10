//! InteractContextSystem — "交互"键上下文解析（ActionResolver 第四层，004 §三/§四）
//!
//! `Interact` 键按下 → `resolve_interact_target`（core 纯仲裁）：
//!   - `Chosen` → 直接发 `ActionRequest`（params.target = 目标实体），隐藏轮盘
//!   - `Ambiguous` → 填充 `ActionWheelData`（→轮盘），不自动执行
//!   - `NoTarget` → 无操作
//!
//! 仅玩家实体（`With<PlayerComponent>`）且手控 Interaction 域。
//! 感官系统未建——`NearbyInteractables` 为 stub（测试/占位填充），接线后零改动。
//!
//! 参见: `WoWorld-Design/.../角色控制器/004-ActionResolver与输入解析.md` §三/§四

use crate::components::action_state::CActionRequestBuf;
use crate::components::player::{ControlModeComponent, PlayerComponent};
use crate::components::transform::{Position, Rotation};
use crate::resources::interact::{ActionWheelData, NearbyInteractables, WheelActionEntry};
use glam::Vec3;
use woworld_core::action::{ActionParams, ActionRequest, ActionSource};
use woworld_core::input::{InputAction, InputState};
use woworld_core::interact::{resolve_interact_target, ResolvedInteract};
use woworld_core::player::ActionDomain;

/// InteractContextSystem —— 交互键上下文解析 → ActionRequest / 动作轮盘。
pub fn interact_context_system(
    world: &mut hecs::World,
    input: &InputState,
    nearby: &NearbyInteractables,
    wheel: &mut ActionWheelData,
) {
    // 仅在按下 Interact 键这一帧动作
    if !input.was_pressed(InputAction::Interact) {
        return;
    }

    for (_, (ctrl, req_buf, pos, rot)) in world
        .query_mut::<(
            &ControlModeComponent,
            &mut CActionRequestBuf,
            &Position,
            &Rotation,
        )>()
        .with::<&PlayerComponent>()
    {
        if !ctrl.mode.controls_domain(ActionDomain::Interaction) {
            continue;
        }

        // 朝向：Rotation(Quat) 的前向（-Z）。
        let facing = rot.0 * Vec3::new(0.0, 0.0, -1.0);

        match resolve_interact_target(&nearby.candidates, pos.0, facing) {
            ResolvedInteract::Chosen(target) => {
                req_buf.0.push(ActionRequest {
                    action_id: target.action_id,
                    priority: target.priority(),
                    source: ActionSource::Player,
                    params: ActionParams {
                        target: Some(target.entity),
                        position: Some(target.position),
                        data: 0,
                    },
                });
                wheel.hide();
            }
            ResolvedInteract::Ambiguous(candidates) => {
                wheel.entries = candidates
                    .into_iter()
                    .map(|c| WheelActionEntry {
                        action_id: c.action_id,
                        label: format!("{:?}", c.kind),
                        target: Some(c.entity),
                        disabled_reason: None,
                    })
                    .collect();
                wheel.has_ambiguity = true;
                wheel.visible = true;
            }
            ResolvedInteract::NoTarget => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::transform::{Position, Rotation};
    use glam::Quat;
    use woworld_core::action::ActionId;
    use woworld_core::interact::{InteractKind, Interactable};
    use woworld_core::player::ControlMode;
    use woworld_core::types::EntityId;

    fn mk(kind: InteractKind, pos: Vec3, id: u32) -> Interactable {
        Interactable {
            entity: EntityId(id as u64),
            kind,
            position: pos,
            action_id: ActionId(id),
        }
    }

    fn spawn_player(world: &mut hecs::World) -> hecs::Entity {
        world.spawn((
            PlayerComponent::default(),
            ControlModeComponent {
                mode: ControlMode::Manual,
            },
            CActionRequestBuf::default(),
            Position(Vec3::ZERO),
            Rotation(Quat::IDENTITY), // 朝 -Z
        ))
    }

    #[test]
    fn test_chosen_target_emits_request_with_target() {
        let mut world = hecs::World::new();
        let e = spawn_player(&mut world);
        let mut nearby = NearbyInteractables::new();
        nearby
            .candidates
            .push(mk(InteractKind::Talk, Vec3::new(0.0, 0.0, -1.0), 7));
        let mut wheel = ActionWheelData::new();
        let mut input = InputState::default();
        input.press(InputAction::Interact);

        interact_context_system(&mut world, &input, &nearby, &mut wheel);

        let buf = world.get::<&CActionRequestBuf>(e).unwrap();
        assert_eq!(buf.0.len(), 1);
        assert_eq!(buf.0[0].action_id, ActionId(7));
        assert_eq!(buf.0[0].params.target, Some(EntityId(7)));
        assert!(!wheel.visible);
    }

    #[test]
    fn test_ambiguous_populates_wheel_not_request() {
        let mut world = hecs::World::new();
        let e = spawn_player(&mut world);
        let mut nearby = NearbyInteractables::new();
        // 两个同优先级近距离对话 → 歧义
        nearby
            .candidates
            .push(mk(InteractKind::Talk, Vec3::new(0.0, 0.0, -1.0), 1));
        nearby
            .candidates
            .push(mk(InteractKind::Talk, Vec3::new(0.2, 0.0, -1.18), 2));
        let mut wheel = ActionWheelData::new();
        let mut input = InputState::default();
        input.press(InputAction::Interact);

        interact_context_system(&mut world, &input, &nearby, &mut wheel);

        let buf = world.get::<&CActionRequestBuf>(e).unwrap();
        assert!(buf.0.is_empty());
        assert!(wheel.visible);
        assert!(wheel.has_ambiguity);
        assert_eq!(wheel.entries.len(), 2);
    }

    #[test]
    fn test_no_interact_press_is_noop() {
        let mut world = hecs::World::new();
        let e = spawn_player(&mut world);
        let mut nearby = NearbyInteractables::new();
        nearby
            .candidates
            .push(mk(InteractKind::Talk, Vec3::new(0.0, 0.0, -1.0), 7));
        let mut wheel = ActionWheelData::new();
        let input = InputState::default(); // 未按 Interact

        interact_context_system(&mut world, &input, &nearby, &mut wheel);

        let buf = world.get::<&CActionRequestBuf>(e).unwrap();
        assert!(buf.0.is_empty());
        assert!(!wheel.visible);
    }
}
