# 会话交接 — 2026-07-07 (晚间: 社会系统 Phase 1 × 4 + 审计三轮)

## 💾 恢复点

```bash
cd woworld
cargo check --workspace   # 5 crates, 零错误
cargo test --workspace    # 578/578 ✅
cargo clippy --workspace -- -D warnings  # 零警告
cargo build --workspace   # ✅ DLL 已更新
```

## 本次交付

### 四大社会系统 Phase 1

| 系统 | Phase 1 核心 | 测试数 | 文件数 |
|------|-------------|--------|--------|
| **文化** | CultureId, CultureCoreParams(10), 6推导类型, CultureQuery trait | ~99 | 7 core + 3 ecs |
| **经济** | Wallet, EconomicCognition(6), 10行为经济学概念, EconomyQuery trait | ~63 | 2 core + 3 ecs |
| **信仰** | FaithId, FaithTheology(10), ReligiousMotivation(9), FaithQuery trait | ~36 | 1 core + 3 ecs |
| **权力** | PowerAtom(17), PowerSource(8), Legitimacy(5因子), PowerQuery trait | ~18 | 1 core + 2 ecs |
| **合计** | | **+216 tests** (381→597→578) | **25 new files** |

### 架构模式

所有四系统遵循统一 pattern:
```
woworld_core/src/{domain}/mod.rs  → trait + 共享类型（零依赖）
woworld_ecs/src/
  components/{domain}.rs          → tag Component (4B)
  resources/{domain}_registry.rs → SoA Resource + trait impl
  systems/{domain}/mod.rs        → seed_system
```

### 审计三轮

| 轮次 | 发现 | 自动修复 | 需确认 |
|------|------|---------|--------|
| 第一轮 | 35 | 28 | 7 |
| 第二轮 | 8 | 6 | 2 |
| 第三轮 | 5 | 5 | 0 |
| **合计** | **48** | **39** | **2**(已确认) |

## 关键架构决策

- **货币**: 金:银:铜 = 1:20:400（临时设计，未来可能涌现变化）
- **CultureId**: 扁平 u32，谱系独立存储
- **文化公式**: 全部严格对齐设计文档 003/004
- **Thatched 屋顶**: 设计文档 bug——枚举定义了但派生函数无分支。代码补上 `is_forested → Thatched`，已同步更新设计文档
- **residence_pattern**: 从设计文档的 2 项简化为代码的 4 项扩展（所有权重归一化到 1.0），已同步更新设计文档
- **FaithRegistry**: 使用 `EntityId` 类型安全接口（非裸 u64）
- **SuccessionRule**: `ExtinguishWithHolder` 为默认，`Unspecified` 为异常态

## 文件变更总览

### 新建文件 (29)

```
woworld_core/src/culture/{mod,communication,building,dietary,fertility,relationship,beauty}.rs
woworld_core/src/economy/{mod,behavioral}.rs
woworld_core/src/faith/mod.rs
woworld_core/src/power/mod.rs
woworld_ecs/src/components/{culture,economy,faith}.rs
woworld_ecs/src/resources/{culture,economy,faith,power}_registry.rs
woworld_ecs/src/systems/{culture,economy,faith,power}/mod.rs
```

### 修改文件 (7)

- `woworld_core/src/lib.rs` — +4 modules + prelude
- `woworld_ecs/src/lib.rs` — prelude
- `woworld_ecs/src/{components,resources,systems}/mod.rs` — 模块声明
- `CLAUDE-INTERFACES.md` — CHG-024 CommunicationNorms 字段修正
- `WoWorld-Design/.../文化系统/004-文化审美与物质.md` — Thatched + residence_pattern 更新

## 下一步建议

| 方向 | 内容 |
|------|------|
| **物品系统** | 解锁经济 Phase 2 (订单簿/交易执行) |
| **可视化** | CapsuleMesh + NPC 名字标签 + billboard |
| **对话雏形** | NPC-NPC 基础对话模板 |
| **社会 Phase 2** | CultureDriftSystem, MarketMatchingSystem, FaithPropagationSystem |
| **世界生成 P2.5** | 将 seed-based 文化分配替换为 Barrier Voronoi 区域共享 |
| **玩家系统** | 玩家夺舍 NPC + 控制切换 |

## 关键接口速查

```rust
// Culture
CultureQuery::core_params(id) -> Option<&CultureCoreParams>
CultureQuery::communication_norms(id) -> Option<&CommunicationNorms>
CultureQuery::building_style(id) -> Option<&BuildingStylePreferences>
CultureQuery::relationship_norms(id) -> Option<&RelationshipNorms>

// Economy
EconomyQuery::query_wallet(entity) -> Option<WalletSnapshot>
EconomyQuery::query_price(market, item) -> Option<PriceSnapshot>

// Faith
FaithQuery::theology(id) -> Option<&FaithTheology>
FaithQuery::tolerance_between(a, b) -> f32
FaithQuery::hostility_between(a, b) -> f32

// Power
PowerQuery::powers_of(holder) -> Vec<PowerEdge>
PowerQuery::constraints_on(subject) -> Vec<PowerEdge>
PowerQuery::perceived_legitimacy(subject, holder) -> f32
```
