//! 需求类别与挂单类型
//!
//! NPC 经济决策的结构化需求评估——替代 Phase 2 的简化 surplus/deficit 计算。
//! Phase 3 采用设计 004 的细化 ListingType（含 Urgent payload + Passive）。
//!
//! 参见: `WoWorld-Design/Happy Game/开发阶段/经济系统/004-交易主体与角色涌现.md`

use crate::id::ItemDefId;
use crate::types::EntityId;

// ── NeedCategory ──────────────────────────────────────

/// 需求来源分类（设计 004 §2.1）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NeedCategory {
    /// 生存必需——饥饿/口渴/健康
    Physiological,
    /// 职业驱动——工匠缺原料/农民缺种子
    Occupational,
    /// 社交/情感驱动——送礼/奢侈品/地位消费
    Social,
}

// ── NeedReason ────────────────────────────────────────

/// 需求起因追溯（设计 004 §2.4）
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NeedReason {
    /// 饥饿驱动——买食物
    Hunger,
    /// 口渴驱动——买水/饮品
    Thirst,
    /// 职业·缺原料——制造物品缺材料
    CraftingIngredient {
        /// 制造配方 ID（Phase 3 stub: 0）
        recipe_id: u64,
        /// 产出物品
        product: ItemDefId,
    },
    /// 社交·送礼
    Gift {
        /// 收礼人
        recipient_id: EntityId,
        /// 场合（如 "生日"、"求婚"）
        occasion: String,
    },
    /// 社交·地位消费——openness × extraversion 驱动
    LuxuryDesire,
}

// ── Urgency ───────────────────────────────────────────

/// 需求紧急度（设计 004 §2.2）
///
/// 驱动 ListingType 映射和订单撮合优先级。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum Urgency {
    /// 愿望清单——低价才买
    Low = 0,
    /// 日常补货
    Normal = 1,
    /// 严重影响——原料耗尽→停产
    High = 2,
    /// 生命威胁——饥饿=0→任何价格买入食物
    Critical = 3,
}

// ── ListingType ───────────────────────────────────────

/// 挂单类型——驱动订单簿行为和撮合优先级（设计 004 §2.3）。
///
/// 注: 设计 002 定义了 7 个平级变体（含 StandingOrder/GovernmentQuota/BarterOffer/LoanRequest/IOUTrade），
/// 004 细化了 Urgent 的载荷字段并新增 Passive。Phase 3 采用 004 版本。
/// 002 中需外部系统支持的变体延后至 Phase 4+。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListingType {
    /// 正常挂单——标准撮合规则
    #[default]
    Normal,
    /// 紧急挂单——价格溢价 + 可自动接受卖单
    Urgent {
        /// 价格溢价百分比（如 50 = 愿意多付 50%）
        premium_pct: u8,
        /// 是否自动接受任何合理卖单（不等到最优惠价格）
        auto_accept: bool,
    },
    /// 被动挂单——仅低价才成交（愿望清单型）
    Passive,
}

// ── ListingStatus ─────────────────────────────────────

/// 挂单生命周期状态（设计 002 §3.1）
///
/// Partial fill 追踪——撮合后未完全成交的订单留在簿中继续等待。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListingStatus {
    /// 活跃中——等待撮合
    #[default]
    Active,
    /// 部分成交——部分 quantity 已匹配，剩余保留在簿
    PartiallyFilled,
    /// 完全成交——filled_quantity >= quantity，从簿中移除
    Filled,
    /// 已撤销——NPC 主动取消
    /// Phase 4: 添加 payload 字段 (by: NpcId, reason: String, at: GameTimestamp)
    ///   对齐设计 002 §3.1
    Cancelled,
    /// 已过期——超过 ORDER_LIFETIME
    Expired,
}

// ── urgency_to_listing_type ───────────────────────────

/// 紧急度 → 挂单类型映射（设计 004 §2.3）
pub fn urgency_to_listing_type(urgency: Urgency) -> ListingType {
    match urgency {
        Urgency::Critical => ListingType::Urgent {
            premium_pct: 50,
            auto_accept: true,
        },
        Urgency::High => ListingType::Urgent {
            premium_pct: 20,
            auto_accept: false,
        },
        Urgency::Normal => ListingType::Normal,
        Urgency::Low => ListingType::Passive,
    }
}

// ── tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Urgency ────────────────────────────────────────

    #[test]
    fn test_urgency_ordering() {
        assert!(Urgency::Critical > Urgency::High);
        assert!(Urgency::High > Urgency::Normal);
        assert!(Urgency::Normal > Urgency::Low);
    }

    #[test]
    fn test_urgency_discriminants_unique() {
        let variants = [Urgency::Critical, Urgency::High, Urgency::Normal, Urgency::Low];
        let mut seen = std::collections::HashSet::new();
        for v in variants {
            assert!(seen.insert(v as u8));
        }
    }

    // ── urgency_to_listing_type ────────────────────────

    #[test]
    fn test_critical_maps_to_urgent() {
        let lt = urgency_to_listing_type(Urgency::Critical);
        match lt {
            ListingType::Urgent { premium_pct, auto_accept } => {
                assert_eq!(premium_pct, 50);
                assert!(auto_accept);
            }
            _ => panic!("expected Urgent"),
        }
    }

    #[test]
    fn test_high_maps_to_urgent_no_auto() {
        let lt = urgency_to_listing_type(Urgency::High);
        match lt {
            ListingType::Urgent { premium_pct, auto_accept } => {
                assert_eq!(premium_pct, 20);
                assert!(!auto_accept);
            }
            _ => panic!("expected Urgent"),
        }
    }

    #[test]
    fn test_normal_maps_to_normal() {
        let lt = urgency_to_listing_type(Urgency::Normal);
        assert_eq!(lt, ListingType::Normal);
    }

    #[test]
    fn test_low_maps_to_passive() {
        let lt = urgency_to_listing_type(Urgency::Low);
        assert_eq!(lt, ListingType::Passive);
    }

    // ── ListingType ────────────────────────────────────

    #[test]
    fn test_listing_type_default_is_normal() {
        assert_eq!(ListingType::default(), ListingType::Normal);
    }

    // ── ListingStatus ──────────────────────────────────

    #[test]
    fn test_listing_status_default_is_active() {
        assert_eq!(ListingStatus::default(), ListingStatus::Active);
    }

    // ── NeedCategory ───────────────────────────────────

    #[test]
    fn test_need_category_equality() {
        assert_eq!(NeedCategory::Physiological, NeedCategory::Physiological);
        assert_ne!(NeedCategory::Physiological, NeedCategory::Occupational);
    }

    // ── NeedReason ─────────────────────────────────────

    #[test]
    fn test_need_reason_hunger() {
        let r = NeedReason::Hunger;
        assert_eq!(r, NeedReason::Hunger);
    }

    #[test]
    fn test_need_reason_crafting() {
        let r = NeedReason::CraftingIngredient {
            recipe_id: 42,
            product: ItemDefId(100),
        };
        match r {
            NeedReason::CraftingIngredient { recipe_id, product } => {
                assert_eq!(recipe_id, 42);
                assert_eq!(product, ItemDefId(100));
            }
            _ => panic!("expected CraftingIngredient"),
        }
    }
}
