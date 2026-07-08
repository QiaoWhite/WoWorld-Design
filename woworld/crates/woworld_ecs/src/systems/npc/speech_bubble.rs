//! 对话气泡 System — NPC 头顶文字气泡（自言自语）
//!
//! MVP 桩化：从 NPC 自身 needs/emotion/goal 状态生成固定短语。未来接入
//! `概念与语言地基/003` 的 Utterance → TextGenerator 概念驱动文本链。
//!
//! ⚠️ 这是 **UI 文字气泡**（眼睛看），与 `音频系统/007` 的 Bark（语声，耳朵听）
//! 正交。类型见 `woworld_core::speech_bubble::BubbleType`。
//!
//! pull 管线：本 system 每帧更新 SpeechBubbleState，随后 entity_visual_system
//! 从中读取填入 EntityVisual，由 EntityRenderer 渲染。逻辑全留 Rust 侧。

use std::collections::HashSet;

use woworld_core::speech_bubble::BubbleType;

use crate::components::emotion::Emotion;
use crate::components::goal::{Goal, GoalType};
use crate::components::needs::Needs;
use crate::prng::pseudo_random_f32;
use crate::resources::speech_bubble_state::{ActiveBubble, SpeechBubbleState};

/// 气泡显示时长（tick）——设计 002 未规定数值，MVP 取 ~3s @ 60fps
const BUBBLE_DURATION_TICKS: u64 = 180;
/// 同一 NPC 两次气泡的最小间隔（tick）——~10s @ 60fps，防止话痨
const BUBBLE_COOLDOWN_TICKS: u64 = 600;
/// 冷却后每帧尝试触发的概率——低值使各 NPC 自然错峰，不同时刷屏
const TRIGGER_PROBABILITY: f32 = 0.05;

// ── 触发阈值（对齐 emotion.rs 现有惯例）────────
const HUNGER_THRESHOLD: f32 = 0.6;
const THIRST_THRESHOLD: f32 = 0.6;
const FATIGUE_THRESHOLD: f32 = 0.7;
const SOCIAL_THRESHOLD: f32 = 0.5;
const PLEASURE_HIGH: f32 = 0.3;
const PLEASURE_LOW: f32 = -0.3;
const GOAL_URGENCY_THRESHOLD: f32 = 0.6;
/// 外向性高于此值 → 语气加感叹号
const EXTRAVERSION_TALKATIVE: f32 = 0.6;

/// 从 NPC 状态挑选气泡内容——纯函数，确定性，可直接测试。
///
/// 优先级从高到低：生理需求 → 社交 → 情绪 → 目标。返回 None 表示此刻无话可说。
/// `extraversion` 用于语气微调（高外向 → 感叹号）。
pub fn pick_bubble(
    emotion: &Emotion,
    needs: &Needs,
    goal: &Goal,
    extraversion: f32,
) -> Option<(String, BubbleType)> {
    let talkative = extraversion > EXTRAVERSION_TALKATIVE;
    let excl = |s: &str| {
        if talkative {
            format!("{s}！")
        } else {
            s.to_string()
        }
    };

    // 生理需求（Ambient — 蓝灰）
    if needs.hunger > HUNGER_THRESHOLD {
        return Some(("肚子饿了…".to_string(), BubbleType::Ambient));
    }
    if needs.thirst > THIRST_THRESHOLD {
        return Some(("口渴…".to_string(), BubbleType::Ambient));
    }
    if needs.fatigue > FATIGUE_THRESHOLD {
        return Some(("好累…".to_string(), BubbleType::Ambient));
    }

    // 社交缺失（Normal — 白）
    if needs.social > SOCIAL_THRESHOLD {
        return Some((excl("有人吗"), BubbleType::Normal));
    }

    // 情绪极值（Emotion — 黄）
    if emotion.pleasure > PLEASURE_HIGH {
        return Some((excl("今天心情不错"), BubbleType::Emotion));
    }
    if emotion.pleasure < PLEASURE_LOW {
        return Some(("唉…".to_string(), BubbleType::Emotion));
    }

    // 目标驱动（Normal — 白）
    if goal.urgency > GOAL_URGENCY_THRESHOLD {
        let line = match goal.goal_type {
            GoalType::FindFood => "得找点吃的",
            GoalType::FindWater => "去找水喝",
            GoalType::FindRest => "想歇会儿",
            GoalType::FindSafePlace => "这儿不安全",
            GoalType::FindSocialContact => "找人聊聊",
            GoalType::BalanceElements => "得调理一下",
            GoalType::ExpressLibido => "有点寂寞",
            GoalType::Idle => return None,
        };
        return Some((excl(line), BubbleType::Normal));
    }

    None
}

