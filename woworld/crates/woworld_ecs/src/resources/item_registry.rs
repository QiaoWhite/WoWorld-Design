//! ItemRegistry — 物品定义的 SoA 存储 + ItemQuery 实现
//!
//! HashMap 提供 O(1) 属性查询，SoA 列支持快速过滤和批量查询。
//! TOML 数据文件通过 `load_from_toml()` 手动解析加载。

use std::collections::{BTreeMap, HashMap};

use woworld_core::id::ItemDefId;
use woworld_core::item::{
    AestheticProps, ConsumableEffect, ItemCategory, ItemPlacementProps, ItemProperties, ItemQuery,
    ItemTag, Quality, Rarity, ITEM_DEF_ID_NONE,
};

// ── ItemRegistry ────────────────────────────────────

#[derive(Debug, Default)]
pub struct ItemRegistry {
    /// ItemDefId → ItemProperties 主存储（O(1) 查询）
    definitions: HashMap<ItemDefId, ItemProperties>,
    /// 所有已注册 ItemDefId（保持插入顺序）
    def_ids: Vec<ItemDefId>,
    /// SoA: 分类列（快速过滤）
    categories: Vec<ItemCategory>,
    /// SoA: 堆叠上限列
    stack_sizes: Vec<u32>,
    /// SoA: 基础价值列（铜币）
    base_values: Vec<u32>,
    /// SoA: 稀有度列
    rarities: Vec<Rarity>,
}

