//! ActionResolverSystem — 输入解析（六层映射）→ ActionRequest
//!
//! **唯一感知游戏上下文的输入层**。ActionController 完全不感知——它只看到
//! `ActionRequest { action_id, target, .. }`。resolver 不检查体力/魔力/冷却
//! （ActionController 的事），不改 MovementState（MovementModeSystem 的事）。
//!
//! 仅玩家实体（`With<PlayerComponent>`）——绞杀者：旧 NPC 无新组件不受影响。
//! NPC 不经此系统：GOAP 直写 `CActionRequestBuf`（004 §七）。
//!
//! 数据驱动的缓冲/即时决策：`ActionDef.bufferable` → 入 `CInputBuffer`（时敏，
//! 由 input_buffer_system 下一帧 drain）；否则直写 `CActionRequestBuf`（即时）。
//!
//! 参见: `WoWorld-Design/.../角色控制器/004-ActionResolver与输入解析.md` §二/§五/§六
//!
//! ⚠️ 优雅降级（本冲刺范围）:
//!   - 第二层装备映射用 `CHeldItem` stub（缺省 Empty）——装备系统接线后零改动。
//!   - 第四层上下文解析（Interact→具体动作）见 `interact_context_system`。
//!   - 第五层特殊技能用 `CEquippedSkills` stub（未绑定→不发）——技能系统接线后零改动。

use crate::components::action_state::CActionRequestBuf;
use crate::components::input_state::{CEquippedSkills, CHeldItem, CInputBuffer};
use crate::components::player::{ControlModeComponent, PlayerComponent};
use crate::resources::action_registry::ActionRegistry;
use woworld_core::action::{ActionId, ActionParams, ActionRequest, ActionSource};
use woworld_core::input::{
    BufferPriority, BufferedInput, HeldItemKind, HotbarConfig, InputAction, InputState,
};
use woworld_core::player::ControlMode;

/// 玩家是否手控该输入动作所属的域。元操作（domain=None）恒可用。
fn controls(mode: ControlMode, action: InputAction) -> bool {
    match action.domain() {
        Some(d) => mode.controls_domain(d),
        None => true,
    }
}

/// 输入动作 → 缓冲优先级（满容量淘汰用，008 §二）。
fn buffer_priority_for(action: InputAction) -> BufferPriority {
    use InputAction::*;
    match action {
        Jump => BufferPriority::Movement,
        Dodge | Parry | Block => BufferPriority::Defensive,
        LightAttack | HeavyAttack | SpecialSkill(_) => BufferPriority::Combat,
        Interact | InteractWheel | PickUpAll | Talk => BufferPriority::Interaction,
        _ => BufferPriority::Combat,
    }
}

/// 发出一个已解析的动作——按 `ActionDef.bufferable` 路由到缓冲或即时队列。
///
/// registry 中不存在的 ActionId → 视为即时（ActionController 会 Failed 拒绝）。
#[allow(clippy::too_many_arguments)]
fn emit(
    action: InputAction,
    id: ActionId,
    req_buf: &mut CActionRequestBuf,
    in_buf: &mut CInputBuffer,
    registry: &ActionRegistry,
    now: f32,
) {
    let def = registry.get(id);
    let priority = def.map_or(0, |d| d.priority);
    let req = ActionRequest {
        action_id: id,
        priority,
        source: ActionSource::Player,
        params: ActionParams::default(),
    };

    let bufferable = def.map(|d| d.bufferable).unwrap_or(false);
    if bufferable {
        let window_ms = def.map(|d| d.buffer_window_ms).unwrap_or(0) as f32;
        in_buf.push_bounded(BufferedInput::new(
            req,
            now,
            window_ms,
            buffer_priority_for(action),
        ));
    } else {
        req_buf.0.push(req);
    }
}

