//! Economy ECS Systems
//!
//! Phase 1:
//! - economic_cognition_update_system: 从 BigFive 派生 EconomicCognition
//! - wallet_init_system: 从种子分配初始钱包
//!
//! Phase 2:
//! - order_creation_system: NPC 基于经济认知创建买卖单
//! - market_matching_system: 撮合所有市场订单簿

use std::collections::HashMap;

use hecs::{CommandBuffer, World};
use woworld_core::economy::behavioral::EconBehaviorParams;
use woworld_core::economy::{EconomyQuery, Order, OrderSide};
use woworld_core::id::ItemDefId;
use woworld_core::item::ItemQuery;
use woworld_core::types::EntityId;

use crate::components::bigfive::BigFive;
use crate::components::economy::{EconomicCognition, Wallet};
use crate::resources::economy_registry::EconomyRegistry;

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
pub fn order_creation_system(
    world: &World,
    registry: &mut EconomyRegistry,
    item_registry: &dyn ItemQuery,
    tick: u64,
) {
    let items = item_registry.all_def_ids().to_vec();
    if items.is_empty() || registry.all_markets().is_empty() {
        return;
    }
    let market_id = registry.all_markets()[0];

    // 首帧：给所有 NPC 分配初始物品
    let is_first_tick = tick <= 1;

    for (entity, (wallet, cognition)) in world.query::<(&Wallet, &EconomicCognition)>().iter() {
        let entity_id = EntityId(entity.to_bits().get());
        let entity_seed = entity.to_bits().get();

        // 首帧物品种子
        if is_first_tick {
            registry.seed_npc_items(entity_id, &items, entity_seed);
        }

        // ── 活动概率 ──
        let activity_prob = 0.05 + (cognition.market_search_breadth as f32) * 0.05;
        let seed = entity_seed.wrapping_add(tick);
        let roll = ((seed.wrapping_mul(1_103_515_245).wrapping_add(12_345) & 0x7FFF_FFFF) as f64)
            / 0x7FFF_FFFF as f64;
        if roll > activity_prob as f64 {
            continue;
        }

        let wallet_balance = wallet.total_copper();
        let satisficing = cognition.satisficing_threshold;
        // reserve_days = 3..17, 由 satisficing 代理 neuroticism
        // satisficing = 0.5 + (1-C)*0.3 + N*0.2 → N 高则 satisficing 高
        // 设计文档 004 §1.2: reserve_days = neuroticism × 14 + 3
        let reserve_days = 3.0 + satisficing * 14.0;

        // ── 遍历 NPC 持有的物品 + 一些随机物品 ──
        let holdings = registry.get_holdings(entity_id);
        let holdings_map: HashMap<ItemDefId, u32> =
            holdings.cloned().unwrap_or_default();

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
        // Phase 2 简化：daily_consumption 暂用种子微量变化替代完整估计
        // 设计文档 004 §1.2: estimate_daily_consumption(npc, item_id) —
        //   需要 vitals(hunger/thirst)、profession 需求、family_size
        //   这些系统就位后替换
        let daily_need = 1.0 + (entity_seed.wrapping_mul(17) % 3) as f32 * 0.2; // [1.0, 1.4]
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
            registry.submit_order(market_id, Order {
                order_id: 0, entity_id, item_id, quantity: qty,
                limit_price_copper: ask_price, side: OrderSide::Ask, created_tick: tick,
            });
        } else if surplus > 0 && !wallet_poor {
            // 有 surplus + 不缺钱 → 正常卖单
            let premium = price * satisficing * 0.15;
            let ask_price = (price + premium).max(1.0) as u64;
            let qty = surplus.min(2);
            registry.submit_order(market_id, Order {
                order_id: 0, entity_id, item_id, quantity: qty,
                limit_price_copper: ask_price, side: OrderSide::Ask, created_tick: tick,
            });
        } else if deficit > 0 {
            // 有 deficit → 买单（补货），只要付得起
            let urgency = if wallet_poor { 0.6 } else { 0.3 }; // 缺钱则更压价
            let discount = price * satisficing * urgency;
            let bid_price = (price - discount).max(1.0) as u64;
            if wallet_balance >= bid_price {
                let qty = deficit.min(2);
                registry.submit_order(market_id, Order {
                    order_id: 0, entity_id, item_id, quantity: qty,
                    limit_price_copper: bid_price, side: OrderSide::Bid, created_tick: tick,
                });
            }
        } else if wallet_rich && held_qty > 0 {
            // 不缺物品 + 有钱 → 投资性买入
            let bid_price = (price * 0.9).max(1.0) as u64;
            if wallet_balance >= bid_price {
                registry.submit_order(market_id, Order {
                    order_id: 0, entity_id, item_id, quantity: 1,
                    limit_price_copper: bid_price, side: OrderSide::Bid, created_tick: tick,
                });
            }
        }
    }
}

