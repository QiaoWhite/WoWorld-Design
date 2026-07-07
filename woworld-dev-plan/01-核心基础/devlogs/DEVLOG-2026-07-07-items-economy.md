# DEVLOG — 2026-07-07 深夜 (物品系统 Phase 1 + 经济系统 Phase 2 + 两轮审计)

## 摘要

承接晚间社会系统 Phase 1（578 tests），本段交付物品系统的核心类型体系 + 经济系统的订单簿撮合引擎。物品 Phase 1 建立了完整的类型/注册表/TOML 数据层，经济 Phase 2 实现了从订单创建到交易执行的完整循环。两轮审计发现 42 个问题，7 个自动修复，其余确认延后。

**总计**: 5 新建 + ~15 修改，624 tests 全绿，clippy 零警告。

---

## 物品系统 Phase 1

### 核心类型 (`woworld_core/src/item/mod.rs` ~410行)

- **ItemCategory** — 44 变体 `#[repr(u8)]` flat enum（0x00 Weapon → 0x63 MagicConstruct），`from_u8()` + `category_group()` → 6 组
- **Quality** (4档: Rough/Standard/Refined/Perfect) — durability_multiplier(0.6/1.0/1.3/1.8) + stat_multiplier + consumable_multiplier
- **Rarity** (5档: Common → Legendary) — PartialOrd 排序
- **ItemProperties** — 28 字段（13 必填 + 10 Option/占位 + 5 默认值），对齐设计文档 003
- **ItemState** — durability + quality + custom_name + inscription（4 字段，其余延后）
- **ItemStack** — (ItemDefId, u32 count)
- **ItemQuery trait** — 8 方法：get_properties/get_category/get_stack_size/get_base_value/get_rarity/get_name/all_def_ids/def_count
- **ITEM_DEF_ID_NONE** = `u64::MAX`（非零哨兵——避免与合法 Weapon/sub=0 冲突）

### ItemDefId 编码 (`woworld_core/src/id.rs`)

三字段布局: `category(8bit) + sub_category(8bit) + def_index(48bit)`
- `new(category, sub_category, def_index)` — 构造函数
- `category() → Option<ItemCategory>` — 0x70-0xFF 返回 None
- `sub_category() → u8` / `def_index() → u64`
- 添加 PartialOrd + Ord derive（BTreeMap 键需求）

### ItemRegistry (`woworld_ecs/src/resources/item_registry.rs` ~280行)

- HashMap<ItemDefId, ItemProperties> 主存储 + SoA 列（categories/stack_sizes/base_values/rarities）
- `register(ItemProperties)` / `load_from_toml(&str)` — 手动 TOML 解析（不用 serde derive）
- `ItemQuery for ItemRegistry` 全方法实现
- Phase 2 预留：`get_price_snapshot` / `query_market_volume`

### TOML 数据 (`assets/items/test_items.toml`)

20 个测试物品：兽皮/兽骨/生肉/植物纤维/种子/铁矿/铜矿/煤炭/花岗岩/橡木/松木/淡水/药草/兽牙/黏土/沙/铜币/银币/金币/铁剑
- category 字符串匹配 → ItemCategory
- 可选字段：is_placeable/tool_tags/is_consumable/visual_quality/min_strength

### 接入点

- **Item Component** (`components/item.rs`) — 8 字节 tag，替代 item_spawn 中裸 ItemDefId
- **LootTable 重构** — 硬编码 `ItemDefId(1)` → `ItemDefId::new(LeatherMat, 1, 0)`
- **item_spawn 重构** — 裸 ItemDefId → `Item { item_def_id }`

### 测试: +33 (core +14, ecs +19)

---

## 经济系统 Phase 2

### 核心类型扩展 (`woworld_core/src/economy/mod.rs`)

- **Market** — market_id + economy_id + BTreeMap<ItemDefId, OrderBook>
- **OrderBook** — bids(Vec<Order> 降序) + asks(Vec<Order> 升序)
- **Order** — order_id/entity_id/item_id/quantity/limit_price_copper/side/created_tick
- **OrderSide** — Bid/Ask
- **TradeRecord** — item_id/quantity/price_copper/buyer_id/seller_id/tick

### 订单簿撮合引擎 (`woworld_ecs/src/resources/economy_registry.rs`)

新增存储列：
- `markets: HashMap<MarketId, Market>` + `economy_market_map: HashMap<EconomyId, MarketId>`
- `item_holdings: HashMap<EntityId, HashMap<ItemDefId, u32>>` — 物品持有
- `trade_histories` + `price_snapshots` (EMA α=0.3)
- `next_order_id: u64`

关键方法：
- `submit_order()` — 提交 + 自动排序（bids 降序/asks 升序）
- `match_orders()` — 三阶段：预取钱包→收集候选→执行交易
- `match_all_markets()` — 遍历全市场全物品
- `execute_trade()` — 原子操作：验证库存→转账铜币→转移物品
- `transfer_copper()` — SoA 钱包原子转账
- `add_items/remove_items/get_item_count/seed_npc_items` — 物品持有管理
- `record_trade()` — EMA 价格更新 + 波动率计算

EconomyQuery 升级：
- `query_price()` → 返回真实 PriceSnapshot ✅
- `query_market_volume()` → 从 trade_history 计算 ✅
- 其余方法仍为 stub（Phase 3）

