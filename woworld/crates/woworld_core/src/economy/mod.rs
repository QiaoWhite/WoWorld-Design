//! 经济系统 — 核心类型与只读查询 trait
//!
//! 经济是"价值交换的物理引擎"——只定义底层规则（订单簿撮合、价格涌现、
//! 货币供给），不定义"东西值多少钱"或"NPC 该买什么"。
//!
//! Phase 1: EconomyQuery trait + 共享类型。
//! 延后: 订单簿引擎、价格发现、交易执行、货币管线（需物品系统实现）。
//!
//! 参见: `WoWorld-Design/Happy Game/开发阶段/经济系统/`
//! 参见: [[CLAUDE-INTERFACES.md]] CHG-022

pub mod behavioral;
pub mod bootstrap;
pub mod listing;

use serde::{Deserialize, Serialize};

use crate::id::ItemDefId;
use crate::types::EntityId;
pub use bootstrap::{initial_money_supply, inject_liquidity, BootstrapParams, LiquidityInjection};
pub use listing::{
    urgency_to_listing_type, ListingStatus, ListingType, NeedCategory, NeedReason, Urgency,
};
use std::collections::BTreeMap;

// ── 经济 ID 类型 ───────────────────────────────────────

/// 经济区域标识符
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct EconomyId(pub u32);

impl Default for EconomyId {
    fn default() -> Self {
        Self(u32::MAX)
    }
}

/// 市场标识符
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MarketId(pub u32);

/// 无市场哨兵值
pub const MARKET_ID_NONE: MarketId = MarketId(u32::MAX);

/// 无经济区域哨兵值
pub const ECONOMY_ID_NONE: EconomyId = EconomyId(u32::MAX);

// ── 市场、订单簿与交易 ─────────────────────────────────

/// 市场——一个经济体内的交易场所。
///
/// Phase 2: 一经济体一市场。Phase 3+ 扩展为多层级市场体系。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Market {
    pub market_id: MarketId,
    pub economy_id: EconomyId,
    /// (ItemDefId → OrderBook)
    pub order_books: BTreeMap<ItemDefId, OrderBook>,
}

impl Market {
    pub fn new(market_id: MarketId, economy_id: EconomyId) -> Self {
        Self {
            market_id,
            economy_id,
            order_books: BTreeMap::new(),
        }
    }
}

/// 订单簿——某一物品的买卖单集合。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OrderBook {
    /// 买单（按价格降序排列——最高买价在前）
    pub bids: Vec<Order>,
    /// 卖单（按价格升序排列——最低卖价在前）
    pub asks: Vec<Order>,
}

/// 订单
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order_id: u64,
    pub entity_id: EntityId,
    pub item_id: ItemDefId,
    pub quantity: u32,
    pub limit_price_copper: u64,
    pub side: OrderSide,
    pub created_tick: u64,
    /// ★ Phase 3: 挂单类型（Normal/Urgent/Passive）
    pub listing_type: ListingType,
    /// ★ Phase 3: 已成交数量（partial fill 追踪）
    pub filled_quantity: u32,
    /// ★ Phase 3: 挂单生命周期状态
    pub status: ListingStatus,
}

impl Order {
    /// 创建新订单——新字段自动填充默认值。
    ///
    /// `listing_type` = Normal, `filled_quantity` = 0, `status` = Active。
    pub fn new(
        entity_id: EntityId,
        item_id: ItemDefId,
        quantity: u32,
        limit_price_copper: u64,
        side: OrderSide,
        created_tick: u64,
    ) -> Self {
        Self {
            order_id: 0,
            entity_id,
            item_id,
            quantity,
            limit_price_copper,
            side,
            created_tick,
            listing_type: ListingType::Normal,
            filled_quantity: 0,
            status: ListingStatus::Active,
        }
    }
}

/// 订单方向
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderSide {
    Bid,
    Ask,
}

/// 交易记录
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TradeRecord {
    pub item_id: ItemDefId,
    pub quantity: u32,
    pub price_copper: u64,
    pub buyer_id: EntityId,
    pub seller_id: EntityId,
    pub tick: u64,
}

// ── 钱包快照 ───────────────────────────────────────────

/// 钱包只读快照（供 EconomyQuery 返回）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct WalletSnapshot {
    pub copper: u64,
    pub silver: u64,
    pub gold: u64,
}

impl WalletSnapshot {
    /// 换算为总铜币
    /// 1 silver = 20 copper, 1 gold = 400 copper (金:银:铜 = 1:20:400)
    pub fn total_copper(&self) -> u64 {
        self.copper + self.silver * 20 + self.gold * 400
    }

    /// 从总铜币创建（余数保留在 copper）
    pub fn from_copper(total: u64) -> Self {
        let gold = total / 400;
        let remainder = total % 400;
        let silver = remainder / 20;
        let copper = remainder % 20;
        Self {
            copper,
            silver,
            gold,
        }
    }
}

