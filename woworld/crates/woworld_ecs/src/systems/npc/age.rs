//! 年龄推进 System — Phase 1 年龄跟踪 + 阶段切换
//!
//! 每决策周期推进 Age，跨越阶段边界时更新 LifeStage。
//! Phase 2: Gompertz 死亡判定 + 认知老化路径。

use hecs::CommandBuffer;

use crate::components::lifecycle::{Age, LifeStage};

/// 年龄推进——跨越阶段边界时触发 LifeStage 切换
///
/// `dt_days`: 自上次调用以来经过的游戏天数
///
/// 调用者负责在返回后执行 `cmd.run_on(&mut world)`。
pub fn age_system(world: &mut hecs::World, _cmd: &mut CommandBuffer, dt_days: f32) {
    for (_entity, (age, stage)) in world.query_mut::<(&mut Age, &mut LifeStage)>() {
        let old_ratio = age.age_ratio();
        age.age_days += dt_days;
        let new_ratio = age.age_ratio();

        let old_stage = LifeStage::from_age_ratio(old_ratio);
        let new_stage = LifeStage::from_age_ratio(new_ratio);

        // 仅跨越边界时触发切换（CommandBuffer 延迟执行，无借用冲突）
        if old_stage != new_stage {
            *stage = new_stage;
            // Phase 2: 发射 LifeEvent::StageTransition 事件
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_age_advances() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let a = Age::new(70.0, 20.0);
        let initial_stage = LifeStage::from_age_ratio(a.age_ratio());
        let e = world.spawn((a, initial_stage));

        age_system(&mut world, &mut cmd, 365.0); // +1 年
        cmd.run_on(&mut world);

        let age = world.get::<&Age>(e).unwrap();
        assert!(age.age_days > 20.0 * 360.0);
    }

    #[test]
    fn test_stage_transition_triggers() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        // 从 Adolescent (15-25%) 推进到 YoungAdult (25-45%)
        // 16 岁 / 80 年 = 0.20 → Adolescent，推进到 22 岁 = 0.275 → YoungAdult
        let a = Age::new(80.0, 16.0);
        let stage = LifeStage::from_age_ratio(a.age_ratio());
        assert_eq!(stage, LifeStage::Adolescent);
        let e = world.spawn((a, stage));

        age_system(&mut world, &mut cmd, 360.0 * 6.0); // +6 年 → 22/80 = 0.275
        cmd.run_on(&mut world);

        let new_stage = world.get::<&LifeStage>(e).unwrap();
        assert_eq!(*new_stage, LifeStage::YoungAdult);
    }

    #[test]
    fn test_stage_no_change_within_boundary() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        // 30 岁 / 80 年 = 0.375 → YoungAdult，推进到 31 岁 = 0.3875 → 仍在 YoungAdult
        let a = Age::new(80.0, 30.0);
        let stage = LifeStage::from_age_ratio(a.age_ratio());
        assert_eq!(stage, LifeStage::YoungAdult);
        let e = world.spawn((a, stage));

        age_system(&mut world, &mut cmd, 360.0); // +1 年
        cmd.run_on(&mut world);

        let new_stage = world.get::<&LifeStage>(e).unwrap();
        assert_eq!(*new_stage, LifeStage::YoungAdult, "should still be YoungAdult");
    }

    #[test]
    fn test_stage_transition_elder() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        // 74 岁 / 80 年 = 0.925 → MiddleAge，推进到 77 岁 = 0.9625 → Elder
        let a = Age::new(80.0, 74.0);
        let stage = LifeStage::from_age_ratio(a.age_ratio());
        assert_eq!(stage, LifeStage::MiddleAge);
        let e = world.spawn((a, stage));

        age_system(&mut world, &mut cmd, 360.0 * 4.0); // +4 年
        cmd.run_on(&mut world);

        let new_stage = world.get::<&LifeStage>(e).unwrap();
        assert_eq!(*new_stage, LifeStage::Elder);
    }

    #[test]
    fn test_empty_world_no_panic() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        age_system(&mut world, &mut cmd, 1.0);
    }
}