### 需求驱动订单创建 (`woworld_ecs/src/systems/economy/mod.rs`)

`order_creation_system` 重写：
- **reserve_days** = 3.0 + satisficing × 14.0 → [3, 17]（satisficing 正比于 neuroticism）
- **surplus/deficit** = held_qty vs reserve_need
- **市场参考价** = PriceSnapshot.ema_price_copper（fallback → base_value）
- 决策树：
  - surplus + 缺钱 → 紧急卖单（打折 40%）
  - surplus + 不缺钱 → 正常卖单（加价 15%）
  - deficit → 买单（压价 30-60%）
  - 有钱 + 不缺物品 → 投资性买入

`market_matching_system` — 每 tick 调用 match_all_markets

### Pareto 钱包分配 (`components/economy.rs`)

`Wallet::from_seed` 改为 Type I Pareto 逆 CDF：
- x_min = 50, α = 1.5, 上限 5000
- 确定性 hash seed → uniform [0,1) → Pareto
- 结果：50% < 79, 90% < 232, 99% < 1077 copper

### 游戏循环接入 (`terrain_chunk.rs`)

- WorldDriver 添加 `economy_registry` + `item_registry` + `item_seeded`
- 首帧：item_seed_system(TOML) + create_economy + create_market
- 每 tick：economic_cognition_update → wallet_init → order_creation → market_matching
- spawn_npc：添加 `Wallet::from_seed(seed)` + `EconomicCognition::default()`

### 测试: +13 (economy_registry +9, systems +4)

---

## 审计两轮

### 第一轮（物品+经济 全面审计）

| 类别 | 发现 | 修复 |
|------|------|------|
| 🔴 CRITICAL | 3 | 3 |
| 🟡 GAP | ~32 | 0（确认延后） |
| 🔵 OBSERVATION | ~7 | 0 |

修复项：
1. `price_perception` wisdom 系数加倍（0.1→0.2, 0.05→0.1）对齐 007
2. `query_market_volume` 返回交易量（sum quantity）非交易额
3. `ItemProperties` 添加 `description` 字段

### 第二轮（E-13/14/15 针对性审计）

| 类别 | 发现 | 修复 |
|------|------|------|
| 🔴 CRITICAL | 3 | 3 |
| 🟡 GAP | 8 | 0（确认延后） |

修复项：
1. `reserve_days` 公式去反转（`1.0 -` 删除）
2. 定价优先使用市场 EMA 价格（fallback base_value）
3. `daily_consumption` 加种子派生微量变化 + 注释

### 延后共识

17 项 GAP/SIMPLIFICATION 确认 Phase 3 处理：需求类别/紧急度/scarcity_bonus/结构化评估/bootstra_economy/部分成交/职业驱动物品分配/TradeError/多市场/市场监管

---

## 变更文件清单

| 文件 | 操作 |
|------|------|
| `woworld_core/src/item/mod.rs` | 新建 |
| `woworld_core/src/id.rs` | 修改 (+ItemDefId方法+Ord) |
| `woworld_core/src/lib.rs` | 修改 (+item module+prelude) |
| `woworld_core/src/economy/mod.rs` | 修改 (+Market/OrderBook/Order/TradeRecord) |
| `woworld_core/src/economy/behavioral.rs` | 修改 (price_perception系数修复) |
| `woworld_ecs/Cargo.toml` | 修改 (+toml依赖) |
| `woworld_ecs/src/components/item.rs` | 新建 |
| `woworld_ecs/src/components/economy.rs` | 修改 (Pareto钱包) |
| `woworld_ecs/src/components/mod.rs` | 修改 (+item) |
| `woworld_ecs/src/resources/item_registry.rs` | 新建 |
| `woworld_ecs/src/resources/economy_registry.rs` | 修改 (市场+订单簿+物品持有+撮合) |
| `woworld_ecs/src/resources/mod.rs` | 修改 (+item_registry) |
| `woworld_ecs/src/systems/item/mod.rs` | 新建 |
| `woworld_ecs/src/systems/economy/mod.rs` | 修改 (需求驱动订单+市场撮合) |
| `woworld_ecs/src/systems/mod.rs` | 修改 (+item) |
| `woworld_ecs/src/systems/life/loot_roll.rs` | 修改 (真实ItemDefId) |
| `woworld_ecs/src/systems/life/item_spawn.rs` | 修改 (Item组件) |
| `woworld_ecs/src/lib.rs` | 修改 (+Item prelude) |
| `woworld_godot/src/terrain_chunk.rs` | 修改 (经济+物品接入游戏循环) |
| `assets/items/test_items.toml` | 新建 |
| `CLAUDE.md` | 修改 (状态更新) |

## 最终状态

| 指标 | 值 |
|------|-----|
| Crate | 5 (core, worldgen, atmosphere, ecs, godot) |
| Tests | **624** |
| ECS Component | 40 (~原39 + Item) |
| ECS Resource | 8 (~原7 + ItemRegistry) |
| ECS System | 27 (~原25 + item_seed + economy) |
| 物品系统 Phase 1 | ✅ |
| 经济系统 Phase 2 | ✅ 核心循环完整 |
