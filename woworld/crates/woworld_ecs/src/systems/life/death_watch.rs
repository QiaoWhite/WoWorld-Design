//! DeathWatchSystem — 检测 Vitals.hp <= 0，触发死亡 Component 拆装
//!
//! 查询所有有 Vitals 的 Entity。hp <= 0 时：
//! - 移除: Vitals
//! - 插入: Corpse + PendingLoot + DeathCause
//!
//! 不指定掉落表——DeathWatch 不需要知道"掉什么"。LootRoll 自己决定。

use hecs::CommandBuffer;

use crate::components::vitals::{Corpse, DeathCause, PendingLoot, Vitals};

/// 每帧执行——检测死亡，触发 Component 拆装。
///
/// `current_tick`: 当前 WorldClock 帧计数（用于 Corpse.death_tick）
pub fn death_watch_system(world: &hecs::World, cmd: &mut CommandBuffer, current_tick: u64) {
    for (entity, vitals) in world.query::<&Vitals>().iter() {
        if vitals.hp > 0.0 {
            continue;
        }

        // 构造死亡原因（Phase 1 默认衰老——Phase 2 战斗 System 设置精确死因）
        // ⚠️ DeathCause 当前无消费者——Sprint 042+ 感官/调查 System 接入后消费
        let death_cause = DeathCause::default();

        // 构造尸体
        let corpse = Corpse {
            death_tick: current_tick,
            corpse_temperature: vitals.body_temp, // 死亡瞬间体温 = 当前体温
        };

        // Component 拆装：活→死
        cmd.remove_one::<Vitals>(entity);
        cmd.insert_one(entity, corpse);
        cmd.insert_one(entity, PendingLoot);
        cmd.insert_one(entity, death_cause);

        // 如果实体有 EntityKind，保留它——LootRoll 需要
        // （EntityKind 不受影响，因为我们只 remove Vitals）
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::entity_kind::EntityKind;

    #[test]
    fn test_death_watch_kills_zero_hp() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        let e = world.spawn((Vitals {
            hp: 0.0,
            ..Vitals::default()
        },));

        death_watch_system(&world, &mut cmd, 100);
        cmd.run_on(&mut world);

        // Vitals 被移除
        assert!(world.get::<&Vitals>(e).is_err());
        // Corpse 被装上
        let corpse = world.get::<&Corpse>(e).expect("should have Corpse");
        assert_eq!(corpse.death_tick, 100);
        // PendingLoot 被装上
        assert!(world.get::<&PendingLoot>(e).is_ok());
        // DeathCause 被装上
        assert!(world.get::<&DeathCause>(e).is_ok());
    }

    #[test]
    fn test_death_watch_ignores_alive() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        let e = world.spawn((Vitals::default(),)); // hp=100

        death_watch_system(&world, &mut cmd, 100);
        cmd.run_on(&mut world);

        // Vitals 仍在
        assert!(world.get::<&Vitals>(e).is_ok());
        // Corpse 未装上
        assert!(world.get::<&Corpse>(e).is_err());
    }

    #[test]
    fn test_death_watch_preserves_entity_kind() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        let e = world.spawn((Vitals {
            hp: 0.0,
            ..Vitals::default()
        }, EntityKind::Creature));

        death_watch_system(&world, &mut cmd, 42);
        cmd.run_on(&mut world);

        // EntityKind 保留（DeathWatch 不碰它）
        let kind = world.get::<&EntityKind>(e).expect("EntityKind should persist");
        assert_eq!(*kind, EntityKind::Creature);
    }

    #[test]
    fn test_death_watch_uses_body_temp_for_corpse() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        let e = world.spawn((Vitals {
            hp: 0.0,
            body_temp: 25.0, // 低温症死亡
            ..Vitals::default()
        },));

        death_watch_system(&world, &mut cmd, 100);
        cmd.run_on(&mut world);

        let corpse = world.get::<&Corpse>(e).expect("should have Corpse");
        assert_eq!(corpse.corpse_temperature, 25.0);
    }
}
