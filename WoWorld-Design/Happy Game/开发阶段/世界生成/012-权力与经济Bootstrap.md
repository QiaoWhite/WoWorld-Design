# 012-权力与经济 Bootstrap

> **开发代号**: WoWorld (Wonder World)
> **版本**: v1.0 — 全新（P10 权力 Bootstrap·P11 经济 Bootstrap）
> **日期**: 2026-06-20
> **关联**: [[001-世界生成总流程|001 总流程]] · [[004-聚落选址与社会结构推导|004 社会结构]]（PowerSkeleton） · [[008-人口投影与家族协同生成|008 人口]]（NpcData）
> **消费**: 权力系统 `bootstrap_power_topology()` · 经济系统 `bootstrap_economy()`
> **产出**: `PowerTopology` · `EconomyState`

---

## §一、P10：权力 Bootstrap

### 1.1 模板→边实例化

P5 产出的 `PowerEdgeTemplate` 使用角色索引（role_index）作为占位符。P10 将角色索引映射到具体的 NPC EntityId。

```rust
fn bootstrap_power(
    skeleton: &[PowerEdgeTemplate],
    roles: &[RoleTemplate],
    npcs: &[NpcData],
    families: &[FamilyTree],
    settlements: &[SettlementData],
    culture: &CultureCoreParams,
    rng: &mut DeterministicRng,
) -> PowerTopology {
    // 1. 对每个模板，找到匹配的 holder NPC 和 subject NPC(s)
    // 2. 按 SelectionPreference 评分选择
    // 3. 创建 PowerEdge (holder, subject, atom, domain, source, legitimacy)
    // 4. 运行 emerge_polities() → 识别初始政治实体
    
    power_crate::bootstrap_power_topology(skeleton, roles, npcs, families, settlements, culture, rng)
}
```

### 1.2 关键边类型

每条边从 `PowerEdgeTemplate` 实例化：

| 模板 | Cardinality | Holder 角色 | Subject | 说明 |
|------|------------|-----------|---------|------|
| Constrain(Territory) | OneToMany | 统治者 | 所有居民 | 领土管辖权 |
| Extract(tax) | OneToMany | 统治者/税吏 | 所有成年居民 | 税收 |
| Delegate | SinglePair | 统治者 | 官员/管家 | 委托链 |
| Adjudicate | OneToMany | 法官/长老 | 所有居民 | 裁决权 |
| Represent | SinglePair | 统治者 | 其他统治者 | 对外代表 |
| Sanction | ManyToOne | 统治者/法官 | 违法者 | 惩罚（运行时使用） |

### 1.3 边实例化的选择算法

```rust
fn select_holder_for_edge(
    role_index: usize, roles: &[RoleTemplate], npcs: &[NpcData],
    preference: &SelectionPreference, rng: &mut DeterministicRng,
) -> NpcId {
    let role = &roles[role_index];
    let candidates: Vec<&NpcData> = npcs.iter()
        .filter(|npc| npc_matches_role(npc, role))
        .collect();
    
    // 加权评分
    candidates.iter()
        .map(|npc| {
            let score =
                npc.social_standing * preference.social_standing_weight +
                age_midpoint_score(npc.age, MID_AGE, AGE_SPREAD) * preference.age_midpoint_weight +
                npc.big_five.conscientiousness * preference.conscientiousness_weight +
                rng.gen::<f32>() * preference.noise_weight;
            (npc.id, score)
        })
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
        .map(|(id, _)| id)
        .unwrap()
}
```

**不是随机** —— BigFive 责任心驱动选择。高责任心者更可能被选为统治者。噪声分量（0.1）保证不完全确定性。

### 1.4 初始合法性的设计约束

`legitimacy_initial` 不设为 1.0。留空间给运行时波动:
- 新征服的领土: 0.2-0.4
- 文化认可的世袭统治: 0.6-0.8
- 选举产生: 0.5-0.7

运行时: 治理效果→legitimacy 上涨或下跌。能力低或腐败的统治者→legitimacy 自然下降→可能被推翻。

---

## §二、P11：经济 Bootstrap

### 2.1 哲学声明