impl ItemRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一个物品定义。使用 ItemProperties 中已有的 def_id。
    ///
    /// 若 def_id 为 ITEM_DEF_ID_NONE，静默跳过（不注册）。
    /// 若 def_id 已存在，覆盖旧定义。
    pub fn register(&mut self, props: ItemProperties) {
        if props.def_id == ITEM_DEF_ID_NONE {
            return;
        }

        let def_id = props.def_id;
        self.categories.push(props.category);
        self.stack_sizes.push(props.stack_size);
        self.base_values.push(props.base_value_copper);
        self.rarities.push(props.rarity);

        if self.definitions.insert(def_id, props).is_none() {
            self.def_ids.push(def_id);
        }
        // else: 覆盖——def_ids 不变，SoA 列已 push
    }

    /// 从 TOML 字符串加载物品定义。
    ///
    /// TOML 格式:
    /// ```toml
    /// [[items]]
    /// category = "MineralOre"
    /// sub_category = 1
    /// def_index = 0
    /// name = "铁矿"
    /// weight_grams = 2000
    /// # ...
    /// ```
    ///
    /// # Panics
    /// 若 TOML 格式错误或必需字段缺失则 panic（数据文件错误应尽早暴露）。
    pub fn load_from_toml(&mut self, toml_str: &str) {
        let root: toml::Value = toml_str
            .parse()
            .expect("item TOML parse error");
        let items = root["items"]
            .as_array()
            .expect("TOML missing [[items]] array");

        for (i, item) in items.iter().enumerate() {
            let table = item
                .as_table()
                .unwrap_or_else(|| panic!("items[{i}] is not a table"));

            let category_str = get_str(table, "category", i);
            let category = parse_category(&category_str)
                .unwrap_or_else(|| panic!("items[{i}]: unknown category '{category_str}'"));
            let sub_category = get_u8(table, "sub_category", i);
            let def_index = get_u64(table, "def_index", i);

            let def_id = ItemDefId::new(category, sub_category, def_index);

            let name = get_str(table, "name", i);
            let description = table
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default();
            let weight_grams = get_u32(table, "weight_grams", i);
            let bulk_factor = get_f32(table, "bulk_factor", i);
            let volume_liters = get_f32(table, "volume_liters", i);

            let base_quality = parse_quality(&get_str(table, "base_quality", i))
                .unwrap_or_else(|| panic!("items[{i}]: invalid base_quality"));
            let rarity = parse_rarity(&get_str(table, "rarity", i))
                .unwrap_or_else(|| panic!("items[{i}]: invalid rarity"));
            let quality_range_min = parse_quality(&get_str(table, "quality_range_min", i))
                .unwrap_or_else(|| panic!("items[{i}]: invalid quality_range_min"));
            let quality_range_max = parse_quality(&get_str(table, "quality_range_max", i))
                .unwrap_or_else(|| panic!("items[{i}]: invalid quality_range_max"));

            let stack_size = get_u32(table, "stack_size", i);
            let base_value_copper = get_u32(table, "base_value_copper", i);
            let max_durability = get_f32(table, "max_durability", i);
            let durability_loss_per_use = get_f32(table, "durability_loss_per_use", i);
            let magic_capacity_ke = table
                .get("magic_capacity_ke")
                .and_then(|v| v.as_integer())
                .unwrap_or(0) as u32;

            let tags = parse_tags(table, i);

            let mod_tags = BTreeMap::new(); // Phase 1: empty

            let min_skill = None;
            let min_strength = table
                .get("min_strength")
                .and_then(|v| v.as_float())
                .map(|f| f as f32);
            let required_body_part = None;
            let element_affinity = None;

            let placement = table
                .get("is_placeable")
                .and_then(|v| v.as_bool())
                .map(|b| ItemPlacementProps { is_placeable: b });

            let tool_tags = table
                .get("tool_tags")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .map(|v| v.as_str().unwrap().to_string())
                        .collect()
                });

            let consumable = table
                .get("is_consumable")
                .and_then(|v| v.as_bool())
                .map(|b| ConsumableEffect { is_consumable: b });

            let audio_material = None;
            let aesthetic_props = table
                .get("visual_quality")
                .and_then(|v| v.as_float())
                .map(|f| AestheticProps {
                    visual_quality: f as f32,
                });

            let props = ItemProperties {
                def_id,
                category,
                name,
                description,
                weight_grams,
                bulk_factor,
                volume_liters,
                base_quality,
                rarity,
                quality_range_min,
                quality_range_max,
                stack_size,
                base_value_copper,
                max_durability,
                durability_loss_per_use,
                magic_capacity_ke,
                tags,
                mod_tags,
                min_skill,
                min_strength,
                required_body_part,
                element_affinity,
                placement,
                tool_tags,
                consumable,
                audio_material,
                aesthetic_props,
            };

            self.register(props);
        }
    }

    pub fn len(&self) -> usize {
        self.def_ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.def_ids.is_empty()
    }
}

// ── ItemQuery impl ──────────────────────────────────

impl ItemQuery for ItemRegistry {
    fn get_properties(&self, id: ItemDefId) -> Option<&ItemProperties> {
        if id == ITEM_DEF_ID_NONE {
            return None;
        }
        self.definitions.get(&id)
    }

    fn get_category(&self, id: ItemDefId) -> Option<ItemCategory> {
        self.get_properties(id).map(|p| p.category)
    }

    fn get_stack_size(&self, id: ItemDefId) -> Option<u32> {
        self.get_properties(id).map(|p| p.stack_size)
    }

    fn get_base_value(&self, id: ItemDefId) -> Option<u32> {
        self.get_properties(id).map(|p| p.base_value_copper)
    }

    fn get_rarity(&self, id: ItemDefId) -> Option<Rarity> {
        self.get_properties(id).map(|p| p.rarity)
    }

    fn get_name(&self, id: ItemDefId) -> Option<&str> {
        self.get_properties(id).map(|p| p.name.as_str())
    }

    fn all_def_ids(&self) -> &[ItemDefId] {
        &self.def_ids
    }

    fn def_count(&self) -> usize {
        self.len()
    }
}

// ── TOML helpers ────────────────────────────────────

fn get_str(table: &toml::Table, key: &str, idx: usize) -> String {
    table
        .get(key)
        .unwrap_or_else(|| panic!("items[{idx}]: missing '{key}'"))
        .as_str()
        .unwrap_or_else(|| panic!("items[{idx}]: '{key}' must be string"))
        .to_string()
}

