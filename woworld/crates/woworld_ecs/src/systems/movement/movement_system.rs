//! MovementSystem — 连续执行层核心
//!
//! 消费 CMoveIntent + CMovementState + CMovementControl → 产出 Position + Velocity。
//! 绞杀者模式: query 带 `Without<Movement>`——只处理新实体，不碰旧 NPC。
//!
//! 参见: `WoWorld-Design/.../角色控制器/001-角色控制器总纲.md` §四

use glam::Vec3;
use woworld_core::kinematics::{MovementLock, RotationLock};
use woworld_core::movement::{AirState, SpecialMode};
use woworld_core::spatial::TerrainQuery;
use woworld_core::types::WorldPos;

use crate::components::input_state::CInputFeelConfig;
use crate::components::movement_state::{CMoveIntent, CMovementControl, CMovementState};
use crate::components::transform::{Position, Velocity};
use crate::resources::movement_profile_registry::MovementProfileRegistry;

/// 最大可行走坡度（cos 值）——45° ≈ 0.707
const MAX_WALKABLE_SLOPE_COS: f32 = 0.707;
/// 趋近零的阈值
const DIR_EPSILON: f32 = 0.001;

/// 从 MovementLock 计算本帧允许的最大速度。
fn max_speed_from_lock(lock: MovementLock, base_max: f32) -> f32 {
    match lock {
        MovementLock::Free => base_max,
        MovementLock::Partial { speed_cap } => speed_cap.min(base_max),
        MovementLock::Full => 0.0,
        MovementLock::Override(_) => base_max, // Override 使用位移量而非速度
    }
}

