//! 食物消费系统——consume_system
//!
//! 饥饿 NPC 自动食用库存中的食物。
//! 对应设计: `Life::ingest_food()` (生命 004 §14.1)。
//! V3a 简化: 仅 hunger_restore + hp_restore；跳过 bottleneck/stamina/moisture/element_surplus。
//!
//! Phase 3: 实现完整 `ingest_food(vitals, race, consumable, portion)` 摄入管线。

use hecs::{CommandBuffer, World};

use crate::components::inventory::HasInventory;
use crate::components::needs::Needs;
use crate::components::vitals::Vitals;
use crate::resources::inventory_registry::InventoryRegistry;
use woworld_core::item::{ItemQuery, ItemTag};
use woworld_core::types::EntityId;

/// 饥饿阈值——Needs.hunger > 此值才触发自动进食。
/// 代码方向：0=满足, 1=极度缺乏。
const HUNGER_CONSUME_THRESHOLD: f32 = 0.5;

/// 食物消费系统。
///
/// 遍历有 Needs + Vitals + HasInventory 的实体，
/// hunger > 阈值 → 查找库存中 Edible+consumable 物品 → 消费 1 个 → 降 hunger + 回 HP。
///
/// 应放在 need_evaluation 之前执行——吃完再判断是否仍饿。
pub fn consume_system(
    world: &World,
    cmd: &mut CommandBuffer,
    registry: &mut InventoryRegistry,
    item_registry: &dyn ItemQuery,
) {
    for (entity, (needs, vitals)) in world
        .query::<(&Needs, &Vitals)>()
        .with::<&HasInventory>()
        .iter()
    {
        if needs.hunger <= HUNGER_CONSUME_THRESHOLD {
            continue; // 不饿——不浪费食物
        }

        let raw_bits: u64 = entity.to_bits().into();
        let eid = EntityId(raw_bits);

        // 查找库存中第一个可食用物品
        let holdings = registry.get_holdings(eid);
        let mut found: Option<(usize, f32, f32)> = None; // (slot_idx, hunger_restore, hp_restore)

        for (def_id, _qty) in &holdings {
            if let Some(props) = item_registry.get_properties(*def_id) {
                let is_edible = props.tags.iter().any(|t| matches!(t, ItemTag::Edible));
                if is_edible {
                    if let Some(consumable) = &props.consumable {
                        if consumable.is_consumable {
                            // 在库存槽位中定位此 def_id
                            if let Some(inv) = registry.get_inventory(eid) {
                                if let Some(slot_idx) =
                                    inv.slots().iter().position(|s| s.item_def_id == *def_id)
                                {
                                    found = Some((
                                        slot_idx,
                                        consumable.hunger_restore,
                                        consumable.hp_restore,
                                    ));
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }

        if let Some((slot_idx, hunger_restore, hp_restore)) = found {
            // 消费 1 个物品
            let _ = registry.remove_item(eid, slot_idx, 1);

            // 降 hunger（代码方向：0=满足, 1=缺乏）
            let new_hunger = (needs.hunger - hunger_restore).max(0.0);
            // 回 HP（不超过 max_hp）
            let new_hp = (vitals.hp + hp_restore).min(vitals.max_hp);

            // 通过 cmd 覆盖写入 Needs/Vitals
            cmd.insert_one(
                entity,
                Needs {
                    hunger: new_hunger,
                    ..*needs
                },
            );
            cmd.insert_one(
                entity,
                Vitals {
                    hp: new_hp,
                    ..*vitals
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::inventory::HasInventory;
    use crate::resources::inventory_registry::InventoryRegistry;
    use crate::resources::item_registry::ItemRegistry;
    use std::collections::BTreeMap;
    use woworld_core::id::ItemDefId;
    use woworld_core::item::{
        ConsumableEffect, ItemCategory, ItemProperties, ItemTag, Quality, Rarity,
    };

    /// Register a test food item with known hunger_restore=0.5 and hp_restore=10.0.
    fn register_test_food(reg: &mut ItemRegistry) -> ItemDefId {
        let food_id = ItemDefId::new(ItemCategory::Food, 99, 0);
        reg.register(ItemProperties {
            def_id: food_id,
            category: ItemCategory::Food,
            name: "test_food".into(),
            description: String::new(),
            weight_grams: 100,
            bulk_factor: 1.0,
            volume_liters: 0.2,
            base_quality: Quality::Standard,
            rarity: Rarity::Common,
            quality_range_min: Quality::Rough,
            quality_range_max: Quality::Perfect,
            stack_size: 10,
            base_value_copper: 10,
            max_durability: 0.0,
            durability_loss_per_use: 0.0,
            magic_capacity_ke: 0,
            tags: vec![ItemTag::Edible],
            mod_tags: BTreeMap::new(),
            min_skill: None,
            min_strength: None,
            required_body_part: None,
            element_affinity: None,
            placement: None,
            tool_tags: None,
            consumable: Some(ConsumableEffect {
                is_consumable: true,
                hunger_restore: 0.5,
                hp_restore: 10.0,
            }),
            audio_material: None,
            aesthetic_props: None,
        });
        food_id
    }

    #[test]
    fn test_consume_reduces_hunger_and_restores_hp() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut inv = InventoryRegistry::new();
        let mut item_reg = ItemRegistry::new();
        let food_id = register_test_food(&mut item_reg);

        let e = world.spawn((
            Needs {
                hunger: 0.8,
                ..Needs::default()
            },
            Vitals {
                hp: 30.0,
                max_hp: 100.0,
                ..Vitals::default()
            },
            HasInventory,
        ));
        let raw_bits: u64 = e.to_bits().into();
        let eid = EntityId(raw_bits);
        inv.init_inventory(eid, 30);

        let _ = inv.add_item(eid, food_id, 3, &item_reg);

        consume_system(&world, &mut cmd, &mut inv, &item_reg);
        cmd.run_on(&mut world);

        let needs = world.get::<&Needs>(e).unwrap();
        let vitals = world.get::<&Vitals>(e).unwrap();
        assert!(needs.hunger < 0.8, "hunger should decrease after eating");
        assert!(vitals.hp > 30.0, "hp should increase after eating");
        assert!(vitals.hp <= vitals.max_hp, "hp should not exceed max_hp");
    }

    #[test]
    fn test_consume_low_hunger_skips() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut inv = InventoryRegistry::new();
        let mut item_reg = ItemRegistry::new();
        let food_id = register_test_food(&mut item_reg);

        let e = world.spawn((
            Needs {
                hunger: 0.3,
                ..Needs::default()
            },
            Vitals::default(),
            HasInventory,
        ));
        let raw_bits: u64 = e.to_bits().into();
        let eid = EntityId(raw_bits);
        inv.init_inventory(eid, 30);

        let _ = inv.add_item(eid, food_id, 3, &item_reg);

        let holdings_before = inv.get_holdings(eid).len();
        consume_system(&world, &mut cmd, &mut inv, &item_reg);
        cmd.run_on(&mut world);

        let needs = world.get::<&Needs>(e).unwrap();
        assert!(
            (needs.hunger - 0.3).abs() < 0.001,
            "hunger should not change"
        );
        let holdings_after = inv.get_holdings(eid).len();
        assert_eq!(
            holdings_before, holdings_after,
            "no food should be consumed"
        );
    }

    #[test]
    fn test_consume_no_food_no_panic() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut inv = InventoryRegistry::new();
        let item_reg = ItemRegistry::new();

        let e = world.spawn((
            Needs {
                hunger: 0.9,
                ..Needs::default()
            },
            Vitals::default(),
            HasInventory,
        ));
        let raw_bits: u64 = e.to_bits().into();
        let eid = EntityId(raw_bits);
        inv.init_inventory(eid, 30);

        consume_system(&world, &mut cmd, &mut inv, &item_reg);
        cmd.run_on(&mut world);

        let needs = world.get::<&Needs>(e).unwrap();
        assert!((needs.hunger - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_consume_removes_item_from_inventory() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut inv = InventoryRegistry::new();
        let mut item_reg = ItemRegistry::new();
        let food_id = register_test_food(&mut item_reg);

        let e = world.spawn((
            Needs {
                hunger: 0.9,
                ..Needs::default()
            },
            Vitals::default(),
            HasInventory,
        ));
        let raw_bits: u64 = e.to_bits().into();
        let eid = EntityId(raw_bits);
        inv.init_inventory(eid, 30);

        let _ = inv.add_item(eid, food_id, 3, &item_reg);
        let count_before = inv.count_item(eid, food_id);
        assert!(count_before > 0);

        consume_system(&world, &mut cmd, &mut inv, &item_reg);
        cmd.run_on(&mut world);

        let count_after = inv.count_item(eid, food_id);
        assert_eq!(count_after, count_before - 1, "one item should be consumed");
    }

    #[test]
    fn test_consume_respects_max_hp() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut inv = InventoryRegistry::new();
        let mut item_reg = ItemRegistry::new();
        let food_id = register_test_food(&mut item_reg);

        // HP already at max
        let e = world.spawn((
            Needs {
                hunger: 0.9,
                ..Needs::default()
            },
            Vitals {
                hp: 100.0,
                max_hp: 100.0,
                ..Vitals::default()
            },
            HasInventory,
        ));
        let raw_bits: u64 = e.to_bits().into();
        let eid = EntityId(raw_bits);
        inv.init_inventory(eid, 30);

        let _ = inv.add_item(eid, food_id, 3, &item_reg);

        consume_system(&world, &mut cmd, &mut inv, &item_reg);
        cmd.run_on(&mut world);

        let vitals = world.get::<&Vitals>(e).unwrap();
        assert!(vitals.hp <= vitals.max_hp, "hp should not exceed max_hp");
    }

    #[test]
    fn test_empty_world_no_panic() {
        let world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut inv = InventoryRegistry::new();
        let item_reg = ItemRegistry::new();
        consume_system(&world, &mut cmd, &mut inv, &item_reg);
    }
}
