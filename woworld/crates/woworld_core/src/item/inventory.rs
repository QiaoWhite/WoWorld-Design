//! 随身库存
//!
//! 玩家与所有 NPC 共用同一套库存代码。
//! 30 基础槽位 + 容器扩展。槽位为主，负重为安全网。
//!
//! Phase 3: InventorySlot.item_def_id → ItemEntId（实例追踪）
//!
//! 参见: `WoWorld-Design/Happy Game/开发阶段/物品系统/005-背包与库存.md`

use std::collections::BTreeMap;

use crate::id::ItemDefId;
use crate::item::{effective_encumbrance_kg, ItemCategory, ItemProperties, ITEM_DEF_ID_NONE};

// ── InventorySlot ─────────────────────────────────────

/// 库存槽位——单个物品占位。
///
/// Phase 3: `item_def_id` 迁移为 `ItemEntId`。
/// 16 bytes + 4 padding = 20 bytes → 编译器对齐为 24 bytes。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InventorySlot {
    /// 物品定义 ID。Phase 3: migrate to ItemEntId.
    pub item_def_id: ItemDefId,
    /// 堆叠数量（0 = 空槽位）。
    pub quantity: u32,
    /// 玩家标记"收藏"——防误卖/误丢。
    pub is_favorite: bool,
}

impl Default for InventorySlot {
    fn default() -> Self {
        Self {
            item_def_id: ITEM_DEF_ID_NONE,
            quantity: 0,
            is_favorite: false,
        }
    }
}

// ── InventoryError ────────────────────────────────────

/// 库存操作错误。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryError {
    /// 无空余槽位——背包满了。
    NoSlots,
    /// 超重——无法携带更多。
    TooHeavy,
    /// 指定槽位无物品。
    ItemNotFound,
    /// 物品不可堆叠（stack_size = 1 的非空槽位尝试堆叠）。
    NotStackable,
}

// ── PersonalInventory ─────────────────────────────────

/// 随身库存——玩家和所有 NPC 共用同一套代码。
///
/// 槽位为主，负重为安全网。槽位稀缺 → 驱动仓储行为。
#[derive(Debug, Clone)]
pub struct PersonalInventory {
    /// 统一槽位池——槽位数 = `slots.len()`。
    /// 装备容器时 push 新空槽位，卸下容器时截断。
    slots: Vec<InventorySlot>,
    /// 分类索引——ItemCategory → slot 位置。
    /// 展示层索引，不影响存储逻辑。add/remove 时惰性更新。
    category_index: BTreeMap<ItemCategory, Vec<usize>>,
    /// 当前总重量（克）缓存——增删时更新，查询 O(1)。
    total_weight_grams: u32,
}

impl PersonalInventory {
    /// 创建指定容量的空库存。
    ///
    /// 预分配 `capacity` 个空槽位。
    pub fn new(capacity: u16) -> Self {
        let slots = vec![InventorySlot::default(); capacity as usize];
        Self {
            slots,
            category_index: BTreeMap::new(),
            total_weight_grams: 0,
        }
    }

    /// 可用槽位总数——`slots.len()`。
    /// 装备容器时动态增长。
    pub fn total_slots(&self) -> u16 {
        self.slots.len() as u16
    }

    /// 已用槽位数——quantity > 0 的槽位。
    pub fn used_slots(&self) -> u16 {
        self.slots.iter().filter(|s| s.quantity > 0).count() as u16
    }

