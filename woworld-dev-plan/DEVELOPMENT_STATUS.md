# DEVELOPMENT_STATUS.md — WoWorld 全局状态追踪

> **最后更新**: 2026-06-23
> **维护者**: Claude Code（按 CONSTITUTION.md §7 更新）
> **关联文件**: `CONSTITUTION.md` · `../WoWorld-Design/开发路线图/` · `../CLAUDE-INTERFACES.md`

---

## 总体状态

| 指标 | 值 |
|------|-----|
| 设计规格行数 | ~107,000 行（~300+ 文件） |
| 完成模块数 | 25 独立系统 + 1 子模块（家具与放置物品） |
| 游戏代码 | **零** — `woworld/` 目录不存在 |
| Rust toolchain | 未安装 |
| Godot 项目 | 未创建 |
| 当前冲刺 | 元冲刺 — 宪法系统搭建中 |
| 最新 CHG | CHG-060（2026-06-22） |

## 模块准入等级一览

### 全局基础（3 个）

| 模块 | 等级 | 完成度 | 备注 |
|------|------|--------|------|
| [技术栈方案](../../WoWorld-Design/Happy%20Game/开发阶段/技术栈方案/) | 🟡 就绪 | 95% | v4.0 权威方案。Bevy 审计待用户裁决——裁决后可能修订 |
| [模块接头总览](../../WoWorld-Design/Happy%20Game/开发阶段/模块接头总览/) | 🟡 就绪 | 85% | 102 文件/~6,100 行。12/27 模块时间戳保鲜延迟。README 46 行（未达 60 行门槛） |
| [存档系统](../../WoWorld-Design/Happy%20Game/开发阶段/存档系统/) | 🟡 就绪 | 90% | v2.0 (CHG-055/056)。NoiseHistoryQuery 11 方法 vs CLAUDE-INTERFACES.md 7 方法差异待修复 |

### 世界框架（4 个）

| 模块 | 等级 | 完成度 | 备注 |
|------|------|--------|------|
| [世界生成](../../WoWorld-Design/Happy%20Game/开发阶段/世界生成/) | 🟡 就绪 | 85% | v2.1。5 篇文档仍为 v1.0 待升级。15 阶段管线(P0-P13)完整 |
| [生命](../../WoWorld-Design/Happy%20Game/开发阶段/生命/) | 🟡 就绪 | 90% | v1.0。Vitals/Mana/DeathCause(30种6类)契约完整 |
| [历史](../../WoWorld-Design/Happy%20Game/开发阶段/历史/) | 🟡 就绪 | 85% | 开发规格。AetherImprint/KnowledgeSeed 契约完整。README 151 行 |
| [天气与季节系统](../../WoWorld-Design/Happy%20Game/开发阶段/天气与季节系统/) | 🟡 就绪 | 90% | v1.0。WeatherSample/Markov 6-state 契约完整。README 59 行（差 1 行达门槛） |

### NPC 核心（2 个 + 7 子模块）

| 模块 | 等级 | 完成度 | 备注 |
|------|------|--------|------|
| [NPC活人感模块](../../WoWorld-Design/Happy%20Game/开发阶段/NPC活人感模块/) | 🟡 就绪 | 80% | v2.0 总规格。子模块状态见下 |
| ↳ 03-基本需求系统 | 🔴 冻结 | 70% | 待审核 vs CHG-027。未审核前不可编码 |
| ↳ 04-进阶需求系统 | 🔴 冻结 | 55% | 草稿状态。ERG 挫折回归模型已定义但细节待填充 |
| ↳ 05-审美与艺术 | 🔴 冻结 | 60% | 草稿状态。AestheticSignal(6 dims) 已定义，待审核 |
| ↳ 06-认知与智慧系统 | 🟡 就绪 | 85% | v1.1 开发中 (CHG-057/058/059)。PatternExpression 数学地基已完成 |
| ↳ 07-生命周期系统 | 🟡 就绪 | 90% | v1.0 (CHG-041)。AgeClock/Gompertz/InfantDependency FSM 完整 |
| ↳ 08-NPC行动涌现与分类 | 🟡 就绪 | 85% | v1.0 (CHG-042)。3 层原子架构(35 physical+~40 domain+~25 social) |
| [概念与语言地基](../../WoWorld-Design/Happy%20Game/开发阶段/概念与语言地基/) | 🟡 就绪 | 85% | v1.0 (CHG-044)。3 层模型(PatternSignature→文化概念空间→语言词汇) |

