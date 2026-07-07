//! InventoryRegistry — 库存与装备数据的权威存储
//!
//! 存储在 WorldDriver 中（非 hecs World）。
//! ECS 侧使用 HasInventory/HasEquipment ZST 标签标记实体。
//!
//! 替代 EconomyRegistry.item_holdings 成为 NPC 物品持有量的唯一权威源。
//!
//! 参见: woworld_core::item::inventory / woworld_core::item::equipment

use std::collections::HashMap;

use woworld_core::id::ItemDefId;
use woworld_core::item::equipment::{CharacterEquipment, ContainerSet, SlotId};
use woworld_core::item::inventory::{InventoryError, PersonalInventory};
use woworld_core::item::inventory_tuning;
use woworld_core::item::ItemQuery;
use woworld_core::types::EntityId;

// ── InventoryRegistry ────────────────────────────────────

/// 库存与装备注册表——SoA 模式的权威数据存储。
#[derive(Debug, Default)]
pub struct InventoryRegistry {
    /// EntityId → PersonalInventory
    inventories: HashMap<EntityId, PersonalInventory>,
    /// EntityId → CharacterEquipment
    equipment: HashMap<EntityId, CharacterEquipment>,
    /// 容器 ItemDefId → 额外槽位数
    container_bonuses: HashMap<ItemDefId, u16>,
}