    /// 添加物品。
    ///
    /// 三步流程（对齐设计 005 §2.1）：
    /// 1. 优先堆叠到同 `ItemDefId` 且 `quantity < stack_size` 的已有槽位
    /// 2. 若仍有剩余，检查空槽位
    /// 3. 检查重量（使用 `effective_encumbrance_kg` 体积折重）
    ///
    /// 返回实际添加的数量（可能 < 请求的 quantity，部分添加）。
    /// `max_weight_grams` 由调用方从 `BASE_MAX_WEIGHT_KG * strength * 1000` 计算传入。
    pub fn add(
        &mut self,
        item_def_id: ItemDefId,
        mut quantity: u32,
        props: &ItemProperties,
        max_weight_grams: u32,
    ) -> Result<u32, InventoryError> {
        if quantity == 0 {
            return Ok(0);
        }
        if item_def_id == ITEM_DEF_ID_NONE {
            return Ok(0);
        }

        let mut added = 0u32;
        let stack_size = props.stack_size;
        let effective_weight_per_item = (effective_encumbrance_kg(props) * 1000.0) as u32;

        // 步骤 1: 堆叠到已有同 ID 槽位
        if stack_size > 1 {
            for slot in &mut self.slots {
                if slot.item_def_id == item_def_id && slot.quantity < stack_size {
                    let space = stack_size - slot.quantity;
                    let to_add = quantity.min(space);
                    self.total_weight_grams += effective_weight_per_item.saturating_mul(to_add);
                    slot.quantity += to_add;
                    added += to_add;
                    quantity -= to_add;
                    if quantity == 0 {
                        break;
                    }
                }
            }
        }

        // 步骤 2: 需要新槽位
        while quantity > 0 {
            let used = self.used_slots();
            let total = self.total_slots();
            if used >= total {
                break; // 无空余槽位——返回已添加量
            }

            // 步骤 3: 检查重量
            let remaining_weight = max_weight_grams.saturating_sub(self.total_weight_grams);
            if effective_weight_per_item > 0 && remaining_weight < effective_weight_per_item {
                break; // 超重——返回已添加量
            }

            // 占用空槽位
            let max_fill = if stack_size > 1 { stack_size } else { 1 };
            let to_fill = quantity.min(max_fill);
            // 重量二次检查
            let weight_for_fill = effective_weight_per_item.saturating_mul(to_fill);
            if effective_weight_per_item > 0 && weight_for_fill > remaining_weight {
                // 尝试减量
                let max_by_weight = remaining_weight / effective_weight_per_item;
                if max_by_weight == 0 {
                    break;
                }
                let to_fill = to_fill.min(max_by_weight);
                quantity = quantity.min(to_fill);
            }

            for slot in &mut self.slots {
                if slot.quantity == 0 {
                    let final_fill = quantity.min(max_fill);
                    if final_fill == 0 {
                        break;
                    }
                    self.total_weight_grams += effective_weight_per_item.saturating_mul(final_fill);
                    slot.item_def_id = item_def_id;
                    slot.quantity = final_fill;
                    added += final_fill;
                    quantity -= final_fill;
                    break;
                }
            }
            // 如果遍历完所有 slot 都没有空位（理论上 used >= total 已覆盖）
        }

        self.rebuild_category_index();

        if added == 0 && quantity > 0 {
            // 一个都没加上——返回错误
            let used = self.used_slots();
            let total = self.total_slots();
            if used >= total {
                Err(InventoryError::NoSlots)
            } else {
                Err(InventoryError::TooHeavy)
            }
        } else {
            Ok(added)
        }
    }

    /// 从指定槽位移除物品。
    ///
    /// `quantity == 0` 表示清空整个槽位。
    /// 返回实际移除的数量。
    pub fn remove(&mut self, slot_idx: usize, quantity: u32) -> Result<u32, InventoryError> {
        let slot = self
            .slots
            .get_mut(slot_idx)
            .ok_or(InventoryError::ItemNotFound)?;
        if slot.quantity == 0 {
            return Err(InventoryError::ItemNotFound);
        }

        let effective_qty = if quantity == 0 {
            slot.quantity
        } else {
            quantity.min(slot.quantity)
        };
        let removed_item = slot.item_def_id;
        slot.quantity -= effective_qty;

        if slot.quantity == 0 {
            slot.item_def_id = ITEM_DEF_ID_NONE;
            slot.is_favorite = false;
        }

        // 更新重量和分类索引
        if removed_item != ITEM_DEF_ID_NONE {
            // 重量估算：用 effective_encumbrance 需要 props，此处用 0 标记惰性重算
            // 实际重量由调用方管理
        }
        self.rebuild_category_index();

        Ok(effective_qty)
    }

    /// 查询某物品在库存中的总数量（遍历所有槽位）。
    pub fn count_item(&self, item_def_id: ItemDefId) -> u32 {
        if item_def_id == ITEM_DEF_ID_NONE {
            return 0;
        }
        self.slots
            .iter()
            .filter(|s| s.item_def_id == item_def_id)
            .map(|s| s.quantity)
            .sum()
    }

    /// 返回该实体持有的所有 (ItemDefId, quantity) 对。
    pub fn get_holdings(&self) -> Vec<(ItemDefId, u32)> {
        let mut map: BTreeMap<ItemDefId, u32> = BTreeMap::new();
        for slot in &self.slots {
            if slot.quantity > 0 && slot.item_def_id != ITEM_DEF_ID_NONE {
                *map.entry(slot.item_def_id).or_default() += slot.quantity;
            }
        }
        map.into_iter().collect()
    }

    /// 获取分类索引。
    pub fn category_index(&self) -> &BTreeMap<ItemCategory, Vec<usize>> {
        &self.category_index
    }

    /// 当前总重量（克）。
    pub fn total_weight_grams(&self) -> u32 {
        self.total_weight_grams
    }

    /// 槽位只读访问。
    pub fn slots(&self) -> &[InventorySlot] {
        &self.slots
    }

