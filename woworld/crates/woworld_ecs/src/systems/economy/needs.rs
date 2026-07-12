//! NPC 需求评估系统
//!
//! 连接 ECS Needs/Vitals 系统与经济订单创建。
//! Phase 3: 生理需求（Physiological）完全实现；职业/社交需求 stub。
//!
//! 参见: `WoWorld-Design/Happy Game/开发阶段/经济系统/004-交易主体与角色涌现.md §2`

use woworld_core::economy::listing::{NeedCategory, NeedReason, Urgency};
use woworld_core::id::ItemDefId;
use woworld_core::item::ItemQuery;

use crate::components::vitals::Vitals;

/// 结构化需求评估结果
#[derive(Debug, Clone)]
pub struct NeedAssessment {
    pub item_id: ItemDefId,
    pub need_category: NeedCategory,
    pub desired_quantity: u32,
    pub urgency: Urgency,
    /// 最高愿付价格（铜币/单位），0 = 不限价
    pub max_acceptable_price: u64,
    pub reason: NeedReason,
}

/// 评估 NPC 的生理需求——替代 Phase 2 的种子随机 daily_need。
///
/// 从 Vitals 读取实际 hunger/thirst 状态，对可食用/可饮用物品生成对应的买单需求。
pub fn assess_physiological_needs(
    vitals: &Vitals,
    item_registry: &dyn ItemQuery,
) -> Vec<NeedAssessment> {
    let mut needs = Vec::new();

    // 饥饿检查：hp < max_hp 或 spirit < 0.6 → 需要食物
    let hunger_ratio = if vitals.max_hp > 0.0 {
        1.0 - (vitals.hp / vitals.max_hp)
    } else {
        0.0
    };
    let spirit_low = vitals.spirit < 0.6;

    // 综合生理紧迫度
    let physiological_distress = hunger_ratio.max(if spirit_low { 0.5 } else { 0.0 });

    if physiological_distress > 0.1 {
        let urgency = if hunger_ratio > 0.6 || vitals.spirit < 0.3 {
            Urgency::Critical
        } else if hunger_ratio > 0.3 {
            Urgency::High
        } else {
            Urgency::Normal
        };

        // 查找可食用物品生成买单
        for def_id in item_registry.all_def_ids() {
            if let Some(props) = item_registry.get_properties(*def_id) {
                if let Some(consumable) = &props.consumable {
                    if consumable.is_consumable {
                        // 检查是否为食物（Edible tag）
                        let is_food = props
                            .tags
                            .iter()
                            .any(|t| matches!(t, woworld_core::item::ItemTag::Edible));
                        if is_food {
                            let qty = if urgency >= Urgency::High { 3 } else { 1 };
                            needs.push(NeedAssessment {
                                item_id: *def_id,
                                need_category: NeedCategory::Physiological,
                                desired_quantity: qty,
                                urgency,
                                max_acceptable_price: 0, // 不限价（饥饿时）
                                reason: NeedReason::Hunger,
                            });
                        }
                    }
                }
            }
        }
    }

    needs
}

/// 评估 NPC 的职业需求。
///
/// Phase 3 stub——依赖 ProfessionTag + pending_crafting_recipes。
pub fn assess_occupational_needs(_item_registry: &dyn ItemQuery) -> Vec<NeedAssessment> {
    // Phase 4: 读取 pending_crafting_recipes → 缺原料 → 生成 Occupational 买单
    Vec::new()
}

/// 评估 NPC 的社交需求。
///
/// Phase 3 stub——依赖 gift_intents + luxury_desire。
pub fn assess_social_needs(_item_registry: &dyn ItemQuery) -> Vec<NeedAssessment> {
    // Phase 4: 读取 gift_intents/luxury_desire → 生成 Social 买单
    Vec::new()
}