**这不是破坏"价格从交易涌现"**。这是初始条件。玩家进入时世界已交易千百年——初始订单簿是那个历史过程的当前快照。运行时第一次交易即可更新价格。

### 2.2 供需聚合

```rust
fn bootstrap_economy(
    settlements: &[SettlementData],
    npcs: &[NpcData],
    resources: &ResourceMap,
    transport: &TransportNetwork,
    item_defs: &ItemRegistry,
    rng: &mut DeterministicRng,
) -> EconomyState {
    // 1. 每聚落供需聚合
    let supply_demand: HashMap<SettlementId, SupplyDemandSummary> = settlements.iter()
        .map(|s| (s.id, compute_settlement_supply_demand(s, npcs, resources)))
        .collect();
    
    // 2. 创建 Market
    let mut markets = Vec::new();
    for settlement in settlements {
        markets.push(Market::new_local_bazaar(settlement));
    }
    // 有贸易路线的聚落对 → TradeNetwork Market
    for (a, b) in connected_settlements(transport) {
        markets.push(Market::new_trade_network(a, b));
    }
    
    // 3. 填充初始订单簿
    for market in &mut markets {
        for (item, surplus) in supply_demand[&market.settlement_id()].surpluses() {
            if surplus > 0 {
                market.add_ask(item, surplus, initial_price(item, market, &supply_demand, transport));
            } else if surplus < 0 {
                market.add_bid(item, -surplus, initial_price(item, market, &supply_demand, transport));
            }
        }
    }
    
    // 4. NPC 钱包分配 (Pareto × 阶层 × 年龄 × 继承)
    let npc_wallets = assign_wallets(npcs, settlements, rng);
    
    // 5. EconomicRelation 推导
    let relations = derive_initial_economic_relations(settlements, &supply_demand, transport, rng);
    
    // 6. 第一笔交易验证
    verify_bootstrap_liquidity(&markets).unwrap_or_else(|e| inject_minimum_liquidity(&mut markets, e));
    
    EconomyState { markets, npc_economic_states: npc_wallets, economic_relations: relations, .. }
}
```

### 2.3 供需聚合的数据驱动路径

```
ProfessionTag TOML (consumes/produces)
  × NPC profession_tag + proficiency + time_share
  = 该 NPC 的年供需量

Σ 所有 NPC
  + 本地资源年产出
  = 聚落级供需总结
```

**新增职业 = 新增 TOML 条目 → 供需自动更新。零硬编码。**

### 2.4 初始价格公式

```rust
fn initial_price(item: ItemDefId, settlement: SettlementId,
                 supply_demand: &SupplyDemandSummary, trade_access: f32, rng: &mut DeterministicRng) -> u32 {
    let base = item_def(item).base_value_copper;
    let ratio = (supply_demand.supply_of(item) as f64 / supply_demand.demand_of(item).max(1) as f64)
        .clamp(0.1, 10.0);
    let trade_mod = 1.0 - trade_access * 0.5;  // 封闭经济价格偏离更大
    let noise = rng.gen_range(0.9, 1.1);
    (base as f64 * ratio.powf(-0.5) * trade_mod * noise) as u32
}
```

### 2.5 MarketScope 与物理位置

`LocalBazaar` 挂在聚落下（`MarketScope::LocalBazaar { settlement }`）——非物理建筑。当交易密度超过阈值时，系统识别为 de facto 集市。如果聚落有 BuildingFunction::Market 建筑，NPC 会在建筑附近聚集。

**关键**: 市场不是建筑模块生成的。Storefront 是 NPC 的物品——铁匠铺主人可以在自己的铺子里开店。

---

## §三、性能

| 步骤 | 预估 |
|------|------|
| P10 边实例化 (500K 边) | ~200ms |
| P10 Polity 涌现 | ~50ms |
| P11 供需聚合 (100K NPC × ~4 职业标签) | ~100ms |
| P11 订单簿填充 | ~50ms |
| P11 钱包分配 | ~50ms |
| P11 EconomicRelation 推导 | ~30ms |
| **总计 P10+P11** | **~0.5s** |

---

> **关联**: [[004-聚落选址与社会结构推导]] · [[008-人口投影与家族协同生成]] · [[013-生成后校验与完整性保证]]
