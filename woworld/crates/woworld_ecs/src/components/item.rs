//! Item Component — 物品实体的 ECS 标签
//!
//! 用于替代 item_spawn 中裸 `ItemDefId` 组件。
//! 所有物品定义数据驻留在 ItemRegistry 中。

use woworld_core::id::ItemDefId;
use woworld_core::item::ITEM_DEF_ID_NONE;

/// 物品 Entity 的标签 Component（8 字节）。
///
/// `item_def_id` 指向 ItemRegistry 中的物品定义。
/// 用于 DroppedItem、Inventory 槽位、装备槽位等场景。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Item {
    pub item_def_id: ItemDefId,
}

impl Default for Item {
    fn default() -> Self {
        Self {
            item_def_id: ITEM_DEF_ID_NONE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_none() {
        let item = Item::default();
        assert_eq!(item.item_def_id, ITEM_DEF_ID_NONE);
    }

    #[test]
    fn test_size_8_bytes() {
        assert_eq!(std::mem::size_of::<Item>(), 8);
    }

    #[test]
    fn test_copy_semantics() {
        let a = Item {
            item_def_id: woworld_core::id::ItemDefId(42),
        };
        let b = a; // Copy
        assert_eq!(a.item_def_id, b.item_def_id);
    }
}
