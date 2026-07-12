//! 到达采集系统——harvest_on_arrival_system
//!
//! movement_system 到达 FindFood 目标后插入 ArrivedAtTarget 标记，
//! 本系统读取标记 → 查询植被 → yield resolver → 入库 → 移除标记。
//!
//! 设计对应:
//! - 交互配方表 `yield.resolver = "plant_harvest"`（物品系统 007 §9.2）
//! - HARVEST 复合原子 = GRASP+CUT+SCOOP+STACK（NPC行动涌现 001 §2.4）
//!   V3a shortcut: 跳过物理原子管线，直接操作库存。
//!
//! Phase 3: 迁移到完整 TOML acquisition recipe + 复合原子执行。

use glam::Vec2;
use hecs::{CommandBuffer, World};

use crate::components::goal::{ArrivedAtTarget, GoalType};
use crate::components::inventory::HasInventory;
use crate::components::transform::Position;
use crate::resources::inventory_registry::InventoryRegistry;
use woworld_core::item::ItemQuery;
use woworld_core::types::EntityId;
use woworld_core::vegetation::VegetationProvider;

/// 到达采集系统。
///
/// Query 所有带 `ArrivedAtTarget { goal_type: FindFood }` 的实体，
/// 查询附近植被采集点 → yield resolver → 入库。
///
/// 无植被/无产物 → 仍移除标记（诚实涌现：走到了但没有可采集的）。
pub fn harvest_on_arrival_system(
    world: &World,
    cmd: &mut CommandBuffer,
    inventory: &mut InventoryRegistry,
    vegetation: Option<&dyn VegetationProvider>,
    item_registry: &dyn ItemQuery,
) {
    for (entity, (pos, marker)) in world.query::<(&Position, &ArrivedAtTarget)>().iter() {
        if marker.goal_type != GoalType::FindFood {
            // 非 FindFood 标记——不应出现（防御性跳过）
            cmd.remove_one::<ArrivedAtTarget>(entity);
            continue;
        }

        // 检查是否有库存（harvest 需要 HasInventory 标签）
        if world.get::<&HasInventory>(entity).is_err() {
            cmd.remove_one::<ArrivedAtTarget>(entity);
            continue;
        }

        let npc_xz = Vec2::new(pos.0.x, pos.0.z);
        let raw_bits: u64 = entity.to_bits().into();
        let eid = EntityId(raw_bits);

        // 查询 2m 内的采集物
        if let Some(veg) = vegetation {
            let harvestables = veg.query_harvestable(npc_xz, 2.0);
            if let Some(nearest) = harvestables.first() {
                if let Some(yield_result) = nearest.product_category.resolve_plant_yield() {
                    // V3a shortcut: 直接入库，跳过 HARVEST 复合原子
                    let _ = inventory.add_item(eid, yield_result.item_def_id, 1, item_registry);
                }
            }
        }

        cmd.remove_one::<ArrivedAtTarget>(entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::goal::GoalType;
    use crate::components::inventory::HasInventory;
    use crate::resources::inventory_registry::InventoryRegistry;
    use crate::resources::item_registry::ItemRegistry;
    use crate::systems::item::item_seed_system;
    use glam::Vec3;
    use woworld_core::vegetation::{
        HarvestableInfo, PlantCommunitySnapshot, ProductCategory, RegenState, TimberAvailability,
        TimberQuality,
    };

    /// Mock vegetation with harvestable at given position.
    struct MockVegetation {
        harvestables: Vec<HarvestableInfo>,
    }
    impl VegetationProvider for MockVegetation {
        fn query_harvestable(&self, _pos: Vec2, radius: f32) -> Vec<HarvestableInfo> {
            self.harvestables
                .iter()
                .filter(|h| {
                    let dx = h.position.x;
                    let dz = h.position.z;
                    (dx * dx + dz * dz).sqrt() <= radius
                })
                .cloned()
                .collect()
        }
        fn query_community(&self, _pos: Vec2, _radius: f32) -> PlantCommunitySnapshot {
            PlantCommunitySnapshot {
                dominant_species: vec![],
                companion_species: vec![],
                canopy_closure: 0.0,
                shannon_diversity: 0.0,
            }
        }
        fn canopy_closure(&self, _pos: Vec2) -> f32 {
            0.0
        }
        fn timber_availability(&self, _pos: Vec2) -> TimberAvailability {
            TimberAvailability {
                available: false,
                quality: TimberQuality::Softwood,
                abundance: 0.0,
                harvest_difficulty: 0.0,
                dominant_species: vec![],
            }
        }
        fn ground_cover(&self, _pos: Vec2) -> woworld_core::vegetation::GroundCoverMap {
            woworld_core::vegetation::GroundCoverMap::default()
        }
        fn fuel_load(&self, _pos: Vec2) -> f32 {
            0.0
        }
        fn root_interference(&self, _pos: glam::Vec3) -> f32 {
            0.0
        }
        fn set_scene_lod(&self, _lod: u8) {}
    }

    fn make_harvestable(x: f32, z: f32, category: ProductCategory) -> HarvestableInfo {
        HarvestableInfo {
            instance_id: 1,
            species_id: woworld_core::id::SpeciesId(1),
            position: Vec3::new(x, 0.0, z),
            product_category: category,
            yield_base: 1.0,
            season_optimal: true,
            regen_state: RegenState::Full,
        }
    }

    // ── Tests ──

    #[test]
    fn test_harvest_adds_food_to_inventory() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut inv = InventoryRegistry::new();
        let mut item_reg = ItemRegistry::new();
        item_seed_system(&mut item_reg);

        // Register the berry ItemDefId that resolve_plant_yield produces
        use std::collections::BTreeMap;
        use woworld_core::id::ItemDefId;
        use woworld_core::item::{
            ConsumableEffect, ItemCategory, ItemProperties, ItemTag, Quality, Rarity,
        };
        let berry_id = ItemDefId::new(ItemCategory::Food, 2, 0);
        item_reg.register(ItemProperties {
            def_id: berry_id,
            category: ItemCategory::Food,
            name: "raw_berry".into(),
            description: String::new(),
            weight_grams: 50,
            bulk_factor: 1.0,
            volume_liters: 0.1,
            base_quality: Quality::Standard,
            rarity: Rarity::Common,
            quality_range_min: Quality::Rough,
            quality_range_max: Quality::Perfect,
            stack_size: 20,
            base_value_copper: 5,
            max_durability: 0.0,
            durability_loss_per_use: 0.0,
            magic_capacity_ke: 0,
            tags: vec![ItemTag::Edible, ItemTag::Stackable],
            mod_tags: BTreeMap::new(),
            min_skill: None,
            min_strength: None,
            required_body_part: None,
            element_affinity: None,
            placement: None,
            tool_tags: None,
            consumable: Some(ConsumableEffect {
                is_consumable: true,
                hunger_restore: 0.35,
                hp_restore: 5.0,
            }),
            audio_material: None,
            aesthetic_props: None,
        });

        let veg = MockVegetation {
            harvestables: vec![make_harvestable(0.5, 0.0, ProductCategory::Berry)],
        };

        let e = world.spawn((
            Position(Vec3::ZERO),
            HasInventory,
            ArrivedAtTarget {
                goal_type: GoalType::FindFood,
                target_pos: Vec3::new(0.5, 0.0, 0.0),
            },
        ));
        let raw_bits: u64 = e.to_bits().into();
        let eid = EntityId(raw_bits);
        inv.init_inventory(eid, 30);

        harvest_on_arrival_system(&world, &mut cmd, &mut inv, Some(&veg), &item_reg);
        cmd.run_on(&mut world);

        // ArrivedAtTarget should be removed
        assert!(world.get::<&ArrivedAtTarget>(e).is_err());
        // Inventory should have the berry item
        let holdings = inv.get_holdings(eid);
        assert!(!holdings.is_empty(), "should have harvested food");
    }

    #[test]
    fn test_harvest_no_vegetation_removes_marker() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut inv = InventoryRegistry::new();
        let mut item_reg = ItemRegistry::new();
        item_seed_system(&mut item_reg);

        let e = world.spawn((
            Position(Vec3::ZERO),
            HasInventory,
            ArrivedAtTarget {
                goal_type: GoalType::FindFood,
                target_pos: Vec3::new(0.5, 0.0, 0.0),
            },
        ));
        let raw_bits: u64 = e.to_bits().into();
        let eid = EntityId(raw_bits);
        inv.init_inventory(eid, 30);

        // No vegetation provider
        harvest_on_arrival_system(
            &world,
            &mut cmd,
            &mut inv,
            None::<&dyn VegetationProvider>,
            &item_reg,
        );
        cmd.run_on(&mut world);

        // Marker removed, no panic
        assert!(world.get::<&ArrivedAtTarget>(e).is_err());
        // Inventory empty
        let holdings = inv.get_holdings(eid);
        assert!(holdings.is_empty());
    }

    #[test]
    fn test_harvest_no_inventory_tag_removes_marker() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut inv = InventoryRegistry::new();
        let mut item_reg = ItemRegistry::new();
        item_seed_system(&mut item_reg);

        let veg = MockVegetation {
            harvestables: vec![make_harvestable(0.5, 0.0, ProductCategory::Berry)],
        };

        // Spawn WITHOUT HasInventory
        let e = world.spawn((
            Position(Vec3::ZERO),
            ArrivedAtTarget {
                goal_type: GoalType::FindFood,
                target_pos: Vec3::new(0.5, 0.0, 0.0),
            },
        ));

        harvest_on_arrival_system(&world, &mut cmd, &mut inv, Some(&veg), &item_reg);
        cmd.run_on(&mut world);

        // Marker removed even without inventory
        assert!(world.get::<&ArrivedAtTarget>(e).is_err());
    }

    #[test]
    fn test_harvest_fiber_returns_none() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut inv = InventoryRegistry::new();
        let mut item_reg = ItemRegistry::new();
        item_seed_system(&mut item_reg);

        let veg = MockVegetation {
            harvestables: vec![make_harvestable(0.5, 0.0, ProductCategory::Fiber)],
        };

        let e = world.spawn((
            Position(Vec3::ZERO),
            HasInventory,
            ArrivedAtTarget {
                goal_type: GoalType::FindFood,
                target_pos: Vec3::new(0.5, 0.0, 0.0),
            },
        ));
        let raw_bits: u64 = e.to_bits().into();
        let eid = EntityId(raw_bits);
        inv.init_inventory(eid, 30);

        harvest_on_arrival_system(&world, &mut cmd, &mut inv, Some(&veg), &item_reg);
        cmd.run_on(&mut world);

        // Marker removed
        assert!(world.get::<&ArrivedAtTarget>(e).is_err());
        // No food added (Fiber = inedible)
        let holdings = inv.get_holdings(eid);
        assert!(holdings.is_empty());
    }

    #[test]
    fn test_harvest_multiple_npcs() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut inv = InventoryRegistry::new();
        let mut item_reg = ItemRegistry::new();
        item_seed_system(&mut item_reg);

        let veg = MockVegetation {
            harvestables: vec![
                make_harvestable(0.5, 0.0, ProductCategory::Berry),
                make_harvestable(1.0, 0.0, ProductCategory::Mushroom),
            ],
        };

        // Two NPCs at same spot, both arriving
        for i in 0..2 {
            let e = world.spawn((
                Position(Vec3::new(i as f32 * 0.1, 0.0, 0.0)),
                HasInventory,
                ArrivedAtTarget {
                    goal_type: GoalType::FindFood,
                    target_pos: Vec3::new(0.5, 0.0, 0.0),
                },
            ));
            let raw_bits: u64 = e.to_bits().into();
            let eid = EntityId(raw_bits);
            inv.init_inventory(eid, 30);
        }

        harvest_on_arrival_system(&world, &mut cmd, &mut inv, Some(&veg), &item_reg);
        cmd.run_on(&mut world);

        // Both should have harvested (V3a MVP: no depletion tracking)
        for (_, ()) in world.query::<()>().iter() {
            // Just verify no markers remain
        }
        assert_eq!(world.query::<&ArrivedAtTarget>().iter().count(), 0);
    }
}