/// 市场撮合系统：遍历所有市场的所有订单簿，撮合并执行交易。
pub fn market_matching_system(registry: &mut EconomyRegistry, tick: u64) {
    let trades = registry.match_all_markets(tick);
    // 交易结果由 record_trade 在 match_all_markets 内部处理
    // trades 计数用于日志/统计（Phase 2+）
    let _ = trades.len();
}

// ── 测试 ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::bigfive::BigFive;
    use crate::components::economy::{EconomicCognition, Wallet};
    use woworld_core::economy::EconomyQuery;

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

        world.spawn((
            BigFive::from_seed(42),
            EconomicCognition::default(),
        ));

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

        order_creation_system(&world, &mut reg, &item_reg, 0);
        // 空世界——不 panic
    }

    #[test]
    fn test_order_creation_with_npc() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut reg = EconomyRegistry::new();
        reg.create_market();
        let item_reg = fake_item_registry();

        // spawn NPC with BigFive → need cognition + wallet first
        world.spawn((BigFive::from_seed(42),));

        economic_cognition_update_system(&world, &mut cmd, &mut reg);
        wallet_init_system(&world, &mut cmd, &mut reg);
        cmd.run_on(&mut world);

        // 现在创建订单——应至少有一些 orders
        for tick in 0..100 {
            order_creation_system(&world, &mut reg, &item_reg, tick);
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

        // 创建两个 NPC
        world.spawn((BigFive::from_seed(42),));
        world.spawn((BigFive::from_seed(43),));

        economic_cognition_update_system(&world, &mut cmd, &mut reg);
        wallet_init_system(&world, &mut cmd, &mut reg);
        cmd.run_on(&mut world);

        // 运行多轮订单创建
        for tick in 0..50 {
            order_creation_system(&world, &mut reg, &item_reg, tick);
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
            order_creation_system(&world, &mut reg, &item_reg, tick);
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

    // ── 辅助 ───────────────────────────────────────────

    fn fake_item_registry() -> crate::resources::item_registry::ItemRegistry {
        let mut reg = crate::resources::item_registry::ItemRegistry::new();
        use woworld_core::id::ItemDefId;
        use woworld_core::item::{ItemCategory, ItemProperties, Quality, Rarity};

        let items: Vec<(ItemDefId, ItemCategory, &str, u32)> = vec![
            (ItemDefId::new(ItemCategory::Food, 1, 0), ItemCategory::Food, "生肉", 20),
            (ItemDefId::new(ItemCategory::MineralOre, 1, 0), ItemCategory::MineralOre, "铁矿", 8),
            (ItemDefId::new(ItemCategory::LeatherMat, 1, 0), ItemCategory::LeatherMat, "兽皮", 15),
            (ItemDefId::new(ItemCategory::Weapon, 0, 0), ItemCategory::Weapon, "铁剑", 50),
            (ItemDefId::new(ItemCategory::WoodMat, 0, 0), ItemCategory::WoodMat, "橡木", 6),
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
}