/// 每帧更新对话气泡状态。
///
/// - 跳过 `player_entity`（被夺舍 NPC 由 CharacterBody3D 表示，冒泡会在退出夺舍时闪现残留）
/// - 活跃气泡过期 → 清除
/// - 无活跃气泡 && 过冷却 && 触发条件满足 && 概率门通过 → 生成新气泡
/// - 末尾剔除 despawn 实体，防止 slots 泄漏
pub fn speech_bubble_system(
    world: &hecs::World,
    current_tick: u64,
    player_entity: Option<hecs::Entity>,
    state: &mut SpeechBubbleState,
) {
    let mut visited: HashSet<hecs::Entity> = HashSet::new();

    for (entity, (emotion, needs, goal)) in world.query::<(&Emotion, &Needs, &Goal)>().iter() {
        if player_entity == Some(entity) {
            continue;
        }
        visited.insert(entity);

        let slot = state.slots.entry(entity).or_default();

        // 1. 过期检查
        if let Some(active) = &slot.active {
            if current_tick > active.expiry_tick {
                slot.active = None;
            }
        }

        // 2. 若已有活跃气泡，保留不变（避免闪烁）
        if slot.active.is_some() {
            continue;
        }

        // 3. 冷却门控
        if current_tick < slot.next_allowed_tick {
            continue;
        }

        // 4. 概率门——错峰，避免所有 NPC 同 tick 触发
        let roll = pseudo_random_f32(entity.to_bits().get() ^ current_tick);
        if roll >= TRIGGER_PROBABILITY {
            continue;
        }

        // 5. 触发条件 + 内容选择
        let extraversion = world
            .get::<&crate::components::bigfive::BigFive>(entity)
            .map(|bf| bf.extraversion)
            .unwrap_or(0.5);
        if let Some((text, bubble_type)) = pick_bubble(emotion, needs, goal, extraversion) {
            slot.active = Some(ActiveBubble {
                text,
                bubble_type,
                expiry_tick: current_tick + BUBBLE_DURATION_TICKS,
            });
            slot.next_allowed_tick = current_tick + BUBBLE_COOLDOWN_TICKS;
        }
    }

    // 6. 剔除本帧未见（despawn）的实体
    state.slots.retain(|e, _| visited.contains(e));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::bigfive::BigFive;
    use crate::components::transform::Position;
    use crate::resources::speech_bubble_state::BubbleSlot;
    use glam::Vec3;

    fn hungry_needs() -> Needs {
        Needs {
            hunger: 0.8,
            ..Needs::default()
        }
    }

    fn neutral_state() -> (Emotion, Needs, Goal) {
        (Emotion::default(), Needs::default(), Goal::default())
    }

    // ── pick_bubble 纯函数测试 ──────────────

    #[test]
    fn test_pick_hunger_ambient() {
        let (e, _, g) = neutral_state();
        let r = pick_bubble(&e, &hungry_needs(), &g, 0.5);
        assert_eq!(r, Some(("肚子饿了…".to_string(), BubbleType::Ambient)));
    }

    #[test]
    fn test_pick_neutral_none() {
        let (e, n, g) = neutral_state();
        assert!(pick_bubble(&e, &n, &g, 0.5).is_none());
    }

    #[test]
    fn test_pick_happy_emotion() {
        let e = Emotion {
            pleasure: 0.5,
            ..Emotion::default()
        };
        let (_, n, g) = neutral_state();
        let r = pick_bubble(&e, &n, &g, 0.5).unwrap();
        assert_eq!(r.1, BubbleType::Emotion);
    }

    #[test]
    fn test_pick_extraversion_exclamation() {
        let e = Emotion {
            pleasure: 0.5,
            ..Emotion::default()
        };
        let (_, n, g) = neutral_state();
        let quiet = pick_bubble(&e, &n, &g, 0.2).unwrap().0;
        let loud = pick_bubble(&e, &n, &g, 0.9).unwrap().0;
        assert!(!quiet.ends_with('！'));
        assert!(loud.ends_with('！'));
    }

    #[test]
    fn test_pick_priority_hunger_over_emotion() {
        // 又饿又开心 → 生理需求优先
        let e = Emotion {
            pleasure: 0.5,
            ..Emotion::default()
        };
        let (_, _, g) = neutral_state();
        let r = pick_bubble(&e, &hungry_needs(), &g, 0.5).unwrap();
        assert_eq!(r.1, BubbleType::Ambient);
    }

    #[test]
    fn test_pick_deterministic() {
        let (e, _, g) = neutral_state();
        let a = pick_bubble(&e, &hungry_needs(), &g, 0.5);
        let b = pick_bubble(&e, &hungry_needs(), &g, 0.5);
        assert_eq!(a, b);
    }

    // ── speech_bubble_system 集成测试 ────────

    #[test]
    fn test_empty_world_no_panic() {
        let world = hecs::World::new();
        let mut state = SpeechBubbleState::new();
        speech_bubble_system(&world, 0, None, &mut state);
        assert!(state.slots.is_empty());
    }

    #[test]
    fn test_trigger_eventually_fires() {
        let mut world = hecs::World::new();
        let e = world.spawn((Emotion::default(), hungry_needs(), Goal::default()));
        let mut state = SpeechBubbleState::new();
        // 概率门 5%/帧 → 200 帧内几乎必然触发（0.95^200 ≈ 3e-5）
        let mut fired = false;
        for tick in 0..200 {
            speech_bubble_system(&world, tick, None, &mut state);
            if state.active_for(e).is_some() {
                fired = true;
                break;
            }
        }
        assert!(fired, "hungry NPC should bark within 200 ticks");
        assert_eq!(state.active_for(e).unwrap().text, "肚子饿了…");
    }

    #[test]
    fn test_neutral_npc_never_fires() {
        let mut world = hecs::World::new();
        world.spawn((Emotion::default(), Needs::default(), Goal::default()));
        let mut state = SpeechBubbleState::new();
        for tick in 0..300 {
            speech_bubble_system(&world, tick, None, &mut state);
        }
        // 中性 NPC pick_bubble 永远 None → 无气泡
        assert!(state.slots.values().all(|s| s.active.is_none()));
    }

    #[test]
    fn test_expiry_removes_active() {
        let mut world = hecs::World::new();
        let e = world.spawn((Emotion::default(), hungry_needs(), Goal::default()));
        let mut state = SpeechBubbleState::new();
        // 手动放置一个 tick=50 过期的气泡
        state.slots.insert(
            e,
            BubbleSlot {
                active: Some(ActiveBubble {
                    text: "肚子饿了…".into(),
                    bubble_type: BubbleType::Ambient,
                    expiry_tick: 50,
                }),
                next_allowed_tick: 1000, // 冷却未到，不会立刻重新触发
            },
        );
        speech_bubble_system(&world, 51, None, &mut state);
        assert!(state.active_for(e).is_none(), "expired bubble should clear");
    }

    #[test]
    fn test_active_preserved_before_expiry() {
        let mut world = hecs::World::new();
        let e = world.spawn((Emotion::default(), hungry_needs(), Goal::default()));
        let mut state = SpeechBubbleState::new();
        state.slots.insert(
            e,
            BubbleSlot {
                active: Some(ActiveBubble {
                    text: "口渴…".into(),
                    bubble_type: BubbleType::Ambient,
                    expiry_tick: 100,
                }),
                next_allowed_tick: 0,
            },
        );
        // 未过期 → 文本保持不变（不被 hunger 覆盖，无闪烁）
        speech_bubble_system(&world, 50, None, &mut state);
        assert_eq!(state.active_for(e).unwrap().text, "口渴…");
    }

    #[test]
    fn test_cooldown_gates_retrigger() {
        let mut world = hecs::World::new();
        let e = world.spawn((Emotion::default(), hungry_needs(), Goal::default()));
        let mut state = SpeechBubbleState::new();
        // 气泡已过期，但冷却到 tick=500
        state.slots.insert(
            e,
            BubbleSlot {
                active: None,
                next_allowed_tick: 500,
            },
        );
        for tick in 100..200 {
            speech_bubble_system(&world, tick, None, &mut state);
        }
        assert!(
            state.active_for(e).is_none(),
            "should not retrigger before cooldown expires"
        );
    }

    #[test]
    fn test_player_excluded() {
        let mut world = hecs::World::new();
        let e = world.spawn((Emotion::default(), hungry_needs(), Goal::default()));
        let mut state = SpeechBubbleState::new();
        for tick in 0..200 {
            speech_bubble_system(&world, tick, Some(e), &mut state);
        }
        assert!(
            !state.slots.contains_key(&e),
            "player entity should have no bubble slot"
        );
    }

    #[test]
    fn test_despawn_pruned() {
        let mut world = hecs::World::new();
        let e = world.spawn((Emotion::default(), hungry_needs(), Goal::default()));
        let mut state = SpeechBubbleState::new();
        // 触发一个气泡
        for tick in 0..200 {
            speech_bubble_system(&world, tick, None, &mut state);
            if state.active_for(e).is_some() {
                break;
            }
        }
        assert!(state.slots.contains_key(&e));
        // despawn 后 slot 应被剔除
        world.despawn(e).unwrap();
        speech_bubble_system(&world, 300, None, &mut state);
        assert!(!state.slots.contains_key(&e), "despawned entity pruned");
    }

    #[test]
    fn test_extraversion_from_bigfive() {
        // 带高外向 BigFive 的 NPC 触发社交气泡时应带感叹号
        let mut world = hecs::World::new();
        let bf = BigFive {
            extraversion: 0.9,
            ..BigFive::default()
        };
        let social_needs = Needs {
            social: 0.8,
            ..Needs::default()
        };
        let e = world.spawn((
            Emotion::default(),
            social_needs,
            Goal::default(),
            bf,
            Position(Vec3::ZERO),
        ));
        let mut state = SpeechBubbleState::new();
        for tick in 0..200 {
            speech_bubble_system(&world, tick, None, &mut state);
            if state.active_for(e).is_some() {
                break;
            }
        }
        assert!(state.active_for(e).unwrap().text.ends_with('！'));
    }
}
