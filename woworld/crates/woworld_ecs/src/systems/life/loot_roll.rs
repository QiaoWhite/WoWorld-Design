//! LootRollSystem — 读取 Corpse + PendingLoot + EntityKind，确定掉落物
//!
//! 查询所有同时有 Corpse + PendingLoot 的 Entity。
//! 读 EntityKind（或未来 SpeciesId）确定掉落表 → 加权随机 → LootResult。
//!
//! CommandBuffer: remove PendingLoot, insert LootResult

use hecs::CommandBuffer;

use crate::components::entity_kind::EntityKind;
use crate::components::vitals::{Corpse, LootResult, PendingLoot};

/// 掉落表条目——物品 + 权重
#[derive(Debug, Clone)]
pub struct LootEntry {
    pub item_id: woworld_core::id::ItemDefId,
    pub weight: f32,
}

/// 掉落表——一组可掉落物品
#[derive(Debug, Clone)]
pub struct LootTable {
    pub entries: Vec<LootEntry>,
}

impl LootTable {
    /// 按权重随机选择掉落物（简单实现——Phase 2 替换为 WeightedIndex）
    pub fn roll(&self, _seed: u64) -> Vec<woworld_core::id::ItemDefId> {
        // Phase 1 简化：返回所有物品（测试用）
        // Phase 2 替换为 rand::distributions::WeightedIndex
        self.entries.iter().map(|e| e.item_id).collect()
    }
}

/// 掉落表注册表——EntityKind → LootTable 映射
#[derive(Debug, Clone)]
pub struct LootTableRegistry {
    /// key = EntityKind discriminant (0-4), value = LootTable
    tables: [Option<LootTable>; 8],
}

impl LootTableRegistry {
    pub fn new() -> Self {
        Self {
            tables: Default::default(),
        }
    }

    pub fn set_table(&mut self, kind: EntityKind, table: LootTable) {
        self.tables[kind as usize] = Some(table);
    }

    pub fn get_table(&self, kind: EntityKind) -> Option<&LootTable> {
        self.tables.get(kind as usize).and_then(|t| t.as_ref())
    }
}

impl Default for LootTableRegistry {
    fn default() -> Self {
        let mut registry = Self::new();

        // 内置测试掉落表：Creature → 兽皮 + 兽骨 + 生肉
        // sub_category 编码见 assets/items/test_items.toml
        registry.set_table(EntityKind::Creature, LootTable {
            entries: vec![
                LootEntry {
                    item_id: woworld_core::id::ItemDefId::new(
                        woworld_core::item::ItemCategory::LeatherMat, 1, 0,
                    ),
                    weight: 1.0,
                }, // 兽皮
                LootEntry {
                    item_id: woworld_core::id::ItemDefId::new(
                        woworld_core::item::ItemCategory::LeatherMat, 2, 0,
                    ),
                    weight: 0.8,
                }, // 兽骨
                LootEntry {
                    item_id: woworld_core::id::ItemDefId::new(
                        woworld_core::item::ItemCategory::Food, 1, 0,
                    ),
                    weight: 0.5,
                }, // 生肉
            ],
        });

        // 内置测试掉落表：Plant → 纤维 + 种子
        registry.set_table(EntityKind::Plant, LootTable {
            entries: vec![
                LootEntry {
                    item_id: woworld_core::id::ItemDefId::new(
                        woworld_core::item::ItemCategory::FiberMat, 1, 0,
                    ),
                    weight: 1.0,
                }, // 植物纤维
                LootEntry {
                    item_id: woworld_core::id::ItemDefId::new(
                        woworld_core::item::ItemCategory::OrganicMat, 1, 0,
                    ),
                    weight: 0.6,
                }, // 种子
            ],
        });

        registry
    }
}

/// 每帧执行——为待掉落尸体确定掉落物。
pub fn loot_roll_system(
    world: &hecs::World,
    cmd: &mut CommandBuffer,
    registry: &LootTableRegistry,
) {
    for (entity, (_corpse, _pending_loot, kind)) in
        world.query::<(&Corpse, &PendingLoot, &EntityKind)>().iter()
    {
        let table = match registry.get_table(*kind) {
            Some(t) => t,
            None => {
                // 无对应掉落表——清理 PendingLoot 不放 LootResult（不掉落）
                cmd.remove_one::<PendingLoot>(entity);
                continue;
            }
        };

        let items = table.roll(0); // Phase 2: 使用 WorldClock tick 作为 seed
        let mut loot_result = LootResult::default();
        for (i, item_id) in items.iter().take(8).enumerate() {
            loot_result.items[i] = Some(*item_id);
            loot_result.count = (i + 1) as u8;
        }

        cmd.remove_one::<PendingLoot>(entity);
        cmd.insert_one(entity, loot_result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::vitals::Corpse;

    #[test]
    fn test_loot_roll_creature_drops_items() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let registry = LootTableRegistry::default();

        let e = world.spawn((
            Corpse::default(),
            PendingLoot,
            EntityKind::Creature,
        ));

        loot_roll_system(&world, &mut cmd, &registry);
        cmd.run_on(&mut world);

        // PendingLoot 被移除
        assert!(world.get::<&PendingLoot>(e).is_err());
        // LootResult 被装上
        let loot = world.get::<&LootResult>(e).expect("should have LootResult");
        assert!(loot.count > 0);
        assert!(loot.items[0].is_some());
    }

    #[test]
    fn test_loot_roll_unknown_kind_clears_pending() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let registry = LootTableRegistry::default();

        // BuildingComponent 没有注册掉落表
        let e = world.spawn((
            Corpse::default(),
            PendingLoot,
            EntityKind::BuildingComponent,
        ));

        loot_roll_system(&world, &mut cmd, &registry);
        cmd.run_on(&mut world);

        // PendingLoot 被移除
        assert!(world.get::<&PendingLoot>(e).is_err());
        // 但没有 LootResult（无对应掉落表）
        assert!(world.get::<&LootResult>(e).is_err());
    }

    #[test]
    fn test_loot_roll_ignores_no_pending_loot() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let registry = LootTableRegistry::default();

        // Corpse 但没有 PendingLoot
        let _e = world.spawn((Corpse::default(), EntityKind::Creature));

        loot_roll_system(&world, &mut cmd, &registry);
        cmd.run_on(&mut world);

        // 没有 Entity 被匹配——无变化
    }
}
