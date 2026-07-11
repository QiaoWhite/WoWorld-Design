//! 行为权重 System — Phase 1 乘性权重链（6 因子子集）
//!
//! weight = need_action_match × emotion_modifier × personality_modifier
//!        × survival_suppression × lifestage_restriction × time_modifier
//!
//! 完整链见 `NPC活人感开发文档ver2.0.md` §select_action (lines 1607-1654)；
//! time_modifier = 该链 v3「昼夜修正」因子，提前到 Phase-1 子集实现（垂直切片 V1）。

use hecs::CommandBuffer;

use crate::components::action::{ActionCategory, ActionIntent};
use crate::components::bigfive::BigFive;
use crate::components::emotion::Emotion;
use crate::components::goal::{Goal, GoalType};
use crate::components::lifecycle::LifeStage;
use crate::components::needs::{NeedSensitivity, Needs};

/// GoalType → ActionCategory 映射
fn goal_to_action(goal: &GoalType) -> ActionCategory {
    match goal {
        GoalType::FindFood => ActionCategory::Eat,
        GoalType::FindWater => ActionCategory::Drink,
        GoalType::FindRest => ActionCategory::Rest,
        GoalType::FindSafePlace => ActionCategory::SeekSafety,
        GoalType::FindSocialContact => ActionCategory::Socialize,
        GoalType::BalanceElements => ActionCategory::Explore, // 寻找元素平衡素材
        GoalType::ExpressLibido => ActionCategory::Socialize, // 社交化表达
        GoalType::Idle => ActionCategory::Idle,
    }
}

/// need_action_match — 需求-行为匹配权重
///
/// 设计文档: `03-基本需求系统/004-决策器集成方案.md` lines 85-109
/// 返回 [1.0, 2.0]
fn need_action_match(goal: &GoalType, needs: &Needs, sens: &NeedSensitivity) -> f32 {
    let deviation = match goal {
        GoalType::FindFood => needs.hunger * sens.hunger_sens,
        GoalType::FindWater => needs.thirst * sens.thirst_sens,
        GoalType::FindRest => needs.fatigue * sens.fatigue_sens,
        GoalType::FindSafePlace => needs.safety * sens.safety_sens,
        GoalType::FindSocialContact => needs.social * sens.social_sens,
        GoalType::BalanceElements => needs.element_balance * sens.element_sens,
        GoalType::ExpressLibido => needs.libido * sens.libido_sens,
        GoalType::Idle => 0.0,
    };
    (1.0 + deviation).clamp(1.0, 2.0)
}

/// emotion_modifier — 情绪对行为权重的调制
///
/// PAD → action bias. Phase 1 简化自 EMOTION_ACTION_MODIFIERS 表 (48 条目).
fn emotion_modifier(category: ActionCategory, emotion: &Emotion) -> f32 {
    let mut modifier: f32 = 1.0;

    // pleasure < 0 → 逃避/攻击倾向上升
    if emotion.pleasure < -0.3 {
        match category {
            ActionCategory::Flee => modifier += 0.3,
            ActionCategory::Fight => modifier += 0.2,
            ActionCategory::Socialize => modifier -= 0.1,
            _ => {}
        }
    }
    // pleasure > 0 → 社交/探索倾向上升
    if emotion.pleasure > 0.3 {
        match category {
            ActionCategory::Socialize => modifier += 0.2,
            ActionCategory::Explore => modifier += 0.15,
            _ => {}
        }
    }
    // arousal > 0.6 → 活跃行为
    if emotion.arousal > 0.6 {
        match category {
            ActionCategory::Explore => modifier += 0.2,
            ActionCategory::Socialize => modifier += 0.1,
            ActionCategory::Fight => modifier += 0.15,
            _ => {}
        }
    }
    // arousal < 0.3 → 休息倾向
    if emotion.arousal < 0.3 {
        match category {
            ActionCategory::Rest => modifier += 0.2,
            ActionCategory::Idle => modifier += 0.1,
            _ => {}
        }
    }
    // control < -0.3 → 逃避/寻求安全
    if emotion.control < -0.3 {
        match category {
            ActionCategory::Flee => modifier += 0.3,
            ActionCategory::SeekSafety => modifier += 0.25,
            ActionCategory::Fight => modifier -= 0.2,
            _ => {}
        }
    }

    modifier.clamp(0.5, 2.0)
}

