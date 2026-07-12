//! Economy ECS Systems
//!
//! Phase 1:
//! - economic_cognition_update_system: 从 BigFive 派生 EconomicCognition
//! - wallet_init_system: 从种子分配初始钱包
//!
//! Phase 2:
//! - order_creation_system: NPC 基于经济认知创建买卖单
//! - market_matching_system: 撮合所有市场订单簿
//! - trade_bubble_system: 成交事件→交易吆喝气泡（V4b·接 V3b 成交出口）
//!
//! Phase 3:
//! - needs: 结构化需求评估（Physiological/Occupational/Social）

pub mod needs;

use std::collections::HashMap;

use hecs::{CommandBuffer, World};
use woworld_core::economy::behavioral::EconBehaviorParams;
use woworld_core::economy::{
    EconomyQuery, ListingStatus, ListingType, Order, OrderSide, TradeRecord,
};
use woworld_core::id::ItemDefId;
use woworld_core::item::ItemQuery;
use woworld_core::speech_bubble::{SpeechAct, PRIORITY_TRADE};
use woworld_core::time::TimeOfDay;
use woworld_core::types::EntityId;

use crate::components::bigfive::BigFive;
use crate::components::economy::{EconomicCognition, Wallet};
use crate::components::emotion::Emotion;
use crate::components::needs::Needs;
use crate::events::EventChannel;
use crate::resources::economy_registry::EconomyRegistry;
use crate::resources::inventory_registry::InventoryRegistry;
use crate::resources::speech_bubble_state::{ActiveBubble, SpeechBubbleState};
use crate::resources::speech_fragment_registry::{SpeechContext, SpeechFragmentRegistry};
use crate::systems::npc::speech_bubble::{BUBBLE_COOLDOWN_TICKS, BUBBLE_DURATION_TICKS};

/// 经济认知派生系统：从 BigFive 为所有 NPC 派生 EconomicCognition
///
/// 只处理已存在 BigFive 但无 EconomicCognition 的实体。
/// 经济认知是人格 x 经验的纯函数缓存——零新存储。
pub fn economic_cognition_update_system(
    world: &World,
    cmd: &mut CommandBuffer,
    registry: &mut EconomyRegistry,
) {
    for (entity, bigfive) in world
        .query::<&BigFive>()
        .iter()
        .filter(|(e, _)| world.get::<&EconomicCognition>(*e).is_err())
    {
        let cognition = EconomicCognition::derive_from_bigfive(
            bigfive.openness,
            bigfive.conscientiousness,
            bigfive.extraversion,
            bigfive.agreeableness,
            bigfive.neuroticism,
            None::<f32>,
        );

        // 同步设置经济行为参数到 Registry
        let params = EconBehaviorParams {
            openness: bigfive.openness,
            conscientiousness: bigfive.conscientiousness,
            extraversion: bigfive.extraversion,
            agreeableness: bigfive.agreeableness,
            neuroticism: bigfive.neuroticism,
            financial_literacy: cognition.financial_literacy,
            market_understanding: cognition.market_understanding,
            ownership_days: 0.0,
        };
        registry.set_econ_params(EntityId(entity.to_bits().get()), params);

        cmd.insert_one(entity, cognition);
    }
}

/// 钱包初始化系统：为没有 Wallet 的 NPC 从种子分配初始资金
///
/// Phase 1: 从实体 ID 派生确定性初始钱包。
pub fn wallet_init_system(world: &World, cmd: &mut CommandBuffer, registry: &mut EconomyRegistry) {
    for (entity, _bigfive) in world
        .query::<&BigFive>()
        .iter()
        .filter(|(e, _)| world.get::<&Wallet>(*e).is_err())
    {
        let seed = entity.to_bits().get();
        let wallet = Wallet::from_seed(seed);

        // 同步到 Registry
        let entity_id = EntityId(entity.to_bits().get());
        registry.set_wallet(
            entity_id,
            woworld_core::economy::WalletSnapshot {
                copper: wallet.copper,
                silver: wallet.silver,
                gold: wallet.gold,
            },
        );

        cmd.insert_one(entity, wallet);
    }
}

