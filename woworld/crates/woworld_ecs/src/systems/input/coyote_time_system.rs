//! CoyoteTimeSystem — 土狼时间窗口管理
//!
//! ★ 关键管线约束: 本 System 必须在 MovementModeSystem 之前运行，
//!    因为 MovementModeSystem 会更新 CPrevMovementState 覆盖上一帧的真实状态。
//!
//! 触发: was_grounded（上一帧 special=None）→ now not walkable 且非主动跳跃。
//! 仅 Falling 状态触发——KnockedBack/Terminal/Gliding 不触发。
//!
//! 参见: `WoWorld-Design/.../角色控制器/008-手感系统.md` §四

use woworld_core::kinematics::{base_locomotion, LocomotionMode};
use woworld_core::movement::{AirState, SpecialMode};
use woworld_core::spatial::TerrainQuery;

use crate::components::input_state::{CCoyoteTime, CInputFeelConfig};
use crate::components::movement_state::{CMovementState, CPrevMovementState};
use crate::components::transform::Position;

/// 默认土狼时间 (s)——实体无 CInputFeelConfig 时的回退值（M4 之前为硬编码）。
const DEFAULT_COYOTE_TIME: f32 = 0.15;

/// 土狼时间管理——在 MovementModeSystem 之前运行，读上一帧真实状态。
pub fn coyote_time_system(world: &mut hecs::World, dt: f32, terrain: &dyn TerrainQuery) {
    for (_, (coyote, move_state, prev_state, pos, feel)) in world.query_mut::<(
        &mut CCoyoteTime,
        &CMovementState,
        &CPrevMovementState,
        &Position,
        Option<&CInputFeelConfig>,
    )>() {
        // M4: 土狼时间从 CInputFeelConfig 读取（缺组件回退默认 0.15s）
        let coyote_time = feel
            .map(|f| f.coyote_time_secs)
            .unwrap_or(DEFAULT_COYOTE_TIME);
        let current_pos = pos.0;
        let current_loco = base_locomotion(current_pos, terrain);

        // ── 着地 → 归零 ──
        if current_loco == LocomotionMode::Grounded {
            coyote.remaining = 0.0;
            continue;
        }

        // ── 递减当前土狼时间 ──
        if coyote.remaining > 0.0 {
            coyote.remaining = (coyote.remaining - dt).max(0.0);
        }

        // ── 踩空触发: prev was ground voluntarily, now not walkable ──
        // 仅当上一帧在自愿地面（special=None）且当前不在地面时触发。
        // 排除主动跳跃、被击飞、滑翔、完全坠落。
        let prev_was_grounded = prev_state.0.special.is_none();

        let is_excluded_air_state = matches!(
            move_state.0.special,
            Some(SpecialMode::Airborne(
                AirState::Jumping { .. }
                    | AirState::KnockedBack { .. }
                    | AirState::Terminal
                    | AirState::Gliding
            ))
        );

        if prev_was_grounded
            && current_loco == LocomotionMode::PhysicsBody
            && !is_excluded_air_state
            && coyote.remaining <= 0.0
        // 尚未在窗口内
        {
            coyote.remaining = coyote_time;
            coyote.left_ground_at = current_pos;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;
    use woworld_core::material::SurfaceMaterial;
    use woworld_core::movement::{JumpHeight, MovementState, Pace, Stance};
    use woworld_core::types::{TerrainHit, WorldPos};

    struct TestTerrain {
        walkable: bool,
        height: f32,
    }

    impl TestTerrain {
        fn ground() -> Self {
            Self {
                walkable: true,
                height: 0.0,
            }
        }
        fn air() -> Self {
            Self {
                walkable: false,
                height: 0.0,
            }
        }
    }

    impl TerrainQuery for TestTerrain {
        fn height_at(&self, _pos: WorldPos) -> f32 {
            self.height
        }
        fn normal_at(&self, _pos: WorldPos) -> Vec3 {
            Vec3::Y
        }
        fn terrain_raycast(&self, _o: WorldPos, _d: Vec3, _m: f32) -> Option<TerrainHit> {
            None
        }
        fn density_at(&self, _pos: WorldPos) -> f32 {
            0.0
        }
        fn is_walkable(&self, _pos: WorldPos) -> bool {
            self.walkable
        }
        fn surface_material_at(&self, _pos: WorldPos) -> SurfaceMaterial {
            SurfaceMaterial::Grass
        }
        fn medium_at(&self, _pos: WorldPos) -> woworld_core::material::Medium {
            woworld_core::material::Medium::Air
        }
        fn light_level_at(&self, _pos: WorldPos) -> f32 {
            1.0
        }
        fn sample_horizon(&self, _pos: WorldPos, _dirs: &[Vec3]) -> Vec<f32> {
            vec![]
        }
    }

    #[test]
    fn test_coyote_trigger_on_walk_off_edge() {
        let mut world = hecs::World::new();
        let terrain = TestTerrain::air();

        world.spawn((
            CCoyoteTime::default(),
            CMovementState(MovementState::default()),
            CPrevMovementState(MovementState::default()), // was grounded
            Position(Vec3::ZERO),
        ));

        coyote_time_system(&mut world, 0.016, &terrain);

        for (_, (coyote, _, _, _)) in world.query_mut::<(
            &CCoyoteTime,
            &CMovementState,
            &CPrevMovementState,
            &Position,
        )>() {
            assert!(coyote.remaining > 0.0);
        }
    }

    #[test]
    fn test_coyote_not_trigger_on_voluntary_jump() {
        let mut world = hecs::World::new();
        let terrain = TestTerrain::air();

        world.spawn((
            CCoyoteTime::default(),
            CMovementState(MovementState {
                special: Some(SpecialMode::Airborne(AirState::Jumping {
                    control_ratio: 0.7,
                    height: JumpHeight::Normal,
                })),
                ..Default::default()
            }),
            CPrevMovementState(MovementState::default()),
            Position(Vec3::ZERO),
        ));

        coyote_time_system(&mut world, 0.016, &terrain);

        for (_, (coyote, _, _, _)) in world.query_mut::<(
            &CCoyoteTime,
            &CMovementState,
            &CPrevMovementState,
            &Position,
        )>() {
            assert_eq!(coyote.remaining, 0.0);
        }
    }

    #[test]
    fn test_coyote_not_trigger_on_knocked_back() {
        let mut world = hecs::World::new();
        let terrain = TestTerrain::air();

        world.spawn((
            CCoyoteTime::default(),
            CMovementState(MovementState {
                special: Some(SpecialMode::Airborne(AirState::KnockedBack {
                    recoverable_at_secs: 0.5,
                })),
                ..Default::default()
            }),
            CPrevMovementState(MovementState::default()),
            Position(Vec3::ZERO),
        ));

        coyote_time_system(&mut world, 0.016, &terrain);

        for (_, (coyote, _, _, _)) in world.query_mut::<(
            &CCoyoteTime,
            &CMovementState,
            &CPrevMovementState,
            &Position,
        )>() {
            assert_eq!(coyote.remaining, 0.0);
        }
    }

    #[test]
    fn test_coyote_resets_on_landing() {
        let mut world = hecs::World::new();
        let terrain = TestTerrain::ground();

        world.spawn((
            CCoyoteTime {
                remaining: 0.1,
                left_ground_at: Vec3::ZERO,
            },
            CMovementState::default(),
            CPrevMovementState(MovementState::default()),
            Position(Vec3::ZERO),
        ));

        coyote_time_system(&mut world, 0.016, &terrain);

        for (_, (coyote, _, _, _)) in world.query_mut::<(
            &CCoyoteTime,
            &CMovementState,
            &CPrevMovementState,
            &Position,
        )>() {
            assert_eq!(coyote.remaining, 0.0);
        }
    }

    #[test]
    fn test_coyote_counts_down() {
        let mut world = hecs::World::new();
        let terrain = TestTerrain::air();

        world.spawn((
            CCoyoteTime {
                remaining: 0.15,
                left_ground_at: Vec3::ZERO,
            },
            CMovementState::default(),
            CPrevMovementState(MovementState::default()),
            Position(Vec3::ZERO),
        ));

        coyote_time_system(&mut world, 0.05, &terrain);

        for (_, (coyote, _, _, _)) in world.query_mut::<(
            &CCoyoteTime,
            &CMovementState,
            &CPrevMovementState,
            &Position,
        )>() {
            assert!((coyote.remaining - 0.10).abs() < 0.01);
        }
    }

    #[test]
    fn test_coyote_uses_input_feel_config() {
        // ★ M4: 带 CInputFeelConfig 的实体用其 coyote_time_secs（0.3），而非硬编码 0.15
        use crate::components::input_state::CInputFeelConfig;
        let mut world = hecs::World::new();
        let terrain = TestTerrain::air();

        let e = world.spawn((
            CCoyoteTime::default(),
            CMovementState(MovementState::default()),
            CPrevMovementState(MovementState::default()), // was grounded
            Position(Vec3::ZERO),
            CInputFeelConfig {
                coyote_time_secs: 0.3,
                ..Default::default()
            },
        ));

        coyote_time_system(&mut world, 0.016, &terrain);

        let coyote = world.get::<&CCoyoteTime>(e).unwrap();
        assert!(
            (coyote.remaining - 0.3).abs() < 1e-6,
            "应用 CInputFeelConfig 的 0.3s，实得 {}",
            coyote.remaining
        );
    }
}
