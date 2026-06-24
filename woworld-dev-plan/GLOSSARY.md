# GLOSSARY.md — 中英术语映射登记册

> 每次在代码中创建新的公开类型（enum/struct/trait）时，必须在此登记中英对照。
> 已有术语来自 `CLAUDE-INTERFACES.md` 27 个契约段。新术语在编码冲刺中追加。
>
> **维护者**: Claude Code（宪法 §4 提交前置检查强制登记）

---

## 核心类型

| 中文 | 英文 | 定义位置 | 说明 |
|------|------|---------|------|
| 世界坐标 | `WorldPos` | `woworld_core` | f64×3，世界空间绝对坐标 |
| 实体标识 | `EntityId` | `woworld_core` | u64，全局唯一 |
| 物品定义标识 | `ItemDefId` | `woworld_core` | u64 (8+56bit)，全局恒定 |
| 物品实体标识 | `ItemEntId` | `woworld_core` | u64，存档内唯一 |
| 技能标识 | `SkillId` | `woworld_core` | u64 (8+8+16+32bit)，5 分类 22 子组 |
| 块坐标 | `ChunkCoord` | `woworld_core` | 垂直稀疏 Chunk 索引 |

---

## 空间查询四 trait

| 中文 | 英文 | 定义位置 |
|------|------|---------|
| 地形查询 | `TerrainQuery` | `woworld_core::spatial` |
| 实体索引 | `EntityIndex` | `woworld_core::spatial` |
| 空间事件总线 | `SpatialEventBus` | `woworld_core::spatial` |
| 可见性查询 | `VisibilityQuery` | `woworld_core::spatial` |

---

## 世界框架

| 中文 | 英文 | 定义位置 |
|------|------|---------|
| 群系 | `Biome` | 世界生成（参数场，非离散标签） |
| 生命体征 | `Vitals` | Life `004-身体状态与生命过程` |
| 魔力/法力 | `Mana` | Life `004 §四` |
| 死亡原因 | `DeathCause` | Life `004 §九`（30 种 6 类） |
| 死亡原因注册表 | `DeathCauseRegistry` | Life |
| 灵元素印记 | `AetherImprint` | History `004` |
| 灵元素查询 | `AetherQuery` | History `006` |
| 天气样本 | `WeatherSample` | 天气与季节系统 |
| 季节时钟 | `SeasonClock` | 天气与季节系统 |
| 可保存模块 | `SaveableModule` | 存档系统（8→14 方法） |
| 噪声历史查询 | `NoiseHistoryQuery` | 存档系统（11 方法） |

---

## NPC 核心

| 中文 | 英文 | 定义位置 |
|------|------|---------|
| 认知风格 | `CognitiveStyle` | NPC 认知（4 维度） |
| 认知潮汐 | `CognitiveTide` | NPC 认知（3 维度） |
| 心智模型 | `MentalModel` | NPC 认知（≤20 条目，6 继承路径） |
| 模式表达 | `PatternExpression` | NPC 认知 v1.1 |
| 年龄时钟 | `AgeClock` | NPC 生命周期 |
| 婴儿依赖状态机 | `InfantDependency` | NPC 生命周期 |
| 繁殖潜力 | `FertilityPotential` | NPC 生命周期 |
| 行动原子（物理/领域/社交） | `ActionAtom` | NPC 行动涌现（3 层 ~100 个） |
| 智能体快照 | `AgentSnapshot` | NPC 行动涌现 |
| 材料属性 | `MaterialProperties` | NPC 行动涌现 |
| 技能条目 | `SkillEntry` | 技能系统（xp/level/innate_aptitude） |
| 需求敏感度 | `NeedSensitivity` | NPC 基本需求（8 字段） |
| 审美信号 | `AestheticSignal` | NPC 审美（6 维度） |
| 拥有审美信号 | `HasAestheticSignal` | NPC 审美（12 实现者） |

---

## 社会系统

| 中文 | 英文 | 定义位置 |
|------|------|---------|
| 订单簿匹配 | `OrderBook` | 经济系统 |
| 市场 | `Market` | 经济系统 |
| 商店门面 | `Storefront` | 经济系统 |
| NPC 经济状态 | `NpcEconomicState` | 经济系统 |
| 权力原子 | `PowerAtom` | 权力系统（17 种） |
| 权力拓扑 | `PowerTopology` | 权力系统（有向多重图） |
| 合法性 | `Legitimacy` | 权力系统（5 因子） |
| 文化核心参数 | `CultureCoreParams` | 文化系统（10 参数） |
| 交流规范 | `CommunicationNorms` | 文化系统 |
| 仪式定义 | `RitualDef` | 文化系统 |
| 信仰神学参数 | `FaithTheology` | 信仰系统（10 参数） |
| 信仰查询 | `FaithQuery` | 信仰系统（30 方法） |

---

## 交互/表现/建造

| 中文 | 英文 | 定义位置 |
|------|------|---------|
| 物品属性 | `ItemProperties` | 物品系统（Quality 4 档 × Rarity 5 档） |
| 装备槽位 | `EquipmentSlots` | 物品系统 |
| 制造配方 | `CraftingRecipe` | 物品系统（TOML 数据） |
| 技能分类 | `SkillCategory` | 技能系统（5 类） |
| 表达引用 | `ExpressionRef` | 语言表达（8B） |
| 内容解析器 | `ContentResolver` | 语言表达 |
| 文本生成器 | `TextGenerator` | 语言表达 |
| LLM 场景配置 | `LlmSceneConfig` | 语言表达（19 开关） |
| LLM 后端 | `LlmBackend` | 语言表达（trait） |
| 语音配置 | `VoiceProfile` | 音频系统 |
| 语音优先级 | `VoicePriority` | 音频系统（5 级） |
| 声音足迹 | `SoundFootprint` | 音频系统 |
| 感知批次 | `PerceptBatch` | 感官与知觉系统 |
| 建筑组件族 | `ComponentFamily` | 建筑模块（9 core + Mod） |
| 建筑生成器 | `BuildingGenerator` | 建筑模块（trait，8 类型） |
| 载具标识 | `VehicleId` | 载具系统 |
| 载具原型 | `VehicleArchetype` | 载具系统（5 动力类型） |
| 植被供应者 | `VegetationProvider` | 世界生成（trait，7 方法） |
| 海洋供应者 | `OceanProvider` | 世界生成（trait，6 方法） |
| 放置商店 | `PlacementStore` | 家具与放置物品（32B hot） |
| 功能集 | `AffordanceSet` | 家具与放置物品（64-bit） |
| LOD 处方 | `LodPrescription` | LOD 架构（7 维） |
| LOD 协调器 | `LODCoordinator` | LOD 架构（8 步） |

---

> **最后更新**: 2026-06-23 — 初始登记，从 CLAUDE-INTERFACES.md 契约段批量导入。后续冲刺按需追加。
