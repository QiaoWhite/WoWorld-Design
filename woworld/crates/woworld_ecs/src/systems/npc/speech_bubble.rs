//! 对话气泡 System — NPC 头顶文字气泡（问候/告别 + 自言自语）
//!
//! ## barrier-free 定位（对齐 `NPC活人感/08 行动涌现总纲` §1.2 原则2）
//! 气泡系统是**表达渲染器，不决策行为**。"见到人做什么"由既有 `ActionWeight`→`ActionIntent`
//! 在 `ActionCategory`（Fight/Flee/Socialize/…）上**涌现**决定；问候只是 `Socialize`/reactive
//! 反射赢时的**语音表达**，被 `Fight`/`Flee`/`SeekSafety` 的 `ActionIntent` **否决**。
//! → 打斗/沉默/单方不回应皆经此自然涌现，无语音决策 silo、无壁障。
//!
//! ## 数据驱动（对齐 `语言表达/003` §一「片段组合非单体模板」）
//! 无内联短语——全走 `SpeechFragmentRegistry`（TOML）。涌现在「选择」：人格×情绪×关系×时段
//! → 不同候选集 → 不同话。发话频率从 extraversion 涌现（006 `frequency_modifier`）。
//!
//! ## 两条产出车道
//! - **遭遇驱动**（新）：消费 `EncounterEvent` → 问候（Enter）/告别（Leave）。各 NPC 独立反应，
//!   非对称/时序天然涌现；朝向门（D5 感知赝品）+ 人格 occurrence + per-pair 冷却。
//! - **自言自语**（迁移）：从需求/情绪/目标涌现 `NeedMutter`/`EmotionVent`。
//!
//! 单槽仲裁（N1）：问候/告别**抢占**自言自语（Pass 1 先于 Pass 2）。
//! pull 管线：本 system 每帧更新 `SpeechBubbleState`，`entity_visual_system` 消费渲染。

use std::collections::HashSet;

use glam::Vec3;
use woworld_core::speech_bubble::{BubbleType, SpeechAct};
use woworld_core::time::TimeOfDay;
use woworld_core::types::EntityId;

use crate::components::action::{ActionCategory, ActionIntent};
use crate::components::bigfive::BigFive;
use crate::components::emotion::Emotion;
use crate::components::goal::{Goal, GoalType};
use crate::components::needs::Needs;
use crate::components::transform::{Position, Rotation};
use crate::prng::pseudo_random_f32;
use crate::resources::encounter_state::{EncounterKind, EncounterState};
use crate::resources::relation_storage::RelationStorage;
use crate::resources::speech_bubble_state::{ActiveBubble, SpeechBubbleState};
use crate::resources::speech_fragment_registry::{SpeechContext, SpeechFragmentRegistry};

/// 气泡显示时长（tick）——~3s @ 60fps
const BUBBLE_DURATION_TICKS: u64 = 180;
/// 同一 NPC 两次气泡最小间隔（tick）——~10s @ 60fps
const BUBBLE_COOLDOWN_TICKS: u64 = 600;
/// 同一对 NPC 重复问候的最小间隔（tick）——~30s @ 60fps（G2 防重逢刷屏）
const GREET_COOLDOWN_TICKS: u64 = 1800;
/// 自言自语基础触发概率——乘人格因子后错峰
const BASE_SELF_TALK_PROB: f32 = 0.05;
/// 朝向门阈（点积）——FOV≈±120° 视锥，< 此值视为在背后（D5）
const FACING_DOT_THRESHOLD: f32 = -0.5;

// ── 自言自语触发阈值（沿用 emotion.rs 惯例）────────
const HUNGER_THRESHOLD: f32 = 0.6;
const THIRST_THRESHOLD: f32 = 0.6;
const FATIGUE_THRESHOLD: f32 = 0.7;
const SOCIAL_THRESHOLD: f32 = 0.5;
const PLEASURE_HIGH: f32 = 0.3;
const PLEASURE_LOW: f32 = -0.3;
const GOAL_URGENCY_THRESHOLD: f32 = 0.6;