/// 连续执行层——加速度积分 + 地形跟随 + 坡度检测。
///
/// 仅处理带 `CMovementState` 但无旧 `Movement` 组件的实体（绞杀者模式）。
#[allow(clippy::needless_pass_by_ref_mut)]
pub fn movement_system(
    world: &mut hecs::World,
    dt: f32,
    terrain: &dyn TerrainQuery,
    profiles: &MovementProfileRegistry,
) {
    let profile = profiles.default_profile();

    for (_, (move_intent, move_state, move_control, pos, vel, feel_opt)) in world
        .query_mut::<(
            &CMoveIntent,
            &CMovementState,
            &CMovementControl,
            &mut Position,
            &mut Velocity,
            Option<&CInputFeelConfig>,
        )>()
        .with::<&CMovementState>()
        .without::<&crate::components::movement::Movement>()
    // ★ 绞杀者门控
    {
        let current_pos = pos.0;
        let current_vel = vel.0;
        let ms = move_state.0;
        let lock = move_control.movement_lock;

        // ── SpecialMode::Airborne — 腾空垂直积分（重力）+ 空中控制，忽略 MovementLock ──
        //   设计 001 §三：PhysicsBody/Airborne 时 MovementState 被忽略，物理接管。
        //   置于 lock 判断之前——跳跃动作 movement_lock=Full，但腾空必须无视锁。
        //   002 §四：Jumping 有 control_ratio(0.7) 空中转向；水平动量无摩擦保留。
        //   落地由本系统贴地清垂直速度，special→Grounded 的恢复栈弹回下一帧交
        //   movement_mode_system（002 §二）。
        if let Some(SpecialMode::Airborne(air)) = ms.special {
            let mut v = current_vel;
            v.y -= profile.gravity * dt;

            // 空中控制：Jumping 按 control_ratio 朝输入方向加速（不超 jump_horizontal_speed，
            // 不施摩擦——保留起跳动量）。KnockedBack/Falling/Terminal 不可控。
            if let AirState::Jumping { control_ratio, .. } = air {
                let dir = move_intent.direction;
                if control_ratio > 0.0 && dir.length_squared() > DIR_EPSILON * DIR_EPSILON {
                    let d = dir.normalize_or_zero();
                    let vh = Vec3::new(v.x, 0.0, v.z);
                    if vh.dot(d) < profile.jump_horizontal_speed {
                        let air_accel = profile.ground_accel * control_ratio * dt;
                        v.x += d.x * air_accel;
                        v.z += d.z * air_accel;
                    }
                }
            }

            let mut new_pos = current_pos + v * dt;
            let terrain_y = terrain.height_at(WorldPos {
                x: new_pos.x as f64,
                y: 0.0,
                z: new_pos.z as f64,
            });
            if v.y <= 0.0 && new_pos.y <= terrain_y {
                // 落地：贴地 + 清垂直速度（水平保留）
                new_pos.y = terrain_y;
                v.y = 0.0;
            }
            pos.0 = new_pos;
            vel.0 = v;
            continue;
        }

        // ── MovementLock::Override — 强制位移 ──
        if let MovementLock::Override(displacement) = lock {
            let new_pos = current_pos + displacement * dt;
            let wp = WorldPos {
                x: new_pos.x as f64,
                y: new_pos.y as f64,
                z: new_pos.z as f64,
            };
            let terrain_y = terrain.height_at(wp);
            pos.0 = Vec3::new(new_pos.x, terrain_y.max(new_pos.y), new_pos.z);
            vel.0 = displacement;
            continue;
        }

        // ── MovementLock::Full — 原地摩擦减速 ──
        if lock == MovementLock::Full {
            let friction = ms.friction(profile);
            let new_speed = (current_vel.length() - friction * dt).max(0.0);
            if new_speed < DIR_EPSILON {
                vel.0 = Vec3::ZERO;
            } else {
                vel.0 = current_vel.normalize_or_zero() * new_speed;
            }
            // 更新 Y 以跟随地形
            let wp = WorldPos {
                x: current_pos.x as f64,
                y: 0.0,
                z: current_pos.z as f64,
            };
            pos.0.y = terrain.height_at(wp);
            continue;
        }

        // ── 正常/Partial — 加速度积分 ──
        let direction = move_intent.direction;
        if direction.length_squared() < DIR_EPSILON * DIR_EPSILON {
            // 无输入——摩擦力减速
            let friction = ms.friction(profile);
            let current_speed = current_vel.length();
            let new_speed = (current_speed - friction * dt).max(0.0);
            vel.0 = if current_speed > DIR_EPSILON {
                current_vel.normalize_or_zero() * new_speed
            } else {
                Vec3::ZERO
            };
        } else {
            let dir = direction.normalize_or_zero();
            let target_speed = max_speed_from_lock(lock, ms.max_speed(profile));
            let accel = ms.acceleration(profile);
            let friction = ms.friction(profile);

            // 当前速度在移动方向上的投影
            let current_speed_in_dir = current_vel.dot(dir).max(0.0);

            let new_speed = if current_speed_in_dir < target_speed {
                // 加速
                (current_speed_in_dir + accel * dt).min(target_speed)
            } else {
                // 减速到目标速度
                (current_speed_in_dir - friction * dt).max(target_speed)
            };

            vel.0 = dir * new_speed;
        }

        // ── XZ 位移 ──
        let new_xz = Vec3::new(
            current_pos.x + vel.0.x * dt,
            0.0,
            current_pos.z + vel.0.z * dt,
        );

        // ── 地形跟随 + 坡度检测 ──
        let wp = WorldPos {
            x: new_xz.x as f64,
            y: 0.0,
            z: new_xz.z as f64,
        };
        let terrain_y = terrain.height_at(wp);
        let normal = terrain.normal_at(wp);

        // 陡坡阻挡
        if normal.y < MAX_WALKABLE_SLOPE_COS {
            // 保持在当前位置
            let stay_wp = WorldPos {
                x: current_pos.x as f64,
                y: 0.0,
                z: current_pos.z as f64,
            };
            pos.0.y = terrain.height_at(stay_wp);
            continue;
        }

        pos.0 = Vec3::new(new_xz.x, terrain_y, new_xz.z);

        // ── 边缘吸附（008 §七，I4）——仅玩家（CInputFeelConfig 在场）手感辅助 ──
        //   在纯高度场下与 height_at 落点重合（几近无操作）；待体素碰撞移动接管后，
        //   raycast 会命中体素边缘几何，让"刚好踩在边缘"贴地而非悬浮。
        if let Some(feel) = feel_opt {
            if let Some(snapped_y) = apply_ledge_snap(
                pos.0,
                terrain,
                feel.ledge_snap_distance,
                feel.ledge_snap_angle_deg,
            ) {
                pos.0.y = snapped_y;
            }
        }
    }
}