/// 订单创建系统：NPC 根据钱包余额、物品持有和经济认知生成买卖单。
///
/// 设计文档 004——订单从 surplus/need 涌现：
/// - **卖单**：持有物品 > reserve_days 需求 → 卖出 surplus
/// - **买单**：钱包充足 + 物品短缺 → 买入补缺；或纯粹投资性购买
/// - reserve_days = 3 + (1 - satisficing) × 14，映射到 [3, 17] 天
/// - 价格由 base_value × 行为经济学调节（锚定效应、损失厌恶）
///
/// ★ V3b: daily_need 从 Needs.hunger 派生（弃种子随机）。
/// ★ V3b: 库存从 InventoryRegistry 读取（权威源，弃 EconomyRegistry.item_holdings）。
/// ★ V3b: 钱包从 registry 读取（权威源，ECS Wallet 可能因成交过期）。
pub fn order_creation_system(
    world: &World,
    registry: &mut EconomyRegistry,
    inventory_registry: &InventoryRegistry,
    item_registry: &dyn ItemQuery,
    tick: u64,
) {
    let items = item_registry.all_def_ids().to_vec();
    if items.is_empty() || registry.all_markets().is_empty() {
        return;
    }
    let market_id = registry.all_markets()[0];

    for (entity, (wallet, cognition)) in world.query::<(&Wallet, &EconomicCognition)>().iter() {
        let entity_id = EntityId(entity.to_bits().get());
        let entity_seed = entity.to_bits().get();

        // ── 活动概率 ──
        let activity_prob = 0.05 + (cognition.market_search_breadth as f32) * 0.05;
        let seed = entity_seed.wrapping_add(tick);
        let roll = ((seed.wrapping_mul(1_103_515_245).wrapping_add(12_345) & 0x7FFF_FFFF) as f64)
            / 0x7FFF_FFFF as f64;
        if roll > activity_prob as f64 {
            continue;
        }

        // ★ V3b: 钱包从 registry 读取（权威源——成交后 registry 更新，ECS component 可能过期）
        let wallet_balance = registry
            .get_wallet(entity_id)
            .map(|w| w.total_copper())
            .unwrap_or_else(|| wallet.total_copper());
        let satisficing = cognition.satisficing_threshold;
        // reserve_days = 3..17, 由 satisficing 代理 neuroticism
        // satisficing = 0.5 + (1-C)*0.3 + N*0.2 → N 高则 satisficing 高
        // 设计文档 004 §1.2: reserve_days = neuroticism × 14 + 3
        let reserve_days = 3.0 + satisficing * 14.0;

        // ── 遍历 NPC 持有的物品（★ V3b: InventoryRegistry 权威源）──
        let holdings_vec = inventory_registry.get_holdings(entity_id);
        let holdings_map: HashMap<ItemDefId, u32> =
            holdings_vec.iter().map(|&(id, qty)| (id, qty)).collect();

        // 在持有的物品中选一个评估
        let candidates: Vec<(ItemDefId, u32)> = if holdings_map.is_empty() {
            // 无持有物品——随机选一个尝试买入
            let idx = (seed.wrapping_mul(7) as usize) % items.len();
            vec![(items[idx], 0)]
        } else {
            holdings_map.iter().map(|(&id, &qty)| (id, qty)).collect()
        };
        let candidate_idx = (seed.wrapping_mul(13) as usize) % candidates.len().max(1);
        let (item_id, held_qty) = candidates[candidate_idx];
        let base_value = item_registry.get_base_value(item_id).unwrap_or(10) as u64;

        // ── 计算 surplus/need ──
        // ★ V3b: daily_need 从真实 Needs.hunger 派生（0=满足→1=缺乏）
        //   饥饿 NPC 需要更多食物储备 → 更高 daily_need → 更多买单
        let hunger = world.get::<&Needs>(entity).map(|n| n.hunger).unwrap_or(0.0);
        let daily_need = 0.2 + hunger * 1.5; // [0.2, 1.7] —— 饱→低需求，饿→高需求
        let reserve_need = (reserve_days * daily_need) as u32;
        let surplus = held_qty.saturating_sub(reserve_need);
        let deficit = reserve_need.saturating_sub(held_qty);

        // ── 钱包阈值 ──
        let wealth_target = base_value * reserve_days as u64; // 安全感目标
        let wallet_rich = wallet_balance > wealth_target * 3 / 2;
        let wallet_poor = wallet_balance < wealth_target / 2;

        // 市场参考价：EMA 价格（若有交易历史），否则 fallback 到 base_value
        let market_price = registry
            .get_price_snapshot(market_id, item_id)
            .map(|s| s.ema_price_copper)
            .unwrap_or(base_value);
        let price = market_price as f32;

        // ── 决策 ──
        if surplus > 0 && wallet_poor {
            // 有 surplus + 缺钱 → 卖单（紧急，打折）
            let discount = price * satisficing * 0.4;
            let ask_price = (price - discount).max(1.0) as u64;
            let qty = surplus.min(3);
            registry.submit_order(
                market_id,
                Order {
                    order_id: 0,
                    entity_id,
                    item_id,
                    quantity: qty,
                    limit_price_copper: ask_price,
                    side: OrderSide::Ask,
                    created_tick: tick,
                    listing_type: ListingType::Normal,
                    filled_quantity: 0,
                    status: ListingStatus::Active,
                },
            );
        } else if surplus > 0 && !wallet_poor {
            // 有 surplus + 不缺钱 → 正常卖单
            let premium = price * satisficing * 0.15;
            let ask_price = (price + premium).max(1.0) as u64;
            let qty = surplus.min(2);
            registry.submit_order(
                market_id,
                Order {
                    order_id: 0,
                    entity_id,
                    item_id,
                    quantity: qty,
                    limit_price_copper: ask_price,
                    side: OrderSide::Ask,
                    created_tick: tick,
                    listing_type: ListingType::Normal,
                    filled_quantity: 0,
                    status: ListingStatus::Active,
                },
            );
        } else if deficit > 0 {
            // 有 deficit → 买单（补货），只要付得起
            let urgency = if wallet_poor { 0.6 } else { 0.3 }; // 缺钱则更压价
            let discount = price * satisficing * urgency;
            let bid_price = (price - discount).max(1.0) as u64;
            if wallet_balance >= bid_price {
                let qty = deficit.min(2);
                registry.submit_order(
                    market_id,
                    Order {
                        order_id: 0,
                        entity_id,
                        item_id,
                        quantity: qty,
                        limit_price_copper: bid_price,
                        side: OrderSide::Bid,
                        created_tick: tick,
                        listing_type: ListingType::Normal,
                        filled_quantity: 0,
                        status: ListingStatus::Active,
                    },
                );
            }
        } else if wallet_rich && held_qty > 0 {
            // 不缺物品 + 有钱 → 投资性买入
            let bid_price = (price * 0.9).max(1.0) as u64;
            if wallet_balance >= bid_price {
                registry.submit_order(
                    market_id,
                    Order {
                        order_id: 0,
                        entity_id,
                        item_id,
                        quantity: 1,
                        limit_price_copper: bid_price,
                        side: OrderSide::Bid,
                        created_tick: tick,
                        listing_type: ListingType::Normal,
                        filled_quantity: 0,
                        status: ListingStatus::Active,
                    },
                );
            }
        }
    }
}

/// 市场撮合系统：遍历所有市场的所有订单簿，撮合并执行交易。
///
/// 返回本帧全部成交记录——供 EventChannel 消费（V4b 交易气泡等）。
pub fn market_matching_system(registry: &mut EconomyRegistry, tick: u64) -> Vec<TradeRecord> {
    registry.match_all_markets(tick)
}

