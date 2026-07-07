# 会话交接 — 2026-07-07 (深夜: 物品 Phase 1 + 经济 Phase 2 + 审计两轮)

## 💾 恢复点

```bash
cd woworld
cargo check --workspace   # 5 crates, 零错误
cargo test --workspace    # 624/624 ✅
cargo clippy --workspace -- -D warnings  # 零警告
cargo build --workspace   # ✅ DLL 已更新
```

已 push: `faa08ca` → `master`

## 本次交付

### 物品系统 Phase 1

| 交付 | 详情 |
|------|------|
| 核心类型 | ItemCategory(44), Quality(4), Rarity(5), ItemProperties(28字段), ItemState, ItemStack |
| ItemQuery trait | 8 方法——所有消费模块的只读入口 |
| ItemDefId 编码 | 三字段 category(8)+sub_category(8)+def_index(48)，含 Ord |
| ItemRegistry | HashMap+SoA + TOML 手动解析 |
| TOML 数据 | 20 测试物品 (assets/items/test_items.toml) |
| 集成 | Item Component 替代裸 ItemDefId；LootTable 接入真实 ID |

### 经济系统 Phase 2

| 交付 | 详情 |
|------|------|
| 核心类型 | Market, OrderBook, Order, OrderSide, TradeRecord |
| 撮合引擎 | submit_order → match_orders(三阶段) → execute_trade(原子) |
| 需求驱动订单 | surplus/deficit + reserve_days(3-17) + 市场参考价 |
| Pareto 钱包 | x_min=50, α=1.5, 确定性 seed→Pareto 逆 CDF |
| 物品持有 | item_holdings + seed_npc_items + 交易时转移物品 |
| EMA 价格 | α=0.3, 波动率 CV |
| 游戏循环 | EconomyRegistry+ItemRegistry 接入 WorldDriver, NPC spawn 含 Wallet+EconCog |

### 审计两轮

| 轮次 | 发现 | 自动修复 |
|------|------|---------|
| 第一轮 | 42 | 3 |
| 第二轮 | 11 | 3 |
| **合计** | **53** | **6** (其余确认延后到 Phase 3) |

## 关键架构决策

- **物品系统不新建 crate**——沿用社会系统模式（core 放类型+trait，ecs 放实现）
- **ItemDefId(0) 非合法哨兵**——`ITEM_DEF_ID_NONE = u64::MAX`
- **一经济体一市场**（Phase 2 简化，不做 Storefront 集群/多层级市场）
- **需求驱动 > 随机**——订单从 surplus/deficit + 经济认知涌现，非随机买卖
- **Pareto 分布钱包**——50% NPC < 79 copper，富 NPC 长尾到 5000
- **reserve_days = 3 + satisficing × 14**——satisficing 正比于 neuroticism（焦虑=囤积）
- **定价优先市场 EMA 价格**——fallback 到 base_value

## 文件变更总览

### 新建文件 (7)

```
woworld_core/src/item/mod.rs                         ← 核心类型 + ItemQuery trait
woworld_ecs/src/components/item.rs                    ← Item Component (8B)
woworld_ecs/src/resources/item_registry.rs           ← ItemRegistry + TOML解析
woworld_ecs/src/systems/item/mod.rs                  ← item_seed_system
assets/items/test_items.toml                          ← 20 测试物品
woworld-dev-plan/.../DEVLOG-2026-07-07-items-economy.md
woworld-dev-plan/.../handoff-20260707-items-economy.md
```

### 修改文件 (15)

```
woworld_core/src/id.rs                  ← ItemDefId方法 + Ord
woworld_core/src/lib.rs                 ← +item module + prelude
woworld_core/src/economy/mod.rs         ← +Market/OrderBook/Order/TradeRecord
woworld_core/src/economy/behavioral.rs  ← price_perception系数修复
woworld_ecs/Cargo.toml                  ← +toml依赖
woworld_ecs/src/components/economy.rs   ← Pareto钱包
woworld_ecs/src/components/mod.rs       ← +item
woworld_ecs/src/resources/economy_registry.rs ← 市场+订单簿+撮合+物品持有
woworld_ecs/src/resources/mod.rs        ← +item_registry
woworld_ecs/src/systems/economy/mod.rs  ← 需求驱动订单+市场撮合
woworld_ecs/src/systems/mod.rs          ← +item
woworld_ecs/src/systems/life/loot_roll.rs   ← 真实ItemDefId
woworld_ecs/src/systems/life/item_spawn.rs  ← Item组件
woworld_ecs/src/lib.rs                  ← +Item prelude
woworld_godot/src/terrain_chunk.rs      ← 经济+物品接入游戏循环
```

## 下一步建议

| 方向 | 内容 |
|------|------|
| **社会 Phase 3** | 需求类别/紧急度/scarcity_bonus/结构化评估/bootstra_economy |
| **物品 Phase 2** | Assembly 装配框架、背包/库存、装备槽位 |
| **可视化** | CapsuleMesh + NPC 名字标签 + billboard |
| **对话雏形** | NPC-NPC 基础对话模板 |
| **玩家系统** | 夺舍 NPC + 控制切换 |

## 关键接口速查

```rust
// ItemQuery (woworld_core::item)
ItemQuery::get_properties(id) -> Option<&ItemProperties>
ItemQuery::get_base_value(id) -> Option<u32>
ItemQuery::get_rarity(id) -> Option<Rarity>

// EconomyQuery (woworld_core::economy)
EconomyQuery::query_price(market, item) -> Option<PriceSnapshot>  // ✅ 真实数据
EconomyQuery::query_wallet(entity) -> Option<WalletSnapshot>       // ✅ 真实数据
EconomyQuery::query_market_volume(market, item) -> Option<u64>     // ✅ 真实数据

// EconomyRegistry 核心方法
reg.submit_order(market, order) -> u64
reg.match_orders(market, item, tick) -> Vec<TradeRecord>
reg.execute_trade(buyer, seller, amount, item, qty) -> bool
reg.seed_npc_items(entity, item_pool, seed)
reg.add_items/remove_items/get_item_count

// ItemDefId
ItemDefId::new(category, sub_category, def_index) -> Self
id.category() -> Option<ItemCategory>
id.sub_category() -> u8
```

## ⚠️ 已知简化（延后到 Phase 3）

- 三种需求类别 (Physiological/Occupational/Social) 未区分
- 四级紧急度 + ListingType 映射未实现
- 无部分成交——剩余量丢弃
- 无多市场/区域选择
- 初始物品随机分配（非职业驱动）
- 无市场监管检查
- bootstrap_economy / initial_money_supply 未实现
