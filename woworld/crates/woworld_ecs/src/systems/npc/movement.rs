//! NPC 地形感知移动 System — Phase 2 地形跟随 + 坡度减速
//!
//! 每帧驱动 NPC 位移：
//! - Goal.target_pos = Some(pos) → 朝目标直线移动
//! - Goal.target_pos = None       → 漫游（Wander 方向，定期重选）
//! - 到达判定：距离 < Movement.arrival_radius → 满足需求 + 移除 Goal + Wander
//! - ★ 地形感知：XZ 位移后 Y 跟随地形、坡度减速、陡坡不可行走
//!
//! Phase 3: 体素 A* 寻路 + 碰撞避开 + Velocity 组件接入

use glam::Vec3;
use hecs::CommandBuffer;
use woworld_core::spatial::TerrainQuery;
use woworld_core::types::WorldPos;

use crate::components::goal::{Goal, GoalType};
use crate::components::movement::{Movement, Wander};
use crate::components::needs::Needs;
use crate::components::transform::{Position, Rotation};
use crate::components::vitals::Corpse;
use crate::prng::pseudo_random_f32;

/// 漫游方向变更间隔 (s)
const WANDER_CHANGE_INTERVAL: f32 = 3.0;
/// 接近零的阈值——避免归一化零向量
const DIR_EPSILON: f32 = 0.001;
/// 最大可行走坡度（cos 值）——45° ≈ 0.707
const MAX_WALKABLE_SLOPE_COS: f32 = 0.707;
/// 坡度减速参考角——cos(15°) ≈ 0.966，比此更陡开始减速
const SLOPE_FULL_SPEED_COS: f32 = 0.966;

/// Goal 到达 → 对应需求恢复
///
/// Phase 1: 固定恢复量。Phase 2: 根据目标质量/消耗品效果动态计算。
fn satisfy_goal(goal: &Goal, needs: &mut Needs) {
    use crate::components::goal::GoalType;
    match goal.goal_type {
        GoalType::FindFood => needs.hunger = (needs.hunger - 0.5).max(0.0),
        GoalType::FindWater => needs.thirst = (needs.thirst - 0.5).max(0.0),
        GoalType::FindRest => needs.fatigue = (needs.fatigue - 0.6).max(0.0),
        GoalType::FindSafePlace => needs.safety = (needs.safety - 0.4).max(0.0),
        GoalType::FindSocialContact => needs.social = (needs.social - 0.3).max(0.0),
        GoalType::BalanceElements => needs.element_balance = (needs.element_balance - 0.3).max(0.0),
        GoalType::ExpressLibido => needs.libido = (needs.libido - 0.4).max(0.0),
        GoalType::Idle => {} // 空闲无需求恢复
    }
}

/// 地形查询辅助——将 glam Vec3 转为 WorldPos 后查高度
fn terrain_y_at(pos: Vec3, terrain: &dyn TerrainQuery) -> f32 {
    terrain.height_at(WorldPos {
        x: pos.x as f64,
        y: 0.0,
        z: pos.z as f64,
    })
}