/// 该 `ActionIntent` 是否否决问候——barrier-free：**真正**打斗/逃跑者不寒暄。
///
/// ⚠️ 只否决 `Fight`/`Flee`（真实敌意/逃命）。**不否决 `SeekSafety`**——
/// 它由安全需求随时间累积触发（"找个安全地儿"），在无威胁村庄=环境需求，非遇袭逃命，
/// 不该压制问候（否则 needs 一涨全村沉默·Sprint-068 实机诊断实证）。
/// 未来战斗行为就位后，`Fight`/`Flee` 自然否决问候（无需改本函数）。
pub fn action_vetoes_greeting(cat: ActionCategory) -> bool {
    matches!(cat, ActionCategory::Fight | ActionCategory::Flee)
}

/// 问候发生概率——从人格/情绪涌现（内向/坏心情 → 常沉默）。纯函数·单调可测。
///
/// extraversion 主导，agreeableness/pleasure 微调。返回 [0,1]。
pub fn greeting_occurrence(extraversion: f32, agreeableness: f32, pleasure: f32) -> f32 {
    (0.15 + extraversion * 0.7 + agreeableness * 0.15 + pleasure * 0.2).clamp(0.0, 1.0)
}

/// 朝向门——perceiver 是否面向 other（无 `Rotation` → 全向不设门）。
fn faces_toward(p_pos: Vec3, p_rot: Option<Rotation>, other_pos: Vec3) -> bool {
    let Some(rot) = p_rot else {
        return true;
    };
    let forward = rot.0 * Vec3::Z; // movement 用 from_rotation_arc(Z, dir) → forward=旋转后的 Z
    let fwd = Vec3::new(forward.x, 0.0, forward.z);
    let to = other_pos - p_pos;
    let to = Vec3::new(to.x, 0.0, to.z);
    if fwd.length_squared() < 1e-6 || to.length_squared() < 1e-6 {
        return true;
    }
    fwd.normalize().dot(to.normalize()) >= FACING_DOT_THRESHOLD
}

/// 自言自语分类——纯函数，确定性。优先级：生理 → 社交 → 情绪 → 目标。
///
/// 返回 `(SpeechAct, topic)`；`None` 表示无话可说。文本由 registry 按 topic/情绪选。
pub fn classify_self_talk(
    emotion: &Emotion,
    needs: &Needs,
    goal: &Goal,
) -> Option<(SpeechAct, Option<&'static str>)> {
    if needs.hunger > HUNGER_THRESHOLD {
        return Some((SpeechAct::NeedMutter, Some("hunger")));
    }
    if needs.thirst > THIRST_THRESHOLD {
        return Some((SpeechAct::NeedMutter, Some("thirst")));
    }
    if needs.fatigue > FATIGUE_THRESHOLD {
        return Some((SpeechAct::NeedMutter, Some("fatigue")));
    }
    if needs.social > SOCIAL_THRESHOLD {
        return Some((SpeechAct::NeedMutter, Some("social")));
    }
    if emotion.pleasure > PLEASURE_HIGH || emotion.pleasure < PLEASURE_LOW {
        return Some((SpeechAct::EmotionVent, None));
    }
    if goal.urgency > GOAL_URGENCY_THRESHOLD {
        let topic = match goal.goal_type {
            GoalType::FindFood => "find_food",
            GoalType::FindWater => "find_water",
            GoalType::FindRest => "find_rest",
            GoalType::FindSafePlace => "find_safe",
            GoalType::FindSocialContact => "find_social",
            GoalType::BalanceElements => "balance",
            GoalType::ExpressLibido => "libido",
            GoalType::Idle => return None,
        };
        return Some((SpeechAct::NeedMutter, Some(topic)));
    }
    None
}