/// ActionResolverSystem —— 六层输入映射 + ControlMode 域过滤（004 §二/§五/§六）。
pub fn action_resolver_system(
    world: &mut hecs::World,
    input: &InputState,
    hotbar: &HotbarConfig,
    registry: &ActionRegistry,
    now: f32,
) {
    for (_, (ctrl, req_buf, in_buf, held, skills)) in world
        .query_mut::<(
            &mut ControlModeComponent,
            &mut CActionRequestBuf,
            &mut CInputBuffer,
            Option<&CHeldItem>,
            Option<&CEquippedSkills>,
        )>()
        .with::<&PlayerComponent>()
    {
        // ── 第六层：ControlModeToggle（元操作，恒可用，先处理）──
        if input.was_pressed(InputAction::ControlModeToggle) {
            ctrl.mode = match ctrl.mode {
                ControlMode::Auto => ControlMode::Manual,
                ControlMode::Manual => ControlMode::Auto,
            };
        }
        let mode = ctrl.mode;
        let held_kind = held.map(|h| h.0).unwrap_or(HeldItemKind::Empty);

        // ── 第一层：直接动作键（不可覆盖）──
        if input.was_pressed(InputAction::Jump) && controls(mode, InputAction::Jump) {
            emit(
                InputAction::Jump,
                ActionRegistry::id_of("jump"),
                req_buf,
                in_buf,
                registry,
                now,
            );
        }
        if input.was_pressed(InputAction::Dodge) && controls(mode, InputAction::Dodge) {
            emit(
                InputAction::Dodge,
                ActionRegistry::id_of("dodge"),
                req_buf,
                in_buf,
                registry,
                now,
            );
        }
        if input.was_pressed(InputAction::Parry) && controls(mode, InputAction::Parry) {
            emit(
                InputAction::Parry,
                ActionRegistry::id_of("parry"),
                req_buf,
                in_buf,
                registry,
                now,
            );
        }

        // ── 第二层：装备相关动作（优雅降级：CHeldItem stub，缺省 Empty）──
        if input.was_pressed(InputAction::LightAttack) && controls(mode, InputAction::LightAttack) {
            let name = match held_kind {
                HeldItemKind::Ranged => "aim_bow", // 弓→拉弓充能（Sprint-065 006 §四 [action.aim_bow]）
                // Tool 采集需上下文（第四层 P4）——此处退化为通用攻击
                _ => "light_attack",
            };
            emit(
                InputAction::LightAttack,
                ActionRegistry::id_of(name),
                req_buf,
                in_buf,
                registry,
                now,
            );
        }
        if input.was_pressed(InputAction::HeavyAttack) && controls(mode, InputAction::HeavyAttack) {
            emit(
                InputAction::HeavyAttack,
                ActionRegistry::id_of("heavy_attack"),
                req_buf,
                in_buf,
                registry,
                now,
            );
        }

        // ── Block：按下起防御 / 释放发 RELEASE（§六，Continuous 运行时属 006）──
        if input.was_pressed(InputAction::Block) && controls(mode, InputAction::Block) {
            emit(
                InputAction::Block,
                ActionRegistry::id_of("block"),
                req_buf,
                in_buf,
                registry,
                now,
            );
        }
        if input.was_released(InputAction::Block) && controls(mode, InputAction::Block) {
            // resolver 不判断当前是否在防御——ActionController 解释同 id 的释放语义。
            req_buf.0.push(ActionRequest {
                action_id: ActionRegistry::id_of("block"),
                priority: 0,
                source: ActionSource::Player,
                params: ActionParams::default(),
            });
        }

        // ── 第三层：热键栏（数字键 1-9 → 玩家配置的 ActionId，走 ItemUse 域）──
        for slot in 1u8..=9 {
            if input.was_pressed(InputAction::HotbarSlot(slot))
                && controls(mode, InputAction::HotbarSlot(slot))
            {
                if let Some(id) = hotbar.get(slot) {
                    emit(
                        InputAction::HotbarSlot(slot),
                        id,
                        req_buf,
                        in_buf,
                        registry,
                        now,
                    );
                }
            }
        }

        // ── 第五层：特殊技能键（装备的技能）──
        // 优雅降级：CEquippedSkills stub 数据源——未挂组件/未绑定槽 → 不发请求。
        // 技能系统建好后填充 slots，此逻辑零改动即接线。
        if let Some(skills) = skills {
            for n in 0u8..4 {
                if input.was_pressed(InputAction::SpecialSkill(n))
                    && controls(mode, InputAction::SpecialSkill(n))
                {
                    if let Some(id) = skills.get(n) {
                        emit(
                            InputAction::SpecialSkill(n),
                            id,
                            req_buf,
                            in_buf,
                            registry,
                            now,
                        );
                    }
                }
            }
        }
        // 第四层（上下文交互）由 interact_context_system + resolve_interact_target 提供。
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::player::ControlModeComponent;
    use woworld_core::action::{ActionKind, CommitmentLevel};
    use woworld_core::kinematics::PhysicsRequirement;

    fn reg_with(key: &str, bufferable: bool) -> ActionRegistry {
        let mut r = ActionRegistry::new();
        let def = woworld_core::action::ActionDef {
            name: key.to_string(),
            category: "Combat".to_string(),
            kind: ActionKind::Discrete,
            priority: 30,
            commitment: CommitmentLevel::Soft,
            windup_ms: 50,
            active_ms: 100,
            recovery_ms: 100,
            cancel_set: vec![],
            cancel_window_ms: 0,
            bufferable,
            buffer_window_ms: if bufferable { 200 } else { 0 },
            physics_req: PhysicsRequirement::Grounded,
            movement_lock: Default::default(),
            rotation_lock: Default::default(),
            interrupt_on_move: false,
            sustain_drain: None,
            release_behavior: None,
            overextend_threshold_secs: None,
            critical_threshold_secs: None,
        };
        r.register(ActionRegistry::id_of(key), def);
        r
    }

    fn spawn_player(world: &mut hecs::World, mode: ControlMode) -> hecs::Entity {
        world.spawn((
            PlayerComponent::default(),
            ControlModeComponent { mode },
            CActionRequestBuf::default(),
            CInputBuffer::default(),
        ))
    }

    #[test]
    fn test_immediate_action_goes_to_request_buf() {
        // block 非缓冲 → 直写 CActionRequestBuf
        let reg = reg_with("block", false);
        let mut world = hecs::World::new();
        let e = spawn_player(&mut world, ControlMode::Manual);
        let mut input = InputState::default();
        input.press(InputAction::Block);

        action_resolver_system(&mut world, &input, &HotbarConfig::new(), &reg, 1.0);

        let buf = world.get::<&CActionRequestBuf>(e).unwrap();
        assert_eq!(buf.0.len(), 1);
        assert_eq!(buf.0[0].action_id, ActionRegistry::id_of("block"));
        assert_eq!(buf.0[0].source, ActionSource::Player);
    }

    #[test]
    fn test_bufferable_action_goes_to_input_buffer() {
        // jump 可缓冲 → 入 CInputBuffer，不直写 CActionRequestBuf
        let reg = reg_with("jump", true);
        let mut world = hecs::World::new();
        let e = spawn_player(&mut world, ControlMode::Manual);
        let mut input = InputState::default();
        input.press(InputAction::Jump);

        action_resolver_system(&mut world, &input, &HotbarConfig::new(), &reg, 2.0);

        let ib = world.get::<&CInputBuffer>(e).unwrap();
        let rb = world.get::<&CActionRequestBuf>(e).unwrap();
        assert_eq!(ib.entries.len(), 1);
        assert_eq!(
            ib.entries[0].action_request.action_id,
            ActionRegistry::id_of("jump")
        );
        assert!((ib.entries[0].pressed_at - 2.0).abs() < 0.001);
        assert!(rb.0.is_empty());
    }

    #[test]
    fn test_auto_mode_suppresses_gameplay_actions() {
        // Auto: 玩家不控任何域 → 不发请求
        let reg = reg_with("block", false);
        let mut world = hecs::World::new();
        let e = spawn_player(&mut world, ControlMode::Auto);
        let mut input = InputState::default();
        input.press(InputAction::Block);

        action_resolver_system(&mut world, &input, &HotbarConfig::new(), &reg, 1.0);

        let buf = world.get::<&CActionRequestBuf>(e).unwrap();
        assert!(buf.0.is_empty());
    }

    #[test]
    fn test_control_mode_toggle_flips_mode_and_is_meta() {
        // ControlModeToggle 是元操作——即使 Auto 也生效
        let reg = ActionRegistry::new();
        let mut world = hecs::World::new();
        let e = spawn_player(&mut world, ControlMode::Auto);
        let mut input = InputState::default();
        input.press(InputAction::ControlModeToggle);

        action_resolver_system(&mut world, &input, &HotbarConfig::new(), &reg, 1.0);

        let ctrl = world.get::<&ControlModeComponent>(e).unwrap();
        assert_eq!(ctrl.mode, ControlMode::Manual);
    }

    #[test]
    fn test_hotbar_slot_resolves_configured_action() {
        let reg = reg_with("dodge", false);
        let mut hotbar = HotbarConfig::new();
        hotbar.set(3, ActionRegistry::id_of("dodge"));
        let mut world = hecs::World::new();
        let e = spawn_player(&mut world, ControlMode::Manual);
        let mut input = InputState::default();
        input.press(InputAction::HotbarSlot(3));

        action_resolver_system(&mut world, &input, &hotbar, &reg, 1.0);

        let buf = world.get::<&CActionRequestBuf>(e).unwrap();
        assert_eq!(buf.0.len(), 1);
        assert_eq!(buf.0[0].action_id, ActionRegistry::id_of("dodge"));
    }

    #[test]
    fn test_block_release_emits_request() {
        let reg = reg_with("block", false);
        let mut world = hecs::World::new();
        let e = spawn_player(&mut world, ControlMode::Manual);
        let mut input = InputState::default();
        input.press(InputAction::Block);
        input.begin_frame(); // 清边沿，保留 held
        input.release(InputAction::Block);

        action_resolver_system(&mut world, &input, &HotbarConfig::new(), &reg, 1.0);

        let buf = world.get::<&CActionRequestBuf>(e).unwrap();
        // 释放帧：仅 release 请求（无 press）
        assert_eq!(buf.0.len(), 1);
        assert_eq!(buf.0[0].action_id, ActionRegistry::id_of("block"));
    }

    #[test]
    fn test_non_player_entity_ignored() {
        let reg = reg_with("block", false);
        let mut world = hecs::World::new();
        // 无 PlayerComponent
        let e = world.spawn((
            ControlModeComponent {
                mode: ControlMode::Manual,
            },
            CActionRequestBuf::default(),
            CInputBuffer::default(),
        ));
        let mut input = InputState::default();
        input.press(InputAction::Block);

        action_resolver_system(&mut world, &input, &HotbarConfig::new(), &reg, 1.0);

        let buf = world.get::<&CActionRequestBuf>(e).unwrap();
        assert!(buf.0.is_empty());
    }

    #[test]
    fn test_light_attack_empty_hands_maps_to_light_attack() {
        let reg = reg_with("light_attack", false);
        let mut world = hecs::World::new();
        // 无 CHeldItem → 缺省 Empty
        let e = spawn_player(&mut world, ControlMode::Manual);
        let mut input = InputState::default();
        input.press(InputAction::LightAttack);

        action_resolver_system(&mut world, &input, &HotbarConfig::new(), &reg, 1.0);

        let buf = world.get::<&CActionRequestBuf>(e).unwrap();
        assert_eq!(buf.0.len(), 1);
        assert_eq!(buf.0[0].action_id, ActionRegistry::id_of("light_attack"));
    }

    #[test]
    fn test_special_skill_uses_equipped_stub() {
        use crate::components::input_state::CEquippedSkills;
        let reg = reg_with("dodge", false); // 复用一个已注册动作作为技能
        let mut world = hecs::World::new();
        let e = spawn_player(&mut world, ControlMode::Manual);
        // 绑定 SpecialSkill(1) → dodge 的 ActionId
        let mut skills = CEquippedSkills::default();
        skills.slots[1] = Some(ActionRegistry::id_of("dodge"));
        world.insert_one(e, skills).unwrap();

        let mut input = InputState::default();
        input.press(InputAction::SpecialSkill(1));
        action_resolver_system(&mut world, &input, &HotbarConfig::new(), &reg, 1.0);

        let buf = world.get::<&CActionRequestBuf>(e).unwrap();
        assert_eq!(buf.0.len(), 1);
        assert_eq!(buf.0[0].action_id, ActionRegistry::id_of("dodge"));
    }

    #[test]
    fn test_special_skill_no_component_is_noop() {
        let reg = reg_with("dodge", false);
        let mut world = hecs::World::new();
        let e = spawn_player(&mut world, ControlMode::Manual);
        // 未挂 CEquippedSkills → 第五层不发
        let mut input = InputState::default();
        input.press(InputAction::SpecialSkill(0));
        action_resolver_system(&mut world, &input, &HotbarConfig::new(), &reg, 1.0);

        let buf = world.get::<&CActionRequestBuf>(e).unwrap();
        assert!(buf.0.is_empty());
    }
}