/// 每帧驱动 NPC 位移（地形感知）
///
/// `dt`: 帧间隔 (s)
/// `tick`: 当前游戏 tick，用作随机种子的一部分
/// `terrain`: 地形查询——用于 Y 跟随和坡度计算
///
/// 调用者负责在返回后执行 `cmd.run_on(&mut world)`。
#[allow(clippy::needless_pass_by_ref_mut)] // cmd 签名一致性
pub fn movement_system(
    world: &mut hecs::World,
    cmd: &mut CommandBuffer,
    dt: f32,
    tick: u64,
    terrain: &dyn TerrainQuery,
) {
    // ⚠️ Without<&Corpse> 门控（审计 stopgap·2026-07-11）：death_watch 死亡时仅 remove Vitals
    //   + 插 Corpse，保留 Goal/Needs/Movement——不过滤则带目标的尸体会被继续驱动滑向目标点。
    //   009 控制器关闭层的 CDead 全局门控（13 System Without<CDead>）落地后取代此补丁。
    for (entity, (pos, mov, goal, needs, wander_opt, mut rot_opt)) in world
        .query_mut::<hecs::Without<
            (
                &mut Position,
                &Movement,
                &Goal,
                &mut Needs,
                Option<&mut Wander>,
                Option<&mut Rotation>,
            ),
            &Corpse,
        >>()
    {
        let current = pos.0;

        let (direction, new_wander) = if let Some(target) = goal.target_pos {
            // ── 有目标 → 朝目标直线移动 ──
            let to_target = target - current;
            // 忽略 Y 分量——地面移动在 XZ 平面
            let to_target_xz = Vec3::new(to_target.x, 0.0, to_target.z);
            let dist = to_target_xz.length();

            if dist < mov.arrival_radius {
                // ★ 写入朝向——到达后仍面向目标方向
                if let Some(ref mut rot) = rot_opt {
                    if dist < DIR_EPSILON {
                        rot.0 = glam::Quat::IDENTITY;
                    } else {
                        rot.0 = glam::Quat::from_rotation_arc(glam::Vec3::Z, to_target_xz / dist);
                    }
                }
                // ★ V3a: FindFood → 插入 ArrivedAtTarget 标记（harvest_on_arrival 消费）
                //   其他 GoalType → 保持原 satisfy_goal（Phase 3+ 逐一改造）
                if goal.goal_type == GoalType::FindFood {
                    use crate::components::goal::ArrivedAtTarget;
                    cmd.insert_one(
                        entity,
                        ArrivedAtTarget {
                            goal_type: goal.goal_type,
                            target_pos: target,
                        },
                    );
                } else {
                    satisfy_goal(goal, needs);
                }
                cmd.remove_one::<Goal>(entity);
                cmd.remove_one::<Wander>(entity);
                continue;
            }

            let dir = if dist > DIR_EPSILON {
                to_target_xz / dist
            } else {
                wander_direction(tick.wrapping_add(entity.to_bits().get()))
            };
            (dir, None)
        } else {
            // ── 无目标 → 漫游 ──
            match wander_opt {
                Some(w) => {
                    w.remaining -= dt;
                    if w.remaining <= 0.0 {
                        w.direction = wander_direction(tick.wrapping_add(entity.to_bits().get()));
                        w.remaining = WANDER_CHANGE_INTERVAL;
                    }
                    (w.direction, None)
                }
                None => {
                    let dir = wander_direction(tick.wrapping_add(entity.to_bits().get()));
                    let w = Wander {
                        direction: dir,
                        remaining: WANDER_CHANGE_INTERVAL,
                    };
                    (dir, Some(w))
                }
            }
        };

        // ★ 写入 Rotation——朝向移动方向
        if let Some(ref mut rot) = rot_opt {
            rot.0 = glam::Quat::from_rotation_arc(glam::Vec3::Z, direction);
        }

        // ── 计算新位置 + 地形感知 ──
        let new_xz = Vec3::new(
            current.x + direction.x * mov.speed * dt,
            0.0,
            current.z + direction.z * mov.speed * dt,
        );
        let new_y = terrain_y_at(new_xz, terrain);

        // 坡度计算：cos(θ) = 法线·上 ≈ normal.y
        let normal = terrain.normal_at(WorldPos {
            x: new_xz.x as f64,
            y: new_y as f64,
            z: new_xz.z as f64,
        });

        // 陡坡阻挡——太陡则原地不动
        if normal.y < MAX_WALKABLE_SLOPE_COS {
            // 保持当前位置，不移动
            continue;
        }

        // 坡度减速——15° 以下全速，15°-45° 逐渐减速
        let slope_factor = if normal.y >= SLOPE_FULL_SPEED_COS {
            1.0
        } else {
            (normal.y - MAX_WALKABLE_SLOPE_COS) / (SLOPE_FULL_SPEED_COS - MAX_WALKABLE_SLOPE_COS)
        };

        // 应用位移（含坡度减速）
        let actual_speed = mov.speed * slope_factor;
        let actual_xz = Vec3::new(
            current.x + direction.x * actual_speed * dt,
            0.0,
            current.z + direction.z * actual_speed * dt,
        );
        let actual_y = terrain_y_at(actual_xz, terrain);

        pos.0 = Vec3::new(actual_xz.x, actual_y, actual_xz.z);

        // 为新创建的 Wander 插入组件（不能通过 query_mut insert）
        if let Some(w) = new_wander {
            cmd.insert_one(entity, w);
        }
    }
}

