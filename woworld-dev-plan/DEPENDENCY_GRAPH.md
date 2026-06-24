# DEPENDENCY_GRAPH.md — 模块实现依赖图

> 从 `CLAUDE-INTERFACES.md` 契约推导。每往 `woworld_core` 新增类型时同步更新。
> Claude 在冲刺提案中必须标注"本冲刺的依赖前提已满足"。
>
> **维护者**: Claude Code（宪法冲刺级合规自审）

---

## 层 0 — 零依赖（所有模块平等依赖）

**`woworld_core`** — ID 类型 + trait 签名 + 共享数据结构：

| 类型/trait | 消费者（设计模块） |
|-----------|-----------------|
| `WorldPos`, `Vec3`, `Quat`, `Aabb` | 全部 25 模块 |
| `EntityId` | 全部模块 |
| `ItemDefId`, `ItemEntId` | 物品系统 / 经济系统 / 战斗 / NPC / 技能 |
| `SkillId`, `SkillCategory`, `SkillEntry` | 技能系统 / NPC / 战斗 / 魔法 |
| `TerrainQuery` trait | 世界生成 / NPC / 载具 / 建筑 |
| `EntityIndex` trait | 全部需要空间查询的模块 |
| `SpatialEventBus` trait | 战斗 / NPC / 物理 |
| `VisibilityQuery` trait | NPC 感知 / 战斗 / 建筑 |
| `SaveableModule` trait | 全部需要持久化的模块 |
| `OceanProvider` trait | 世界生成 / 载具 |
| `VegetationProvider` trait | 世界生成 / 物品系统（采集） |

**无 crate 依赖，所有模块可并行启动** → 这是第一个冲刺的"安全区"。

---

## 层 1 — 仅依赖层 0（模块间互相无依赖）

| 模块 | 依赖前提 | 可并行 |
|------|---------|--------|
| **世界生成** (P0-P13) | `WorldPos`, `Biome`, `TerrainQuery` trait | ✅ 与层 1 其他模块并行 |
| **存档系统** | `SaveableModule` trait | ✅ |
| **物品系统** (ID + Properties + Assembly) | `ItemDefId`, `ItemEntId` | ✅ |
| **技能系统** (ID + XP + Teaching) | `SkillId`, `SkillCategory` | ✅ |
| **生命系统** (Vitals + DeathCause) | `EntityId` | ✅ |
| **天气与季节系统** | `WeatherSample`, `SeasonClock` | ✅ |

---

## 层 2 — 依赖层 1 的具体类型

| 模块 | 依赖 |
|------|------|
| **经济系统** | 物品系统（`ItemDefId`, `ItemProperties`） |
| **战斗系统** | 技能系统（`SkillId`）+ 物品系统（武器/防具 `ItemDefId`）+ 生命系统（`Vitals`） |
| **魔法系统** | 技能系统 + 生命系统（`Mana`, `SpiritState`） |
| **NPC 行动涌现** | 技能系统（`SkillEntry`）+ 生命系统（`Vitals`） |
| **NPC 基本需求** | 物品系统（消费品 `ConsumableEffect`） |

---

## 层 3 — 依赖层 2

| 模块 | 依赖 |
|------|------|
| **NPC 认知与智慧** | NPC 行动涌现 + 概念与语言地基 |
| **NPC 审美** | NPC 认知 + 物品系统（`AestheticProps`） |
| **NPC 生命周期** | NPC 基本需求 + 生命系统 |
| **文化系统** | NPC 认知 + 语言表达 + 权力系统 |
| **信仰系统** | NPC 认知 + 文化系统 |
| **权力系统** | NPC 行动涌现 + 经济系统 |

---

## 层 4 — 表现与交互（依赖层 2-3）

| 模块 | 依赖 |
|------|------|
| **模型动作与物理** | 世界生成（地形） + NPC 行动涌现 + 物品系统 |
| **感官与知觉** | NPC 认知 + 世界生成（地形采样） |
| **语言表达** | NPC 认知 + 概念与语言地基 |
| **音频系统** | 世界生成（环境） + NPC 行动涌现 + 战斗 |
| **建筑模块** | 世界生成（地形） + 物品系统（材料） |
| **载具系统** | 世界生成（地形/海洋） + 物理 |
| **大气与氛围** | 天气与季节 + 世界生成 |

---

## 实现顺序建议

```
Phase 1 (Sprint 1-N):  层 0 核心类型 + GDExtension 链路验证
Phase 2:               层 1 全部（世界生成优先——地形是一切的基础）
Phase 3:               层 2 全部（经济/战斗/魔法/NPC 行动/需求可并行）
Phase 4:               层 3 社会系统（权力/文化/信仰可并行）+ NPC 高阶（认知/审美/生命周期可并行）
Phase 5:               层 4 表现系统（按依赖链逐个，部分可并行）
```

> **最后更新**: 2026-06-23 — 初始版本，从 CLAUDE-INTERFACES.md 27 契约段推导。