/// perceiver 对 other 尝试一次社交发话（问候/告别）——经 barrier-free 门控涌现。
///
/// 门序：`ActionIntent` 否决 → 朝向门 → 人格 occurrence → formality 选句。任一不过返回 `None`。
#[allow(clippy::too_many_arguments)]
fn try_social_utterance(
    world: &hecs::World,
    perceiver: hecs::Entity,
    other: hecs::Entity,
    act: SpeechAct,
    time_of_day: TimeOfDay,
    relations: &RelationStorage,
    current_tick: u64,
    fragments: &SpeechFragmentRegistry,
) -> Option<(String, BubbleType)> {
    // ① ActionIntent 否决（行为涌现决定是否社交·非语音决策）
    if let Ok(ai) = world.get::<&ActionIntent>(perceiver) {
        if action_vetoes_greeting(ai.category) {
            return None;
        }
    }
    // ② 朝向门（D5 感知赝品·背后不问候）
    let p_pos = world.get::<&Position>(perceiver).ok().map(|p| p.0)?;
    let o_pos = world.get::<&Position>(other).ok().map(|p| p.0)?;
    let p_rot = world.get::<&Rotation>(perceiver).ok().map(|r| *r);
    if !faces_toward(p_pos, p_rot, o_pos) {
        return None;
    }
    // ③ 人格 occurrence（内向/坏心情涌现沉默）
    let (extraversion, agreeableness) = match world.get::<&BigFive>(perceiver) {
        Ok(b) => (b.extraversion, b.agreeableness),
        Err(_) => (0.5, 0.5),
    };
    let pleasure = world
        .get::<&Emotion>(perceiver)
        .map(|e| e.pleasure)
        .unwrap_or(0.0);
    let seed = social_seed(perceiver, other, current_tick);
    let occ = greeting_occurrence(extraversion, agreeableness, pleasure);
    if pseudo_random_f32(seed) >= occ {
        return None;
    }
    // ④ formality 从 trust 派生（生人 None → 0.0 陌生档）+ 选句
    let (pid, oid) = (
        EntityId(perceiver.to_bits().get()),
        EntityId(other.to_bits().get()),
    );
    let trust = relations.get(pid, oid).map(|r| r.trust).unwrap_or(0.0);
    let ctx = SpeechContext {
        time_of_day,
        trust,
        pleasure,
        extraversion,
        topic: None,
    };
    fragments.select(act, &ctx, seed)
}

/// perceiver-specific 种子——非对称（A/B 各自不同 → 独立选句/独立决策）。
fn social_seed(perceiver: hecs::Entity, other: hecs::Entity, tick: u64) -> u64 {
    perceiver
        .to_bits()
        .get()
        .wrapping_mul(1_000_003)
        .wrapping_add(other.to_bits().get())
        .wrapping_add(tick)
}