// ── 价格快照 ───────────────────────────────────────────

/// 市场价格快照
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct PriceSnapshot {
    /// EMA 平滑价格（铜币）
    pub ema_price_copper: u64,
    /// 最近原始成交价
    pub raw_last_price: u64,
    /// 最近成交量
    pub last_volume: u32,
    /// 最近交易 tick
    pub last_trade_tick: u64,
    /// 价格波动率 (0-1, CV of last 20 trades)
    pub volatility: f32,
}

// ── 经济健康度指数 ─────────────────────────────────────

/// 经济健康度复合指数（stub — Phase 2+ 填充）
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct EconomicHealthIndex {
    /// 通胀率 (月化, 0=稳定)
    pub inflation_monthly: f32,
    /// 就业率 [0,1]
    pub employment_rate: f32,
    /// 贸易量增长 [-1,1]
    pub trade_volume_growth: f32,
    /// 综合指数 [0,1]（0=崩溃, 1=繁荣）
    pub composite: f32,
}

// ── EconomyQuery trait ─────────────────────────────────

/// 经济只读查询接口 — 所有模块读取经济数据的唯一路径
///
/// 实现者: EconomyRegistry (woworld_ecs)
///
/// Phase 1: 大部分方法返回 None/空——实质数据等物品系统就位后填充。
pub trait EconomyQuery: Send + Sync {
    /// 查询物品市场价格（EMA 平滑）
    fn query_price(&self, market_id: MarketId, item_id: ItemDefId) -> Option<PriceSnapshot>;

    /// 查询市场成交量
    fn query_market_volume(&self, market_id: MarketId, item_id: ItemDefId) -> Option<u64>;

    /// 查询 NPC 钱包余额
    fn query_wallet(&self, entity_id: crate::types::EntityId) -> Option<WalletSnapshot>;

    /// 查询财富分布（Gini, deciles etc.）
    fn query_wealth_distribution(&self, economy_id: EconomyId) -> Option<WealthDistribution>;

    /// 查询生产能力
    fn query_production_capacity(&self, economy_id: EconomyId, item_id: ItemDefId) -> Option<u64>;

    /// 查询消费需求
    fn query_consumption_demand(&self, economy_id: EconomyId, item_id: ItemDefId) -> Option<u64>;

    /// 查询经济健康度
    fn query_economic_health(&self, economy_id: EconomyId) -> Option<EconomicHealthIndex>;

    /// 列出所有活跃市场
    fn all_markets(&self) -> &[MarketId];

    /// 活跃市场数量
    fn market_count(&self) -> usize;
    /// 查询劳动力市场
    fn query_labor_market(&self, economy_id: EconomyId) -> Option<LaborMarketSnapshot>;
    /// 查询贸易路线
    fn query_trade_routes(&self, economy_id: EconomyId) -> Vec<TradeRouteInfo>;
}

// ── LaborMarketSnapshot ────────────────────────────────

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LaborMarketSnapshot {
    pub employment_rate: f32,
    pub avg_daily_wage: u64,
    pub labor_supply: u32,
    pub labor_demand: u32,
}

// ── TradeRouteInfo ─────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TradeRouteInfo {
    pub from: EconomyId,
    pub to: EconomyId,
    pub activity: f32,
    pub primary_items: Vec<ItemDefId>,
}

// ── 财富分布 ───────────────────────────────────────────

/// 财富分布快照
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct WealthDistribution {
    /// 基尼系数 [0,1]
    pub gini: f32,
    /// 各十分位平均财富（铜币）
    pub deciles: [u64; 10],
    /// 总人口
    pub population: u32,
    /// 总财富（铜币）
    pub total_wealth: u64,
}

// ── 测试 ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[test]
    fn test_wallet_snapshot_total_copper() {
        let w = WalletSnapshot {
            copper: 15,
            silver: 3,
            gold: 1,
        };
        // 1 gold = 400, 3 silver = 60, 15 copper = 15 -> 475
        assert_eq!(w.total_copper(), 475);
    }

    #[test]
    fn test_wallet_snapshot_from_copper() {
        let w = WalletSnapshot::from_copper(850);
        assert_eq!(w.gold, 2); // 850 / 400 = 2
        assert_eq!(w.silver, 2); // (850 - 800) / 20 = 2
        assert_eq!(w.copper, 10); // remainder
    }

    #[test]
    fn test_wallet_snapshot_from_copper_roundtrip() {
        for total in [0, 15, 35, 85, 450, 920] {
            let w = WalletSnapshot::from_copper(total);
            assert_eq!(w.total_copper(), total, "roundtrip failed for {total}");
        }
    }

    #[test]
    fn test_market_id_none() {
        assert_eq!(MARKET_ID_NONE.0, u32::MAX);
    }

    #[test]
    fn test_economy_id_none() {
        assert_eq!(ECONOMY_ID_NONE.0, u32::MAX);
    }
}
