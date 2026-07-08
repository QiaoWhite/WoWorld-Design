//! 情感 System — 基线漂移 + 生理拉动
//!
//! Phase 1: 仅 PAD 三轴漂移 + Needs 拉动。不含基本情绪分解/情绪传染/心境。
//! 参见: `开发文档/01-NPC核心/NPC活人感开发文档ver2.0.md` §2.1.5

use crate::components::bigfive::BigFive;
use crate::components::emotion::Emotion;
use crate::components::needs::Needs;

/// 情感漂移速率——≈2% 差距/天（设计文档 §2.1.5）
const DRIFT_SPEED: f32 = 0.02;
/// 一天的秒数
const SECONDS_PER_DAY: f32 = 86400.0;

// ── 生理拉动系数（设计文档 §2.1.5a）──────────

const HUNGER_PLEASURE_COEFF: f32 = 0.001;
const HUNGER_THRESHOLD: f32 = 0.4;

const FATIGUE_AROUSAL_COEFF: f32 = 0.001;
const FATIGUE_THRESHOLD: f32 = 0.7;

const SOCIAL_PLEASURE_COEFF: f32 = 0.001;
const SOCIAL_THRESHOLD: f32 = 0.5;

const LIBIDO_AROUSAL_COEFF: f32 = 0.001;
const LIBIDO_THRESHOLD: f32 = 0.7;

// ── emotion_drift_system ──────────────────

/// 每决策周期将 PAD 拉向 BigFive 定义的基线
///
/// `dt`: 自上次调用以来的秒数（用于按天缩放漂移速率）
pub fn emotion_drift_system(world: &mut hecs::World, dt: f32) {
    let factor = DRIFT_SPEED * dt / SECONDS_PER_DAY;

    for (_entity, (bf, emotion)) in world.query_mut::<(&BigFive, &mut Emotion)>() {
        let (bp, ba, bc) = Emotion::baseline_from_bigfive(bf);

        emotion.pleasure += (bp - emotion.pleasure) * factor;
        emotion.arousal += (ba - emotion.arousal) * factor;
        emotion.control += (bc - emotion.control) * factor;

        // clamp
        emotion.pleasure = emotion.pleasure.clamp(-1.0, 1.0);
        emotion.arousal = emotion.arousal.clamp(0.0, 1.0);
        emotion.control = emotion.control.clamp(-1.0, 1.0);
    }
}

// ── physiological_pull_system ─────────────