/// 每帧更新对话气泡状态。
///
/// - Pass 1 遭遇驱动问候/告别（抢占自言自语）
/// - Pass 2 自言自语（数据驱动·不抢占）
/// - 末尾剔除 despawn 实体，防 slots 泄漏
#[allow(clippy::too_many_arguments)]
pub fn speech_bubble_system(
    world: &hecs::World,
    current_tick: u64,
    player_entity: Option<hecs::Entity>,
    state: &mut SpeechBubbleState,
    encounters: &mut EncounterState,
    fragments: &SpeechFragmentRegistry,
    relations: &RelationStorage,
    day_progress: f32,
) {
    let time_of_day = TimeOfDay::from_progress(day_progress as f64);
    let mut visited: HashSet<hecs::Entity> = HashSet::new();

    // ── Pass 1: 遭遇驱动问候/告别（各方独立涌现·抢占自言自语）──
    let events = encounters.events.clone();
    for ev in &events {
        for &(perceiver, other) in &[(ev.a, ev.b), (ev.b, ev.a)] {
            if player_entity == Some(perceiver) {
                continue; // 玩家实体不冒泡（延续排除）
            }
            let act = match ev.kind {
                EncounterKind::Enter => SpeechAct::Greeting,
                EncounterKind::Leave => {
                    if ev.by_despawn {
                        continue; // G3：不对已消失者道别
                    }
                    SpeechAct::Farewell
                }
            };
            if act == SpeechAct::Greeting
                && !encounters.can_greet(perceiver, other, current_tick, GREET_COOLDOWN_TICKS)
            {
                continue; // G2 冷却
            }
            if let Some((text, bt)) = try_social_utterance(
                world,
                perceiver,
                other,
                act,
                time_of_day,
                relations,
                current_tick,
                fragments,
            ) {
                visited.insert(perceiver);
                let slot = state.slots.entry(perceiver).or_default();
                slot.active = Some(ActiveBubble {
                    text,
                    bubble_type: bt,
                    expiry_tick: current_tick + BUBBLE_DURATION_TICKS,
                });
                slot.next_allowed_tick = current_tick + BUBBLE_COOLDOWN_TICKS;
                if act == SpeechAct::Greeting {
                    encounters.mark_greet(perceiver, other, current_tick);
                }
            }
        }
    }

    // ── Pass 2: 自言自语（数据驱动·不抢占已有气泡）──
    for (entity, (emotion, needs, goal)) in world.query::<(&Emotion, &Needs, &Goal)>().iter() {
        if player_entity == Some(entity) {
            continue;
        }
        visited.insert(entity);
        let slot = state.slots.entry(entity).or_default();

        // 过期检查
        if let Some(active) = &slot.active {
            if current_tick > active.expiry_tick {
                slot.active = None;
            }
        }
        // 已有活跃气泡（含 Pass 1 问候）→ 保留不覆盖
        if slot.active.is_some() {
            continue;
        }
        if current_tick < slot.next_allowed_tick {
            continue;
        }

        // 发话驱动——外向者更话痨（涌现频率·006 frequency_modifier）
        let extraversion = world
            .get::<&BigFive>(entity)
            .map(|b| b.extraversion)
            .unwrap_or(0.5);
        let speak_drive = BASE_SELF_TALK_PROB * (0.4 + extraversion * 1.2);
        let roll = pseudo_random_f32(entity.to_bits().get() ^ current_tick);
        if roll >= speak_drive {
            continue;
        }

        if let Some((act, topic)) = classify_self_talk(emotion, needs, goal) {
            let ctx = SpeechContext {
                time_of_day,
                trust: 0.0,
                pleasure: emotion.pleasure,
                extraversion,
                topic,
            };
            let seed = entity
                .to_bits()
                .get()
                .wrapping_mul(31)
                .wrapping_add(current_tick);
            if let Some((text, bt)) = fragments.select(act, &ctx, seed) {
                slot.active = Some(ActiveBubble {
                    text,
                    bubble_type: bt,
                    expiry_tick: current_tick + BUBBLE_DURATION_TICKS,
                });
                slot.next_allowed_tick = current_tick + BUBBLE_COOLDOWN_TICKS;
            }
        }
    }

    // 剔除本帧未见（despawn）的实体
    state.slots.retain(|e, _| visited.contains(e));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::action::ActionIntent;
    use crate::systems::npc::encounter::encounter_system;

    fn reg() -> SpeechFragmentRegistry {
        SpeechFragmentRegistry::load_embedded()
    }

    fn face(dir: Vec3) -> Rotation {
        Rotation(glam::Quat::from_rotation_arc(Vec3::Z, dir.normalize()))
    }

    fn extravert() -> BigFive {
        BigFive {
            extraversion: 1.0,
            agreeableness: 1.0,
            ..BigFive::default()
        }
    }

    // ── 纯函数 ─────────────────────────────

    #[test]
    fn test_classify_hunger_priority() {
        let n = Needs {
            hunger: 0.8,
            ..Needs::default()
        };
        assert_eq!(
            classify_self_talk(&Emotion::default(), &n, &Goal::default()),
            Some((SpeechAct::NeedMutter, Some("hunger")))
        );
    }

    #[test]
    fn test_classify_neutral_none() {
        assert!(
            classify_self_talk(&Emotion::default(), &Needs::default(), &Goal::default()).is_none()
        );
    }

    #[test]
    fn test_classify_emotion_vent() {
        let e = Emotion {
            pleasure: 0.5,
            ..Emotion::default()
        };
        assert_eq!(
            classify_self_talk(&e, &Needs::default(), &Goal::default()),
            Some((SpeechAct::EmotionVent, None))
        );
    }

    #[test]
    fn test_action_veto() {
        assert!(action_vetoes_greeting(ActionCategory::Fight));
        assert!(action_vetoes_greeting(ActionCategory::Flee));
        // SeekSafety 不否决——安全需求≠遇袭逃命（实机诊断实证）
        assert!(!action_vetoes_greeting(ActionCategory::SeekSafety));
        assert!(!action_vetoes_greeting(ActionCategory::Socialize));
        assert!(!action_vetoes_greeting(ActionCategory::Idle));
        assert!(!action_vetoes_greeting(ActionCategory::Eat));
    }

    #[test]
    fn test_greeting_occurrence_monotonic() {
        // 外向性单调驱动问候概率（涌现）
        assert!(greeting_occurrence(0.9, 0.5, 0.0) > greeting_occurrence(0.1, 0.5, 0.0));
        assert!(greeting_occurrence(1.0, 1.0, 0.0) >= 0.99);
    }

    #[test]
    fn test_greet_rate_correlates_with_extraversion() {
        // ★ N6 涌现属性测试：外向者问候率显著更高（统计·1000 样本）
        let occ_high = greeting_occurrence(0.9, 0.5, 0.0);
        let occ_low = greeting_occurrence(0.1, 0.5, 0.0);
        let (mut high, mut low) = (0u32, 0u32);
        for seed in 0..1000u64 {
            if pseudo_random_f32(seed) < occ_high {
                high += 1;
            }
            if pseudo_random_f32(seed) < occ_low {
                low += 1;
            }
        }
        assert!(high > low + 200, "外向者问候率应显著更高: {high} vs {low}");
    }

    #[test]
    fn test_faces_toward() {
        let a = Vec3::ZERO;
        let b_front = Vec3::new(2.0, 0.0, 0.0);
        let b_behind = Vec3::new(-2.0, 0.0, 0.0);
        let rot = face(Vec3::X); // 面向 +X
        assert!(faces_toward(a, Some(rot), b_front), "正前方可见");
        assert!(!faces_toward(a, Some(rot), b_behind), "正后方不可见");
        assert!(faces_toward(a, None, b_behind), "无朝向→全向");
    }

    // ── 遭遇集成 ───────────────────────────

    /// 造一个"远→近"的遭遇：先播种（远），再移近产 Enter 事件。
    fn setup_encounter(
        world: &mut hecs::World,
        a_bf: BigFive,
        a_rot: Rotation,
    ) -> (hecs::Entity, hecs::Entity, EncounterState) {
        let a = world.spawn((
            Position(Vec3::new(0.0, 0.0, 0.0)),
            a_rot,
            Needs::default(),
            Emotion::default(),
            Goal::default(),
            a_bf,
        ));
        let b = world.spawn((
            Position(Vec3::new(10.0, 0.0, 0.0)),
            face(Vec3::NEG_X),
            Needs::default(),
            Emotion::default(),
            Goal::default(),
            extravert(),
        ));
        let mut enc = EncounterState::new();
        encounter_system(world, &mut enc); // 播种（远·无对）
        *world.get::<&mut Position>(b).unwrap() = Position(Vec3::new(2.0, 0.0, 0.0));
        encounter_system(world, &mut enc); // Enter
        (a, b, enc)
    }

    #[test]
    fn test_greeting_fires_on_encounter() {
        let mut world = hecs::World::new();
        let (a, _b, mut enc) = setup_encounter(&mut world, extravert(), face(Vec3::X));
        let mut state = SpeechBubbleState::new();
        let rels = RelationStorage::default();
        speech_bubble_system(&world, 100, None, &mut state, &mut enc, &reg(), &rels, 0.35);
        // a 面向 b、外向 occ=1.0 → 必问候
        assert!(state.active_for(a).is_some(), "外向面向者应问候");
    }

    #[test]
    fn test_greeting_vetoed_by_fight() {
        let mut world = hecs::World::new();
        let (a, _b, mut enc) = setup_encounter(&mut world, extravert(), face(Vec3::X));
        // a 处于战斗意图 → 否决问候
        world
            .insert_one(
                a,
                ActionIntent {
                    category: ActionCategory::Fight,
                    weight: 5.0,
                },
            )
            .unwrap();
        let mut state = SpeechBubbleState::new();
        let rels = RelationStorage::default();
        speech_bubble_system(&world, 100, None, &mut state, &mut enc, &reg(), &rels, 0.35);
        assert!(state.active_for(a).is_none(), "战斗意图应否决问候");
    }

    #[test]
    fn test_greeting_suppressed_facing_away() {
        let mut world = hecs::World::new();
        // a 背对 b（面向 -X，b 在 +X）
        let (a, _b, mut enc) = setup_encounter(&mut world, extravert(), face(Vec3::NEG_X));
        let mut state = SpeechBubbleState::new();
        let rels = RelationStorage::default();
        speech_bubble_system(&world, 100, None, &mut state, &mut enc, &reg(), &rels, 0.35);
        assert!(state.active_for(a).is_none(), "背对不问候");
    }

    #[test]
    fn test_greet_cooldown_blocks_immediate_regreet() {
        let mut world = hecs::World::new();
        let (a, b, mut enc) = setup_encounter(&mut world, extravert(), face(Vec3::X));
        let mut state = SpeechBubbleState::new();
        let rels = RelationStorage::default();
        speech_bubble_system(&world, 100, None, &mut state, &mut enc, &reg(), &rels, 0.35);
        assert!(state.active_for(a).is_some());
        // 强制再产 Enter（离开再回来）
        *world.get::<&mut Position>(b).unwrap() = Position(Vec3::new(10.0, 0.0, 0.0));
        encounter_system(&world, &mut enc);
        *world.get::<&mut Position>(b).unwrap() = Position(Vec3::new(2.0, 0.0, 0.0));
        encounter_system(&world, &mut enc);
        state.slots.clear(); // 清气泡，只看是否重新问候
        speech_bubble_system(&world, 200, None, &mut state, &mut enc, &reg(), &rels, 0.35);
        assert!(state.active_for(a).is_none(), "冷却内不重复问候");
    }

    #[test]
    fn test_self_talk_still_works() {
        let mut world = hecs::World::new();
        let e = world.spawn((
            Emotion::default(),
            Needs {
                hunger: 0.8,
                ..Needs::default()
            },
            Goal::default(),
        ));
        let mut state = SpeechBubbleState::new();
        let mut enc = EncounterState::new();
        let rels = RelationStorage::default();
        let reg = reg();
        let mut fired = false;
        for tick in 0..300 {
            speech_bubble_system(&world, tick, None, &mut state, &mut enc, &reg, &rels, 0.5);
            if state.active_for(e).is_some() {
                fired = true;
                break;
            }
        }
        assert!(fired, "饿的 NPC 应自言自语");
        assert_eq!(state.active_for(e).unwrap().text, "肚子饿了…");
    }

    #[test]
    fn test_player_excluded_from_bubbles() {
        let mut world = hecs::World::new();
        let (a, _b, mut enc) = setup_encounter(&mut world, extravert(), face(Vec3::X));
        let mut state = SpeechBubbleState::new();
        let rels = RelationStorage::default();
        speech_bubble_system(
            &world,
            100,
            Some(a),
            &mut state,
            &mut enc,
            &reg(),
            &rels,
            0.35,
        );
        assert!(state.active_for(a).is_none(), "玩家实体不冒泡");
    }

    #[test]
    fn test_neutral_npc_silent() {
        let mut world = hecs::World::new();
        let e = world.spawn((Emotion::default(), Needs::default(), Goal::default()));
        let mut state = SpeechBubbleState::new();
        let mut enc = EncounterState::new();
        let rels = RelationStorage::default();
        let reg = reg();
        for tick in 0..300 {
            speech_bubble_system(&world, tick, None, &mut state, &mut enc, &reg, &rels, 0.5);
        }
        assert!(
            state.slots.values().all(|s| s.active.is_none()),
            "中性 NPC 无话"
        );
    }
}