/// personality_modifier — BigFive 行为倾向
fn personality_modifier(category: ActionCategory, bf: &BigFive) -> f32 {
    let mut modifier: f32 = 1.0;

    match category {
        ActionCategory::Socialize => {
            modifier += bf.extraversion * 0.3;
            modifier += bf.agreeableness * 0.15;
        }
        ActionCategory::Explore => {
            modifier += bf.openness * 0.25;
            modifier += bf.extraversion * 0.15;
        }
        ActionCategory::SeekSafety => {
            modifier += bf.neuroticism * 0.3;
        }
        ActionCategory::Flee => {
            modifier += bf.neuroticism * 0.25;
        }
        ActionCategory::Fight => {
            modifier -= bf.agreeableness * 0.2;
            modifier += bf.neuroticism * 0.15;
        }
        ActionCategory::Rest => {
            modifier -= bf.extraversion * 0.1;
        }
        _ => {}
    }

    modifier.clamp(0.5, 2.0)
}

/// survival_suppression — 生存危机时压制非生存行为
///
/// 设计文档: `NPC活人感开发文档ver2.0.md` lines 1660-1673
fn survival_suppression(goal: &GoalType, needs: &Needs) -> f32 {
    let max_urgency = needs
        .hunger
        .max(needs.thirst)
        .max(needs.fatigue)
        .max(1.0 - needs.safety); // safety=0 最危险

    // sigmoid: 1/(1 + e^(10*(urgency-0.7)))
    let suppression = 1.0 / (1.0 + (10.0 * (max_urgency - 0.7)).exp());

    // 生存行为本身不受压制
    let is_survival = matches!(
        goal,
        GoalType::FindFood | GoalType::FindWater | GoalType::FindRest | GoalType::FindSafePlace
    );
    if is_survival {
        suppression.max(0.9) // 生存行为权重至少 0.9
    } else {
        suppression
    }
}

/// LifeStage → 行为限制
fn lifestage_restriction(category: ActionCategory, stage: &LifeStage) -> f32 {
    match stage {
        LifeStage::Infant => match category {
            ActionCategory::Fight | ActionCategory::Flee | ActionCategory::Work => 0.0,
            _ => 1.0,
        },
        LifeStage::Elder => match category {
            ActionCategory::Fight => 0.3,
            ActionCategory::Flee => 0.5,
            _ => 1.0,
        },
        _ => 1.0,
    }
}

/// time_modifier — 昼夜修正（设计 ver2.0 §select_action `time_modifier(&action, world)`, v3 因子）
///
/// 行动适宜度随世界时段**连续**调制——非门控、非日程、无「if 时间 then 意图」。
/// 纯世界时驱动，**不含个人 chronotype**（chronotype 已在需求层 `circadian_factor`
/// 以相位偏移生效，见 `components/circadian.rs`；R1 裁决遵设计 time_modifier 语义）。
///
/// 白昼度 `daylight = -cos(day_progress·TAU)` ∈ [-1, 1]：午夜 -1、正午 +1、日出/日落 0。
/// Rest 夜偏好、Socialize/Explore 日偏好；进食/饮水/生存/战斗/漫游中性（时段不压制生存）。
/// 返回 `1.0 ± TIME_AMPLITUDE`。
fn time_modifier(category: ActionCategory, day_progress: f32) -> f32 {
    const TIME_AMPLITUDE: f32 = 0.08; // ±8%（对齐需求层 circadian §3.4）
    let daylight = -(day_progress * std::f32::consts::TAU).cos(); // [-1, 1]
    let bias = match category {
        // 白昼活动：白天适宜度↑
        ActionCategory::Socialize | ActionCategory::Explore => daylight,
        // 休息：夜晚适宜度↑（反相）
        ActionCategory::Rest => -daylight,
        // 进食/饮水/寻求安全/战斗/逃跑/漫游：不受时段压制（中性）
        _ => 0.0,
    };
    1.0 + bias * TIME_AMPLITUDE
}

