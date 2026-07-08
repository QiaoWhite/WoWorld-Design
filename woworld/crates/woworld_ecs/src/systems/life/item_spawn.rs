//! ItemSpawnSystem — 读取 Corpse + LootResult + Position，生成掉落物 Entity
//!
//! 查询所有同时有 Corpse + LootResult + Position 的 Entity。
//! 每物品 spawn 一个新 DroppedItem Entity。
//!
//! CommandBuffer: remove LootResult, spawn item entities

use hecs::CommandBuffer;

use crate::components::entity_kind::EntityKind;
use crate::components::item::Item;
use crate::components::transform::Position;
use crate::components::vitals::{Corpse, CorpseLooted, LootResult};

/// 每帧执行——将 LootResult 转换为实际的物品 Entity。
pub fn item_spawn_system(world: &hecs::World, cmd: &mut CommandBuffer) {
    for (entity, (_corpse, loot, pos)) in world.query::<(&Corpse, &LootResult, &Position)>().iter()
    {
        // 为每个掉落物 spawn 新 Entity
        for i in 0..loot.count as usize {
            if let Some(item_id) = loot.items[i] {
                // 掉落物 Entity：DroppedItem tag + Item component + Position（尸体旁微偏移）
                cmd.spawn((
                    EntityKind::DroppedItem,
                    Item {
                        item_def_id: item_id,
                    },
                    Position(glam::Vec3::new(
                        pos.0.x + random_offset(i),
                        pos.0.y,
                        pos.0.z + random_offset(i + 7),
                    )),
                ));
            }
        }

        // 移除 LootResult，标记已搜刮（CorpseDecay 的前提条件）
        cmd.remove_one::<LootResult>(entity);
        cmd.insert_one(entity, CorpseLooted);
    }
}

/// 简易位置偏移（避免所有物品堆在同一点）
fn random_offset(seed: usize) -> f32 {
    let s = seed as f32;
    ((s * 0.618_034) % 1.0 - 0.5) * 0.5 // 黄金比例散列, ±0.25m
}

#[cfg(test)]
mod tests {
    use super::*;
    use woworld_core::id::ItemDefId;

    #[test]
    fn test_item_spawn_creates_dropped_items() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        let corpse_entity = world.spawn((
            Corpse::default(),
            LootResult {
                items: [
                    Some(ItemDefId(1)),
                    Some(ItemDefId(2)),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                ],
                count: 2,
            },
            Position(glam::Vec3::new(10.0, 0.0, 10.0)),
            EntityKind::Creature,
        ));

        let entity_count_before = world.query::<&EntityKind>().iter().count();

        item_spawn_system(&world, &mut cmd);
        cmd.run_on(&mut world);

        // LootResult 被移除
        assert!(world.get::<&LootResult>(corpse_entity).is_err());

        // Corpse 保留
        assert!(world.get::<&Corpse>(corpse_entity).is_ok());

        // 新 Entity 被创建（+2 dropped items）
        let entity_count_after = world.query::<&EntityKind>().iter().count();
        assert_eq!(entity_count_after, entity_count_before + 2);

        // 验证新 Entity 有 Item component
        let mut item_count = 0;
        for (_, (kind, item)) in world.query::<(&EntityKind, &Item)>().iter() {
            if *kind == EntityKind::DroppedItem {
                assert_ne!(item.item_def_id, woworld_core::item::ITEM_DEF_ID_NONE);
                item_count += 1;
            }
        }
        assert_eq!(item_count, 2);
    }

    #[test]
    fn test_item_spawn_empty_loot_does_nothing() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        let e = world.spawn((
            Corpse::default(),
            LootResult::default(), // count = 0
            Position::default(),
        ));

        let count_before = world.query::<&EntityKind>().iter().count();

        item_spawn_system(&world, &mut cmd);
        cmd.run_on(&mut world);

        // LootResult 被移除（即使为空）
        assert!(world.get::<&LootResult>(e).is_err());
        // 没有新 Entity 被创建
        let count_after = world.query::<&EntityKind>().iter().count();
        assert_eq!(count_after, count_before);
    }
}
