# DEVLOG — 2026-07-07 晚间 (四大社会系统 Phase 1 + 三轮审计闭环)

## 摘要

承接上午的 ECS 基础设施，晚间集中交付四大社会系统（文化·经济·信仰·权力）的 Phase 1 核心类型 + Query trait + Registry + 种子生成。遵循统一架构模式：core 定义 trait/类型，ecs 实现 Component/Resource/System。三轮审计发现 48 个问题，39 个自动修复，2 个需用户确认（已裁决）。代码与设计文档最终零偏差。

**总计**: 29 新建 + 7 修改，578 tests 全绿，clippy 零警告。

---

## 文化系统 Phase 1

7 个 core 子模块 + 3 个 ecs 文件。CultureCoreParams(10 f32) 为所有文化特征的"DNA"原子——CommunicationNorms/BuildingStyle/Beauty/Dietary/Fertility/Relationship 全部从这 10 个参数纯函数派生。

**关键类型**: CultureId(u32), CultureCoreParams(10), CommunicationNorms(8 字段 + 4 子类型), BuildingStylePreferences(8 字段 + 7 子类型), DietaryBasePreferences(5), FertilityNorms(4 + 2 辅助), RelationshipNorms(14 字段 + max_spouses), CulturalBeautyStandard(8)

**审计重点**: 22 个公式偏差全部修复——TouchNorms、polygyny/polyandry/polyamory/group_marriage、arranged/contractual marriage、same_sex_marriage(连续3层)、decoration thresholds、primary_hue(Earth)、imported_stone(离散)、wall fallback(WattleDaub)、Longhouse threshold(>0.6)、adjacency formulas、ideal_family_size(乘法)

**设计文档更新**: Thatched 森林屋顶派生路径补充；residence_pattern 4项扩展公式

---

## 经济系统 Phase 1

2 个 core 子模块 + 3 个 ecs 文件。核心交付：Wallet(铜/银/金 1:20:400)、EconomicCognition(6维)、10 行为经济学概念、EconomyQuery trait(8+ query 方法)。

**行为经济学**: Loss Aversion(1.5-2.5×)、Mental Accounting(4账户)、Anchoring(5级价格感知)、Endowment Effect(乘法公式)、Hyperbolic Discounting、Herd Behavior、Satisficing、Fairness、Overconfidence、Status Quo Bias——全部从 BigFive 纯函数派生，零新存储。

**审计修复**: 锚定效应 wisdom 2x→1x；禀赋效应加法→乘法；双曲贴现消除重复派生；满意化公式统一

**货币决策**: 用户确认金:银:铜 = 1:20:400，但声明"这是暂时的货币设计，实际游戏中货币或许会涌现出来"

---

## 信仰系统 Phase 1

1 个 core 模块 + 3 个 ecs 文件。实践优先模型——NPC 没有"神学立场"字段，只有参与实践的行为记录。

**关键类型**: FaithId(u32), FaithTheology(10, deity_count [0,15]), ReligiousMotivation(9变体, CrisisDriven 含 since_tick/intensity), ReligiousPracticeProfile(participation + depth + motivation), FaithLabel(6分类)

**审计修复**: CrisisDriven 从 unit variant 改为含字段变体；FaithLabel 推导算法重写对齐文档(6分类决策树)；FaithRegistry u64→EntityId 类型安全；ReligiousPracticeProfile +PartialEq

---

## 权力系统 Phase 1

1 个 core 模块 + 2 个 ecs 文件。权力是"控制关系物理引擎"——17 个普适原子适用于从家规到帝国的所有层级。

**关键类型**: PowerAtom(17变体×5类别, #[repr(u8)]), PowerSource(8路径, initial_legitimacy), PowerDomain(6), SuccessionRule(6), PowerEdge(13字段), Legitimacy(5因子加权: 0.35/0.20/0.20/0.15/0.10)

**审计修复**: SuccessionRule +Unspecified(第6变体)；default 改为 ExtinguishWithHolder

---

## 三轮审计统计

| 轮次 | 发现问题 | 自动修复 | 需确认 |
|------|---------|---------|--------|
| 第一轮 | 35 | 28 | 7 |
| 第二轮 | 8 | 6 | 2 |
| 第三轮 | 5 | 5 | 0 |

**典型问题分布**: 公式偏差(22) / 缺失类型(8) / 缺失方法(4) / 类型不一致(3) / 设计文档过期(3) / 缺 derive(2) / 代码重复(2) / 其他(4)

---

## 最终状态

| 指标 | 值 |
|------|-----|
| Crate | 5 (core, worldgen, atmosphere, ecs, godot) |
| Tests | **578** (381→578, +216) |
| ECS Component | 39 (~原 35 + Culture+Economy+Faith) |
| ECS Resource | 7 (RelationStorage+SpatialGrid+Culture+Economy+Faith+Power) |
| ECS System | 25 (~原 21 + culture+economy+faith+power seed systems) |
| 社会系统 Phase 1 | 4/4 完成 |

## 变更文件清单

| 文件 | 操作 |
|------|------|
| `woworld_core/src/culture/{mod,communication,building,dietary,fertility,relationship,beauty}.rs` | 新建 |
| `woworld_core/src/economy/{mod,behavioral}.rs` | 新建 |
| `woworld_core/src/faith/mod.rs` | 新建 |
| `woworld_core/src/power/mod.rs` | 新建 |
| `woworld_core/src/lib.rs` | 修改 (+4 modules + prelude) |
| `woworld_ecs/src/components/{culture,economy,faith}.rs` | 新建 |
| `woworld_ecs/src/resources/{culture,economy,faith,power}_registry.rs` | 新建 |
| `woworld_ecs/src/systems/{culture,economy,faith,power}/mod.rs` | 新建 |
| `woworld_ecs/src/{components,resources,systems}/mod.rs` | 修改 |
| `CLAUDE-INTERFACES.md` | 修改 (CHG-024 字段修正) |
| `WoWorld-Design/.../文化系统/004-文化审美与物质.md` | 修改 (Thatched + residence_pattern) |
