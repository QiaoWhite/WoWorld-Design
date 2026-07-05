//! NPC 简单移动 System — Phase 1 线性移动（无寻路）
//!
//! 每帧驱动 NPC 位移：
//! - Goal.target_pos = Some(pos) → 朝目标直线移动
//! - Goal.target_pos = None       → 漫游（Wander 方向，定期重选）
//! - 到达判定：距离 < Movement.arrival_radius → 满足需求 + 移除 Goal + Wander
//!
//! Phase 2: 体素 A* 寻路 + 碰撞避开 + Velocity 组件接入

use glam::Vec3;
use hecs::CommandBuffer;

use crate::components::goal::Goal;
use crate::components::movement::{Movement, Wander};
use crate::components::needs::Needs;
use crate::components::transform::Position;
use crate::prng::pseudo_random_f32;

/// 漫游方向变更间隔 (s)
const WANDER_CHANGE_INTERVAL: f32 = 3.0;
/// 接近零的阈值——避免归一化零向量
const DIR_EPSILON: f32 = 0.001;

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

/// 每帧驱动 NPC 位移（线性移动）
///
/// `dt`: 帧间隔 (s)
/// `tick`: 当前游戏 tick，用作随机种子的一部分
///
/// 调用者负责在返回后执行 `cmd.run_on(&mut world)`。
#[allow(clippy::needless_pass_by_ref_mut)] // cmd 签名一致性
pub fn movement_system(world: &mut hecs::World, cmd: &mut CommandBuffer, dt: f32, tick: u64) {
    // 一次 query_mut 完成: Position 更新 + Wander 读/写 + Goal 到达判定
    // 含 Option<&mut Wander> —— 处理"无 Wander 时插入"的退化情况
    for (entity, (pos, mov, goal, needs, wander_opt)) in
        world.query_mut::<(&mut Position, &Movement, &Goal, &mut Needs, Option<&mut Wander>)>()
    {
        let current = pos.0;

        let (direction, new_wander) = if let Some(target) = goal.target_pos {
            // ── 有目标 → 朝目标直线移动 ──
            let to_target = target - current;
            let dist = to_target.length();

            if dist < mov.arrival_radius {
                // 到达 → 满足需求 + 移除 Goal + Wander
                satisfy_goal(goal, needs);
                cmd.remove_one::<Goal>(entity);
                cmd.remove_one::<Wander>(entity);
                continue;
            }

            let dir = if dist > DIR_EPSILON {
                to_target / dist
            } else {
                wander_direction(tick.wrapping_add(entity.to_bits().get()))
            };
            (dir, None) // None = 不更新 Wander
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

        // 应用位移
        pos.0 += direction * mov.speed * dt;

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

    #[test]
    fn test_movement_toward_target() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let target = Vec3::new(10.0, 0.0, 0.0);
        let e = spawn_npc(&mut world, Some(target));

        movement_system(&mut world, &mut cmd, 1.0, 0);
        cmd.run_on(&mut world);

        let pos = world.get::<&Position>(e).unwrap();
        // speed=3, dt=1, direction=(1,0,0) → pos ≈ (3, 0, 0)
        assert!(pos.0.x > 0.0);
        assert!(pos.0.x < 5.0);
        assert!((pos.0.z).abs() < 0.01);
    }

    #[test]
    fn test_movement_arrival_removes_goal() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        // 目标极近 → 应判定到达
        let target = Vec3::new(0.1, 0.0, 0.0);
        let e = spawn_npc(&mut world, Some(target));

        movement_system(&mut world, &mut cmd, 1.0, 0);
        cmd.run_on(&mut world);

        // Goal 应被移除
        assert!(world.get::<&Goal>(e).is_err(), "Goal should be removed on arrival");
    }

    #[test]
    fn test_movement_wander_created() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        // 无 target → 应创建 Wander
        let e = spawn_npc(&mut world, None);

        movement_system(&mut world, &mut cmd, 1.0, 0);
        cmd.run_on(&mut world);

        assert!(world.get::<&Wander>(e).is_ok(), "Wander should be created");
    }

    #[test]
    fn test_movement_wander_changes_direction() {
        let mut world = hecs::World::new();
        let e = world.spawn((
            Position(Vec3::ZERO),
            Movement::default(),
            Goal {
                goal_type: GoalType::FindFood,
                urgency: 0.9,
                target_pos: None,
            },
            Wander {
                direction: Vec3::X,
                remaining: 0.0, // 立即触发重选
            },
            Needs::default(),
        ));

        let mut cmd = CommandBuffer::new();
        movement_system(&mut world, &mut cmd, 0.5, 100);
        cmd.run_on(&mut world);

        let w = world.get::<&Wander>(e).unwrap();
        // 方向应已更新（remaining 重置为 3.0，新方向）
        assert!(w.remaining > 0.0);
    }

    #[test]
    fn test_movement_speed_respected() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let e = world.spawn((
            Position(Vec3::ZERO),
            Movement { speed: 5.0, arrival_radius: 0.5 },
            Goal {
                goal_type: GoalType::FindRest,
                urgency: 0.9,
                target_pos: Some(Vec3::new(100.0, 0.0, 0.0)),
            },
            Needs::default(),
        ));

        movement_system(&mut world, &mut cmd, 1.0, 0);
        cmd.run_on(&mut world);

        let pos = world.get::<&Position>(e).unwrap();
        // speed=5, dt=1, direction=(1,0,0) → pos.x ≈ 5.0
        assert!((pos.0.x - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_movement_wander_is_xz_plane() {
        for seed in 0..20 {
            let dir = wander_direction(seed);
            // Y 分量应为 0（地面移动）
            assert!((dir.y).abs() < 0.001, "seed {seed}: direction should be in XZ plane");
            // 应为单位向量
            assert!((dir.length() - 1.0).abs() < 0.001, "seed {seed}: should be unit vector");
        }
    }

    #[test]
    fn test_wander_direction_deterministic() {
        let a = wander_direction(42);
        let b = wander_direction(42);
        assert!((a - b).length() < 0.001);
    }

    #[test]
    fn test_empty_world_no_panic() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        movement_system(&mut world, &mut cmd, 1.0, 0);
    }

    #[test]
    fn test_satisfy_goal_hunger() {
        let mut needs = Needs { hunger: 0.9, ..Needs::default() };
        let goal = Goal { goal_type: GoalType::FindFood, urgency: 0.9, target_pos: Some(Vec3::ZERO) };
        satisfy_goal(&goal, &mut needs);
        assert!((needs.hunger - 0.4).abs() < 0.01, "hunger should drop by 0.5");
    }

    #[test]
    fn test_satisfy_goal_fatigue() {
        let mut needs = Needs { fatigue: 0.9, ..Needs::default() };
        let goal = Goal { goal_type: GoalType::FindRest, urgency: 0.9, target_pos: Some(Vec3::ZERO) };
        satisfy_goal(&goal, &mut needs);
        assert!((needs.fatigue - 0.3).abs() < 0.01, "fatigue should drop by 0.6");
    }

    #[test]
    fn test_satisfy_goal_never_negative() {
        let mut needs = Needs { social: 0.1, ..Needs::default() };
        let goal = Goal { goal_type: GoalType::FindSocialContact, urgency: 0.5, target_pos: Some(Vec3::ZERO) };
        satisfy_goal(&goal, &mut needs);
        assert_eq!(needs.social, 0.0, "should clamp at 0.0");
    }

    #[test]
    fn test_satisfy_goal_idle_no_change() {
        let mut needs = Needs::default();
        let goal = Goal { goal_type: GoalType::Idle, urgency: 0.0, target_pos: None };
        satisfy_goal(&goal, &mut needs);
        assert_eq!(needs.hunger, 0.0);
        assert_eq!(needs.fatigue, 0.0);
    }

    #[test]
    fn test_arrival_satisfies_needs() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let target = Vec3::new(0.1, 0.0, 0.0);
        let e = world.spawn((
            Position(Vec3::ZERO),
            Movement::default(),
            Goal { goal_type: GoalType::FindFood, urgency: 0.9, target_pos: Some(target) },
            Needs { hunger: 0.9, ..Needs::default() },
        ));

        movement_system(&mut world, &mut cmd, 1.0, 0);
        cmd.run_on(&mut world);

        let needs = world.get::<&Needs>(e).unwrap();
        assert!(needs.hunger < 0.9, "arrival should satisfy hunger");
        assert!(world.get::<&Goal>(e).is_err(), "Goal should be removed");
    }
}