/// 行为权重引擎——计算当前最优 ActionIntent
///
/// 查询 (&Goal, &Needs, &NeedSensitivity, &Emotion, &BigFive, &LifeStage)
/// → 插入 ActionIntent
///
/// `day_progress`: 世界日内进度 0-1（0=午夜），喂 time_modifier；None 则跳过昼夜修正。
/// 调用者负责 `cmd.run_on(&mut world)`
pub fn action_weight_system(
    world: &hecs::World,
    cmd: &mut CommandBuffer,
    day_progress: Option<f32>,
) {
    for (_entity, (goal, needs, sens, emotion, bf, stage)) in world
        .query::<(
            &Goal,
            &Needs,
            &NeedSensitivity,
            &Emotion,
            &BigFive,
            &LifeStage,
        )>()
        .iter()
    {
        let category = goal_to_action(&goal.goal_type);

        let need_w = need_action_match(&goal.goal_type, needs, sens);
        let emotion_w = emotion_modifier(category, emotion);
        let personality_w = personality_modifier(category, bf);
        let survival_w = survival_suppression(&goal.goal_type, needs);
        let stage_w = lifestage_restriction(category, stage);
        // ★ V1: 第 6 因子——昼夜修正（设计 ver2.0 time_modifier）。时间未知则中性 1.0。
        let time_w = day_progress
            .map(|dp| time_modifier(category, dp))
            .unwrap_or(1.0);

        let weight = need_w * emotion_w * personality_w * survival_w * stage_w * time_w;

        cmd.insert_one(
            _entity,
            ActionIntent {
                category,
                weight: weight.clamp(0.0, 10.0),
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn default_components() -> (Goal, Needs, NeedSensitivity, Emotion, BigFive, LifeStage) {
        (
            Goal {
                goal_type: GoalType::Idle,
                urgency: 0.5,
                target_pos: None,
            },
            Needs::default(),
            NeedSensitivity::default(),
            Emotion::default(),
            BigFive::default(),
            LifeStage::YoungAdult,
        )
    }

    #[test]
    fn test_goal_to_action_eat() {
        assert_eq!(goal_to_action(&GoalType::FindFood), ActionCategory::Eat);
        assert_eq!(goal_to_action(&GoalType::FindRest), ActionCategory::Rest);
        assert_eq!(
            goal_to_action(&GoalType::FindSafePlace),
            ActionCategory::SeekSafety
        );
    }

    #[test]
    fn test_need_match_hungry_eat_high() {
        let (_, needs, sens, ..) = default_components();
        let goal = Goal {
            goal_type: GoalType::FindFood,
            urgency: 0.9,
            target_pos: None,
        };
        let needs = Needs {
            hunger: 0.9,
            ..needs
        };
        let w = need_action_match(&goal.goal_type, &needs, &sens);
        assert!(w > 1.5, "hungry NPC → eat weight high, got {w}");
    }

    #[test]
    fn test_need_match_full_eat_low() {
        let (_goal, needs, sens, ..) = default_components();
        let goal = Goal {
            goal_type: GoalType::FindFood,
            urgency: 0.9,
            target_pos: None,
        };
        let w = need_action_match(&goal.goal_type, &needs, &sens);
        assert!(w < 1.1, "full NPC → eat weight near 1.0, got {w}");
    }

    #[test]
    fn test_emotion_fear_favors_flee() {
        let fear = Emotion {
            pleasure: -0.5,
            arousal: 0.7,
            control: -0.5,
        };
        let happy = Emotion {
            pleasure: 0.5,
            arousal: 0.5,
            control: 0.3,
        };
        let w_fear = emotion_modifier(ActionCategory::Flee, &fear);
        let w_happy = emotion_modifier(ActionCategory::Flee, &happy);
        assert!(w_fear > w_happy, "fear → flee weight higher");
    }

    #[test]
    fn test_personality_extravert_social() {
        let ext = BigFive {
            extraversion: 1.0,
            ..BigFive::default()
        };
        let intro = BigFive {
            extraversion: 0.0,
            ..BigFive::default()
        };
        assert!(
            personality_modifier(ActionCategory::Socialize, &ext)
                > personality_modifier(ActionCategory::Socialize, &intro)
        );
    }

    #[test]
    fn test_personality_neurotic_safety() {
        let high_n = BigFive {
            neuroticism: 1.0,
            ..BigFive::default()
        };
        let low_n = BigFive {
            neuroticism: 0.0,
            ..BigFive::default()
        };
        assert!(
            personality_modifier(ActionCategory::SeekSafety, &high_n)
                > personality_modifier(ActionCategory::SeekSafety, &low_n)
        );
    }

    #[test]
    fn test_survival_suppression_near_death() {
        let goal = Goal {
            goal_type: GoalType::FindSocialContact,
            urgency: 0.5,
            target_pos: None,
        };
        let needs = Needs {
            hunger: 0.95,
            thirst: 0.9,
            fatigue: 0.8,
            ..Needs::default()
        };
        let s = survival_suppression(&goal.goal_type, &needs);
        assert!(s < 0.3, "near death → non-survival suppressed, got {s}");
    }

    #[test]
    fn test_survival_behavior_not_suppressed() {
        let goal = Goal {
            goal_type: GoalType::FindFood,
            urgency: 0.9,
            target_pos: None,
        };
        let needs = Needs {
            hunger: 0.95,
            ..Needs::default()
        };
        let s = survival_suppression(&goal.goal_type, &needs);
        assert!(
            s > 0.8,
            "survival behavior should not be suppressed, got {s}"
        );
    }

    #[test]
    fn test_lifestage_infant_cannot_fight() {
        assert_eq!(
            lifestage_restriction(ActionCategory::Fight, &LifeStage::Infant),
            0.0
        );
        assert_eq!(
            lifestage_restriction(ActionCategory::Work, &LifeStage::Infant),
            0.0
        );
    }

    #[test]
    fn test_lifestage_elder_fight_reduced() {
        assert!(lifestage_restriction(ActionCategory::Fight, &LifeStage::Elder) < 0.5);
    }

    #[test]
    fn test_weight_chain_all_in_range() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        for seed in 0..5 {
            let bf = BigFive::from_seed(seed);
            world.spawn((
                Goal {
                    goal_type: GoalType::FindFood,
                    urgency: 0.7,
                    target_pos: None,
                },
                Needs {
                    hunger: 0.6,
                    ..Needs::default()
                },
                NeedSensitivity::default(),
                Emotion::default(),
                bf,
                LifeStage::YoungAdult,
            ));
        }

        action_weight_system(&world, &mut cmd, Some(0.5));
        cmd.run_on(&mut world);

        for (_, intent) in world.query::<&ActionIntent>().iter() {
            assert!(
                (0.0..=10.0).contains(&intent.weight),
                "weight {} out of range",
                intent.weight
            );
        }
    }

    #[test]
    fn test_empty_world_no_panic() {
        let world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        action_weight_system(&world, &mut cmd, None);
    }

    // ── V1: time_modifier（昼夜修正）单元测试 ──

    #[test]
    fn test_time_modifier_night_favors_rest() {
        // 午夜 dp=0.0：Rest 适宜度 > 活动类，且 Rest>1.0、Explore<1.0
        let rest = time_modifier(ActionCategory::Rest, 0.0);
        let explore = time_modifier(ActionCategory::Explore, 0.0);
        assert!(
            rest > explore,
            "night: rest {rest} should exceed explore {explore}"
        );
        assert!(rest > 1.0 && explore < 1.0);
    }

    #[test]
    fn test_time_modifier_day_favors_activity() {
        // 正午 dp=0.5：活动类 > Rest
        let explore = time_modifier(ActionCategory::Explore, 0.5);
        let rest = time_modifier(ActionCategory::Rest, 0.5);
        assert!(
            explore > rest,
            "day: explore {explore} should exceed rest {rest}"
        );
        assert!(explore > 1.0 && rest < 1.0);
    }

    #[test]
    fn test_time_modifier_range_and_survival_neutral() {
        for i in 0..100 {
            let dp = i as f32 / 100.0;
            for cat in [
                ActionCategory::Rest,
                ActionCategory::Explore,
                ActionCategory::Socialize,
                ActionCategory::Eat,
            ] {
                let m = time_modifier(cat, dp);
                assert!(
                    (0.919..=1.081).contains(&m),
                    "modifier {m} out of ±8% for {cat:?} @ {dp}"
                );
            }
            // 生存类（Eat）时段中性——不被昼夜压制
            assert!((time_modifier(ActionCategory::Eat, dp) - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_time_modifier_continuous_no_gating() {
        // 陡峭段(dp≈0.25)相邻时刻差值远小于满幅——连续调制，无离散门控/查表跳变
        let a = time_modifier(ActionCategory::Rest, 0.24);
        let b = time_modifier(ActionCategory::Rest, 0.26);
        assert!(
            (a - b).abs() < 0.05,
            "should vary continuously, not snap (got {a} vs {b})"
        );
    }
}
