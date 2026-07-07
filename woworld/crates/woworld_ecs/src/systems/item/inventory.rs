//! 库存初始化系统
//!
//! 为新 NPC（有 BigFive 或 Creature 实体）创建默认库存 + 空装备。
//! 遵循 `wallet_init_system` 模式：一次性 + 幂等 + CommandBuffer。
//!
//! 同时负责 NPC 初始物品播种（从 EconomyRegistry.seed_npc_items 迁移）。

use hecs::{CommandBuffer, World};

use crate::components::entity_kind::EntityKind;
use crate::components::inventory::{HasEquipment, HasInventory};
use crate::resources::inventory_registry::InventoryRegistry;
use crate::resources::item_registry::ItemRegistry;
use woworld_core::item::inventory_tuning;
use woworld_core::item::ItemQuery;
use woworld_core::types::EntityId;

/// 库存初始化系统。
///
/// 扫描世界中所有有 BigFive 或 Creature 的实体，
/// 若尚未在 InventoryRegistry 中有库存/装备，创建默认 30 槽库存 + 空装备，
/// 并添加 `HasInventory` + `HasEquipment` 标签。
///
/// 同时播种初始物品（确定性，1-3 种物品各 1-5 个）。
pub fn inventory_init_system(
    world: &World,
    cmd: &mut CommandBuffer,
    registry: &mut InventoryRegistry,
    item_registry: &ItemRegistry,
) {
    // 收集所有 NPC/Creature 实体
    let mut entities: Vec<hecs::Entity> = Vec::new();

    // 有 BigFive 的实体 → NPC
    for (entity, ()) in world.query::<()>().with::<&crate::components::bigfive::BigFive>().iter() {
        entities.push(entity);
    }

    // Creature 实体
    for (entity, (kind,)) in world.query::<(&EntityKind,)>().iter() {
        if matches!(kind, EntityKind::Creature) {
            entities.push(entity);
        }
    }

    for entity in entities {
        let raw_bits: u64 = entity.to_bits().into();
        let eid = EntityId(raw_bits);

        let needs_inventory = !registry.has_inventory(eid);
        let needs_equipment = !registry.has_equipment(eid);

        if needs_inventory {
            registry.init_inventory(eid, inventory_tuning::BASE_SLOTS);
            cmd.insert_one(entity, HasInventory);
        }

        if needs_equipment {
            registry.init_equipment(eid);
            cmd.insert_one(entity, HasEquipment);
        }

        // 播种初始物品（仅在首次初始化库存时）
        if needs_inventory && !item_registry.is_empty() {
            let pool = item_registry.all_def_ids().to_vec();
            let seed = raw_bits;
            registry.seed_npc_items(eid, &pool, seed, item_registry);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::bigfive::BigFive;

    #[test]
    fn test_init_creates_inventory_and_equipment() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut reg = InventoryRegistry::new();
        let item_reg = ItemRegistry::new();

        let entity = world.spawn((BigFive::default(),));

        inventory_init_system(&world, &mut cmd, &mut reg, &item_reg);
        cmd.run_on(&mut world);

        let eid = EntityId(entity.to_bits().into());
        assert!(reg.has_inventory(eid));
        assert!(reg.has_equipment(eid));
        assert!(world.get::<&HasInventory>(entity).is_ok());
        assert!(world.get::<&HasEquipment>(entity).is_ok());
    }

    #[test]
    fn test_init_idempotent() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut reg = InventoryRegistry::new();
        let item_reg = ItemRegistry::new();

        let entity = world.spawn((BigFive::default(),));

        // 第一次
        inventory_init_system(&world, &mut cmd, &mut reg, &item_reg);
        cmd.run_on(&mut world);

        let eid = EntityId(entity.to_bits().into());
        assert!(reg.has_inventory(eid));

        // 第二次——不会重复创建
        let mut cmd2 = CommandBuffer::new();
        inventory_init_system(&world, &mut cmd2, &mut reg, &item_reg);
        cmd2.run_on(&mut world);

        // 仍然只有一个库存条目
        assert!(reg.has_inventory(eid));
    }

    #[test]
    fn test_init_skips_already_tagged() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut reg = InventoryRegistry::new();
        let item_reg = ItemRegistry::new();

        // 先手动创建库存
        let entity = world.spawn((BigFive::default(),));
        let eid = EntityId(entity.to_bits().into());
        reg.init_inventory(eid, 30);

        inventory_init_system(&world, &mut cmd, &mut reg, &item_reg);
        cmd.run_on(&mut world);

        // 库存容量不变
        let inv = reg.get_inventory(eid).unwrap();
        assert_eq!(inv.total_slots(), 30);
    }

    #[test]
    fn test_init_empty_world_no_panic() {
        let world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut reg = InventoryRegistry::new();
        let item_reg = ItemRegistry::new();
        inventory_init_system(&world, &mut cmd, &mut reg, &item_reg);
    }

    #[test]
    fn test_init_creature_entity() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut reg = InventoryRegistry::new();
        let item_reg = ItemRegistry::new();

        let entity = world.spawn((EntityKind::Creature,));

        inventory_init_system(&world, &mut cmd, &mut reg, &item_reg);
        cmd.run_on(&mut world);

        let eid = EntityId(entity.to_bits().into());
        assert!(reg.has_inventory(eid));
    }

    #[test]
    fn test_init_adds_has_inventory_tag() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut reg = InventoryRegistry::new();
        let item_reg = ItemRegistry::new();

        let entity = world.spawn((BigFive::default(),));

        inventory_init_system(&world, &mut cmd, &mut reg, &item_reg);
        cmd.run_on(&mut world);

        assert!(world.get::<&HasInventory>(entity).is_ok());
    }
}