### 社会系统（4 个）

| 模块 | 等级 | 完成度 | 备注 |
|------|------|--------|------|
| [经济系统](../../WoWorld-Design/Happy%20Game/开发阶段/经济系统/) | 🟡 就绪 | 90% | v1.0。OrderBook 匹配/Market+Storefront/NpcEconomicState 完整 |
| [权力系统](../../WoWorld-Design/Happy%20Game/开发阶段/权力系统/) | 🟡 就绪 | 90% | v1.0。17 PowerAtoms/PowerTopology 有向多重图/Legitimacy 5 因子 |
| [文化系统](../../WoWorld-Design/Happy%20Game/开发阶段/文化系统/) | 🟡 就绪 | 85% | v1.0。CultureCoreParams(10)/Voronoi 屏障/4 路径演化。子模块 008-节日与仪式(设计探讨阶段) |
| [信仰系统](../../WoWorld-Design/Happy%20Game/开发阶段/信仰系统/) | 🟡 就绪 | 90% | v1.0。FaithTheology(10 params)/实践先于教义/NPC→NPC 5 通道传播 |

### 交互/表现/建造/辅助（13 个 + 16 子模块）

| 模块 | 等级 | 完成度 | 备注 |
|------|------|--------|------|
| [战斗](../../WoWorld-Design/Happy%20Game/开发阶段/战斗/) | 🟡 就绪 | 85% | v1.0。三层模型(本能→节奏→战略)/半自动/玩家=NPC 同一代码 |
| [魔法](../../WoWorld-Design/Happy%20Game/开发阶段/魔法/) | 🔴 冻结 | 75% | v1.0 设计完整，但**零性能预算**（CHG-047 HIGH 风险）。性能预算建立前不可编码 |
| [物品系统](../../WoWorld-Design/Happy%20Game/开发阶段/物品系统/) | 🟡 就绪 | 90% | v1.0。ItemDefId/ItemEntId/Assembly/Enchantment/CraftingRecipe 完整。子模块 家具与放置物品 v1.0 (CHG-050) |
| [技能系统](../../WoWorld-Design/Happy%20Game/开发阶段/技能系统/) | 🟡 就绪 | 90% | v1.0。SkillId(5分类)/XP公式/天赋三层/教学四路径 |
| [语言表达](../../WoWorld-Design/Happy%20Game/开发阶段/语言表达/) | 🟡 就绪 | 85% | v1.0/v1.1。ExpressionRef/Conversation/信息传播/非语言连接 |
| [模型动作与物理系统](../../WoWorld-Design/Happy%20Game/开发阶段/模型动作与物理系统/) | 🟡 就绪 | 85% | v1.0 (CHG-033)。TerrainQuery/EntityIndex/SpatialEventBus/VisibilityQuery 四 trait。9 层动画栈。5 子模块完整 |
| [音频系统](../../WoWorld-Design/Happy%20Game/开发阶段/音频系统/) | 🟡 就绪 | 85% | v1.0 (CHG-030)。SoundFootprint/AudioQuery(30 methods)/5 声音分类/传播引擎 |
| [感官与知觉系统](../../WoWorld-Design/Happy%20Game/开发阶段/感官与知觉系统/) | 🟡 就绪 | 85% | v1.1 (CHG-031)。PerceptBatch/4 查询 trait/PerceptualCache LRU。4 子模块 |
| [建筑模块](../../WoWorld-Design/Happy%20Game/开发阶段/建筑模块/) | 🟡 就绪 | 85% | v1.0 (CHG-043)。ComponentFamily(9 core+Mod)/WFC 2.5D/BuildingGenerator trait |
| [载具系统](../../WoWorld-Design/Happy%20Game/开发阶段/载具系统/) | 🟡 就绪 | 85% | 开发规格 (CHG-026)。5 动力类型+L1-L3 半自动控制 |
| [大气与氛围系统](../../WoWorld-Design/Happy%20Game/开发阶段/大气与氛围系统/) | 🟡 就绪 | 80% | 开发规格 (CHG-053 新建)。较新模块，细节可能待补充 |
| [小精灵系统](../../WoWorld-Design/Happy%20Game/开发阶段/小精灵系统/) | 🟡 就绪 | 80% | v1.0 (CHG-052)。较新模块，细节可能待补充 |
| [玩家系统](../../WoWorld-Design/Happy%20Game/开发阶段/玩家系统/) | 🟡 就绪 | 90% | ★ CHG-063 新建（2026-06-24）。6 篇~1,448 行。玩家=NPC+I/O适配层——角色创建/双角色托管/PlayerGoal/死亡继承/I/O差异清单 |
| [家具系统(旧)](../../WoWorld-Design/Happy%20Game/开发阶段/家具系统设计-旧.md) | 🔴 冻结 | — | v0.1。已由 物品系统/家具与放置物品 取代 (CHG-050) |