/// 确定性漫游方向（XZ 平面单位向量）
///
/// seed 建议: `tick.wrapping_add(entity.to_bits())`
fn wander_direction(seed: u64) -> Vec3 {
    let angle = pseudo_random_f32(seed) * std::f32::consts::TAU;
    Vec3::new(angle.cos(), 0.0, angle.sin())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::goal::GoalType;
    use glam::Vec3;
    use woworld_core::material::SurfaceMaterial;
    use woworld_core::types::TerrainHit;

    /// Phase 1 测试用的平面地形
    struct FlatTerrain {
        height: f32,
    }

    impl FlatTerrain {
        fn at(height: f32) -> Self {
            Self { height }
        }
    }

    impl TerrainQuery for FlatTerrain {
        fn height_at(&self, _pos: WorldPos) -> f32 {
            self.height
        }
        fn normal_at(&self, _pos: WorldPos) -> Vec3 {
            Vec3::Y // 完全水平
        }
        fn terrain_raycast(
            &self,
            _origin: WorldPos,
            _direction: Vec3,
            _max_dist: f32,
        ) -> Option<TerrainHit> {
            None
        }
        fn density_at(&self, _pos: WorldPos) -> f32 {
            0.0
        }
        fn is_walkable(&self, _pos: WorldPos) -> bool {
            true
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
        fn sample_horizon(&self, _pos: WorldPos, _directions: &[Vec3]) -> Vec<f32> {
            vec![]
        }
    }

    /// 斜面地形——用于测试坡度减速
    struct SlopeTerrain {
        slope_cos: f32, // normal.y 值
    }

    impl TerrainQuery for SlopeTerrain {
        fn height_at(&self, pos: WorldPos) -> f32 {
            // 简单的斜面: y = base_y - (pos.x / 10.0) * sqrt(1 - cos²)
            let sin_theta = (1.0 - self.slope_cos * self.slope_cos).sqrt();
            (0.0 - pos.x as f32 / 10.0 * sin_theta) as f32
        }
        fn normal_at(&self, _pos: WorldPos) -> Vec3 {
            let sin_theta = (1.0 - self.slope_cos * self.slope_cos).sqrt();
            Vec3::new(sin_theta, self.slope_cos, 0.0).normalize()
        }
        fn terrain_raycast(
            &self,
            _origin: WorldPos,
            _direction: Vec3,
            _max_dist: f32,
        ) -> Option<TerrainHit> {
            None
        }
        fn density_at(&self, _pos: WorldPos) -> f32 {
            0.0
        }
        fn is_walkable(&self, _pos: WorldPos) -> bool {
            true
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
        fn sample_horizon(&self, _pos: WorldPos, _directions: &[Vec3]) -> Vec<f32> {
            vec![]
        }
    }

    /// 辅助: 创建带 Goal 的测试 NPC
    fn spawn_npc(world: &mut hecs::World, target: Option<Vec3>) -> hecs::Entity {
        world.spawn((
            Position(Vec3::ZERO),
            Movement::default(),
            Goal {
                goal_type: GoalType::FindFood,
                urgency: 0.9,
                target_pos: target,
            },
            Needs::default(),
        ))
    }

    // ── Phase 1: 基本移动（回归测试）──

    #[test]
    fn test_movement_toward_target() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let target = Vec3::new(10.0, 0.0, 0.0);
        let terrain = FlatTerrain::at(0.0);
        let e = spawn_npc(&mut world, Some(target));

        movement_system(&mut world, &mut cmd, 1.0, 0, &terrain);
        cmd.run_on(&mut world);

        let pos = world.get::<&Position>(e).unwrap();
        assert!(pos.0.x > 0.0);
        assert!(pos.0.x < 5.0);
        assert!((pos.0.z).abs() < 0.01);
    }

    #[test]
    fn test_movement_arrival_removes_goal() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let target = Vec3::new(0.1, 0.0, 0.0);
        let terrain = FlatTerrain::at(0.0);
        let e = spawn_npc(&mut world, Some(target));

        movement_system(&mut world, &mut cmd, 1.0, 0, &terrain);
        cmd.run_on(&mut world);

        assert!(
            world.get::<&Goal>(e).is_err(),
            "Goal should be removed on arrival"
        );
    }

    #[test]
    fn test_movement_wander_created() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let terrain = FlatTerrain::at(0.0);
        let e = spawn_npc(&mut world, None);

        movement_system(&mut world, &mut cmd, 1.0, 0, &terrain);
        cmd.run_on(&mut world);

        assert!(world.get::<&Wander>(e).is_ok(), "Wander should be created");
    }

    #[test]
    fn test_movement_wander_is_xz_plane() {
        for seed in 0..20 {
            let dir = wander_direction(seed);
            assert!(
                (dir.y).abs() < 0.001,
                "seed {seed}: direction should be in XZ plane"
            );
            assert!(
                (dir.length() - 1.0).abs() < 0.001,
                "seed {seed}: should be unit vector"
            );
        }
    }

    // ── Phase 2: 地形感知 ──

    #[test]
    fn test_terrain_y_follows_flat() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let terrain = FlatTerrain::at(5.0); // 地形在 y=5

        let e = world.spawn((
            Position(Vec3::new(0.0, 0.0, 0.0)), // 初始在地下
            Movement::default(),
            Goal {
                goal_type: GoalType::FindFood,
                urgency: 0.9,
                target_pos: Some(Vec3::new(10.0, 0.0, 0.0)),
            },
            Needs::default(),
        ));

        movement_system(&mut world, &mut cmd, 1.0, 0, &terrain);
        cmd.run_on(&mut world);

        let pos = world.get::<&Position>(e).unwrap();
        assert!(
            (pos.0.y - 5.0).abs() < 0.01,
            "NPC should be on terrain surface, got y={}",
            pos.0.y
        );
    }

    #[test]
    fn test_steep_slope_blocks_movement() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        // normal.y = 0.3 → cos(72.5°) → 极陡坡，不应行走
        let terrain = SlopeTerrain { slope_cos: 0.3 };

        let e = world.spawn((
            Position(Vec3::ZERO),
            Movement::default(),
            Goal {
                goal_type: GoalType::FindFood,
                urgency: 0.9,
                target_pos: Some(Vec3::new(10.0, 0.0, 0.0)),
            },
            Needs::default(),
        ));

        movement_system(&mut world, &mut cmd, 1.0, 0, &terrain);
        cmd.run_on(&mut world);

        let pos = world.get::<&Position>(e).unwrap();
        // 陡坡上不应移动（normal.y < MAX_WALKABLE_SLOPE_COS = 0.707）
        assert!((pos.0.x).abs() < 0.01, "should not move on steep slope");
    }

    #[test]
    fn test_gentle_slope_allows_movement() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        // normal.y = 0.95 → cos(18°) → 缓坡，应允许
        let terrain = SlopeTerrain { slope_cos: 0.95 };

        let e = world.spawn((
            Position(Vec3::ZERO),
            Movement::default(),
            Goal {
                goal_type: GoalType::FindFood,
                urgency: 0.9,
                target_pos: Some(Vec3::new(10.0, 0.0, 0.0)),
            },
            Needs::default(),
        ));

        movement_system(&mut world, &mut cmd, 1.0, 0, &terrain);
        cmd.run_on(&mut world);

        let pos = world.get::<&Position>(e).unwrap();
        assert!(pos.0.x > 0.0, "should move on gentle slope");
    }

    #[test]
    fn test_slope_slows_movement() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let terrain_flat = FlatTerrain::at(0.0);
        // 中等坡度: normal.y = 0.82 → 在减速区 (0.707-0.966)
        let terrain_slope = SlopeTerrain { slope_cos: 0.82 };

        // 平面上移动
        let e1 = world.spawn((
            Position(Vec3::ZERO),
            Movement::default(),
            Goal {
                goal_type: GoalType::FindFood,
                urgency: 0.9,
                target_pos: Some(Vec3::new(10.0, 0.0, 0.0)),
            },
            Needs::default(),
        ));
        movement_system(&mut world, &mut cmd, 1.0, 0, &terrain_flat);
        cmd.run_on(&mut world);
        let flat_dist = world.get::<&Position>(e1).unwrap().0.x;

        // 斜坡上移动（需要重新 spawn 因为位置已变）
        let mut world2 = hecs::World::new();
        let mut cmd2 = CommandBuffer::new();
        let e2 = world2.spawn((
            Position(Vec3::ZERO),
            Movement::default(),
            Goal {
                goal_type: GoalType::FindFood,
                urgency: 0.9,
                target_pos: Some(Vec3::new(10.0, 0.0, 0.0)),
            },
            Needs::default(),
        ));
        movement_system(&mut world2, &mut cmd2, 1.0, 0, &terrain_slope);
        cmd2.run_on(&mut world2);
        let slope_dist = world2.get::<&Position>(e2).unwrap().0.x;

        assert!(
            slope_dist < flat_dist,
            "slope ({slope_dist}) should be slower than flat ({flat_dist})"
        );
    }

    #[test]
    fn test_terrain_aware_wander() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let terrain = FlatTerrain::at(3.0);

        let e = world.spawn((
            Position(Vec3::new(0.0, 100.0, 0.0)), // 初始在空中
            Movement::default(),
            Goal {
                goal_type: GoalType::Idle,
                urgency: 0.0,
                target_pos: None, // 漫游
            },
            Needs::default(),
        ));

        movement_system(&mut world, &mut cmd, 1.0, 0, &terrain);
        cmd.run_on(&mut world);

        let pos = world.get::<&Position>(e).unwrap();
        assert!(
            (pos.0.y - 3.0).abs() < 0.01,
            "wandering NPC should snap to terrain, got y={}",
            pos.0.y
        );
    }

    #[test]
    fn test_arrival_ignores_y_difference() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let terrain = FlatTerrain::at(0.0);
        // 目标在 XZ 上很近但 Y 很高——应判定到达（忽略 Y）
        let target = Vec3::new(0.1, 50.0, 0.0);
        let e = spawn_npc(&mut world, Some(target));

        movement_system(&mut world, &mut cmd, 1.0, 0, &terrain);
        cmd.run_on(&mut world);

        assert!(
            world.get::<&Goal>(e).is_err(),
            "arrival should ignore Y difference"
        );
    }

    #[test]
    fn test_empty_world_no_panic() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let terrain = FlatTerrain::at(0.0);
        movement_system(&mut world, &mut cmd, 1.0, 0, &terrain);
    }

    #[test]
    fn test_wander_direction_deterministic() {
        let a = wander_direction(42);
        let b = wander_direction(42);
        assert!((a - b).length() < 0.001);
    }

    #[test]
    fn test_satisfy_goal_hunger() {
        let mut needs = Needs {
            hunger: 0.9,
            ..Needs::default()
        };
        let goal = Goal {
            goal_type: GoalType::FindFood,
            urgency: 0.9,
            target_pos: Some(Vec3::ZERO),
        };
        satisfy_goal(&goal, &mut needs);
        assert!((needs.hunger - 0.4).abs() < 0.01);
    }

    #[test]
    fn test_satisfy_goal_never_negative() {
        let mut needs = Needs {
            social: 0.1,
            ..Needs::default()
        };
        let goal = Goal {
            goal_type: GoalType::FindSocialContact,
            urgency: 0.5,
            target_pos: Some(Vec3::ZERO),
        };
        satisfy_goal(&goal, &mut needs);
        assert_eq!(needs.social, 0.0);
    }
}
