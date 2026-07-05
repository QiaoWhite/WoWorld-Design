//! GoalResolutionSystem — Desire → Goal 转换
//!
//! 查询 Desire → 映射到具体 GoalType → cmd: remove Desire, insert Goal.
//! 无 Desire 的实体默认 Idle。

use hecs::CommandBuffer;

use crate::components::goal::{Goal, GoalType};
use crate::components::needs::{Desire, DesireKind};

/// Desire → GoalType 映射
fn goal_for_desire(kind: DesireKind) -> GoalType {
    match kind {
        DesireKind::Eat => GoalType::FindFood,
        DesireKind::Drink => GoalType::FindWater,
        DesireKind::Rest => GoalType::FindRest,
        DesireKind::SeekSafety => GoalType::FindSafePlace,
        DesireKind::Socialize => GoalType::FindSocialContact,
        DesireKind::BalanceElements => GoalType::BalanceElements,
        DesireKind::ExpressLibido => GoalType::ExpressLibido,
    }
}

/// 每帧执行——将 Desire 转换为可执行的 Goal
pub fn goal_resolution_system(world: &hecs::World, cmd: &mut CommandBuffer) {
    for (entity, desire) in world.query::<&Desire>().iter() {
        let goal = Goal {
            goal_type: goal_for_desire(desire.kind),
            urgency: desire.urgency,
            target_pos: None, // Phase 2: 知识 System 填充
        };

        cmd.remove_one::<Desire>(entity);
        cmd.insert_one(entity, goal);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hunger_desire_becomes_find_food() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        let e = world.spawn((Desire { kind: DesireKind::Eat, urgency: 0.9 },));

        goal_resolution_system(&world, &mut cmd);
        cmd.run_on(&mut world);

        let goal = world.get::<&Goal>(e).expect("should have Goal");
        assert_eq!(goal.goal_type, GoalType::FindFood);
        assert_eq!(goal.urgency, 0.9);
        // Desire 已被移除
        assert!(world.get::<&Desire>(e).is_err());
    }

    #[test]
    fn test_thirst_desire_becomes_find_water() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        let e = world.spawn((Desire { kind: DesireKind::Drink, urgency: 0.85 },));

        goal_resolution_system(&world, &mut cmd);
        cmd.run_on(&mut world);

        let goal = world.get::<&Goal>(e).expect("should have Goal");
        assert_eq!(goal.goal_type, GoalType::FindWater);
    }

    #[test]
    fn test_fatigue_desire_becomes_find_rest() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        let e = world.spawn((Desire { kind: DesireKind::Rest, urgency: 0.88 },));

        goal_resolution_system(&world, &mut cmd);
        cmd.run_on(&mut world);

        let goal = world.get::<&Goal>(e).expect("should have Goal");
        assert_eq!(goal.goal_type, GoalType::FindRest);
    }

    #[test]
    fn test_no_desire_no_goal() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        // 空 World——不 panic
        goal_resolution_system(&world, &mut cmd);
        cmd.run_on(&mut world);
    }
}