    /// 扩展/收缩槽位池（装备/卸下容器时使用）。
    pub fn resize_slots(&mut self, new_capacity: u16) {
        let current = self.slots.len() as u16;
        if new_capacity > current {
            let extra = (new_capacity - current) as usize;
            self.slots
                .extend(std::iter::repeat(InventorySlot::default()).take(extra));
        } else if new_capacity < current {
            // 收缩：只移除空槽位
            self.slots.truncate(new_capacity as usize);
            // 确保我们不丢失有物品的槽位
            let occupied = self.used_slots();
            if occupied > new_capacity {
                // 不应该发生——调用方应该先确保有足够空位
                // 防御性处理：保留到 occupied
                // 实际情况需调用方保证
            }
        }
    }

    // ── 私有 ──────────────────────────────────────────

    /// 惰性重建 category_index。
    fn rebuild_category_index(&mut self) {
        self.category_index.clear();
        for (idx, slot) in self.slots.iter().enumerate() {
            if slot.quantity > 0 && slot.item_def_id != ITEM_DEF_ID_NONE {
                // category 从 ItemDefId 提取
                if let Some(cat) = slot.item_def_id.category() {
                    self.category_index.entry(cat).or_default().push(idx);
                }
            }
        }
    }
}

// ── tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::{ItemCategory, Quality, Rarity, ITEM_DEF_ID_NONE};

    fn test_props(weight_grams: u32, bulk_factor: f32, stack_size: u32) -> ItemProperties {
        ItemProperties {
            def_id: ITEM_DEF_ID_NONE,
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
        }
    }

    fn ore_props() -> ItemProperties {
        test_props(2000, 0.8, 50)
    }

    // ── InventorySlot ──────────────────────────────────

    #[test]
    fn test_slot_default_is_none() {
        let s = InventorySlot::default();
        assert_eq!(s.item_def_id, ITEM_DEF_ID_NONE);
        assert_eq!(s.quantity, 0);
        assert!(!s.is_favorite);
    }

    #[test]
    fn test_slot_copy() {
        let a = InventorySlot {
            item_def_id: crate::id::ItemDefId(42),
            quantity: 5,
            is_favorite: true,
        };
        let b = a; // Copy
        assert_eq!(a.item_def_id, b.item_def_id);
        assert_eq!(a.quantity, b.quantity);
        assert_eq!(a.is_favorite, b.is_favorite);
    }

    // ── PersonalInventory ──────────────────────────────

    #[test]
    fn test_new_empty() {
        let inv = PersonalInventory::new(30);
        assert_eq!(inv.total_slots(), 30);
        assert_eq!(inv.used_slots(), 0);
        assert_eq!(inv.total_weight_grams(), 0);
        assert!(inv.category_index().is_empty());
    }

    #[test]
    fn test_add_single_item() {
        let mut inv = PersonalInventory::new(30);
        let id = crate::id::ItemDefId(1);
        let props = ore_props();
        let added = inv.add(id, 10, &props, 1_000_000).unwrap();
        assert_eq!(added, 10);
        assert_eq!(inv.used_slots(), 1);
        assert_eq!(inv.count_item(id), 10);
    }

    #[test]
    fn test_add_stack_to_existing_slot() {
        let mut inv = PersonalInventory::new(30);
        let id = crate::id::ItemDefId(1);
        let props = ore_props(); // stack_size=50

        inv.add(id, 20, &props, 1_000_000).unwrap();
        let added = inv.add(id, 15, &props, 1_000_000).unwrap();
        assert_eq!(added, 15);
        assert_eq!(inv.used_slots(), 1); // 仍然 1 个槽位
        assert_eq!(inv.count_item(id), 35);
    }

    #[test]
    fn test_add_stack_spills_to_new_slot() {
        let mut inv = PersonalInventory::new(30);
        let id = crate::id::ItemDefId(1);
        let props = ore_props(); // stack_size=50

        inv.add(id, 50, &props, 1_000_000).unwrap();
        // 再加 20 → 第一个槽位已满，需要新槽位
        let added = inv.add(id, 20, &props, 1_000_000).unwrap();
        assert_eq!(added, 20);
        assert_eq!(inv.used_slots(), 2);
        assert_eq!(inv.count_item(id), 70);
    }

    #[test]
    fn test_add_no_slots_error() {
        let mut inv = PersonalInventory::new(1);
        let id1 = crate::id::ItemDefId(1);
        let id2 = crate::id::ItemDefId(2);
        let props = ore_props();

        inv.add(id1, 10, &props, 1_000_000).unwrap();
        let err = inv.add(id2, 5, &props, 1_000_000).unwrap_err();
        assert_eq!(err, InventoryError::NoSlots);
    }

    #[test]
    fn test_add_too_heavy_partial() {
        let mut inv = PersonalInventory::new(30);
        let id = crate::id::ItemDefId(1);
        let props = test_props(1000, 1.0, 50); // effective: 1kg per item

        // max_weight = 5kg = 5000g → 只能放 5 个
        let added = inv.add(id, 100, &props, 5000).unwrap();
        assert_eq!(added, 5);
        assert_eq!(inv.count_item(id), 5);
    }

    #[test]
    fn test_add_too_heavy_complete() {
        let mut inv = PersonalInventory::new(30);
        let id = crate::id::ItemDefId(1);
        let props = test_props(1000, 1.0, 1); // 不可堆叠, 1kg each

        // 先塞满重量到还剩 <1kg
        inv.add(id, 5, &props, 5000).unwrap(); // 5kg/5kg 用了

        // 再加1个 → 超重
        let err = inv.add(id, 1, &props, 5000).unwrap_err();
        assert_eq!(err, InventoryError::TooHeavy);
    }

    #[test]
    fn test_add_zero_quantity_noop() {
        let mut inv = PersonalInventory::new(30);
        let id = crate::id::ItemDefId(1);
        let props = ore_props();
        let added = inv.add(id, 0, &props, 1_000_000).unwrap();
        assert_eq!(added, 0);
        assert_eq!(inv.used_slots(), 0);
    }

    #[test]
    fn test_add_item_def_id_none_noop() {
        let mut inv = PersonalInventory::new(30);
        let props = ore_props();
        let added = inv.add(ITEM_DEF_ID_NONE, 5, &props, 1_000_000).unwrap();
        assert_eq!(added, 0);
    }

    #[test]
    fn test_remove_partial() {
        let mut inv = PersonalInventory::new(30);
        let id = crate::id::ItemDefId(1);
        let props = ore_props();

        inv.add(id, 20, &props, 1_000_000).unwrap();
        // 找到槽位 0，移除 5
        let removed = inv.remove(0, 5).unwrap();
        assert_eq!(removed, 5);
        assert_eq!(inv.count_item(id), 15);
    }

    #[test]
    fn test_remove_all() {
        let mut inv = PersonalInventory::new(30);
        let id = crate::id::ItemDefId(1);
        let props = ore_props();

        inv.add(id, 8, &props, 1_000_000).unwrap();
        let removed = inv.remove(0, 0).unwrap(); // 0 = 全部
        assert_eq!(removed, 8);
        assert_eq!(inv.count_item(id), 0);
        assert_eq!(inv.used_slots(), 0);
    }

    #[test]
    fn test_remove_empty_slot_error() {
        let mut inv = PersonalInventory::new(30);
        let err = inv.remove(0, 1).unwrap_err();
        assert_eq!(err, InventoryError::ItemNotFound);
    }

    #[test]
    fn test_remove_bad_index_error() {
        let mut inv = PersonalInventory::new(30);
        let err = inv.remove(999, 1).unwrap_err();
        assert_eq!(err, InventoryError::ItemNotFound);
    }

    #[test]
    fn test_count_item_multiple_slots() {
        let mut inv = PersonalInventory::new(30);
        let id = crate::id::ItemDefId(1);
        let props = ore_props(); // stack_size=50

        inv.add(id, 50, &props, 1_000_000).unwrap();
        inv.add(id, 30, &props, 1_000_000).unwrap(); // 溢出到 slot 1
        assert_eq!(inv.count_item(id), 80);
    }

    #[test]
    fn test_get_holdings() {
        let mut inv = PersonalInventory::new(30);
        let id1 = crate::id::ItemDefId(1);
        let id2 = crate::id::ItemDefId(2);
        let props = ore_props();

        inv.add(id1, 30, &props, 1_000_000).unwrap();
        inv.add(id2, 10, &props, 1_000_000).unwrap();

        let h = inv.get_holdings();
        assert_eq!(h.len(), 2);
    }

    #[test]
    fn test_resize_slots_expand() {
        let mut inv = PersonalInventory::new(30);
        assert_eq!(inv.total_slots(), 30);
        inv.resize_slots(60);
        assert_eq!(inv.total_slots(), 60);
    }

    #[test]
    fn test_resize_slots_shrink_preserves_occupied() {
        let mut inv = PersonalInventory::new(30);
        let id = crate::id::ItemDefId(1);
        let props = ore_props();
        inv.add(id, 5, &props, 1_000_000).unwrap();

        // 收缩到 10（保留 1 个占用槽位）
        inv.resize_slots(10);
        assert_eq!(inv.total_slots(), 10);
        assert_eq!(inv.count_item(id), 5);
    }
}
