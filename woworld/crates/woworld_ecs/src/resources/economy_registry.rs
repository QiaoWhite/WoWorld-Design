//! EconomyRegistry — 经济数据 SoA 存储 + EconomyQuery 实现
//!
//! 存储在 WorldDriver 中（非 hecs World）。
//!
//! Phase 2: 新增 Market/OrderBook 存储、订单撮合、交易执行、价格 EMA 跟踪。
//! 参见: woworld_core::economy::EconomyQuery

use std::collections::HashMap;

use woworld_core::economy::behavioral::EconBehaviorParams;
use woworld_core::economy::{
    EconomicHealthIndex, EconomyId, EconomyQuery, LaborMarketSnapshot, ListingStatus, Market,
    MarketId, Order, OrderSide, PriceSnapshot, TradeRecord, TradeRouteInfo, WalletSnapshot,
    WealthDistribution, MARKET_ID_NONE,
};
use woworld_core::id::ItemDefId;
use woworld_core::types::EntityId;

/// 经济注册表 — 市场/订单簿/价格/钱包 的 SoA 列存储
#[derive(Debug)]
pub struct EconomyRegistry {
    // ── 钱包存储 ──────────────────────────────────────
    wallet_entity_ids: Vec<EntityId>,
    wallet_copper: Vec<u64>,
    wallet_silver: Vec<u64>,
    wallet_gold: Vec<u64>,
    wallet_index: HashMap<EntityId, usize>,

    // ── 行为经济学参数 ────────────────────────────────
    econ_params: HashMap<EntityId, EconBehaviorParams>,

    // ── 物品持有 (Phase 2) ────────────────────────────
    /// EntityId → (ItemDefId → quantity)
    /// ★ Phase 2 注记: InventoryRegistry 已成为库存权威源。
    /// 此字段保留用于经济模拟（surplus/deficit 快速查询），Phase 3 迁移至 InventoryRegistry。
    item_holdings: HashMap<EntityId, HashMap<ItemDefId, u32>>,

    // ── 市场存储 (Phase 2) ────────────────────────────
    /// MarketId → Market
    markets: HashMap<MarketId, Market>,
    /// EconomyId → MarketId（Phase 2 简化：一经济体一市场）
    economy_market_map: HashMap<EconomyId, MarketId>,

    // ── ID 生成 ───────────────────────────────────────
    market_ids: Vec<MarketId>,
    next_market_id: u32,
    economy_ids: Vec<EconomyId>,
    next_economy_id: u32,
    next_order_id: u64,

    // ── 交易历史 ──────────────────────────────────────
    /// (MarketId, ItemDefId) → 最近交易记录
    trade_histories: HashMap<(MarketId, ItemDefId), Vec<TradeRecord>>,
    /// (MarketId, ItemDefId) → PriceSnapshot（EMA 缓存）
    price_snapshots: HashMap<(MarketId, ItemDefId), PriceSnapshot>,

    /// 最大保留交易记录数
    max_trade_history: usize,
    /// EMA 平滑因子
    ema_alpha: f32,
}

impl Default for EconomyRegistry {
    fn default() -> Self {
        Self {
            wallet_entity_ids: Vec::new(),
            wallet_copper: Vec::new(),
            wallet_silver: Vec::new(),
            wallet_gold: Vec::new(),
            wallet_index: HashMap::new(),
            econ_params: HashMap::new(),
            item_holdings: HashMap::new(),
            markets: HashMap::new(),
            economy_market_map: HashMap::new(),
            market_ids: Vec::new(),
            next_market_id: 0,
            economy_ids: Vec::new(),
            next_economy_id: 0,
            next_order_id: 1,
            trade_histories: HashMap::new(),
            price_snapshots: HashMap::new(),
            max_trade_history: 20,
            ema_alpha: 0.3,
        }
    }
}

