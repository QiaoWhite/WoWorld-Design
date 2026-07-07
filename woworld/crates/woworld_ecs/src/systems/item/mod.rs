//! ItemSeedSystem — 从 TOML 加载物品定义到 ItemRegistry
//!
//! Phase 1: 使用 `include_str!` 嵌入测试物品数据。
//! Phase 2+: 从运行时资产路径加载。

use crate::resources::item_registry::ItemRegistry;

/// 种子系统——将编译时嵌入的 TOML 物品定义加载到注册表。
///
/// 可在启动时调用一次，也可在测试中重复调用（幂等——覆盖已存在的 def_id）。
pub fn item_seed_system(registry: &mut ItemRegistry) {
    let toml_data = include_str!("../../../../../assets/items/test_items.toml");
    registry.load_from_toml(toml_data);
}

#[cfg(test)]
mod tests {
    use super::*;
    use woworld_core::item::ItemQuery;

    #[test]
    fn test_seed_system_populates_registry() {
        let mut reg = ItemRegistry::new();
        item_seed_system(&mut reg);

        assert!(reg.def_count() > 0, "registry should have items after seed");
        assert!(reg.def_count() >= 20, "expected at least 20 test items, got {}", reg.def_count());

        // 验证已知物品存在
        let ids = reg.all_def_ids().to_vec();
        let names: Vec<&str> = ids.iter().filter_map(|id| reg.get_name(*id)).collect();

        assert!(names.contains(&"兽皮"), "should contain 兽皮");
        assert!(names.contains(&"兽骨"), "should contain 兽骨");
        assert!(names.contains(&"生肉"), "should contain 生肉");
        assert!(names.contains(&"植物纤维"), "should contain 植物纤维");
        assert!(names.contains(&"种子"), "should contain 种子");
        assert!(names.contains(&"铁矿"), "should contain 铁矿");
        assert!(names.contains(&"铁剑"), "should contain 铁剑");
    }

    #[test]
    fn test_seed_system_deterministic() {
        let mut reg1 = ItemRegistry::new();
        let mut reg2 = ItemRegistry::new();

        item_seed_system(&mut reg1);
        item_seed_system(&mut reg2);

        assert_eq!(reg1.def_count(), reg2.def_count());
        let ids1 = reg1.all_def_ids().to_vec();
        let ids2 = reg2.all_def_ids().to_vec();
        assert_eq!(ids1, ids2);

        // 属性也相同
        for id in &ids1 {
            let p1 = reg1.get_properties(*id).unwrap();
            let p2 = reg2.get_properties(*id).unwrap();
            assert_eq!(p1.name, p2.name);
            assert_eq!(p1.category, p2.category);
            assert_eq!(p1.weight_grams, p2.weight_grams);
        }
    }

    #[test]
    fn test_seed_system_idempotent() {
        let mut reg = ItemRegistry::new();

        item_seed_system(&mut reg);
        let count_first = reg.def_count();

        item_seed_system(&mut reg);
        let count_second = reg.def_count();

        // 调用两次不增加条目（覆盖模式）
        assert_eq!(count_first, count_second);
        assert!(count_first >= 20);
    }

    #[test]
    fn test_seed_system_economy_items_present() {
        let mut reg = ItemRegistry::new();
        item_seed_system(&mut reg);

        let ids = reg.all_def_ids();
        let names: Vec<&str> = ids.iter().filter_map(|id| reg.get_name(*id)).collect();

        // 货币物品（经济系统依赖）
        assert!(names.contains(&"铜币"), "铜币 should be present");
        assert!(names.contains(&"银币"), "银币 should be present");
        assert!(names.contains(&"金币"), "金币 should be present");

        // 验证货币堆叠
        let copper_def = ids.iter().find(|id| reg.get_name(**id) == Some("铜币")).unwrap();
        assert_eq!(reg.get_stack_size(*copper_def), Some(999));
        assert_eq!(reg.get_base_value(*copper_def), Some(1));

        let gold_def = ids.iter().find(|id| reg.get_name(**id) == Some("金币")).unwrap();
        assert_eq!(reg.get_base_value(*gold_def), Some(400));
    }
}