impl InventoryRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    // ── Inventory CRUD ─────────────────────────────────

    /// 初始化实体的库存（默认 30 槽位）。
    pub fn init_inventory(&mut self, entity: EntityId, initial_capacity: u16) {
        self.inventories
            .entry(entity)
            .or_insert_with(|| PersonalInventory::new(initial_capacity));
    }

    /// 获取库存只读引用。
    pub fn get_inventory(&self, entity: EntityId) -> Option<&PersonalInventory> {
        self.inventories.get(&entity)
    }

    /// 获取库存可变引用。
    pub fn get_inventory_mut(&mut self, entity: EntityId) -> Option<&mut PersonalInventory> {
        self.inventories.get_mut(&entity)
    }

    /// 实体是否有库存。
    pub fn has_inventory(&self, entity: EntityId) -> bool {
        self.inventories.contains_key(&entity)
    }

    /// 添加物品到实体库存（委托 PersonalInventory::add）。
    ///
    /// `max_weight_grams` 由调用方从 `BASE_MAX_WEIGHT_KG * strength * 1000` 计算传入。
    /// 默认使用 10kg。
    pub fn add_item(
        &mut self,
        entity: EntityId,
        def_id: ItemDefId,
        quantity: u32,
        item_registry: &dyn ItemQuery,
    ) -> Result<u32, InventoryError> {
        let props = match item_registry.get_properties(def_id) {
            Some(p) => p.clone(),
            None => return Ok(0),
        };

        // 默认 max_weight = 10kg = 10,000g（Phase 2：无 strength 属性）
        let max_weight_grams = (inventory_tuning::BASE_MAX_WEIGHT_KG * 1000.0) as u32;

        let inv = self
            .inventories
            .entry(entity)
            .or_insert_with(|| PersonalInventory::new(inventory_tuning::BASE_SLOTS));
        inv.add(def_id, quantity, &props, max_weight_grams)
    }

    /// 从实体库存的指定槽位移除物品。
    pub fn remove_item(
        &mut self,
        entity: EntityId,
        slot_idx: usize,
        quantity: u32,
    ) -> Result<u32, InventoryError> {
        match self.inventories.get_mut(&entity) {
            Some(inv) => inv.remove(slot_idx, quantity),
            None => Err(InventoryError::ItemNotFound),
        }
    }

    /// 查询实体持有的某物品总数（遍历所有槽位）。
    pub fn count_item(&self, entity: EntityId, def_id: ItemDefId) -> u32 {
        self.inventories
            .get(&entity)
            .map(|inv| inv.count_item(def_id))
            .unwrap_or(0)
    }

    /// 返回实体持有的所有 (ItemDefId, quantity) 摘要。
    pub fn get_holdings(&self, entity: EntityId) -> Vec<(ItemDefId, u32)> {
        self.inventories
            .get(&entity)
            .map(|inv| inv.get_holdings())
            .unwrap_or_default()
    }

    /// 确定性分配初始物品给 NPC。
    ///
    /// 1-3 种物品，各 1-5 个。种子确定性保证可复现。
    /// 从 EconomyRegistry.seed_npc_items 迁移而来。
    pub fn seed_npc_items(
        &mut self,
        entity: EntityId,
        item_pool: &[ItemDefId],
        seed: u64,
        item_registry: &dyn ItemQuery,
    ) {
        if item_pool.is_empty() {
            return;
        }

        // 确定性 seed → item count
        let hash = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        let count = ((hash % 3) + 1) as usize; // 1-3 种物品

        for i in 0..count {
            let item_hash = seed.wrapping_mul(7).wrapping_add(i as u64).wrapping_mul(11);
            let item_idx = (item_hash as usize) % item_pool.len();
            let qty_hash = seed.wrapping_mul(13).wrapping_add((i as u64).wrapping_mul(17));
            let quantity = ((qty_hash % 5) + 1) as u32; // 1-5 个

            let _ = self.add_item(entity, item_pool[item_idx], quantity, item_registry);
        }
    }

    // ── Equipment CRUD ─────────────────────────────────

    /// 初始化实体的装备（空装备）。
    pub fn init_equipment(&mut self, entity: EntityId) {
        self.equipment.entry(entity).or_default();
    }

    /// 获取装备只读引用。
    pub fn get_equipment(&self, entity: EntityId) -> Option<&CharacterEquipment> {
        self.equipment.get(&entity)
    }

    /// 获取装备可变引用。
    pub fn get_equipment_mut(&mut self, entity: EntityId) -> Option<&mut CharacterEquipment> {
        self.equipment.get_mut(&entity)
    }

    /// 实体是否有装备。
    pub fn has_equipment(&self, entity: EntityId) -> bool {
        self.equipment.contains_key(&entity)
    }

    /// 装备物品到指定槽位。
    ///
    /// 对齐设计 004 §四：
    /// 1. 若槽位非空 → 旧装备返回给调用方（放入库存）
    /// 2. 物品从库存消费（调用方负责）
    /// 3. 放入槽位
    ///
    /// 返回被替换的旧装备（如有）。
    pub fn equip_to_slot(
        &mut self,
        entity: EntityId,
        slot: SlotId,
        def_id: ItemDefId,
    ) -> Result<Option<ItemDefId>, InventoryError> {
        let eq = self.equipment.entry(entity).or_default();
        let mode = eq.mode;
        let old = eq.set_slot(slot, mode, Some(def_id));
        Ok(old)
    }

    /// 从装备槽位卸下物品（返回库存）。
    ///
    /// 返回被卸下的物品。
    pub fn unequip_from_slot(
        &mut self,
        entity: EntityId,
        slot: SlotId,
    ) -> Option<ItemDefId> {
        let eq = self.equipment.get_mut(&entity)?;
        let mode = eq.mode;
        eq.set_slot(slot, mode, None)
    }

    // ── Container bonuses ──────────────────────────────

    /// 注册容器物品的槽位加成。
    pub fn register_container_bonus(&mut self, container_def_id: ItemDefId, bonus_slots: u16) {
        self.container_bonuses.insert(container_def_id, bonus_slots);
    }

    /// 根据当前装备的容器重算库存容量。
    ///
    /// 遍历 ContainerSet，汇总所有已注册容器的 slot_bonus，
    /// 调整库存 slots Vec 长度。
    pub fn recalculate_capacity(&mut self, entity: EntityId) -> u16 {
        let base = inventory_tuning::BASE_SLOTS;
        let bonus = match self.equipment.get(&entity) {
            Some(eq) => self.compute_container_bonus(&eq.containers),
            None => 0,
        };
        let total = base.saturating_add(bonus);

        if let Some(inv) = self.inventories.get_mut(&entity) {
            inv.resize_slots(total);
        }

        total
    }

    /// 汇总 ContainerSet 中所有容器的槽位加成。
    fn compute_container_bonus(&self, containers: &ContainerSet) -> u16 {
        let mut bonus = 0u16;
        let slots: [Option<ItemDefId>; 6] = [
            containers.back,
            containers.shoulder,
            containers.waist_left,
            containers.waist_right,
            containers.hand_left,
            containers.hand_right,
        ];
        for def_id in slots.into_iter().flatten() {
            if let Some(b) = self.container_bonuses.get(&def_id) {
                bonus = bonus.saturating_add(*b);
            }
        }
        bonus
    }
}

// ── tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use woworld_core::item::inventory_tuning;
    use woworld_core::item::ItemProperties;
    use woworld_core::item::equipment::OutfitMode;

    fn item_reg_with_props(def_id: ItemDefId, weight_grams: u32, bulk_factor: f32, stack_size: u32) -> impl ItemQuery {
        use woworld_core::item::{ItemCategory, Quality, Rarity};
        struct StubQuery {
            props: ItemProperties,
        }
        impl ItemQuery for StubQuery {
            fn get_properties(&self, _id: ItemDefId) -> Option<&ItemProperties> {
                Some(&self.props)
            }
            fn get_stack_size(&self, _id: ItemDefId) -> Option<u32> {
                Some(self.props.stack_size)
            }
            fn get_category(&self, _id: ItemDefId) -> Option<woworld_core::item::ItemCategory> {
                Some(self.props.category)
            }
            fn get_base_value(&self, _id: ItemDefId) -> Option<u32> {
                Some(self.props.base_value_copper)
            }
            fn get_rarity(&self, _id: ItemDefId) -> Option<woworld_core::item::Rarity> {
                Some(self.props.rarity)
            }
            fn get_name(&self, _id: ItemDefId) -> Option<&str> {
                Some(&self.props.name)
            }
            fn all_def_ids(&self) -> &[ItemDefId] {
                &[]
            }
            fn def_count(&self) -> usize {
                0
            }
        }
        use std::collections::BTreeMap;
        StubQuery {
            props: ItemProperties {
                def_id,
                category: ItemCategory::Material,
                name: "test".into(),
                description: String::new(),
                weight_grams,
                bulk_factor,
                volume_liters: 0.0,
                base_quality: Quality::Standard,
                rarity: Rarity::Common,
                quality_range_min: Quality::Rough,
                quality_range_max: Quality::Perfect,
                stack_size,
                base_value_copper: 10,
                max_durability: 0.0,
                durability_loss_per_use: 0.0,
                magic_capacity_ke: 0,
                tags: vec![],
                mod_tags: BTreeMap::new(),
                min_skill: None,
                min_strength: None,
                required_body_part: None,
                element_affinity: None,
                placement: None,
                tool_tags: None,
                consumable: None,
                audio_material: None,
                aesthetic_props: None,
            },
        }
    }

    // ── Inventory tests ─────────────────────────────────

    #[test]
    fn test_new_empty() {
        let reg = InventoryRegistry::new();
        let e = EntityId(1);
        assert!(!reg.has_inventory(e));
        assert!(reg.get_inventory(e).is_none());
    }

    #[test]
    fn test_init_inventory() {
        let mut reg = InventoryRegistry::new();
        let e = EntityId(1);
        reg.init_inventory(e, inventory_tuning::BASE_SLOTS);
        assert!(reg.has_inventory(e));
        let inv = reg.get_inventory(e).unwrap();
        assert_eq!(inv.total_slots(), inventory_tuning::BASE_SLOTS);
    }

    #[test]
    fn test_add_item_creates_inventory_if_missing() {
        let mut reg = InventoryRegistry::new();
        let e = EntityId(1);
        let id = ItemDefId(1);
        let q = item_reg_with_props(id, 100, 1.0, 50);
        let added = reg.add_item(e, id, 10, &q).unwrap();
        assert!(added > 0);
        assert!(reg.has_inventory(e));
    }

    #[test]
    fn test_count_item() {
        let mut reg = InventoryRegistry::new();
        let e = EntityId(1);
        let id = ItemDefId(1);
        let q = item_reg_with_props(id, 100, 1.0, 50);
        reg.add_item(e, id, 20, &q).unwrap();
        assert_eq!(reg.count_item(e, id), 20);
        assert_eq!(reg.count_item(e, ItemDefId(999)), 0);
    }

    #[test]
    fn test_count_item_unknown_entity() {
        let reg = InventoryRegistry::new();
        assert_eq!(reg.count_item(EntityId(999), ItemDefId(1)), 0);
    }

    #[test]
    fn test_remove_item() {
        let mut reg = InventoryRegistry::new();
        let e = EntityId(1);
        let id = ItemDefId(1);
        let q = item_reg_with_props(id, 100, 1.0, 50);
        reg.add_item(e, id, 30, &q).unwrap();
        let removed = reg.remove_item(e, 0, 10).unwrap();
        assert_eq!(removed, 10);
        assert_eq!(reg.count_item(e, id), 20);
    }

    #[test]
    fn test_remove_item_unknown_entity() {
        let mut reg = InventoryRegistry::new();
        let err = reg.remove_item(EntityId(999), 0, 1).unwrap_err();
        assert_eq!(err, InventoryError::ItemNotFound);
    }

    #[test]
    fn test_get_holdings() {
        let mut reg = InventoryRegistry::new();
        let e = EntityId(1);
        let id1 = ItemDefId(1);
        let _id2 = ItemDefId(2);
        let q = item_reg_with_props(id1, 100, 1.0, 50);
        reg.add_item(e, id1, 15, &q).unwrap();
        let h = reg.get_holdings(e);
        assert!(!h.is_empty());
    }

    #[test]
    fn test_get_holdings_unknown_entity() {
        let reg = InventoryRegistry::new();
        assert!(reg.get_holdings(EntityId(999)).is_empty());
    }

    #[test]
    fn test_seed_npc_items() {
        let mut reg = InventoryRegistry::new();
        let e = EntityId(1);
        let pool = vec![ItemDefId(1), ItemDefId(2), ItemDefId(3)];
        let q = item_reg_with_props(ItemDefId(1), 100, 1.0, 99);
        reg.seed_npc_items(e, &pool, 42, &q);
        // 应该有 1-3 种物品被分配
        let h = reg.get_holdings(e);
        assert!(!h.is_empty());
        assert!(h.len() <= 3);
    }

    #[test]
    fn test_seed_npc_items_deterministic() {
        let pool = vec![ItemDefId(1), ItemDefId(2), ItemDefId(3)];
        let q = item_reg_with_props(ItemDefId(1), 100, 1.0, 99);

        let mut a = InventoryRegistry::new();
        a.seed_npc_items(EntityId(1), &pool, 42, &q);

        let mut b = InventoryRegistry::new();
        b.seed_npc_items(EntityId(1), &pool, 42, &q);

        assert_eq!(a.get_holdings(EntityId(1)), b.get_holdings(EntityId(1)));
    }

    #[test]
    fn test_seed_npc_items_empty_pool() {
        let mut reg = InventoryRegistry::new();
        let q = item_reg_with_props(ItemDefId(1), 100, 1.0, 99);
        reg.seed_npc_items(EntityId(1), &[], 42, &q);
        assert!(reg.get_holdings(EntityId(1)).is_empty());
    }

    // ── Equipment tests ─────────────────────────────────

    #[test]
    fn test_init_equipment() {
        let mut reg = InventoryRegistry::new();
        let e = EntityId(1);
        reg.init_equipment(e);
        assert!(reg.has_equipment(e));
        let eq = reg.get_equipment(e).unwrap();
        assert_eq!(eq.mode, OutfitMode::Combat);
    }

    #[test]
    fn test_equip_to_slot() {
        let mut reg = InventoryRegistry::new();
        let e = EntityId(1);
        reg.init_equipment(e);
        let sword = ItemDefId(100);
        let old = reg.equip_to_slot(e, SlotId::Mainhand, sword).unwrap();
        assert!(old.is_none());
        assert_eq!(
            reg.get_equipment(e).unwrap().combat.mainhand,
            Some(sword)
        );
    }

    #[test]
    fn test_equip_to_slot_swap() {
        let mut reg = InventoryRegistry::new();
        let e = EntityId(1);
        reg.init_equipment(e);
        let sword = ItemDefId(100);
        let axe = ItemDefId(200);

        reg.equip_to_slot(e, SlotId::Mainhand, sword).unwrap();
        let old = reg.equip_to_slot(e, SlotId::Mainhand, axe).unwrap();
        assert_eq!(old, Some(sword));
        assert_eq!(
            reg.get_equipment(e).unwrap().combat.mainhand,
            Some(axe)
        );
    }

    #[test]
    fn test_unequip_from_slot() {
        let mut reg = InventoryRegistry::new();
        let e = EntityId(1);
        reg.init_equipment(e);
        let sword = ItemDefId(100);
        reg.equip_to_slot(e, SlotId::Mainhand, sword).unwrap();
        let removed = reg.unequip_from_slot(e, SlotId::Mainhand);
        assert_eq!(removed, Some(sword));
        assert!(reg.get_equipment(e).unwrap().combat.mainhand.is_none());
    }

    #[test]
    fn test_unequip_unknown_entity() {
        let mut reg = InventoryRegistry::new();
        assert!(reg.unequip_from_slot(EntityId(999), SlotId::Mainhand).is_none());
    }

    // ── Container bonuses ──────────────────────────────

    #[test]
    fn test_register_container_bonus() {
        let mut reg = InventoryRegistry::new();
        let backpack_id = ItemDefId(400);
        reg.register_container_bonus(backpack_id, inventory_tuning::MEDIUM_BACKPACK);
    }

    #[test]
    fn test_recalculate_capacity_no_containers() {
        let mut reg = InventoryRegistry::new();
        let e = EntityId(1);
        reg.init_inventory(e, inventory_tuning::BASE_SLOTS);
        let cap = reg.recalculate_capacity(e);
        assert_eq!(cap, inventory_tuning::BASE_SLOTS);
    }

    #[test]
    fn test_recalculate_capacity_with_container() {
        let mut reg = InventoryRegistry::new();
        let e = EntityId(1);
        let backpack_id = ItemDefId(400);
        reg.register_container_bonus(backpack_id, inventory_tuning::SMALL_BACKPACK);
        reg.init_inventory(e, inventory_tuning::BASE_SLOTS);
        reg.init_equipment(e);
        reg.equip_to_slot(e, SlotId::ContainerBack, backpack_id).unwrap();

        let cap = reg.recalculate_capacity(e);
        assert_eq!(cap, inventory_tuning::BASE_SLOTS + inventory_tuning::SMALL_BACKPACK);
    }

    #[test]
    fn test_recalculate_capacity_unknown_entity() {
        let mut reg = InventoryRegistry::new();
        let cap = reg.recalculate_capacity(EntityId(999));
        assert_eq!(cap, inventory_tuning::BASE_SLOTS);
    }
}