impl EconomyRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    // ── 钱包管理 ──────────────────────────────────────

    pub fn set_wallet(&mut self, entity_id: EntityId, snapshot: WalletSnapshot) {
        if let Some(&idx) = self.wallet_index.get(&entity_id) {
            self.wallet_copper[idx] = snapshot.copper;
            self.wallet_silver[idx] = snapshot.silver;
            self.wallet_gold[idx] = snapshot.gold;
        } else {
            let idx = self.wallet_entity_ids.len();
            self.wallet_entity_ids.push(entity_id);
            self.wallet_copper.push(snapshot.copper);
            self.wallet_silver.push(snapshot.silver);
            self.wallet_gold.push(snapshot.gold);
            self.wallet_index.insert(entity_id, idx);
        }
    }

    pub fn get_wallet(&self, entity_id: EntityId) -> Option<WalletSnapshot> {
        self.wallet_index
            .get(&entity_id)
            .map(|&idx| WalletSnapshot {
                copper: self.wallet_copper[idx],
                silver: self.wallet_silver[idx],
                gold: self.wallet_gold[idx],
            })
    }

    /// 获取可变钱包引用（用于交易执行）
    fn get_wallet_mut(&mut self, entity_id: EntityId) -> Option<WalletMut<'_>> {
        let idx = *self.wallet_index.get(&entity_id)?;
        Some(WalletMut {
            copper: &mut self.wallet_copper[idx],
            silver: &mut self.wallet_silver[idx],
            gold: &mut self.wallet_gold[idx],
        })
    }

    pub fn set_econ_params(&mut self, entity_id: EntityId, params: EconBehaviorParams) {
        self.econ_params.insert(entity_id, params);
    }

    pub fn get_econ_params(&self, entity_id: EntityId) -> Option<&EconBehaviorParams> {
        self.econ_params.get(&entity_id)
    }

    // ── 市场管理 (Phase 2) ────────────────────────────

    /// 创建市场并关联到经济区。
    pub fn create_market_with_economy(&mut self, economy_id: EconomyId) -> MarketId {
        let market_id = MarketId(self.next_market_id);
        self.next_market_id += 1;
        self.market_ids.push(market_id);

        let market = Market::new(market_id, economy_id);
        self.markets.insert(market_id, market);
        self.economy_market_map.insert(economy_id, market_id);

        market_id
    }

    /// 获取经济区的市场 ID。
    pub fn market_for_economy(&self, economy_id: EconomyId) -> Option<MarketId> {
        self.economy_market_map.get(&economy_id).copied()
    }

    /// 获取市场引用。
    pub fn get_market(&self, market_id: MarketId) -> Option<&Market> {
        self.markets.get(&market_id)
    }

    /// 获取市场可变引用。
    pub fn get_market_mut(&mut self, market_id: MarketId) -> Option<&mut Market> {
        self.markets.get_mut(&market_id)
    }

    // 兼容旧 API
    pub fn create_market(&mut self) -> MarketId {
        let economy_id = self.create_economy();
        self.create_market_with_economy(economy_id)
    }

    pub fn create_economy(&mut self) -> EconomyId {
        let id = EconomyId(self.next_economy_id);
        self.next_economy_id += 1;
        self.economy_ids.push(id);
        id
    }

    // ── 订单管理 ──────────────────────────────────────

    /// 提交订单到市场订单簿。返回订单 ID。
    ///
    /// 提交后自动按价格排序：bids 降序、asks 升序。
    pub fn submit_order(&mut self, market_id: MarketId, mut order: Order) -> u64 {
        if market_id == MARKET_ID_NONE {
            return 0;
        }

        order.order_id = self.next_order_id;
        self.next_order_id += 1;

        if let Some(market) = self.markets.get_mut(&market_id) {
            let book = market.order_books.entry(order.item_id).or_default();
            match order.side {
                OrderSide::Bid => {
                    book.bids.push(order);
                    book.bids
                        .sort_by_key(|b| std::cmp::Reverse(b.limit_price_copper));
                }
                OrderSide::Ask => {
                    book.asks.push(order);
                    book.asks.sort_by_key(|a| a.limit_price_copper);
                }
            }
            self.next_order_id - 1
        } else {
            0
        }
    }

    // ── 撮合引擎 ──────────────────────────────────────

    /// 撮合市场中某一物品的订单簿。返回成交记录列表。
    ///
    /// 算法：最高买价 ≥ 最低卖价 → 撮合。成交价 = (bid + ask) / 2。
    /// 往返匹配直到不再满足条件。
    /// Phase 2 简化：全部成交或全不成交（不做部分成交）。
    pub fn match_orders(
        &mut self,
        market_id: MarketId,
        item_id: ItemDefId,
        tick: u64,
    ) -> Vec<TradeRecord> {
        // 阶段 0：预取所有订单簿参与者的钱包余额（避免双重借用）
        let mut wallet_cache: HashMap<EntityId, u64> = HashMap::new();
        {
            if let Some(market) = self.markets.get(&market_id) {
                if let Some(book) = market.order_books.get(&item_id) {
                    for order in book.bids.iter().chain(book.asks.iter()) {
                        use std::collections::hash_map::Entry;
                        if let Entry::Vacant(e) = wallet_cache.entry(order.entity_id) {
                            let balance = self
                                .wallet_index
                                .get(&order.entity_id)
                                .map(|&idx| {
                                    self.wallet_copper[idx]
                                        + self.wallet_silver[idx] * 20
                                        + self.wallet_gold[idx] * 400
                                })
                                .unwrap_or(0);
                            e.insert(balance);
                        }
                    }
                }
            }
        }

        // 阶段 1：撮合（在 markets 借用内）
        struct MatchCandidate {
            buyer: EntityId,
            seller: EntityId,
            amount: u64,
            price: u64,
            quantity: u32,
        }
        let mut candidates: Vec<MatchCandidate> = Vec::new();

        {
            let market = match self.markets.get_mut(&market_id) {
                Some(m) => m,
                None => return Vec::new(),
            };
            let book = match market.order_books.get_mut(&item_id) {
                Some(b) => b,
                None => return Vec::new(),
            };

            while !book.bids.is_empty() && !book.asks.is_empty() {
                let bid_price = book.bids[0].limit_price_copper;
                let ask_price = book.asks[0].limit_price_copper;
                if bid_price < ask_price {
                    break;
                }

                let buyer_id = book.bids[0].entity_id;
                let seller_id = book.asks[0].entity_id;
                let price = (bid_price + ask_price) / 2;
                let trade_qty = book.bids[0].quantity.min(book.asks[0].quantity);
                let amount = price * trade_qty as u64;

                // 用缓存验证钱包
                let balance = wallet_cache.get(&buyer_id).copied().unwrap_or(0);
                if balance < amount {
                    book.bids.remove(0);
                    continue;
                }
                wallet_cache.insert(buyer_id, balance - amount);

                candidates.push(MatchCandidate {
                    buyer: buyer_id,
                    seller: seller_id,
                    amount,
                    price,
                    quantity: trade_qty,
                });

                // ★ Phase 3: Partial fill — 降量而非全删
                {
                    let bid = &mut book.bids[0];
                    bid.quantity -= trade_qty;
                    bid.filled_quantity += trade_qty;
                    if bid.quantity == 0 {
                        bid.status = ListingStatus::Filled;
                    } else {
                        bid.status = ListingStatus::PartiallyFilled;
                    }
                }
                {
                    let ask = &mut book.asks[0];
                    ask.quantity -= trade_qty;
                    ask.filled_quantity += trade_qty;
                    if ask.quantity == 0 {
                        ask.status = ListingStatus::Filled;
                    } else {
                        ask.status = ListingStatus::PartiallyFilled;
                    }
                }

                // 完全成交 → 移除
                if book.bids[0].quantity == 0 {
                    book.bids.remove(0);
                }
                if book.asks[0].quantity == 0 {
                    book.asks.remove(0);
                }
            }
        } // markets 借用释放

        // 阶段 2：执行完整交易（铜币 + 物品）
        let mut trades = Vec::new();
        for c in &candidates {
            if self.execute_trade(c.buyer, c.seller, c.amount, item_id, c.quantity) {
                trades.push(TradeRecord {
                    item_id,
                    quantity: c.quantity,
                    price_copper: c.price,
                    buyer_id: c.buyer,
                    seller_id: c.seller,
                    tick,
                });
            }
        }

        trades
    }

    /// 匹配所有市场的所有物品。返回全部成交记录。
    pub fn match_all_markets(&mut self, tick: u64) -> Vec<TradeRecord> {
        // 收集所有 (market_id, item_id) 对以避免借用冲突
        let targets: Vec<(MarketId, Vec<ItemDefId>)> = self
            .markets
            .iter()
            .map(|(mid, m)| {
                let ids: Vec<ItemDefId> = m.order_books.keys().copied().collect();
                (*mid, ids)
            })
            .collect();

        let mut all_trades = Vec::new();
        for (mid, item_ids) in &targets {
            for &item_id in item_ids {
                let trades = self.match_orders(*mid, item_id, tick);
                for trade in trades {
                    self.record_trade(*mid, trade);
                    all_trades.push(trade);
                }
            }
        }
        all_trades
    }

    // ── 物品持有管理 ──────────────────────────────────

    /// 给实体添加物品库存。
    pub fn add_items(&mut self, entity: EntityId, item_id: ItemDefId, quantity: u32) {
        if quantity == 0 {
            return;
        }
        let holdings = self.item_holdings.entry(entity).or_default();
        *holdings.entry(item_id).or_default() += quantity;
    }

    /// 从实体移除物品库存。返回实际移除数量（可能小于请求）。
    pub fn remove_items(&mut self, entity: EntityId, item_id: ItemDefId, quantity: u32) -> u32 {
        let holdings = match self.item_holdings.get_mut(&entity) {
            Some(h) => h,
            None => return 0,
        };
        let entry = match holdings.get_mut(&item_id) {
            Some(e) => e,
            None => return 0,
        };
        let removed = quantity.min(*entry);
        *entry -= removed;
        if *entry == 0 {
            holdings.remove(&item_id);
        }
        removed
    }

    /// 获取实体持有的某物品数量。
    pub fn get_item_count(&self, entity: EntityId, item_id: ItemDefId) -> u32 {
        self.item_holdings
            .get(&entity)
            .and_then(|h| h.get(&item_id))
            .copied()
            .unwrap_or(0)
    }

    /// 获取实体所有物品持有。
    pub fn get_holdings(&self, entity: EntityId) -> Option<&HashMap<ItemDefId, u32>> {
        self.item_holdings.get(&entity)
    }

    /// 为 NPC 分配初始物品库存（从种子派生）。
    ///
    /// Phase 2 简化：每个 NPC 根据种子随机持有 1-3 种物品。
    pub fn seed_npc_items(&mut self, entity: EntityId, item_pool: &[ItemDefId], seed: u64) {
        let count = 1 + (seed.wrapping_mul(3) % 3) as usize; // 1-3 种物品
        for i in 0..count {
            let idx = (seed.wrapping_mul(7 + i as u64) as usize) % item_pool.len().max(1);
            let item_id = item_pool[idx];
            let quantity = 1 + (seed.wrapping_mul(11 + i as u64) % 5) as u32; // 1-5 个
            self.add_items(entity, item_id, quantity);
        }
    }

    // ── 交易执行 ──────────────────────────────────────

    /// 执行完整交易：转移铜币 + 物品（原子操作）。
    ///
    /// 返回 true 表示全部成功。
    fn execute_trade(
        &mut self,
        buyer: EntityId,
        seller: EntityId,
        amount_copper: u64,
        item_id: ItemDefId,
        quantity: u32,
    ) -> bool {
        // 1. 验证卖方持有足够物品
        let seller_has = self.get_item_count(seller, item_id);
        if seller_has < quantity {
            return false;
        }

        // 2. 转账铜币
        if !self.transfer_copper(buyer, seller, amount_copper) {
            return false;
        }

        // 3. 转移物品
        self.remove_items(seller, item_id, quantity);
        self.add_items(buyer, item_id, quantity);

        true
    }

    /// 在两个 Entity 之间转移铜币（原子操作）。
    ///
    /// 返回 true 表示转账成功。
    fn transfer_copper(&mut self, from: EntityId, to: EntityId, amount_copper: u64) -> bool {
        let from_total = self.get_wallet(from).map(|w| w.total_copper()).unwrap_or(0);
        if from_total < amount_copper {
            return false;
        }

        // 扣款
        {
            let w = match self.get_wallet_mut(from) {
                Some(w) => w,
                None => return false,
            };
            let new_total = w.total_copper() - amount_copper;
            *w.copper = new_total % 20;
            *w.silver = (new_total / 20) % 20;
            *w.gold = new_total / 400;
        }

        // 加款
        if let Some(w) = self.get_wallet_mut(to) {
            let new_total = w.total_copper() + amount_copper;
            *w.copper = new_total % 20;
            *w.silver = (new_total / 20) % 20;
            *w.gold = new_total / 400;
        } else {
            // 卖家还没有钱包——创建一个
            let to_total = amount_copper;
            self.set_wallet(
                to,
                WalletSnapshot {
                    copper: to_total % 20,
                    silver: (to_total / 20) % 20,
                    gold: to_total / 400,
                },
            );
        }

        true
    }

    // ── 交易记录与价格跟踪 ────────────────────────────

    /// 记录交易并更新 EMA 价格快照。
    fn record_trade(&mut self, market_id: MarketId, trade: TradeRecord) {
        let key = (market_id, trade.item_id);

        // 更新交易历史（保留最近 N 条）
        let history = self.trade_histories.entry(key).or_default();
        history.push(trade);
        if history.len() > self.max_trade_history {
            history.remove(0);
        }

        // 更新 EMA 价格快照
        let snapshot = self.price_snapshots.entry(key).or_default();
        if snapshot.ema_price_copper == 0 {
            // 首次交易——直接设置
            snapshot.ema_price_copper = trade.price_copper;
        } else {
            // EMA 平滑
            let ema = snapshot.ema_price_copper as f32;
            let new_price = trade.price_copper as f32;
            snapshot.ema_price_copper =
                (self.ema_alpha * new_price + (1.0 - self.ema_alpha) * ema) as u64;
        }
        snapshot.raw_last_price = trade.price_copper;
        snapshot.last_volume = trade.quantity;
        snapshot.last_trade_tick = trade.tick;

        // 计算波动率（CV of last 20 trades）
        if history.len() >= 2 {
            let prices: Vec<f32> = history.iter().map(|t| t.price_copper as f32).collect();
            let mean = prices.iter().sum::<f32>() / prices.len() as f32;
            let variance =
                prices.iter().map(|p| (p - mean) * (p - mean)).sum::<f32>() / prices.len() as f32;
            let std_dev = variance.sqrt();
            if mean > 0.0 {
                snapshot.volatility = (std_dev / mean).min(1.0);
            }
        }
    }

    /// 获取价格快照。
    pub fn get_price_snapshot(
        &self,
        market_id: MarketId,
        item_id: ItemDefId,
    ) -> Option<PriceSnapshot> {
        self.price_snapshots.get(&(market_id, item_id)).copied()
    }

    // ── 查询辅助 ──────────────────────────────────────

    pub fn wallet_count(&self) -> usize {
        self.wallet_entity_ids.len()
    }
    pub fn econ_params_count(&self) -> usize {
        self.econ_params.len()
    }
    pub fn trade_count(&self) -> usize {
        self.trade_histories.values().map(|v| v.len()).sum()
    }
}