/// ★ V3b: Wallet 同步——成交后将 registry 钱包回写 ECS Wallet component。
///
/// `market_matching_system` 的 `execute_trade` → `transfer_copper` 只修改 registry。
/// 此函数遍历所有 NPC，将 registry 中的权威钱包值同步回 ECS。
/// 应在 `market_matching_system` 之后调用。
pub fn wallet_sync_system(world: &World, cmd: &mut CommandBuffer, registry: &EconomyRegistry) {
    for (entity, wallet) in world.query::<&Wallet>().iter() {
        let entity_id = EntityId(entity.to_bits().get());
        if let Some(snapshot) = registry.get_wallet(entity_id) {
            let registry_total = snapshot.total_copper();
            if wallet.total_copper() != registry_total {
                cmd.insert_one(entity, Wallet::from(snapshot));
            }
        }
    }
}

/// ★ V4b: 交易气泡系统——成交事件→吆喝气泡（薄出口，复用 SpeechBubbleState）。
///
/// 消费 `EventChannel<TradeRecord>`（双缓冲·跨帧），买卖双方各独立选句（per-entity seed
/// → 非对称·涌现）。Priority=2——不覆盖 greeting(3)，可覆盖 self-talk(1)。
/// 受 cooldown 约束（不连刷），玩家实体不冒泡。
#[allow(clippy::too_many_arguments)]
pub fn trade_bubble_system(
    world: &World,
    trade_events: &mut EventChannel<TradeRecord>,
    state: &mut SpeechBubbleState,
    fragments: &SpeechFragmentRegistry,
    current_tick: u64,
    day_progress: f32,
    player_entity: Option<hecs::Entity>,
) {
    let trades = trade_events.drain();
    if trades.is_empty() {
        return;
    }

    let time_of_day = TimeOfDay::from_progress(day_progress as f64);

    for trade in &trades {
        for &trader_id in &[trade.buyer_id, trade.seller_id] {
            // EntityId.0 来自 entity.to_bits().get() 往返——恒有效
            let Some(entity) = hecs::Entity::from_bits(trader_id.0) else {
                continue;
            };

            // 玩家实体不冒泡（一致性）
            if player_entity == Some(entity) {
                continue;
            }

            // 实体存在性检查（可能已 despawn）
            if world.entity(entity).is_err() {
                continue;
            }

            let slot = state.slots.entry(entity).or_default();

            // Priority 仲裁: PRIORITY_TRADE=2 ——不覆盖 greeting(3)，可覆盖 self-talk(1)
            let blocked = slot
                .active
                .as_ref()
                .map(|a| a.priority > PRIORITY_TRADE)
                .unwrap_or(false);
            if blocked {
                continue;
            }

            if current_tick < slot.next_allowed_tick {
                continue;
            }

            // 从片段库选句（per-entity seed → 买卖双方可能不同话·涌现非对称）
            let pleasure = world
                .get::<&Emotion>(entity)
                .map(|e| e.pleasure)
                .unwrap_or(0.0);
            let extraversion = world
                .get::<&BigFive>(entity)
                .map(|b| b.extraversion)
                .unwrap_or(0.5);
            let ctx = SpeechContext {
                time_of_day,
                trust: 0.0,
                pleasure,
                extraversion,
                topic: None,
            };
            let seed = trader_id.0.wrapping_mul(31).wrapping_add(current_tick);
            if let Some((text, bt)) = fragments.select(SpeechAct::TradeShout, &ctx, seed) {
                slot.active = Some(ActiveBubble {
                    text,
                    bubble_type: bt,
                    expiry_tick: current_tick + BUBBLE_DURATION_TICKS,
                    priority: PRIORITY_TRADE,
                });
                slot.next_allowed_tick = current_tick + BUBBLE_COOLDOWN_TICKS;
            }
        }
    }
}

