//! StaminaGateSystem — 体力门控
//!
//! 体力耗尽时强制降级 MovementState.pace。
//! 独立关注点——与 MovementModeSystem 解耦。先于 MovementModeSystem 执行。
//!
//! 参见: `WoWorld-Design/.../角色控制器/002-MovementState与连续移动.md` §六

use woworld_core::movement::Pace;

use crate::components::movement_state::CMovementState;
use crate::components::vitals::Vitals;

/// 冲刺体力最低门槛（单位）。
const SPRINT_MIN_STAMINA: f32 = 8.0;
/// 体力耗尽后禁止冲刺的冷却时间 (s)。
const EXHAUSTION_COOLDOWN: f32 = 1.5;

/// 体力门控——Sprinting 时检查体力，不足则强制降级。
///
/// Query: `(CMovementState, Vitals)`.
/// 执行顺序: 在 MovementModeSystem 之前（Block A1.5 前半）。
pub fn stamina_gate_system(world: &mut hecs::World, dt: f32) {
    for (_, (move_state, vitals)) in world.query_mut::<(&mut CMovementState, &Vitals)>() {
        let ms = &mut move_state.0;

        // ── cooldown 倒计时 ──
        if ms.exhaustion_cooldown > 0.0 {
            ms.exhaustion_cooldown = (ms.exhaustion_cooldown - dt).max(0.0);
        }

        // ── 冷却中——禁止冲刺（即时降级）──
        if ms.exhaustion_cooldown > 0.0 && ms.pace == Pace::Sprinting {
            ms.pace = Pace::Walking;
            continue;
        }

        // ── 非冲刺——不检查 ──
        if ms.pace != Pace::Sprinting {
            continue;
        }

        // ── 体力不足 → 降级 ──
        if vitals.stamina < SPRINT_MIN_STAMINA {
            ms.pace = Pace::Running;
        }

        // ── 体力耗尽 → 强制步行 + 冷却 ──
        if vitals.stamina <= 0.0 {
            ms.pace = Pace::Walking;
            ms.exhaustion_cooldown = EXHAUSTION_COOLDOWN;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use woworld_core::movement::{MovementState, Stance};

    fn make_vitals(stamina_val: f32) -> Vitals {
        Vitals {
            stamina: stamina_val,
            ..Default::default()
        }
    }

    #[test]
    fn test_sprinting_with_sufficient_stamina_unchanged() {
        let mut world = hecs::World::new();
        world.spawn((
            CMovementState(MovementState {
                stance: Stance::Standing,
                pace: Pace::Sprinting,
                ..Default::default()
            }),
            make_vitals(50.0),
        ));

        stamina_gate_system(&mut world, 0.016);

        for (_, (ms, _)) in world.query_mut::<(&CMovementState, &Vitals)>() {
            assert_eq!(ms.0.pace, Pace::Sprinting);
        }
    }

    #[test]
    fn test_sprinting_low_stamina_downgrades_to_running() {
        let mut world = hecs::World::new();
        world.spawn((
            CMovementState(MovementState {
                stance: Stance::Standing,
                pace: Pace::Sprinting,
                ..Default::default()
            }),
            make_vitals(5.0), // 低于 SPRINT_MIN_STAMINA (8.0)
        ));

        stamina_gate_system(&mut world, 0.016);

        for (_, (ms, _)) in world.query_mut::<(&CMovementState, &Vitals)>() {
            assert_eq!(ms.0.pace, Pace::Running);
        }
    }

    #[test]
    fn test_sprinting_zero_stamina_downgrades_to_walking_with_cooldown() {
        let mut world = hecs::World::new();
        world.spawn((
            CMovementState(MovementState {
                stance: Stance::Standing,
                pace: Pace::Sprinting,
                ..Default::default()
            }),
            make_vitals(0.0),
        ));

        stamina_gate_system(&mut world, 0.016);

        for (_, (ms, _)) in world.query_mut::<(&CMovementState, &Vitals)>() {
            assert_eq!(ms.0.pace, Pace::Walking);
            assert!(ms.0.exhaustion_cooldown > 0.0);
        }
    }

    #[test]
    fn test_cooldown_counts_down() {
        let mut world = hecs::World::new();
        world.spawn((
            CMovementState(MovementState {
                stance: Stance::Standing,
                pace: Pace::Walking,
                exhaustion_cooldown: 1.0,
                ..Default::default()
            }),
            make_vitals(50.0),
        ));

        stamina_gate_system(&mut world, 0.5);

        for (_, (ms, _)) in world.query_mut::<(&CMovementState, &Vitals)>() {
            assert!((ms.0.exhaustion_cooldown - 0.5).abs() < 0.01);
        }
    }

    #[test]
    fn test_non_sprinting_unaffected() {
        let mut world = hecs::World::new();
        world.spawn((
            CMovementState(MovementState {
                stance: Stance::Standing,
                pace: Pace::Walking,
                ..Default::default()
            }),
            make_vitals(0.0), // 体力耗尽但非冲刺
        ));

        stamina_gate_system(&mut world, 0.016);

        for (_, (ms, _)) in world.query_mut::<(&CMovementState, &Vitals)>() {
            assert_eq!(ms.0.pace, Pace::Walking); // 不变
        }
    }
}