/// 汇总所有需求类别。
pub fn assess_all_needs(
    vitals: Option<&Vitals>,
    item_registry: &dyn ItemQuery,
) -> Vec<NeedAssessment> {
    let mut all = Vec::new();
    if let Some(v) = vitals {
        all.extend(assess_physiological_needs(v, item_registry));
    }
    all.extend(assess_occupational_needs(item_registry));
    all.extend(assess_social_needs(item_registry));
    all
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use woworld_core::item::{
        ConsumableEffect, ItemCategory, ItemProperties, ItemTag, Quality, Rarity,
    };

    fn make_food_registry() -> impl ItemQuery {
        let props = ItemProperties {
            def_id: ItemDefId(1),
            category: ItemCategory::Food,
            name: "raw_meat".into(),
            description: String::new(),
            weight_grams: 500,
            bulk_factor: 1.0,
            volume_liters: 1.0,
            base_quality: Quality::Standard,
            rarity: Rarity::Common,
            quality_range_min: Quality::Rough,
            quality_range_max: Quality::Perfect,
            stack_size: 10,
            base_value_copper: 20,
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
        };

        struct FoodStub {
            props: ItemProperties,
            ids: &'static [ItemDefId],
        }
        impl ItemQuery for FoodStub {
            fn get_properties(&self, _id: ItemDefId) -> Option<&ItemProperties> {
                Some(&self.props)
            }
            fn get_category(&self, _: ItemDefId) -> Option<ItemCategory> {
                Some(ItemCategory::Food)
            }
            fn get_stack_size(&self, _: ItemDefId) -> Option<u32> {
                Some(10)
            }
            fn get_base_value(&self, _: ItemDefId) -> Option<u32> {
                Some(20)
            }
            fn get_rarity(&self, _: ItemDefId) -> Option<Rarity> {
                Some(Rarity::Common)
            }
            fn get_name(&self, _: ItemDefId) -> Option<&str> {
                Some("raw_meat")
            }
            fn all_def_ids(&self) -> &[ItemDefId] {
                self.ids
            }
            fn def_count(&self) -> usize {
                self.ids.len()
            }
        }

        let ids: &'static [ItemDefId] = Box::leak(Box::new([ItemDefId(1)]));
        FoodStub { props, ids }
    }

    #[test]
    fn test_healthy_npc_no_needs() {
        let vitals = Vitals::default(); // hp=100, max_hp=100, spirit=1.0
        let q = make_food_registry();
        let needs = assess_physiological_needs(&vitals, &q);
        assert!(
            needs.is_empty(),
            "healthy NPC should have no urgent food need"
        );
    }

    #[test]
    fn test_hungry_npc_creates_food_need() {
        let vitals = Vitals {
            hp: 30.0,
            max_hp: 100.0,
            spirit: 0.8,
            ..Vitals::default()
        };
        let q = make_food_registry();
        let needs = assess_physiological_needs(&vitals, &q);
        // hunger_ratio = 1.0 - 30/100 = 0.7 → Critical
        assert!(!needs.is_empty());
        assert_eq!(needs[0].need_category, NeedCategory::Physiological);
        assert_eq!(needs[0].urgency, Urgency::Critical);
    }

    #[test]
    fn test_spirit_low_creates_need() {
        let vitals = Vitals {
            hp: 80.0,
            max_hp: 100.0,
            spirit: 0.2,
            ..Vitals::default()
        };
        let q = make_food_registry();
        let needs = assess_physiological_needs(&vitals, &q);
        // spirit < 0.6 → physiological_distress = 0.5, hunger_ratio = 0.2
        // combined distress > 0.1 → generates needs
        assert!(!needs.is_empty());
    }

    #[test]
    fn test_occupational_stub_empty() {
        let needs = assess_occupational_needs(&make_food_registry());
        assert!(needs.is_empty());
    }

    #[test]
    fn test_social_stub_empty() {
        let needs = assess_social_needs(&make_food_registry());
        assert!(needs.is_empty());
    }

    #[test]
    fn test_assess_all_without_vitals() {
        let needs = assess_all_needs(None, &make_food_registry());
        // No vitals → no physiological needs, occupational/social stubs empty
        assert!(needs.is_empty());
    }
}