/// 边缘吸附（008 §七）——角色刚好踩在边缘（≤ `snap_distance` 内向下有地面）时贴地。
///
/// 从 `pos + 0.1Y` 向下 raycast `snap_distance + 0.1`；命中且坡度角 ≤ `max_angle_deg`
/// → 返回吸附后的 Y（`hit.point.y`）。不处理真悬崖（> `snap_distance` 向下无地面 → `None`）。
///
/// 坡度角 = 命中法线与 +Y 的夹角。纯函数——调用点以 `CInputFeelConfig` 在场做玩家门控。
fn apply_ledge_snap(
    pos: Vec3,
    terrain: &dyn TerrainQuery,
    snap_distance: f32,
    max_angle_deg: f32,
) -> Option<f32> {
    let origin = WorldPos {
        x: pos.x as f64,
        y: (pos.y + 0.1) as f64,
        z: pos.z as f64,
    };
    let hit = terrain.terrain_raycast(origin, Vec3::NEG_Y, snap_distance + 0.1)?;
    let cos_angle = hit.normal.normalize_or_zero().dot(Vec3::Y).clamp(-1.0, 1.0);
    let angle_deg = cos_angle.acos().to_degrees();
    if angle_deg <= max_angle_deg {
        Some(hit.point.y as f32)
    } else {
        None
    }
}

