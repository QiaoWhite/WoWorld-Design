# 建筑模块

> **开发代号**: WoWorld (Wonder World)
> **引擎**: Godot 4.6.2
> **开发者**: 独立游戏开发者（Solo）
> **创建日期**: 2026-06-19
> **状态**: ✅ v1.0
> **位置**: `woworld_building` crate（Rust 侧）

---

## 模块定位

建筑模块是 WoWorld 中**建筑物理与建造规则**的权威模块。定义建筑的组件化数据模型、约束求解、施工调度，通过 trait 与其他模块解耦。

本模块不定义风格（→ [[文化系统]]）、不定义材料（→ [[物品系统]] / [[NPC活人感模块/08-NPC行动涌现与分类/001-NPC行动涌现总纲|NPC 物理原子层]]）、不定义功能（→ 涌现自 NPC 使用）。

---

## 文档索引

| # | 文档 | 内容 |
|---|------|------|
| 001 | [[建筑模块/001-组件族定义与注册|组件族定义与注册]] | ComponentFamily trait、完整族列表、ConnectionFace 规则、ComponentRegistry |
| 002 | [[建筑模块/002-建筑数据模型|建筑数据模型]] | Building、ComponentInstance、BuildingHistory、BuildingSpatialIndex、所有权 |
| 003 | [[建筑模块/003-建筑查询接口|建筑查询接口]] | BuildingQuery trait 完整签名、Surface trait、Portal、StructureValueFactors |
| 004 | [[建筑模块/004-约束求解系统|约束求解系统]] | WFC 2.5D 三阶段、StructuralSolver、BlueprintValidator、LocalIncrementalSolver |
| 005 | [[建筑模块/005-施工调度系统|施工调度系统]] | ConstructionScheduler、ConstructionTask、MaterialRequirementList、ConstructionModifier |
| 006 | [[建筑模块/006-Blueprint蓝图系统|Blueprint 蓝图系统]] | TOML 格式、三段结构、子蓝图引用、验证器、跨文化移植 |
| 007 | [[建筑模块/007-建筑生成器族谱|建筑生成器族谱]] | BuildingGenerator trait、8 种生成器、建筑类型→生成器映射 |
| 008 | [[建筑模块/008-跨模块接口与数据合同|跨模块接口与数据合同]] | BuildContext、BuildingQuery、Surface、依赖清单、所有权裁决 |
| 009 | [[建筑模块/009-性能预算与存储分析|性能预算与存储分析]] | 帧预算、LOD 三层内存、VRAM、存储、WFC 耗时分析 |

---

## 架构速览

```
woworld_building crate:

ComponentRegistry（组件族注册 + 连接兼容性矩阵）
├─ L1 数据层: Building, ComponentInstance, BuildingHistory, BuildingQuery trait
├─ L2 求解层: WfcBuildingSolver, StructuralSolver, BlueprintValidator, LocalIncrementalSolver
└─ L3 施工层: ConstructionScheduler, ConstructionTask, ConstructionModifier

BuildingGenerator trait → 8 种生成器实现
```

**对外三扇门**：
- `BuildContext`（入）— 六个外部参数聚合为一个 struct，调用方填充
- `BuildingQuery`（出）— 建筑世界数据的唯一只读接口
- `ConstructionTask + MaterialRequirementList`（出）— 施工需求的声明式产出

---

## 关联模块

| 模块 | 关联 |
|------|------|
| [[世界生成]] | 编排建筑生成，调用 BuildingGenerator |
| [[物品系统]] | 家具物品定义 + PlacedItem 放置 |
| [[NPC活人感模块]] | CONSTRUCT 原子消费、认知消费 BuildingQuery |
| [[文化系统]] | CultureBuildProfile → BuildContext |
| [[信仰系统]] | FaithBuildProfile + SacredArchitectureParams |
| [[天气与季节系统]] | ClimateBuildProfile → BuildContext |
| [[生命系统]] | RaceBodyPlan → BuildContext |
| [[经济系统]] | StructureValueFactors → 房产估值 |
| [[审美系统]] | 消费 BuildingQuery |
| [[历史系统]] | AetherImprint 消费建筑事件 |
| [[模型动作与物理系统]] | BuildingMaterialProps 派生自 MaterialProperties |
| [[魔法]] | ConstructionModifier trait |

---

## 关联参考文档

- [[035-建筑模块设计探讨-20260619/001-建筑模块大纲|建筑模块大纲（参考文档）]]
- [[CLAUDE-INTERFACES.md]]
- [[CLAUDE.md]]