fn get_u32(table: &toml::Table, key: &str, idx: usize) -> u32 {
    table
        .get(key)
        .unwrap_or_else(|| panic!("items[{idx}]: missing '{key}'"))
        .as_integer()
        .unwrap_or_else(|| panic!("items[{idx}]: '{key}' must be integer")) as u32
}

fn get_u64(table: &toml::Table, key: &str, idx: usize) -> u64 {
    table
        .get(key)
        .unwrap_or_else(|| panic!("items[{idx}]: missing '{key}'"))
        .as_integer()
        .unwrap_or_else(|| panic!("items[{idx}]: '{key}' must be integer")) as u64
}

fn get_u8(table: &toml::Table, key: &str, idx: usize) -> u8 {
    let v = get_u64(table, key, idx);
    assert!(v <= 255, "items[{idx}]: '{key}' must be 0-255");
    v as u8
}

fn get_f32(table: &toml::Table, key: &str, idx: usize) -> f32 {
    table
        .get(key)
        .unwrap_or_else(|| panic!("items[{idx}]: missing '{key}'"))
        .as_float()
        .unwrap_or_else(|| panic!("items[{idx}]: '{key}' must be float")) as f32
}

fn parse_tags(table: &toml::Table, idx: usize) -> Vec<ItemTag> {
    match table.get("tags") {
        None => vec![],
        Some(v) => {
            let arr = v
                .as_array()
                .unwrap_or_else(|| panic!("items[{idx}]: 'tags' must be array"));
            arr.iter()
                .map(|v| {
                    let s = v
                        .as_str()
                        .unwrap_or_else(|| panic!("items[{idx}]: tag must be string"));
                    parse_tag(s).unwrap_or_else(|| panic!("items[{idx}]: unknown tag '{s}'"))
                })
                .collect()
        }
    }
}

fn parse_category(s: &str) -> Option<ItemCategory> {
    match s {
        "Weapon" => Some(ItemCategory::Weapon),
        "Armor" => Some(ItemCategory::Armor),
        "Accessory" => Some(ItemCategory::Accessory),
        "Consumable" => Some(ItemCategory::Consumable),
        "Potion" => Some(ItemCategory::Potion),
        "Food" => Some(ItemCategory::Food),
        "Scroll" => Some(ItemCategory::Scroll),
        "Ammunition" => Some(ItemCategory::Ammunition),
        "Material" => Some(ItemCategory::Material),
        "MineralOre" => Some(ItemCategory::MineralOre),
        "Gemstone" => Some(ItemCategory::Gemstone),
        "StoneMat" => Some(ItemCategory::StoneMat),
        "WoodMat" => Some(ItemCategory::WoodMat),
        "FiberMat" => Some(ItemCategory::FiberMat),
        "LeatherMat" => Some(ItemCategory::LeatherMat),
        "LiquidMat" => Some(ItemCategory::LiquidMat),
        "OrganicMat" => Some(ItemCategory::OrganicMat),
        "MagicMat" => Some(ItemCategory::MagicMat),
        "SoilMat" => Some(ItemCategory::SoilMat),
        "Tool" => Some(ItemCategory::Tool),
        "Pickaxe" => Some(ItemCategory::Pickaxe),
        "Axe" => Some(ItemCategory::Axe),
        "Shovel" => Some(ItemCategory::Shovel),
        "Hammer" => Some(ItemCategory::Hammer),
        "FishingRod" => Some(ItemCategory::FishingRod),
        "Sickle" => Some(ItemCategory::Sickle),
        "CarvingKnife" => Some(ItemCategory::CarvingKnife),
        "AlchemyKit" => Some(ItemCategory::AlchemyKit),
        "Container" => Some(ItemCategory::Container),
        "Backpack" => Some(ItemCategory::Backpack),
        "Pouch" => Some(ItemCategory::Pouch),
        "HandCarry" => Some(ItemCategory::HandCarry),
        "Currency" => Some(ItemCategory::Currency),
        "QuestItem" => Some(ItemCategory::QuestItem),
        "Blueprint" => Some(ItemCategory::Blueprint),
        "Book" => Some(ItemCategory::Book),
        "FurnitureItem" => Some(ItemCategory::FurnitureItem),
        "KeyItem" => Some(ItemCategory::KeyItem),
        "MagicItem" => Some(ItemCategory::MagicItem),
        "EnchantRune" => Some(ItemCategory::EnchantRune),
        "MagicImplement" => Some(ItemCategory::MagicImplement),
        "MagicConstruct" => Some(ItemCategory::MagicConstruct),
        _ => None,
    }
}

