//! MovementModeSystem — 介质切换 + 恢复栈管理
//!
//! 检测介质变化（入水/出水/踩空/着地），自动修正 MovementState 和管理恢复栈。
//!
//! 参见: `WoWorld-Design/.../角色控制器/002-MovementState与连续移动.md` §二/§三

use glam::Vec3;
use woworld_core::kinematics::LocomotionMode;
use woworld_core::movement::{AirState, SpecialMode, SwimPace};
use woworld_core::spatial::TerrainQuery;
use woworld_core::types::WorldPos;

use crate::components::movement_state::{CMovementRecovery, CMovementState, CPrevMovementState};
use crate::components::transform::Position;

/// 计算给定位置的 LocomotionMode（Sprint 1 stub）。
/// 与 movement_system.rs 中的实现一致——CHG-067 会统一为 CLocomotionMode Component。
fn compute_locomotion_mode(pos: Vec3, terrain: &dyn TerrainQuery) -> LocomotionMode {
    let wp = WorldPos {
        x: pos.x as f64,
        y: pos.y as f64,
        z: pos.z as f64,
    };
    if terrain.is_walkable(wp) {
        LocomotionMode::Grounded
    } else {
        LocomotionMode::PhysicsBody
    }
}

/// 介质切换检测——每帧检测状态变化，自动修正 MovementState。
///
/// Query: `(CMovementState, CPrevMovementState, CMovementRecovery, Position)` + TerrainQuery.
pub fn movement_mode_system(world: &mut hecs::World, terrain: &dyn TerrainQuery) {
    for (_, (move_state, prev_state, recovery, pos)) in world.query_mut::<(
        &mut CMovementState,
        &mut CPrevMovementState,
        &mut CMovementRecovery,
        &Position,
    )>() {
        let current_pos = pos.0;
        let current_loco = compute_locomotion_mode(current_pos, terrain);
        let _prev_loco = compute_locomotion_mode(pos.0, terrain);

        // 保存上一帧状态（在修改 move_state 之前）
        let prev_ms = move_state.0;

        // Sprint 1: 用 MovementState.special 推断上一帧的 locomotion
        // prev_ms.special.is_none() → 上一帧在自愿地面 → prev_loco ≈ Grounded
        // prev_ms.special.is_some() → 上一帧在特殊状态 → prev_loco ≈ PhysicsBody
        let prev_was_grounded = prev_ms.special.is_none();
        let current_is_grounded = current_loco == LocomotionMode::Grounded;

        // ── 着地恢复: was PhysicsBody → now Grounded ──
        if !prev_was_grounded && current_is_grounded {
            let medium = terrain.medium_at(WorldPos {
                x: current_pos.x as f64,
                y: current_pos.y as f64,
                z: current_pos.z as f64,
            });
            let recovered = recovery.0.pop_compatible(current_loco, medium);
            move_state.0 = recovered;
        }

        // ── 踩空/入水: was Grounded → now PhysicsBody ──
        if prev_was_grounded && !current_is_grounded {
            let medium = terrain.medium_at(WorldPos {
                x: current_pos.x as f64,
                y: current_pos.y as f64,
                z: current_pos.z as f64,
            });

            // 保存当前自愿状态到恢复栈
            recovery.0.push_if_voluntary(prev_ms);

            // 判断介质类型
            use woworld_core::material::Medium;
            match medium {
                Medium::Water => {
                    move_state.0.special = Some(SpecialMode::Swimming(SwimPace::Slow));
                }
                _ => {
                    // 空中——踩空/坠落
                    move_state.0.special = Some(SpecialMode::Airborne(AirState::Falling {
                        coyote_time_remaining: 0.15,
                    }));
                }
            }
        }

        // ── 更新 CPrevMovementState ──
        prev_state.0 = move_state.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;
    use woworld_core::material::{Medium, SurfaceMaterial};
    use woworld_core::movement::{MovementState, Pace, Stance};
    use woworld_core::types::TerrainHit;

    struct FlatTerrain {
        height: f32,
        walkable: bool,
        medium: woworld_core::material::Medium,
    }

    impl FlatTerrain {
        fn ground() -> Self {
            Self {
                height: 0.0,
                walkable: true,
                medium: Medium::Air,
            }
        }

        fn water() -> Self {
            Self {
                height: 0.0,
                walkable: false,
                medium: Medium::Water,
            }
        }

        fn air_gap() -> Self {
            Self {
                height: 0.0,
                walkable: false,
                medium: Medium::Air,
            }
        }
    }

    impl TerrainQuery for FlatTerrain {
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
        fn medium_at(&self, _pos: WorldPos) -> Medium {
            self.medium
        }
        fn light_level_at(&self, _pos: WorldPos) -> f32 {
            1.0
        }
        fn sample_horizon(&self, _pos: WorldPos, _dirs: &[Vec3]) -> Vec<f32> {
            vec![]
        }
    }

    #[test]
    fn test_ground_stays_grounded() {
        let mut world = hecs::World::new();
        let terrain = FlatTerrain::ground();

        world.spawn((
            CMovementState(MovementState {
                stance: Stance::Standing,
                pace: Pace::Walking,
                ..Default::default()
            }),
            CPrevMovementState(MovementState {
                stance: Stance::Standing,
                pace: Pace::Walking,
                ..Default::default()
            }),
            CMovementRecovery::default(),
            Position(Vec3::ZERO),
        ));

        movement_mode_system(&mut world, &terrain);

        for (_, (ms, _prev, _rec, _pos)) in world.query_mut::<(
            &CMovementState,
            &CPrevMovementState,
            &CMovementRecovery,
            &Position,
        )>() {
            assert!(ms.0.special.is_none());
        }
    }

    #[test]
    fn test_walk_into_water_triggers_swim() {
        let mut world = hecs::World::new();
        let terrain = FlatTerrain::water(); // 不可行走 + 水介质

        world.spawn((
            CMovementState(MovementState {
                stance: Stance::Standing,
                pace: Pace::Walking,
                ..Default::default()
            }),
            CPrevMovementState(MovementState {
                stance: Stance::Standing,
                pace: Pace::Walking,
                ..Default::default()
            }),
            CMovementRecovery::default(),
            Position(Vec3::ZERO),
        ));

        movement_mode_system(&mut world, &terrain);

        for (_, (ms, _, _, _)) in world.query_mut::<(
            &CMovementState,
            &CPrevMovementState,
            &CMovementRecovery,
            &Position,
        )>() {
            match ms.0.special {
                Some(SpecialMode::Swimming(_)) => {} // 正确
                other => panic!("expected Swimming, got {:?}", other),
            }
        }
    }

    #[test]
    fn test_recovery_stack_push_on_fall() {
        let mut world = hecs::World::new();
        let terrain = FlatTerrain::air_gap(); // 不可行走 + 空气介质

        let recovery = CMovementRecovery::default();
        assert_eq!(recovery.0.len(), 0);

        world.spawn((
            CMovementState(MovementState {
                stance: Stance::Standing,
                pace: Pace::Sprinting,
                ..Default::default()
            }),
            CPrevMovementState(MovementState {
                stance: Stance::Standing,
                pace: Pace::Sprinting,
                ..Default::default()
            }),
            recovery,
            Position(Vec3::ZERO),
        ));

        movement_mode_system(&mut world, &terrain);

        for (_, (_, _, rec, _)) in world.query_mut::<(
            &CMovementState,
            &CPrevMovementState,
            &CMovementRecovery,
            &Position,
        )>() {
            assert_eq!(rec.0.len(), 1); // 踩空→自愿状态入栈
        }
    }

    #[test]
    fn test_water_to_ground_recovers() {
        let mut world = hecs::World::new();
        let terrain = FlatTerrain::ground(); // 可行走 + 空气

        // 在水中→走回地面
        let mut rec = CMovementRecovery::default();
        rec.0.push_if_voluntary(MovementState {
            stance: Stance::Standing,
            pace: Pace::Running,
            ..Default::default()
        });

        world.spawn((
            CMovementState(MovementState {
                special: Some(SpecialMode::Swimming(SwimPace::Slow)),
                ..Default::default()
            }),
            CPrevMovementState(MovementState {
                special: Some(SpecialMode::Swimming(SwimPace::Slow)),
                ..Default::default()
            }),
            rec,
            Position(Vec3::ZERO),
        ));

        movement_mode_system(&mut world, &terrain);

        for (_, (ms, _, _, _)) in world.query_mut::<(
            &CMovementState,
            &CPrevMovementState,
            &CMovementRecovery,
            &Position,
        )>() {
            assert!(ms.0.special.is_none());
            assert_eq!(ms.0.pace, Pace::Running); // 恢复自愿状态
        }
    }
}
