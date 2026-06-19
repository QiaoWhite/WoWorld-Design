# 037 — 世界生成重构 · 跨模块补充需求

> **日期**: 2026-06-20
> **状态**: 设计草稿
> **关联**: [[../036-概念与语言地基设计探讨-20260619|036 概念与语言地基]] · [[../../Happy Game/开发阶段/世界生成/README|世界生成模块]] · [[../../Happy Game/开发阶段/模块接头总览/README|模块接头总览]]
> **说明**: 本文档系列输出世界生成模块完全重构对其他模块提出的新需求。每篇文档 = 一个模块需要补充的类型定义、接口暴露、数据合同。不包含世界生成内部逻辑。

---

## 背景

世界生成模块（9 篇文档，CHG-010）正在进行完全重构。新设计中，世界生成从"纯物理空间生成 + 统计 NPC 分配"升级为"编排器模式"——协调调用各领域模块的初始化接口，一次通过产出自洽的完整世界快照。

重构的关键变化：
- **P5 新增社会结构推导阶段**——从资源禀赋+技术+文化推导建筑规格、角色模板、权力骨架
- **P8 完全重写为人口投影与家族协同生成**——逆向人口投影算法，NPC 和 FamilyTree 同时产生
- **P9 完全重写为知识种子与历史事件生成**——KnowledgeSeed 传播链 + 因果事件图
- **P10-P11 新增权力+经济 Bootstrap**——初始权力边、初始订单簿、钱包分配
- **P3.5 新增生命世界初始化**——动植物怪物亡灵初始分布
- **P13 新增生成后完整性校验**——引用一致性、时间一致性、最小可玩性

这要求 12 个领域模块补充暴露初始化接口、新增类型定义、或修改现有类型以支持世界生成阶段的构造。

---

## 文档索引

| 编号 | 文档 | 模块 | 紧迫度 | 核心需求 |
|------|------|------|--------|---------|
| 001 | 建筑模块 | [[../../Happy Game/开发阶段/建筑模块/README|建筑模块 CHG-043]] | 🔴 高 | BuildingSpec 合同、generate_buildings() 接口、owner_npc_id、墓碑 |
| 002 | NPC 模块 | [[../../Happy Game/开发阶段/NPC活人感模块/NPC活人感开发文档ver2.0|NPC 活人感 v2.0]] | 🔴 高 | FamilyTree 类型定义、RelationEdge 初始化、SelfNarrative 废弃 |
| 003 | 生命系统 | [[../../Happy Game/开发阶段/生命/001-生命总纲|生命系统]] | 🔴 高 | initialize_life_world()、种群模板格式、动物简化行为树 |
| 004 | 权力系统 | [[../../Happy Game/开发阶段/权力系统/001-权力系统总纲|权力系统 CHG-023]] | 🔴 高 | PowerEdgeTemplate、bootstrap_power_topology()、边模板选择算法 |
| 005 | 经济系统 | [[../../Happy Game/开发阶段/经济系统/001-经济系统总纲|经济系统 CHG-022]] | 🔴 高 | ProfessionTag TOML consums/produces、bootstrap_economy()、初始价格 |
| 006 | 历史系统 | [[../../Happy Game/开发阶段/历史/001-历史系统总纲|历史系统]] | 🔴 高 | KnowledgeSeed 正式格式、generate_pre_history()、SettlementTechState |
| 007 | 文化系统 | [[../../Happy Game/开发阶段/文化系统/001-文化系统总纲|文化系统 CHG-024]] | 🟡 中 | individualism→家庭结构、power_distance→统治层级、建筑风格 |
| 008 | 天气系统 | [[../../Happy Game/开发阶段/天气与季节系统/001-天气系统总纲|天气系统 CHG-016]] | 🟢 低 | 确认 world_time 初始值下的 WeatherQuery 行为 |
| 009 | 概念与语言地基 | [[../../Happy Game/开发阶段/概念与语言地基/README|概念与语言地基 CHG-044]] | 🟡 中 | NpcData ~4.3KB 字段的初始化公式 |
| 010 | 信仰系统 | [[../../Happy Game/开发阶段/信仰系统/001-信仰系统总纲|信仰系统 CHG-025]] | 🟡 中 | FaithTheology.derive_individual_variant()、FaithProfile 初始化 |
| 011 | 魔法系统 | [[../../Happy Game/开发阶段/魔法/01-基础层/001-魔法总纲|魔法系统]] | 🟡 中 | MagicAffinity 类型、select_primary_element()、initial_mana_pool() |
| 012 | 载具系统 | [[../../Happy Game/开发阶段/载具系统/001-总纲与核心概念|载具系统]] | 🟡 中 | initialize_nomadic_vehicles()、游牧载具预生成、VehicleArchetype TOML |

---

## 总览：各模块需要暴露的接口签名

```
建筑模块:   generate_buildings(specs: &[BuildingSpec], terrain, rng) → Vec<BuildingData>
NPC 模块:   [类型] FamilyTree, FamilyEdge, FamilyRelation, RelationEdge
生命系统:   initialize_life_world(biomes, habitat, history, rng) → LifeWorldState
权力系统:   bootstrap_power_topology(skeleton, npcs, culture, rng) → PowerTopology
经济系统:   bootstrap_economy(settlements, npcs, resources, transport, rng) → EconomyState
历史系统:   generate_pre_history(knowledge_seeds, settlements, culture, rng) → (EventLog, SettlementTechState)
文化系统:   [补充] individualism→FamilyStructure, power_distance→AuthorityDepth
天气系统:   [确认] WeatherQuery::sample() 在 world_time=0 的正确行为
语言地基:   [补充] NpcData 语言字段的 culture×education→proficiency 映射
信仰系统:   FaithTheology::derive_individual_variant(), FaithProfile 初始化
魔法系统:   MagicAffinity 类型, select_primary_element(), initial_mana_pool()
载具系统:   initialize_nomadic_vehicles(), 游牧载具预生成
```

---

## 关键设计原则

1. **世界生成是编排器**——协调各模块的初始化调用。不拥有领域逻辑。
2. **各模块保持数据所有权**——世界生成可以构造模块拥有的数据类型，但运行时修改权归模块。
3. **Bootstrap 不是涌现的破坏**——它是初始条件。运行时的涌现从初始条件出发。
4. **生成与运行时使用同一套数据结构**——NpcData、PowerTopology、Market 等生成填充后，运行时直接消费。
5. **所有接口消费 DeterministicRng**——同一种子 → 同一世界。

---

> **下一步**: 各编号文档逐一展开具体需求。