// ── 内部辅助 ──────────────────────────────────────────

struct WalletMut<'a> {
    copper: &'a mut u64,
    silver: &'a mut u64,
    gold: &'a mut u64,
}

impl WalletMut<'_> {
    fn total_copper(&self) -> u64 {
        *self.copper + *self.silver * 20 + *self.gold * 400
    }
}

// ── EconomyQuery impl (Phase 2 升级) ──────────────────

impl EconomyQuery for EconomyRegistry {
    fn query_price(&self, market_id: MarketId, item_id: ItemDefId) -> Option<PriceSnapshot> {
        self.get_price_snapshot(market_id, item_id)
    }

    fn query_market_volume(&self, market_id: MarketId, item_id: ItemDefId) -> Option<u64> {
        self.trade_histories
            .get(&(market_id, item_id))
            .map(|trades| trades.iter().map(|t| t.quantity as u64).sum())
    }

    fn query_wallet(&self, entity_id: EntityId) -> Option<WalletSnapshot> {
        self.get_wallet(entity_id)
    }

    fn query_wealth_distribution(&self, _economy_id: EconomyId) -> Option<WealthDistribution> {
        None
    }

    fn query_production_capacity(
        &self,
        _economy_id: EconomyId,
        _item_id: ItemDefId,
    ) -> Option<u64> {
        None
    }