// ── 未使用的 RotationLock 处理 ──
// Sprint 1: RotationLock 仅存储，MovementSystem 不处理旋转。
// 旋转由独立的朝向系统或 Godot 桥接层在后续 sprint 中实现。
#[allow(dead_code)]
fn _apply_rotation_lock(
    current_rot: glam::Quat,
    lock: RotationLock,
    _direction: Vec3,
    _camera: glam::Mat4,
) -> glam::Quat {
    match lock {
        RotationLock::Free => current_rot,
        RotationLock::Locked => current_rot,
        _ => current_rot, // Sprint 1 stub
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::movement::Movement;
    use woworld_core::material::{Medium, SurfaceMaterial};
    use woworld_core::movement::{AirState, JumpHeight, MovementState, SpecialMode};
    use woworld_core::types::TerrainHit;

    /// 平地 mock：高度 0，处处可行走。
    struct FlatGround;
    impl TerrainQuery for FlatGround {
        fn height_at(&self, _p: WorldPos) -> f32 {
            0.0
        }
        fn normal_at(&self, _p: WorldPos) -> Vec3 {
            Vec3::Y
        }
        fn terrain_raycast(&self, _o: WorldPos, _d: Vec3, _m: f32) -> Option<TerrainHit> {
            None
        }
        fn density_at(&self, _p: WorldPos) -> f32 {
            0.0
        }
        fn is_walkable(&self, _p: WorldPos) -> bool {
            true
        }
        fn surface_material_at(&self, _p: WorldPos) -> SurfaceMaterial {
            SurfaceMaterial::Grass
        }
        fn medium_at(&self, _p: WorldPos) -> Medium {
            Medium::Air
        }
        fn light_level_at(&self, _p: WorldPos) -> f32 {
            1.0
        }
        fn sample_horizon(&self, _p: WorldPos, _d: &[Vec3]) -> Vec<f32> {
            vec![]
        }
    }

    // ── 边缘吸附 mock（I4，008 §七）──
    //   hit=Some((y, normal)) → raycast 命中；None → 向下无地面（真悬崖）。
    struct LedgeTerrain {
        hit: Option<(f32, Vec3)>,
    }
    impl TerrainQuery for LedgeTerrain {
        fn height_at(&self, _p: WorldPos) -> f32 {
            0.0
        }
        fn normal_at(&self, _p: WorldPos) -> Vec3 {
            Vec3::Y
        }
        fn terrain_raycast(&self, o: WorldPos, _d: Vec3, _m: f32) -> Option<TerrainHit> {
            self.hit.map(|(y, n)| TerrainHit {
                point: WorldPos {
                    x: o.x,
                    y: y as f64,
                    z: o.z,
                },
                normal: n,
                material: SurfaceMaterial::Grass,
                distance: (o.y as f32 - y).abs(),
            })
        }
        fn density_at(&self, _p: WorldPos) -> f32 {
            0.0
        }
        fn is_walkable(&self, _p: WorldPos) -> bool {
            true
        }
        fn surface_material_at(&self, _p: WorldPos) -> SurfaceMaterial {
            SurfaceMaterial::Grass
        }
        fn medium_at(&self, _p: WorldPos) -> Medium {
            Medium::Air
        }
        fn light_level_at(&self, _p: WorldPos) -> f32 {
            1.0
        }
        fn sample_horizon(&self, _p: WorldPos, _d: &[Vec3]) -> Vec<f32> {
            vec![]
        }
    }

    /// 陡坡法线（约 60°）——y 分量小于 cos(45°)。
    fn steep_normal() -> Vec3 {
        Vec3::new(0.87, 0.5, 0.0).normalize()
    }

    #[test]
    fn test_ledge_snap_on_edge_snaps() {
        // 踩在边缘：0.2m 内有平地面 → 吸附到 hit.y
        let terrain = LedgeTerrain {
            hit: Some((0.5, Vec3::Y)),
        };
        let snapped = apply_ledge_snap(Vec3::new(0.0, 0.7, 0.0), &terrain, 0.3, 45.0);
        assert_eq!(snapped, Some(0.5));
    }

    #[test]
    fn test_ledge_snap_true_cliff_no_snap() {
        // 真悬崖：向下无地面（raycast None）→ 不吸附
        let terrain = LedgeTerrain { hit: None };
        let snapped = apply_ledge_snap(Vec3::new(0.0, 0.7, 0.0), &terrain, 0.3, 45.0);
        assert_eq!(snapped, None);
    }

    #[test]
    fn test_ledge_snap_steep_slope_no_snap() {
        // 命中但坡度 >45° → 不吸附
        let terrain = LedgeTerrain {
            hit: Some((0.5, steep_normal())),
        };
        let snapped = apply_ledge_snap(Vec3::new(0.0, 0.7, 0.0), &terrain, 0.3, 45.0);
        assert_eq!(snapped, None);
    }

    fn airborne_state() -> MovementState {
        MovementState {
            special: Some(SpecialMode::Airborne(AirState::Jumping {
                control_ratio: 0.0,
                height: JumpHeight::Normal,
            })),
            ..Default::default()
        }
    }

    #[test]
    fn test_airborne_rises_then_lands() {
        let mut world = hecs::World::new();
        let profiles = MovementProfileRegistry::new();
        let terrain = FlatGround;
        // 起跳：地面 y=0，上抛速度 +7
        let e = world.spawn((
            CMoveIntent::default(),
            CMovementState(airborne_state()),
            CMovementControl::default(),
            Position(Vec3::ZERO),
            Velocity(Vec3::new(0.0, 7.0, 0.0)),
        ));

        let dt = 1.0 / 60.0;
        let mut max_y: f32 = 0.0;
        // 跑 2 秒——足够完成起跳+落地
        for _ in 0..120 {
            movement_system(&mut world, dt, &terrain, &profiles);
            let y = world.get::<&Position>(e).unwrap().0.y;
            max_y = max_y.max(y);
        }

        let pos = world.get::<&Position>(e).unwrap();
        let vel = world.get::<&Velocity>(e).unwrap();
        assert!(max_y > 1.0, "应升到 >1m（实测峰值 {max_y:.2}）");
        assert!(
            (pos.0.y - 0.0).abs() < 0.2,
            "应落回地面 y≈0（实测 {:.2}）",
            pos.0.y
        );
        assert!(
            vel.0.y.abs() < 0.01,
            "落地垂直速度归零（实测 {:.2}）",
            vel.0.y
        );
    }

    #[test]
    fn test_airborne_preserves_horizontal_momentum() {
        let mut world = hecs::World::new();
        let profiles = MovementProfileRegistry::new();
        let terrain = FlatGround;
        // 带水平速度起跳——空中应保留水平动量（无摩擦）
        let e = world.spawn((
            CMoveIntent::default(),
            CMovementState(airborne_state()),
            CMovementControl::default(),
            Position(Vec3::ZERO),
            Velocity(Vec3::new(3.0, 7.0, 0.0)),
        ));

        let dt = 1.0 / 60.0;
        for _ in 0..6 {
            movement_system(&mut world, dt, &terrain, &profiles);
        }
        // 数帧后仍在空中，水平速度保持
        let vel = world.get::<&Velocity>(e).unwrap();
        assert!(
            (vel.0.x - 3.0).abs() < 0.01,
            "水平动量应保留（实测 {:.2}）",
            vel.0.x
        );
    }

    #[test]
    fn test_airborne_ignores_movement_lock_full() {
        // 腾空分支置于 lock 判断之前——jump 的 Full 锁不应冻结垂直运动
        let mut world = hecs::World::new();
        let profiles = MovementProfileRegistry::new();
        let terrain = FlatGround;
        let e = world.spawn((
            CMoveIntent::default(),
            CMovementState(airborne_state()),
            CMovementControl {
                movement_lock: MovementLock::Full,
                ..Default::default()
            },
            Position(Vec3::new(0.0, 5.0, 0.0)),
            Velocity(Vec3::new(0.0, 7.0, 0.0)),
        ));

        movement_system(&mut world, 1.0 / 60.0, &terrain, &profiles);
        let pos = world.get::<&Position>(e).unwrap();
        assert!(
            pos.0.y > 5.0,
            "Full 锁下腾空仍应上升（实测 {:.2}）",
            pos.0.y
        );
    }

    #[test]
    fn test_grounded_strangler_skips_legacy_movement() {
        // 带旧 Movement 组件的实体不被本系统处理
        let mut world = hecs::World::new();
        let profiles = MovementProfileRegistry::new();
        let terrain = FlatGround;
        let e = world.spawn((
            CMoveIntent::default(),
            CMovementState(airborne_state()),
            CMovementControl::default(),
            Position(Vec3::new(0.0, 5.0, 0.0)),
            Velocity(Vec3::new(0.0, 7.0, 0.0)),
            Movement::default(), // 旧组件 → 绞杀者跳过
        ));

        movement_system(&mut world, 1.0 / 60.0, &terrain, &profiles);
        let pos = world.get::<&Position>(e).unwrap();
        assert_eq!(pos.0.y, 5.0, "带旧 Movement 的实体不应被处理");
    }

    #[test]
    fn test_ledge_snap_wired_player_only() {
        // I4 + Q4 裁决：带 CInputFeelConfig 的玩家被边缘吸附；无此组件的 NPC 不吸附。
        // LedgeTerrain：height_at=0（正常落点）vs raycast 命中 y=0.5 → 可观测吸附差异。
        let mut world = hecs::World::new();
        let profiles = MovementProfileRegistry::new();
        let terrain = LedgeTerrain {
            hit: Some((0.5, Vec3::Y)),
        };
        let player = world.spawn((
            CMoveIntent::default(),
            CMovementState(MovementState::default()),
            CMovementControl::default(),
            Position(Vec3::ZERO),
            Velocity(Vec3::ZERO),
            CInputFeelConfig::default(),
        ));
        let npc = world.spawn((
            CMoveIntent::default(),
            CMovementState(MovementState::default()),
            CMovementControl::default(),
            Position(Vec3::ZERO),
            Velocity(Vec3::ZERO),
        ));
        movement_system(&mut world, 1.0 / 60.0, &terrain, &profiles);
        assert!(
            (world.get::<&Position>(player).unwrap().0.y - 0.5).abs() < 1e-6,
            "玩家（有 CInputFeelConfig）应被边缘吸附到 0.5"
        );
        assert!(
            (world.get::<&Position>(npc).unwrap().0.y - 0.0).abs() < 1e-6,
            "NPC（无 CInputFeelConfig）不吸附，停在 height_at=0.0"
        );
    }
}