/// 身体需求对情感的微弱拉动（"背景色调"，非主导因素）
///
/// 系数极低（0.001）——饥饿不会让 NPC 突然暴怒，但持续匮乏会压低愉悦。
pub fn physiological_pull_system(world: &mut hecs::World) {
    for (_entity, (needs, emotion)) in world.query_mut::<(&Needs, &mut Emotion)>() {
        // hunger > threshold → 持续压低 pleasure
        if needs.hunger > HUNGER_THRESHOLD {
            emotion.pleasure -= HUNGER_PLEASURE_COEFF * (needs.hunger - HUNGER_THRESHOLD);
        }

        // fatigue > threshold → 压低 arousal（昏昏欲睡）
        if needs.fatigue > FATIGUE_THRESHOLD {
            emotion.arousal -= FATIGUE_AROUSAL_COEFF * (needs.fatigue - FATIGUE_THRESHOLD);
        }

        // social deficit → 压低 pleasure（孤独感）
        if needs.social > SOCIAL_THRESHOLD {
            emotion.pleasure -= SOCIAL_PLEASURE_COEFF * (needs.social - SOCIAL_THRESHOLD);
        }

        // libido → 拉高 arousal（性驱力激活）
        if needs.libido > LIBIDO_THRESHOLD {
            emotion.arousal += LIBIDO_AROUSAL_COEFF * (needs.libido - LIBIDO_THRESHOLD);
        }

        // clamp 到合法范围
        emotion.pleasure = emotion.pleasure.clamp(-1.0, 1.0);
        emotion.arousal = emotion.arousal.clamp(0.0, 1.0);
        emotion.control = emotion.control.clamp(-1.0, 1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_with_bigfive(world: &mut hecs::World, bf: BigFive) -> hecs::Entity {
        world.spawn((bf, Emotion::default()))
    }

    #[test]
    fn test_drift_toward_baseline() {
        let mut world = hecs::World::new();
        // 高神经质 → baseline_pleasure ≈ -0.2
        let e = spawn_with_bigfive(
            &mut world,
            BigFive {
                neuroticism: 1.0,
                ..BigFive::default()
            },
        );

        // 初始 pleasure=0，应漂向 -0.2
        emotion_drift_system(&mut world, 86400.0); // 1 天
        let em = world.get::<&Emotion>(e).unwrap();
        assert!(em.pleasure < 0.0, "should drift toward negative baseline");
    }

    #[test]
    fn test_drift_no_movement_at_baseline() {
        let mut world = hecs::World::new();
        let (bp, ba, bc) = Emotion::baseline_from_bigfive(&BigFive::default());
        let e = world.spawn((
            BigFive::default(),
            Emotion {
                pleasure: bp,
                arousal: ba,
                control: bc,
            },
        ));

        emotion_drift_system(&mut world, 86400.0);
        let em = world.get::<&Emotion>(e).unwrap();
        assert!(
            (em.pleasure - bp).abs() < 0.01,
            "at baseline, should not move"
        );
        assert!((em.arousal - ba).abs() < 0.01);
        assert!((em.control - bc).abs() < 0.01);
    }

    #[test]
    fn test_drift_partial_day() {
        let mut world = hecs::World::new();
        let e = spawn_with_bigfive(
            &mut world,
            BigFive {
                neuroticism: 1.0,
                ..BigFive::default()
            },
        );

        emotion_drift_system(&mut world, 43200.0); // 半天
        let half_pleasure = world.get::<&Emotion>(e).unwrap().pleasure;

        // 重置并跑全天
        world.insert_one(e, Emotion::default()).unwrap();
        emotion_drift_system(&mut world, 86400.0);
        let full_pleasure = world.get::<&Emotion>(e).unwrap().pleasure;

        // 半天漂移量 < 全天漂移量（绝对值）
        assert!(
            half_pleasure.abs() < full_pleasure.abs(),
            "half={half_pleasure} full={full_pleasure}"
        );
    }

    #[test]
    fn test_physiological_hunger_reduces_pleasure() {
        let mut world = hecs::World::new();
        let e = world.spawn((
            Needs {
                hunger: 0.9,
                ..Needs::default()
            },
            Emotion::default(),
        ));

        physiological_pull_system(&mut world);
        let em = world.get::<&Emotion>(e).unwrap();
        assert!(
            em.pleasure < 0.0,
            "hunger should reduce pleasure, got {}",
            em.pleasure
        );
    }

    #[test]
    fn test_physiological_libido_raises_arousal() {
        let mut world = hecs::World::new();
        let e = world.spawn((
            Needs {
                libido: 0.9,
                ..Needs::default()
            },
            Emotion {
                pleasure: 0.0,
                arousal: 0.5,
                control: 0.0,
            },
        ));

        physiological_pull_system(&mut world);
        let em = world.get::<&Emotion>(e).unwrap();
        assert!(
            em.arousal > 0.5,
            "libido should raise arousal, got {}",
            em.arousal
        );
    }

    #[test]
    fn test_physiological_no_effect_when_satisfied() {
        let mut world = hecs::World::new();
        let e = world.spawn((
            Needs::default(), // all 0
            Emotion::default(),
        ));

        physiological_pull_system(&mut world);
        let em = world.get::<&Emotion>(e).unwrap();
        assert!((em.pleasure - 0.0).abs() < f32::EPSILON);
        assert!((em.arousal - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_emotion_clamped_to_range() {
        let mut world = hecs::World::new();
        // 极端：反复压低 pleasure
        let e = world.spawn((
            BigFive {
                neuroticism: 1.0,
                ..BigFive::default()
            },
            Needs {
                hunger: 0.9,
                social: 0.9,
                ..Needs::default()
            },
            Emotion {
                pleasure: -0.99,
                arousal: 0.5,
                control: 0.0,
            },
        ));

        // 漂移 + 生理拉动
        emotion_drift_system(&mut world, 86400.0);
        physiological_pull_system(&mut world);

        let em = world.get::<&Emotion>(e).unwrap();
        assert!(em.pleasure >= -1.0, "pleasure should never go below -1.0");
        assert!(em.arousal >= 0.0, "arousal should never go below 0.0");
        assert!(em.arousal <= 1.0, "arousal should never go above 1.0");
        assert!(em.control >= -1.0);
        assert!(em.control <= 1.0);
    }

    #[test]
    fn test_empty_world_no_panic() {
        let mut world = hecs::World::new();
        emotion_drift_system(&mut world, 1.0);
        physiological_pull_system(&mut world);
    }
}