    fn query_consumption_demand(&self, _economy_id: EconomyId, _item_id: ItemDefId) -> Option<u64> {
        None
    }

    fn query_labor_market(&self, _economy_id: EconomyId) -> Option<LaborMarketSnapshot> {
        None
    }

    fn query_trade_routes(&self, _economy_id: EconomyId) -> Vec<TradeRouteInfo> {
        Vec::new()
    }

    fn query_economic_health(&self, _economy_id: EconomyId) -> Option<EconomicHealthIndex> {
        None
    }

    fn all_markets(&self) -> &[MarketId] {
        &self.market_ids
    }

    fn market_count(&self) -> usize {
        self.market_ids.len()
    }
}

// ── 测试 ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── 原有测试 ──────────────────────────────────────

    #[test]
    fn test_new_empty() {
        let reg = EconomyRegistry::new();
        assert_eq!(reg.wallet_count(), 0);
        assert_eq!(reg.market_count(), 0);
        assert_eq!(reg.trade_count(), 0);
    }

    #[test]
    fn test_set_get_wallet() {
        let mut reg = EconomyRegistry::new();
        let entity = EntityId(1);
        let ws = WalletSnapshot {
            copper: 15,
            silver: 10,
            gold: 2,
        };
        reg.set_wallet(entity, ws);
        let got = reg.get_wallet(entity).unwrap();
        assert_eq!(got.copper, 15);
    }

    #[test]
    fn test_wallet_update() {
        let mut reg = EconomyRegistry::new();
        let entity = EntityId(42);
        reg.set_wallet(
            entity,
            WalletSnapshot {
                copper: 100,
                silver: 0,
                gold: 0,
            },
        );
        reg.set_wallet(
            entity,
            WalletSnapshot {
                copper: 200,
                silver: 0,
                gold: 0,
            },
        );
        assert_eq!(reg.get_wallet(entity).unwrap().copper, 200);
        assert_eq!(reg.wallet_count(), 1);
    }

    #[test]
    fn test_create_market_sequential() {
        let mut reg = EconomyRegistry::new();
        assert_eq!(reg.create_market(), MarketId(0));
        assert_eq!(reg.create_market(), MarketId(1));
        assert_eq!(reg.market_count(), 2);
    }

    #[test]
    fn test_economy_query_wallet() {
        let mut reg = EconomyRegistry::new();
        let entity = EntityId(99);
        reg.set_wallet(
            entity,
            WalletSnapshot {
                copper: 300,
                silver: 5,
                gold: 1,
            },
        );
        assert_eq!(
            EconomyQuery::query_wallet(&reg, entity).unwrap().copper,
            300
        );
    }

    #[test]
    fn test_query_labor_market_phase1_none() {
        let reg = EconomyRegistry::new();
        assert!(EconomyQuery::query_labor_market(&reg, EconomyId(0)).is_none());
    }

    #[test]
    fn test_query_trade_routes_phase1_empty() {
        let reg = EconomyRegistry::new();
        assert!(EconomyQuery::query_trade_routes(&reg, EconomyId(0)).is_empty());
    }

    // ── Phase 2 测试：市场与订单簿 ─────────────────────

    fn make_test_entity(id: u64) -> EntityId {
        EntityId(id | (1 << 63)) // NonItem bit
    }

    fn seed_wallet(reg: &mut EconomyRegistry, entity: EntityId, total_copper: u64) {
        reg.set_wallet(entity, WalletSnapshot::from_copper(total_copper));
    }

    #[test]
    fn test_create_market_with_economy() {
        let mut reg = EconomyRegistry::new();
        let econ_id = reg.create_economy();
        let market_id = reg.create_market_with_economy(econ_id);

        assert_eq!(reg.market_for_economy(econ_id), Some(market_id));
        let market = reg.get_market(market_id).unwrap();
        assert_eq!(market.economy_id, econ_id);
        assert!(market.order_books.is_empty());
    }

    #[test]
    fn test_submit_order_and_match() {
        let mut reg = EconomyRegistry::new();
        let market_id = reg.create_market();
        let item = ItemDefId(100);

        let buyer = make_test_entity(1);
        let seller = make_test_entity(2);

        seed_wallet(&mut reg, buyer, 1000);
        seed_wallet(&mut reg, seller, 0);
        reg.add_items(seller, item, 1); // 卖家持有物品

        // 买单：100 coin
        reg.submit_order(
            market_id,
            Order::new(buyer, item, 1, 100, OrderSide::Bid, 0),
        );

        // 卖单：80 coin
        reg.submit_order(
            market_id,
            Order::new(seller, item, 1, 80, OrderSide::Ask, 0),
        );

        // 撮合
        let trades = reg.match_orders(market_id, item, 1);
        assert_eq!(trades.len(), 1, "should match one trade");

        let trade = trades[0];
        assert_eq!(trade.buyer_id, buyer);
        assert_eq!(trade.seller_id, seller);
        assert_eq!(trade.price_copper, 90); // (100 + 80) / 2
        assert_eq!(trade.quantity, 1);

        // 记录交易并检查价格
        reg.record_trade(market_id, trade);
        let snap = reg.get_price_snapshot(market_id, item).unwrap();
        assert_eq!(snap.ema_price_copper, 90);
        assert_eq!(snap.raw_last_price, 90);
    }

    #[test]
    fn test_no_match_when_no_cross() {
        let mut reg = EconomyRegistry::new();
        let market_id = reg.create_market();
        let item = ItemDefId(42);

        // 买单：50 coin（低价）
        reg.submit_order(
            market_id,
            Order::new(make_test_entity(1), item, 1, 50, OrderSide::Bid, 0),
        );

        // 卖单：100 coin（高价——无交叉）
        reg.submit_order(
            market_id,
            Order::new(make_test_entity(2), item, 1, 100, OrderSide::Ask, 0),
        );

        let trades = reg.match_orders(market_id, item, 1);
        assert!(trades.is_empty(), "no match expected");
    }

    #[test]
    fn test_match_multiple_pairs() {
        let mut reg = EconomyRegistry::new();
        let market_id = reg.create_market();
        let item = ItemDefId(77);

        let buyer = make_test_entity(1);
        let seller_a = make_test_entity(2);
        let seller_b = make_test_entity(3);

        seed_wallet(&mut reg, buyer, 5000);
        reg.add_items(seller_a, item, 1);
        reg.add_items(seller_b, item, 1);

        // 买单
        reg.submit_order(
            market_id,
            Order::new(buyer, item, 1, 100, OrderSide::Bid, 0),
        );
        reg.submit_order(
            market_id,
            Order::new(buyer, item, 1, 100, OrderSide::Bid, 0),
        );

        // 卖单（两笔）
        reg.submit_order(
            market_id,
            Order::new(seller_a, item, 1, 70, OrderSide::Ask, 0),
        );
        reg.submit_order(
            market_id,
            Order::new(seller_b, item, 1, 80, OrderSide::Ask, 0),
        );

        let trades = reg.match_orders(market_id, item, 1);
        assert_eq!(trades.len(), 2, "should match two trades");
    }

    #[test]
    fn test_buyer_insufficient_funds_skipped() {
        let mut reg = EconomyRegistry::new();
        let market_id = reg.create_market();
        let item = ItemDefId(99);

        let poor_buyer = make_test_entity(1);
        seed_wallet(&mut reg, poor_buyer, 10); // 只有 10 copper

        let seller = make_test_entity(2);

        // 买单：90 coin（但只有 10）
        reg.submit_order(
            market_id,
            Order::new(poor_buyer, item, 1, 90, OrderSide::Bid, 0),
        );

        // 卖单：50 coin
        reg.submit_order(
            market_id,
            Order::new(seller, item, 1, 50, OrderSide::Ask, 0),
        );

        // 成交价 = (90 + 50) / 2 = 70，但 buyer 只有 10
        let trades = reg.match_orders(market_id, item, 1);
        assert!(trades.is_empty(), "buyer can't afford, no trade");
    }

    #[test]
    fn test_transfer_copper_updates_wallets() {
        let mut reg = EconomyRegistry::new();
        let alice = make_test_entity(1);
        let bob = make_test_entity(2);

        seed_wallet(&mut reg, alice, 500);
        seed_wallet(&mut reg, bob, 100);
        let item = ItemDefId(1);
        reg.add_items(bob, item, 1); // bob 持有物品

        // 提交订单 + 撮合
        let market_id = reg.create_market();

        reg.submit_order(
            market_id,
            Order::new(alice, item, 1, 200, OrderSide::Bid, 0),
        );
        reg.submit_order(market_id, Order::new(bob, item, 1, 150, OrderSide::Ask, 0));

        let trades = reg.match_orders(market_id, item, 1);
        assert_eq!(trades.len(), 1);

        // 成交价 = (200 + 150) / 2 = 175
        assert_eq!(reg.get_wallet(alice).unwrap().total_copper(), 500 - 175);
        assert_eq!(reg.get_wallet(bob).unwrap().total_copper(), 100 + 175);
    }

    #[test]
    fn test_ema_price_updates() {
        let mut reg = EconomyRegistry::new();
        let market_id = reg.create_market();
        let item = ItemDefId(10);
        let key = (market_id, item);

        // 第一笔交易：价格 100
        reg.record_trade(
            market_id,
            TradeRecord {
                item_id: item,
                quantity: 1,
                price_copper: 100,
                buyer_id: make_test_entity(1),
                seller_id: make_test_entity(2),
                tick: 0,
            },
        );
        assert_eq!(
            reg.get_price_snapshot(market_id, item)
                .unwrap()
                .ema_price_copper,
            100
        );

        // 第二笔交易：价格 200，EMA = 0.3*200 + 0.7*100 = 130
        reg.record_trade(
            market_id,
            TradeRecord {
                item_id: item,
                quantity: 1,
                price_copper: 200,
                buyer_id: make_test_entity(3),
                seller_id: make_test_entity(4),
                tick: 1,
            },
        );
        let snap = reg.get_price_snapshot(market_id, item).unwrap();
        assert_eq!(snap.ema_price_copper, 130);
        assert_eq!(snap.raw_last_price, 200);
        assert!(snap.volatility >= 0.0);

        // 交易历史应该有两笔
        let history = reg.trade_histories.get(&key).unwrap();
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_query_price_returns_live_data() {
        let mut reg = EconomyRegistry::new();
        let market_id = reg.create_market();
        let item = ItemDefId(5);

        // Phase 1: 无数据 → None
        assert!(EconomyQuery::query_price(&reg, market_id, item).is_none());

        // 记录交易后 → 有数据
        reg.record_trade(
            market_id,
            TradeRecord {
                item_id: item,
                quantity: 2,
                price_copper: 75,
                buyer_id: make_test_entity(1),
                seller_id: make_test_entity(2),
                tick: 1,
            },
        );
        let snap = EconomyQuery::query_price(&reg, market_id, item);
        assert!(snap.is_some());
        assert_eq!(snap.unwrap().ema_price_copper, 75);
    }

    #[test]
    fn test_match_all_markets() {
        let mut reg = EconomyRegistry::new();
        let market_id = reg.create_market();
        let item_a = ItemDefId(1);
        let item_b = ItemDefId(2);

        let buyer = make_test_entity(1);
        let seller = make_test_entity(2);
        seed_wallet(&mut reg, buyer, 10000);
        reg.add_items(seller, item_a, 1);
        reg.add_items(seller, item_b, 1);

        // 物品 A：交叉订单
        reg.submit_order(
            market_id,
            Order::new(buyer, item_a, 1, 50, OrderSide::Bid, 0),
        );
        reg.submit_order(
            market_id,
            Order::new(seller, item_a, 1, 40, OrderSide::Ask, 0),
        );

        // 物品 B：交叉订单
        reg.submit_order(
            market_id,
            Order::new(buyer, item_b, 1, 30, OrderSide::Bid, 0),
        );
        reg.submit_order(
            market_id,
            Order::new(seller, item_b, 1, 25, OrderSide::Ask, 0),
        );

        let trades = reg.match_all_markets(1);
        assert_eq!(trades.len(), 2);

        // 两物品都有价格快照
        assert!(reg.get_price_snapshot(market_id, item_a).is_some());
        assert!(reg.get_price_snapshot(market_id, item_b).is_some());
    }
}