### 缺失模块（设计文档不存在）→ 🔴

| 模块 | 优先级 | 引用量 | 备注 |
|------|--------|--------|------|
| UI/UX 系统 | 致命缺失 → 🟡 已补全 | CHG-062 已创建（6篇·799行） | 轨 B.1 已完成。信息架构+HUD+对话+面板+接口性能预算 |
| ~~玩家系统深度扩展~~ | ✅ 已补全 | CHG-063 已创建（6篇·~1,448行） | 轨 B.2 已完成。玩家=NPC+I/O适配层 |
| 名声系统 | 高度缺失 | 6 文件, 17 引用 | 涌现式名声（分散式 NPC 记忆×信息传播×共识） |
| 法律与秩序 | 接口修补 | 分析结论：不需要独立模块 | 法律从 PowerAtoms+NPC 决策+信息传播涌现 |
| 采矿/农业/地下城/死亡传承/教程 | 中度缺失 | — | 轨 B5，按需排期 |
| 饮食/服饰/娱乐/冒险小队/Storyteller | 低优先级 | — | 轨 B6/D，按兴趣推进 |

---

## 已知问题追踪

### 孤儿接口冲突（5 个）— 轨 C Week 1-2

| # | 冲突描述 | 状态 | CHG |
|---|---------|------|-----|
| 1 | 所有权冲突 — 待 CHG-047 Phase 2 详细定位 | 未修复 | CHG-047/038 |
| 2-5 | 同上 | 未修复 | — |

### CHG-047 延期修复（16 项）

| 优先级 | 数量 | 关键项 |
|--------|------|--------|
| HIGH | 4 | 魔法性能预算缺失 ⬆ |
| MEDIUM | 9 | — |
| LOW | 3 | — |

### 模块保鲜待办

- [ ] 世界生成：5 篇文档 v1.0 → v2.1 升级
- [ ] 天气与季节系统：README 59→60 行
- [ ] 模块接头总览：README 46→60 行 + 12 模块时间戳更新
- [ ] NoiseHistoryQuery：CLAUDE-INTERFACES.md 7 方法 vs CHG-055 11 方法对齐

---

## Phase 里程碑

> Phase = `DEPENDENCY_GRAPH.md` 一个依赖层全部完成。宪法 §13 定义生命周期。

| Phase | 层 | 内容 | 状态 | 预计 Sprint 数 |
|-------|---|------|------|---------------|
| Phase 1 | 层 0 | `woworld_core` 核心类型 + GDExtension 链路验证 | ⏳ 待 Sprint-001 启动 | 2-3 |
| Phase 2 | 层 1 | 世界生成 + 存档 + 物品 + 技能 + 生命 + 天气 | ⏳ 阻塞于 Phase 1 | 6-10 |
| Phase 3 | 层 2 | 经济 + 战斗 + 魔法 + NPC 行动 + NPC 需求 | ⏳ 阻塞于 Phase 2 | 6-10 |
| Phase 4 | 层 3 | NPC 高阶 + 文化 + 信仰 + 权力 | ⏳ 阻塞于 Phase 3 | 4-6 |
| Phase 5 | 层 4 | 表现系统（模型/感官/语言/音频/建筑/载具/大气） | ⏳ 阻塞于 Phase 4 | 8-12 |

---

## 当前冲刺

**元冲刺 (2026-06-23)** ✅ 已完成 — 宪法 v1.1 + 全部附属文档 + CLAUDE.md 更新 + woworld/Cargo.toml release profile

**下一个冲刺**: 待用户启动 — 建议 Sprint-001: GDExtension 端到端链路验证 + woworld_core 最小启动类型

---

## 子组件进展追踪

> 审计缺口 3。模块实现跨多个冲刺时展开子组件。暂无进行中的模块实现——第一份子组件追踪将在 Sprint-001 后建立。

---

## 最近交接摘要

[handoff-20260623-001.md](handoff/handoff-20260623-001.md) — 元冲刺完成，宪法 v1.1 就位，待第一个正式冲刺。