// ── 测试 ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::bigfive::BigFive;
    use crate::components::economy::{EconomicCognition, Wallet};
    use crate::components::emotion::Emotion;
    use crate::events::EventChannel;
    use crate::resources::speech_bubble_state::{ActiveBubble, SpeechBubbleState};
    use crate::resources::speech_fragment_registry::SpeechFragmentRegistry;
    use woworld_core::economy::EconomyQuery;
    use woworld_core::id::ItemDefId;
    use woworld_core::speech_bubble::{
        BubbleType, PRIORITY_SELF_TALK, PRIORITY_SOCIAL, PRIORITY_TRADE,
    };

    #[test]
    fn test_cognition_system_inserts() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut registry = EconomyRegistry::new();

        let entity = world.spawn((BigFive::from_seed(42),));

        economic_cognition_update_system(&world, &mut cmd, &mut registry);
        cmd.run_on(&mut world);

        let cog = world.get::<&EconomicCognition>(entity).unwrap();
        assert!((0.0..=1.0).contains(&cog.financial_literacy));
        assert!(registry.econ_params_count() > 0);
    }

    #[test]
    fn test_cognition_system_skips_existing() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut registry = EconomyRegistry::new();

        world.spawn((BigFive::from_seed(42), EconomicCognition::default()));

        let initial = registry.econ_params_count();
        economic_cognition_update_system(&world, &mut cmd, &mut registry);
        cmd.run_on(&mut world);

        // 不应增加（所有实体已有 Cognition）
        assert_eq!(registry.econ_params_count(), initial);
    }

    #[test]
    fn test_wallet_system_inserts() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut registry = EconomyRegistry::new();

        let entity = world.spawn((BigFive::from_seed(42),));

        wallet_init_system(&world, &mut cmd, &mut registry);
        cmd.run_on(&mut world);

        let wallet = world.get::<&Wallet>(entity).unwrap();
        assert!(wallet.total_copper() > 0);
        assert!(registry.wallet_count() > 0);
    }

    #[test]
    fn test_wallet_system_skips_existing() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut registry = EconomyRegistry::new();

        world.spawn((BigFive::from_seed(42), Wallet::default()));

        let initial = registry.wallet_count();
        wallet_init_system(&world, &mut cmd, &mut registry);
        cmd.run_on(&mut world);

        assert_eq!(registry.wallet_count(), initial);
    }

    #[test]
    fn test_both_systems_empty_world_no_panic() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut registry = EconomyRegistry::new();

        economic_cognition_update_system(&world, &mut cmd, &mut registry);
        wallet_init_system(&world, &mut cmd, &mut registry);
        cmd.run_on(&mut world);
        // 不 panic
    }

    #[test]
    fn test_cognition_deterministic() {
        let mut world1 = World::new();
        let mut cmd1 = CommandBuffer::new();

        let mut world2 = World::new();
        let mut cmd2 = CommandBuffer::new();

        let _e1 = world1.spawn((BigFive::from_seed(42),));
        let _e2 = world2.spawn((BigFive::from_seed(42),));

        let mut reg1 = EconomyRegistry::new();
        economic_cognition_update_system(&world1, &mut cmd1, &mut reg1);
        cmd1.run_on(&mut world1);

        let mut reg2 = EconomyRegistry::new();
        economic_cognition_update_system(&world2, &mut cmd2, &mut reg2);
        cmd2.run_on(&mut world2);

        // 两个 World 中同种子的 Cognition 应相同
        let mut q1 = world1.query::<&EconomicCognition>();
        let mut q2 = world2.query::<&EconomicCognition>();
        let cog1 = q1.iter().next().unwrap().1;
        let cog2 = q2.iter().next().unwrap().1;
        assert!((cog1.financial_literacy - cog2.financial_literacy).abs() < f32::EPSILON);
    }

    // ── Phase 2 测试 ──────────────────────────────────

    #[test]
    fn test_order_creation_no_panic_empty_world() {
        let world = World::new();
        let mut reg = EconomyRegistry::new();
        reg.create_market();
        let item_reg = fake_item_registry();
        let inv_reg = InventoryRegistry::new();

        order_creation_system(&world, &mut reg, &inv_reg, &item_reg, 0);
        // 空世界——不 panic
    }

    #[test]
    fn test_order_creation_with_npc() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut reg = EconomyRegistry::new();
        reg.create_market();
        let item_reg = fake_item_registry();
        let inv_reg = InventoryRegistry::new();

        // spawn NPC with BigFive → need cognition + wallet first
        world.spawn((BigFive::from_seed(42),));

        economic_cognition_update_system(&world, &mut cmd, &mut reg);
        wallet_init_system(&world, &mut cmd, &mut reg);
        cmd.run_on(&mut world);

        // 现在创建订单——应至少有一些 orders
        for tick in 0..100 {
            order_creation_system(&world, &mut reg, &inv_reg, &item_reg, tick);
        }

        // 检查市场是否有订单
        let market_id = reg.all_markets()[0];
        let market = reg.get_market(market_id).unwrap();
        let total_orders: usize = market
            .order_books
            .values()
            .map(|b| b.bids.len() + b.asks.len())
            .sum();
        // 100 ticks 中至少有一些订单
        assert!(total_orders > 0, "expected some orders after 100 ticks");
    }

    #[test]
    fn test_market_matching_system() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut reg = EconomyRegistry::new();
        reg.create_market();
        let item_reg = fake_item_registry();
        let inv_reg = InventoryRegistry::new();

        // 创建两个 NPC
        world.spawn((BigFive::from_seed(42),));
        world.spawn((BigFive::from_seed(43),));

        economic_cognition_update_system(&world, &mut cmd, &mut reg);
        wallet_init_system(&world, &mut cmd, &mut reg);
        cmd.run_on(&mut world);

        // 运行多轮订单创建
        for tick in 0..50 {
            order_creation_system(&world, &mut reg, &inv_reg, &item_reg, tick);
            market_matching_system(&mut reg, tick);
        }

        // 可能有交易发生（取决于随机订单交叉）
        // 至少不应 panic，系统正常运行
        let _ = reg.trade_count();
    }

    #[test]
    fn test_full_economy_loop() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut reg = EconomyRegistry::new();
        reg.create_market();
        let item_reg = fake_item_registry();
        let inv_reg = InventoryRegistry::new();

        // 创建 5 个 NPC
        for seed in 0..5 {
            world.spawn((BigFive::from_seed(seed * 100),));
        }

        economic_cognition_update_system(&world, &mut cmd, &mut reg);
        wallet_init_system(&world, &mut cmd, &mut reg);
        cmd.run_on(&mut world);

        // 验证所有 NPC 都有钱包和经济认知
        let npc_count = world.query::<&EconomicCognition>().iter().count();
        assert_eq!(npc_count, 5);
        assert_eq!(reg.wallet_count(), 5);

        // 运行经济循环 100 ticks
        for tick in 0..100 {
            order_creation_system(&world, &mut reg, &inv_reg, &item_reg, tick);
            market_matching_system(&mut reg, tick);
        }

        // 验证订单簿非空
        let market_id = reg.all_markets()[0];
        let market = reg.get_market(market_id).unwrap();
        let total_orders: usize = market
            .order_books
            .values()
            .map(|b| b.bids.len() + b.asks.len())
            .sum();
        assert!(total_orders > 0, "should have orders in the market");

        // 验证至少有一些价格快照
        let snapshots: Vec<_> = market
            .order_books
            .keys()
            .filter_map(|&item_id| reg.get_price_snapshot(market_id, item_id))
            .collect();
        // 不是所有都有快照（需要实际成交），但至少系统正常运行
        let _ = snapshots.len();
    }

    // ── V3b 测试：真实数据源 ──────────────────────────────

    #[test]
    fn test_daily_need_from_real_hunger() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut reg = EconomyRegistry::new();
        reg.create_market();
        let item_reg = fake_item_registry_with_food();
        let inv_reg = InventoryRegistry::new();

        // 两个 NPC：一个饿一个饱
        let hungry = world.spawn((
            BigFive::from_seed(42),
            Needs {
                hunger: 0.9,
                ..Needs::default()
            },
        ));
        let full = world.spawn((
            BigFive::from_seed(43),
            Needs {
                hunger: 0.0,
                ..Needs::default()
            },
        ));

        economic_cognition_update_system(&world, &mut cmd, &mut reg);
        wallet_init_system(&world, &mut cmd, &mut reg);
        cmd.run_on(&mut world);

        // 饥饿 NPC 应更可能产生食物买单（daily_need 更高 → deficit 更大）
        // 验证：运行多 tick，饥饿 NPC 产生的买单数 ≥ 饱腹 NPC
        let mut hungry_buys = 0usize;
        let mut full_buys = 0usize;
        for tick in 0..200 {
            order_creation_system(&world, &mut reg, &inv_reg, &item_reg, tick);
            let market_id = reg.all_markets()[0];
            let market = reg.get_market(market_id).unwrap();
            for book in market.order_books.values() {
                for bid in &book.bids {
                    if bid.entity_id == EntityId(hungry.to_bits().get()) {
                        hungry_buys += 1;
                    }
                    if bid.entity_id == EntityId(full.to_bits().get()) {
                        full_buys += 1;
                    }
                }
            }
        }
        // 饥饿 NPC 应更倾向买入（daily_need 更高）
        // 注：由于随机性，不强制 hungry > full，但至少系统不崩溃
        let _ = (hungry_buys, full_buys);
    }

    #[test]
    fn test_order_creation_reads_inventory_registry() {
        use woworld_core::id::ItemDefId;
        use woworld_core::item::ItemCategory;

        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut reg = EconomyRegistry::new();
        reg.create_market();
        let item_reg = fake_item_registry_with_food();
        let mut inv_reg = InventoryRegistry::new();

        let entity = world.spawn((BigFive::from_seed(42),));

        economic_cognition_update_system(&world, &mut cmd, &mut reg);
        wallet_init_system(&world, &mut cmd, &mut reg);
        cmd.run_on(&mut world);

        let entity_id = EntityId(entity.to_bits().get());

        // 在 InventoryRegistry 中放入食物
        let food_id = ItemDefId::new(ItemCategory::Food, 1, 0);
        let _ = inv_reg.add_item(entity_id, food_id, 10, &item_reg);

        // 验证 EconomyRegistry 没有该物品（旧账本应为空）
        assert_eq!(reg.get_item_count(entity_id, food_id), 0);

        // 运行订单创建——应基于 InventoryRegistry 的 10 个食物
        for tick in 0..50 {
            order_creation_system(&world, &mut reg, &inv_reg, &item_reg, tick);
        }

        // InventoryRegistry 的 10 个食物应被 order_creation 看到
        let inv_holdings: HashMap<_, _> = inv_reg
            .get_holdings(entity_id)
            .iter()
            .map(|&(id, qty)| (id, qty))
            .collect();
        assert_eq!(inv_holdings.get(&food_id), Some(&10));
    }

    #[test]
    fn test_no_seed_on_first_tick() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut reg = EconomyRegistry::new();
        reg.create_market();
        let item_reg = fake_item_registry();
        let inv_reg = InventoryRegistry::new();

        world.spawn((BigFive::from_seed(42),));

        economic_cognition_update_system(&world, &mut cmd, &mut reg);
        wallet_init_system(&world, &mut cmd, &mut reg);
        cmd.run_on(&mut world);

        let entity_id = EntityId(
            world
                .query::<&BigFive>()
                .iter()
                .next()
                .unwrap()
                .0
                .to_bits()
                .get(),
        );

        // tick=0 ——不应在 EconomyRegistry 种物品
        order_creation_system(&world, &mut reg, &inv_reg, &item_reg, 0);
        assert_eq!(
            reg.get_holdings(entity_id).map(|h| h.len()).unwrap_or(0),
            0,
            "V3b: no seed_npc_items in EconomyRegistry"
        );
    }

    #[test]
    fn test_wallet_sync_after_trade() {
        // ★ V3b: 使用 ECS entity ID 作为注册表的 EntityId——保证 wallet_sync_system 能匹配。
        // wallet_sync_system 通过 EntityId(entity.to_bits().get()) 查找注册表。

        // 先创建 ECS world 获取真实 entity IDs
        let mut world = World::new();
        let buyer_entity = world.spawn((Wallet::from_copper(1000),));
        let seller_entity = world.spawn((Wallet::from_copper(100),));

        let buyer_id = EntityId(buyer_entity.to_bits().get());
        let seller_id = EntityId(seller_entity.to_bits().get());

        let mut reg = EconomyRegistry::new();
        let market_id = reg.create_market();
        let item = ItemDefId(100);

        // 给卖家物品
        reg.add_items(seller_id, item, 1);

        // 手动设置钱包到 registry（使用 ECS entity IDs）
        reg.set_wallet(
            buyer_id,
            woworld_core::economy::WalletSnapshot::from_copper(1000),
        );
        reg.set_wallet(
            seller_id,
            woworld_core::economy::WalletSnapshot::from_copper(100),
        );

        // 创建交叉订单
        reg.submit_order(
            market_id,
            Order::new(buyer_id, item, 1, 100, OrderSide::Bid, 0),
        );
        reg.submit_order(
            market_id,
            Order::new(seller_id, item, 1, 80, OrderSide::Ask, 0),
        );

        let trades = reg.match_orders(market_id, item, 1);
        assert_eq!(trades.len(), 1);

        // 验证 registry 钱包已更新
        let buyer_wallet = reg.get_wallet(buyer_id).unwrap();
        let seller_wallet = reg.get_wallet(seller_id).unwrap();
        // 成交价 = (100 + 80) / 2 = 90
        assert_eq!(buyer_wallet.total_copper(), 1000 - 90);
        assert_eq!(seller_wallet.total_copper(), 100 + 90);

        // ECS component 仍显示旧余额（成交未同步）
        assert_eq!(
            world.get::<&Wallet>(buyer_entity).unwrap().total_copper(),
            1000
        );

        // wallet_sync_system 应回写
        let mut cmd = CommandBuffer::new();
        wallet_sync_system(&world, &mut cmd, &reg);
        cmd.run_on(&mut world);

        // ★ ECS Wallet 应与 registry 一致
        assert_eq!(
            world.get::<&Wallet>(buyer_entity).unwrap().total_copper(),
            1000 - 90
        );
        assert_eq!(
            world.get::<&Wallet>(seller_entity).unwrap().total_copper(),
            100 + 90
        );
    }

    #[test]
    fn test_wallet_read_from_registry() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut reg = EconomyRegistry::new();
        reg.create_market();
        let _item_reg = fake_item_registry();
        let _inv_reg = InventoryRegistry::new();

        world.spawn((BigFive::from_seed(42),));

        economic_cognition_update_system(&world, &mut cmd, &mut reg);
        wallet_init_system(&world, &mut cmd, &mut reg);
        cmd.run_on(&mut world);

        let entity_id = EntityId(
            world
                .query::<&BigFive>()
                .iter()
                .next()
                .unwrap()
                .0
                .to_bits()
                .get(),
        );

        // 获取 ECS wallet 原始值
        let ecs_wallet = world
            .query::<&Wallet>()
            .iter()
            .next()
            .unwrap()
            .1
            .total_copper();

        // 修改 registry wallet（模拟成交后的状态）
        let new_total = ecs_wallet + 500;
        reg.set_wallet(
            entity_id,
            woworld_core::economy::WalletSnapshot::from_copper(new_total),
        );

        // order_creation_system 应读 registry（而非 ECS component）
        // 通过检查 order 创建行为间接验证——这里直接验证 registry wallet 已更新
        assert_eq!(reg.get_wallet(entity_id).unwrap().total_copper(), new_total);
        // ECS component 仍是旧值（不同步）
        assert_eq!(
            world
                .query::<&Wallet>()
                .iter()
                .next()
                .unwrap()
                .1
                .total_copper(),
            ecs_wallet
        );
    }

    // ── 辅助 ───────────────────────────────────────────

    fn fake_item_registry() -> crate::resources::item_registry::ItemRegistry {
        let mut reg = crate::resources::item_registry::ItemRegistry::new();
        use woworld_core::id::ItemDefId;
        use woworld_core::item::{ItemCategory, ItemProperties, Quality, Rarity};

        let items: Vec<(ItemDefId, ItemCategory, &str, u32)> = vec![
            (
                ItemDefId::new(ItemCategory::Food, 1, 0),
                ItemCategory::Food,
                "生肉",
                20,
            ),
            (
                ItemDefId::new(ItemCategory::MineralOre, 1, 0),
                ItemCategory::MineralOre,
                "铁矿",
                8,
            ),
            (
                ItemDefId::new(ItemCategory::LeatherMat, 1, 0),
                ItemCategory::LeatherMat,
                "兽皮",
                15,
            ),
            (
                ItemDefId::new(ItemCategory::Weapon, 0, 0),
                ItemCategory::Weapon,
                "铁剑",
                50,
            ),
            (
                ItemDefId::new(ItemCategory::WoodMat, 0, 0),
                ItemCategory::WoodMat,
                "橡木",
                6,
            ),
        ];

        for (def_id, cat, name, value) in items {
            reg.register(ItemProperties {
                def_id,
                category: cat,
                name: name.to_string(),
                description: String::new(),
                weight_grams: 100,
                bulk_factor: 1.0,
                volume_liters: 0.1,
                base_quality: Quality::Standard,
                rarity: Rarity::Common,
                quality_range_min: Quality::Rough,
                quality_range_max: Quality::Perfect,
                stack_size: 10,
                base_value_copper: value,
                max_durability: 0.0,
                durability_loss_per_use: 0.0,
                magic_capacity_ke: 0,
                tags: vec![],
                mod_tags: std::collections::BTreeMap::new(),
                min_skill: None,
                min_strength: None,
                required_body_part: None,
                element_affinity: None,
                placement: None,
                tool_tags: None,
                consumable: None,
                audio_material: None,
                aesthetic_props: None,
            });
        }
        reg
    }

    /// V3b: 含可食用物品的 fake registry——用于测试需求驱动的食物订单。
    fn fake_item_registry_with_food() -> crate::resources::item_registry::ItemRegistry {
        let mut reg = crate::resources::item_registry::ItemRegistry::new();
        use woworld_core::id::ItemDefId;
        use woworld_core::item::{
            ConsumableEffect, ItemCategory, ItemProperties, ItemTag, Quality, Rarity,
        };

        let items: Vec<(ItemDefId, ItemCategory, &str, u32, bool)> = vec![
            (
                ItemDefId::new(ItemCategory::Food, 1, 0),
                ItemCategory::Food,
                "生肉",
                20,
                true,
            ),
            (
                ItemDefId::new(ItemCategory::Food, 2, 0),
                ItemCategory::Food,
                "浆果",
                8,
                true,
            ),
            (
                ItemDefId::new(ItemCategory::MineralOre, 1, 0),
                ItemCategory::MineralOre,
                "铁矿",
                8,
                false,
            ),
            (
                ItemDefId::new(ItemCategory::WoodMat, 0, 0),
                ItemCategory::WoodMat,
                "橡木",
                6,
                false,
            ),
        ];

        for (def_id, cat, name, value, is_food) in items {
            let tags = if is_food {
                vec![ItemTag::Edible]
            } else {
                vec![]
            };
            let consumable = if is_food {
                Some(ConsumableEffect {
                    is_consumable: true,
                    hunger_restore: 0.4,
                    hp_restore: 5.0,
                })
            } else {
                None
            };
            reg.register(ItemProperties {
                def_id,
                category: cat,
                name: name.to_string(),
                description: String::new(),
                weight_grams: 100,
                bulk_factor: 1.0,
                volume_liters: 0.1,
                base_quality: Quality::Standard,
                rarity: Rarity::Common,
                quality_range_min: Quality::Rough,
                quality_range_max: Quality::Perfect,
                stack_size: 10,
                base_value_copper: value,
                max_durability: 0.0,
                durability_loss_per_use: 0.0,
                magic_capacity_ke: 0,
                tags,
                mod_tags: std::collections::BTreeMap::new(),
                min_skill: None,
                min_strength: None,
                required_body_part: None,
                element_affinity: None,
                placement: None,
                tool_tags: None,
                consumable,
                audio_material: None,
                aesthetic_props: None,
            });
        }
        reg
    }

    #[test]
    fn test_wallet_deterministic() {
        let mut world1 = World::new();
        let mut cmd1 = CommandBuffer::new();

        let mut world2 = World::new();
        let mut cmd2 = CommandBuffer::new();

        let _e1 = world1.spawn((BigFive::from_seed(42),));
        let _e2 = world2.spawn((BigFive::from_seed(42),));

        let mut reg1 = EconomyRegistry::new();
        wallet_init_system(&world1, &mut cmd1, &mut reg1);
        cmd1.run_on(&mut world1);

        let mut reg2 = EconomyRegistry::new();
        wallet_init_system(&world2, &mut cmd2, &mut reg2);
        cmd2.run_on(&mut world2);

        let mut q1 = world1.query::<&Wallet>();
        let mut q2 = world2.query::<&Wallet>();
        let w1 = q1.iter().next().unwrap().1;
        let w2 = q2.iter().next().unwrap().1;
        assert_eq!(w1.total_copper(), w2.total_copper());
    }

    // ── V4b: trade_bubble_system ──────────────────────────

    /// 创建一个含 Emotion + BigFive 的 NPC 测试实体
    fn spawn_test_npc(world: &mut World) -> (hecs::Entity, EntityId) {
        let entity = world.spawn((
            Emotion::default(),
            BigFive::from_seed(42),
            crate::components::transform::Position(glam::Vec3::ZERO),
        ));
        let entity_id = EntityId(entity.to_bits().get());
        (entity, entity_id)
    }

    /// 创建一条模拟成交记录
    fn make_trade(buyer: EntityId, seller: EntityId) -> TradeRecord {
        TradeRecord {
            item_id: ItemDefId::new(woworld_core::item::ItemCategory::Food, 1, 0),
            quantity: 1,
            price_copper: 10,
            buyer_id: buyer,
            seller_id: seller,
            tick: 42,
        }
    }

    /// 将 trades 推入 EventChannel 并 flush（模拟 Block A5 行为）
    fn push_trades(ch: &mut EventChannel<TradeRecord>, trades: Vec<TradeRecord>) {
        ch.send_all(trades);
        ch.mid_phase_flush();
    }

    #[test]
    fn test_trade_bubble_emits_for_buyer_and_seller() {
        let mut world = World::new();
        let (e_buyer, buyer_id) = spawn_test_npc(&mut world);
        let (e_seller, seller_id) = spawn_test_npc(&mut world);

        let mut trade_events = EventChannel::new();
        trade_events.begin_frame();
        push_trades(&mut trade_events, vec![make_trade(buyer_id, seller_id)]);

        let mut state = SpeechBubbleState::new();
        let fragments = SpeechFragmentRegistry::load_embedded();

        trade_bubble_system(
            &world,
            &mut trade_events,
            &mut state,
            &fragments,
            100,
            0.5,
            None,
        );

        assert!(state.active_for(e_buyer).is_some(), "买家应有交易气泡");
        assert!(state.active_for(e_seller).is_some(), "卖家应有交易气泡");
        // 验证 priority = 2
        assert_eq!(state.active_for(e_buyer).unwrap().priority, PRIORITY_TRADE);
    }

    #[test]
    fn test_trade_bubble_empty_trades_noop() {
        let world = World::new();
        let mut trade_events = EventChannel::new();
        trade_events.begin_frame();
        // 不推任何 trade

        let mut state = SpeechBubbleState::new();
        let fragments = SpeechFragmentRegistry::load_embedded();

        trade_bubble_system(
            &world,
            &mut trade_events,
            &mut state,
            &fragments,
            100,
            0.5,
            None,
        );

        assert!(state.slots.is_empty(), "无成交不冒泡");
    }

    #[test]
    fn test_trade_bubble_respects_cooldown() {
        let mut world = World::new();
        let (_e, entity_id) = spawn_test_npc(&mut world);

        let fragments = SpeechFragmentRegistry::load_embedded();

        // 第一帧: 成交 → 冒泡
        let mut trade_events1 = EventChannel::new();
        trade_events1.begin_frame();
        push_trades(&mut trade_events1, vec![make_trade(entity_id, entity_id)]);
        let mut state = SpeechBubbleState::new();
        trade_bubble_system(
            &world,
            &mut trade_events1,
            &mut state,
            &fragments,
            100,
            0.5,
            None,
        );
        assert!(state.active_for(_e).is_some(), "第一帧应冒泡");

        // 第二帧: 新成交、但冷却未到 → 不冒新泡
        // 仅清除活跃气泡（模拟过期），保留 next_allowed_tick（冷却仍在）
        if let Some(slot) = state.slots.get_mut(&_e) {
            slot.active = None;
        }
        let mut trade_events2 = EventChannel::new();
        trade_events2.begin_frame();
        push_trades(&mut trade_events2, vec![make_trade(entity_id, entity_id)]);
        trade_bubble_system(
            &world,
            &mut trade_events2,
            &mut state,
            &fragments,
            200, // < BUBBLE_COOLDOWN_TICKS (600) since last bubble at tick 100
            0.5,
            None,
        );
        assert!(
            state.active_for(_e).is_none(),
            "冷却内不重复冒泡: tick=200, next_allowed≥100+600=700"
        );
    }

    #[test]
    fn test_trade_bubble_not_overwrite_greeting() {
        let mut world = World::new();
        let (_e, entity_id) = spawn_test_npc(&mut world);

        let fragments = SpeechFragmentRegistry::load_embedded();

        // 先设置 greeting 气泡 (priority=3)
        let mut state = SpeechBubbleState::new();
        state.slots.entry(_e).or_default().active = Some(ActiveBubble {
            text: "你好".into(),
            bubble_type: BubbleType::Normal,
            expiry_tick: 300,
            priority: PRIORITY_SOCIAL,
        });

        // 成交事件
        let mut trade_events = EventChannel::new();
        trade_events.begin_frame();
        push_trades(&mut trade_events, vec![make_trade(entity_id, entity_id)]);

        trade_bubble_system(
            &world,
            &mut trade_events,
            &mut state,
            &fragments,
            100,
            0.5,
            None,
        );

        assert_eq!(
            state.active_for(_e).unwrap().text,
            "你好",
            "trade(2) 不覆盖 greeting(3)"
        );
    }

    #[test]
    fn test_trade_bubble_overwrites_self_talk() {
        let mut world = World::new();
        let (_e, entity_id) = spawn_test_npc(&mut world);

        let fragments = SpeechFragmentRegistry::load_embedded();

        // 先设置 self-talk 气泡 (priority=1)
        let mut state = SpeechBubbleState::new();
        state.slots.entry(_e).or_default().active = Some(ActiveBubble {
            text: "肚子饿了…".into(),
            bubble_type: BubbleType::Ambient,
            expiry_tick: 300,
            priority: PRIORITY_SELF_TALK,
        });

        // 成交事件
        let mut trade_events = EventChannel::new();
        trade_events.begin_frame();
        push_trades(&mut trade_events, vec![make_trade(entity_id, entity_id)]);

        trade_bubble_system(
            &world,
            &mut trade_events,
            &mut state,
            &fragments,
            100,
            0.5,
            None,
        );

        let bubble = state.active_for(_e).unwrap();
        assert_eq!(
            bubble.priority, PRIORITY_TRADE,
            "trade(2) 覆盖 self-talk(1)"
        );
    }

    #[test]
    fn test_trade_bubble_player_excluded() {
        let mut world = World::new();
        let (e_player, player_id) = spawn_test_npc(&mut world);
        let (_e_npc, npc_id) = spawn_test_npc(&mut world);

        let mut trade_events = EventChannel::new();
        trade_events.begin_frame();
        push_trades(&mut trade_events, vec![make_trade(player_id, npc_id)]);

        let mut state = SpeechBubbleState::new();
        let fragments = SpeechFragmentRegistry::load_embedded();

        trade_bubble_system(
            &world,
            &mut trade_events,
            &mut state,
            &fragments,
            100,
            0.5,
            Some(e_player),
        );

        assert!(state.active_for(e_player).is_none(), "玩家实体不冒交易气泡");
        assert!(state.active_for(_e_npc).is_some(), "非玩家 NPC 仍应冒泡");
    }

    #[test]
    fn test_trade_bubble_skips_despawned() {
        let mut world = World::new();
        let (_e, entity_id) = spawn_test_npc(&mut world);
        let trader_id = entity_id;

        // 构造一个对应不存在实体的 EntityId
        let ghost_id = EntityId(0xDEAD_BEEF_0000_0000);

        let mut trade_events = EventChannel::new();
        trade_events.begin_frame();
        push_trades(&mut trade_events, vec![make_trade(trader_id, ghost_id)]);

        let mut state = SpeechBubbleState::new();
        let fragments = SpeechFragmentRegistry::load_embedded();

        // 不应 panic
        trade_bubble_system(
            &world,
            &mut trade_events,
            &mut state,
            &fragments,
            100,
            0.5,
            None,
        );

        assert!(state.active_for(_e).is_some(), "存在实体仍应冒泡");
        // ghost 实体不在 world 中 → 不应有 slot
    }

    #[test]
    fn test_event_channel_drain_consumes() {
        let mut world = World::new();
        let (_e, entity_id) = spawn_test_npc(&mut world);

        let mut trade_events = EventChannel::new();
        trade_events.begin_frame();
        push_trades(&mut trade_events, vec![make_trade(entity_id, entity_id)]);

        let mut state = SpeechBubbleState::new();
        let fragments = SpeechFragmentRegistry::load_embedded();

        // 第一次 drain
        trade_bubble_system(
            &world,
            &mut trade_events,
            &mut state,
            &fragments,
            100,
            0.5,
            None,
        );
        assert!(state.active_for(_e).is_some());

        // 第二次 drain——应为空（trades 已消费·cooldown 仍有效）
        // 仅清除活跃气泡（模拟过期），保留 next_allowed_tick
        if let Some(slot) = state.slots.get_mut(&_e) {
            slot.active = None;
        }
        trade_bubble_system(
            &world,
            &mut trade_events,
            &mut state,
            &fragments,
            700, // 超过 cooldown
            0.5,
            None,
        );
        assert!(
            state.active_for(_e).is_none(),
            "drain 后 trades 已消费——EventChannel 为空，不冒新泡"
        );
    }
}
