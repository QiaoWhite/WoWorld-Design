//! Inventory ECS 标签 Component
//!
//! `HasInventory` 和 `HasEquipment` 是零大小类型（ZST）标记组件。
//! 实际库存和装备数据存储在 `InventoryRegistry` 资源中。
//! 遵循 ECS 铁律 1（Component = 纯数据，Copy）。
//!
//! 参见: `woworld_core::item::inventory` / `woworld_core::item::equipment`

use serde::{Deserialize, Serialize};
/// 标记 Entity 拥有随身库存（PersonalInventory 数据在 InventoryRegistry 中）。
///
/// ZST — 0 字节。Copy。
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct HasInventory;

/// 标记 Entity 拥有装备（CharacterEquipment 数据在 InventoryRegistry 中）。
///
/// ZST — 0 字节。Copy。
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct HasEquipment;

// hecs 0.10 blanket impl: all T: Send + Sync + 'static are Component.
// ZST unit structs automatically satisfy this.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_inventory_zst() {
        assert_eq!(std::mem::size_of::<HasInventory>(), 0);
    }

    #[test]
    fn test_has_equipment_zst() {
        assert_eq!(std::mem::size_of::<HasEquipment>(), 0);
    }

    #[test]
    fn test_copy_semantics() {
        let a = HasInventory;
        let b = a; // Copy
        let _ = b;
        let c = HasEquipment;
        let d = c;
        let _ = d;
    }

    #[test]
    fn test_default() {
        let _ = HasInventory::default();
        let _ = HasEquipment::default();
    }

    #[test]
    fn test_insert_into_hecs_world() {
        let mut world = hecs::World::new();
        let entity = world.spawn((HasInventory, HasEquipment));
        // Verify components can be queried (hecs requires &T for get/query)
        assert!(world.get::<&HasInventory>(entity).is_ok());
        assert!(world.get::<&HasEquipment>(entity).is_ok());
    }
}