fn parse_quality(s: &str) -> Option<Quality> {
    match s {
        "Rough" => Some(Quality::Rough),
        "Standard" => Some(Quality::Standard),
        "Refined" => Some(Quality::Refined),
        "Perfect" => Some(Quality::Perfect),
        _ => None,
    }
}

fn parse_rarity(s: &str) -> Option<Rarity> {
    match s {
        "Common" => Some(Rarity::Common),
        "Uncommon" => Some(Rarity::Uncommon),
        "Rare" => Some(Rarity::Rare),
        "Epic" => Some(Rarity::Epic),
        "Legendary" => Some(Rarity::Legendary),
        _ => None,
    }
}

fn parse_tag(s: &str) -> Option<ItemTag> {
    match s {
        "Stackable" => Some(ItemTag::Stackable),
        "Edible" => Some(ItemTag::Edible),
        "Fuel" => Some(ItemTag::Fuel),
        "QuestItem" => Some(ItemTag::QuestItem),
        "TwoHanded" => Some(ItemTag::TwoHanded),
        _ => None,
    }
}

// ── tests ────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_props(def_id: ItemDefId) -> ItemProperties {
        ItemProperties {
            def_id,
            category: ItemCategory::Material,
            name: "test_item".into(),
            description: String::new(),
            weight_grams: 100,
            bulk_factor: 1.0,
            volume_liters: 0.1,
            base_quality: Quality::Standard,
            rarity: Rarity::Common,
            quality_range_min: Quality::Rough,
            quality_range_max: Quality::Perfect,
            stack_size: 99,
            base_value_copper: 10,
            max_durability: 100.0,
            durability_loss_per_use: 0.1,
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

    #[test]
    fn test_new_empty() {
        let reg = ItemRegistry::new();
        assert!(reg.is_empty());
        assert_eq!(reg.len(), 0);
        assert_eq!(reg.def_count(), 0);
        assert!(reg.all_def_ids().is_empty());
    }

    #[test]
    fn test_register_and_query() {
        let mut reg = ItemRegistry::new();
        let id = ItemDefId(42);
        reg.register(make_test_props(id));

        assert_eq!(reg.len(), 1);
        let props = reg.get_properties(id).expect("should exist");
        assert_eq!(props.name, "test_item");
        assert_eq!(reg.get_category(id), Some(ItemCategory::Material));
        assert_eq!(reg.get_stack_size(id), Some(99));
        assert_eq!(reg.get_base_value(id), Some(10));
        assert_eq!(reg.get_rarity(id), Some(Rarity::Common));
        assert_eq!(reg.get_name(id), Some("test_item"));
    }

    #[test]
    fn test_query_none_skipped() {
        let mut reg = ItemRegistry::new();
        reg.register(make_test_props(ITEM_DEF_ID_NONE));
        assert!(reg.is_empty());
    }

    #[test]
    fn test_query_missing_returns_none() {
        let reg = ItemRegistry::new();
        assert!(reg.get_properties(ItemDefId(999)).is_none());
        assert_eq!(reg.get_category(ItemDefId(999)), None);
        assert_eq!(reg.get_stack_size(ItemDefId(999)), None);
        assert_eq!(reg.get_base_value(ItemDefId(999)), None);
        assert_eq!(reg.get_rarity(ItemDefId(999)), None);
        assert_eq!(reg.get_name(ItemDefId(999)), None);
    }

    #[test]
    fn test_query_item_def_id_none_returns_none() {
        let mut reg = ItemRegistry::new();
        reg.register(make_test_props(ItemDefId(1)));
        assert!(reg.get_properties(ITEM_DEF_ID_NONE).is_none());
    }

    #[test]
    fn test_register_overwrite() {
        let mut reg = ItemRegistry::new();
        let id = ItemDefId(1);
        reg.register(make_test_props(id));

        let mut new_props = make_test_props(id);
        new_props.name = "updated".into();
        reg.register(new_props);

        assert_eq!(reg.len(), 1);
        assert_eq!(reg.get_name(id), Some("updated"));
    }

    #[test]
    fn test_multiple_register() {
        let mut reg = ItemRegistry::new();
        for i in 1..=5 {
            reg.register(make_test_props(ItemDefId(i)));
        }
        assert_eq!(reg.len(), 5);
        assert_eq!(reg.def_count(), 5);
        assert_eq!(reg.all_def_ids().len(), 5);
    }

    #[test]
    fn test_load_from_toml_single() {
        let toml = r#"
[[items]]
category = "LeatherMat"
sub_category = 1
def_index = 0
name = "兽皮"
weight_grams = 800
bulk_factor = 1.2
volume_liters = 2.0
base_quality = "Standard"
rarity = "Common"
quality_range_min = "Rough"
quality_range_max = "Perfect"
stack_size = 20
base_value_copper = 15
max_durability = 0.0
durability_loss_per_use = 0.0
tags = ["Stackable"]
"#;
        let mut reg = ItemRegistry::new();
        reg.load_from_toml(toml);

        assert_eq!(reg.len(), 1);
        let def_id = reg.all_def_ids()[0];
        let props = reg.get_properties(def_id).unwrap();
        assert_eq!(props.name, "兽皮");
        assert_eq!(props.category, ItemCategory::LeatherMat);
        assert_eq!(props.weight_grams, 800);
        assert_eq!(props.bulk_factor, 1.2);
        assert_eq!(props.stack_size, 20);
        assert_eq!(props.base_value_copper, 15);
        assert_eq!(props.base_quality, Quality::Standard);
        assert_eq!(props.rarity, Rarity::Common);
        assert!(props.tags.contains(&ItemTag::Stackable));
    }

    #[test]
    fn test_load_from_toml_multiple() {
        let toml = r#"
[[items]]
category = "MineralOre"
sub_category = 1
def_index = 0
name = "铁矿"
weight_grams = 2000
bulk_factor = 0.8
volume_liters = 1.0
base_quality = "Standard"
rarity = "Common"
quality_range_min = "Rough"
quality_range_max = "Refined"
stack_size = 50
base_value_copper = 8
max_durability = 0.0
durability_loss_per_use = 0.0

[[items]]
category = "MineralOre"
sub_category = 2
def_index = 0
name = "铜矿"
weight_grams = 1800
bulk_factor = 0.8
volume_liters = 0.9
base_quality = "Standard"
rarity = "Common"
quality_range_min = "Rough"
quality_range_max = "Refined"
stack_size = 50
base_value_copper = 10
max_durability = 0.0
durability_loss_per_use = 0.0

[[items]]
category = "Food"
sub_category = 1
def_index = 0
name = "生肉"
weight_grams = 500
bulk_factor = 1.1
volume_liters = 1.5
base_quality = "Standard"
rarity = "Common"
quality_range_min = "Rough"
quality_range_max = "Refined"
stack_size = 10
base_value_copper = 20
max_durability = 0.0
durability_loss_per_use = 0.0
is_consumable = true
tags = ["Edible"]
"#;
        let mut reg = ItemRegistry::new();
        reg.load_from_toml(toml);

        assert_eq!(reg.len(), 3);

        // 验证每个物品都可以查到
        let ids = reg.all_def_ids().to_vec();
        assert_eq!(ids.len(), 3);

        // 铁矿
        let iron_def = reg.all_def_ids()[0];
        assert_eq!(reg.get_name(iron_def), Some("铁矿"));
        assert_eq!(reg.get_category(iron_def), Some(ItemCategory::MineralOre));
        assert_eq!(iron_def.sub_category(), 1);

        // 铜矿
        let copper_def = reg.all_def_ids()[1];
        assert_eq!(reg.get_name(copper_def), Some("铜矿"));
        assert_eq!(copper_def.sub_category(), 2);

        // 生肉（消耗品）
        let meat_def = reg.all_def_ids()[2];
        assert_eq!(reg.get_name(meat_def), Some("生肉"));
        let meat = reg.get_properties(meat_def).unwrap();
        assert!(meat.consumable.is_some());
        assert!(meat.consumable.as_ref().unwrap().is_consumable);
        assert!(meat.tags.contains(&ItemTag::Edible));
    }

    #[test]
    fn test_load_from_toml_optional_fields() {
        let toml = r#"
[[items]]
category = "Weapon"
sub_category = 0
def_index = 0
name = "铁剑"
weight_grams = 1500
bulk_factor = 1.5
volume_liters = 3.0
base_quality = "Standard"
rarity = "Common"
quality_range_min = "Rough"
quality_range_max = "Refined"
stack_size = 1
base_value_copper = 50
max_durability = 200.0
durability_loss_per_use = 0.05
min_strength = 8.0
is_placeable = false
tags = ["TwoHanded"]
"#;
        let mut reg = ItemRegistry::new();
        reg.load_from_toml(toml);

        let props = reg.get_properties(reg.all_def_ids()[0]).unwrap();
        assert_eq!(props.name, "铁剑");
        assert_eq!(props.min_strength, Some(8.0));
        assert!(props.placement.is_some());
        assert!(!props.placement.as_ref().unwrap().is_placeable);
        assert!(props.tags.contains(&ItemTag::TwoHanded));
    }

    #[test]
    fn test_load_from_toml_item_def_id_encoding() {
        let toml = r#"
[[items]]
category = "MineralOre"
sub_category = 5
def_index = 42
name = "测试矿"
weight_grams = 1000
bulk_factor = 1.0
volume_liters = 0.5
base_quality = "Standard"
rarity = "Rare"
quality_range_min = "Standard"
quality_range_max = "Perfect"
stack_size = 30
base_value_copper = 100
max_durability = 0.0
durability_loss_per_use = 0.0
"#;
        let mut reg = ItemRegistry::new();
        reg.load_from_toml(toml);

        let def_id = reg.all_def_ids()[0];
        assert_eq!(def_id.category(), Some(ItemCategory::MineralOre));
        assert_eq!(def_id.sub_category(), 5);
        assert_eq!(def_id.def_index(), 42);
    }

    #[test]
    fn test_load_from_toml_idempotent() {
        let toml = r#"
[[items]]
category = "Currency"
sub_category = 0
def_index = 0
name = "铜币"
weight_grams = 3
bulk_factor = 0.5
volume_liters = 0.01
base_quality = "Standard"
rarity = "Common"
quality_range_min = "Standard"
quality_range_max = "Standard"
stack_size = 999
base_value_copper = 1
max_durability = 0.0
durability_loss_per_use = 0.0
tags = ["Stackable"]
"#;
        let mut reg = ItemRegistry::new();
        reg.load_from_toml(toml);
        assert_eq!(reg.len(), 1);

        // 再次加载——覆盖但不增加计数
        reg.load_from_toml(toml);
        assert_eq!(reg.len(), 1);
    }
}
