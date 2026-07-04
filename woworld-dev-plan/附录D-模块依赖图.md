# 附录D-模块依赖图.md — 模块实现依赖图

> **旧文件**: DEPENDENCY_GRAPH.md 内容已迁移至此。原文件保留作为历史参考。

> 从 `CLAUDE-INTERFACES.md` 契约 + 实际代码状态推导。
> Claude 在冲刺提案中必须标注"本冲刺的依赖前提已满足"。
>
> **维护者**: Claude Code（宪法冲刺级合规自审）
> **最后更新**: 2026-07-04 — 反映 Sprint-031 后的实际代码状态 + 5 个红色偏离全部修复

---

## Phase 映射

| 层 | Phase | 模块 | 状态 |
|-----|-------|------|------|
| 0 | Phase 1 | woworld_core + spatial | ✅ 完成 |
| 1 | Phase 1 | 世界生成 / 大气 / 存档 / 物品 / 技能 / 生命 / 天气 | 🟡 进行中 |
| 2 | Phase 3 | 经济 / 战斗 / 魔法 / NPC行动 / NPC基本需求 | 🔴 阻塞于 Phase 1 |
| 3 | Phase 3 | NPC认知 / 审美 / 生命周期 / 文化 / 信仰 / 权力 | 🔴 阻塞于 Phase 2 |
| 4 | Phase 3 | 模型动作 / 感官 / 语言表达 / 音频 / 建筑 / 载具 / UI | 🔴 阻塞于 Phase 2-3 |

---

## 实际代码依赖链（woworld/ Rust workspace）

```
woworld_core (glam 0.28)
  ├── woworld_spatial (woworld_core, glam)
  ├── woworld_worldgen (woworld_core, glam, noise 0.9)
  ├── woworld_atmosphere (woworld_core, serde, toml)
  └── woworld_godot (woworld_core, woworld_worldgen, godot 0.5)
```

woworld_core 是唯一零依赖 crate — 所有其他 crate 平等依赖它。

---

## 层 0 — 零依赖 ✅ 已实现

**`woworld_core`** — ID 类型 + trait 签名 + 共享数据结构：

| 类型/trait | 代码状态 | 消费者 |
|-----------|---------|--------|
| WorldPos, Vec3, Quat, Aabb | ✅ | 全部模块 |
| EntityId | ✅ | 全部模块 |
| ItemDefId, ItemEntId | ✅ 类型已定义 | 物品/经济/战斗/NPC/技能（均待实现） |
| SkillId, ChunkCoord | ✅ 类型已定义 | 技能/NPC/战斗（均待实现） |
| TerrainQuery trait (9方法) | ✅ | worldgen(实现) / godot / 未来NPC/载具/建筑 |
| EntityIndex trait (6方法) | ✅ | spatial(实现) / 未来感官/战斗 |
| SpatialEventBus trait (3方法) | ✅ | spatial(实现) / 未来感官/音频 |
| VisibilityQuery trait (2方法) | ✅ | spatial(实现) / 未来感官/战斗 |
| SaveableModule trait | ✅ trait 已定义 | 未来存档系统 |
| OceanProvider trait | ✅ trait 已定义 | 未来世界生成 |
| VegetationProvider trait | ✅ trait 已定义 | 未来世界生成/物品 |
| SurfaceMaterial (21变体) | ✅ | worldgen / godot |
| Medium (4变体) | ✅ | worldgen |
| WorldTime, WorldClock, TimeOfDay | ✅ | worldgen / atmosphere / godot |

---

## 层 1 — 依赖层 0，模块间独立

| 模块 | 代码状态 | 依赖前提 | 备注 |
|------|---------|---------|------|
| **世界生成** | 🟡 部分 | woworld_core | P0+P2 完成。5 个红色偏离全部修复。其余 P1/P2.25/P2.5/P3-P13 未实现 |
| **空间索引** | ✅ 完成 | woworld_core | GridEntityIndex + DdaVisibility + RingEventBus |
| **大气氛围** | 🟡 部分 | woworld_core | 核心管线完成。3/4 调制层为身份存根 |
| **存档系统** | — 未开始 | SaveableModule trait | 设计完备。LMDB 后端待 Sprint 排期 |
| **物品系统** | — 未开始 | ItemDefId, ItemEntId | 设计完备。待世界生成地基稳固后启动 |
| **技能系统** | — 未开始 | SkillId, SkillCategory | 设计完备 |
| **生命系统** | — 未开始 | EntityId | 设计完备 |
| **天气与季节** | — 未开始 | WeatherSample | 设计完备。大气模块有存根可对接 |

---

## 层 2 — 依赖层 1 的具体类型（全部未开始）

| 模块 | 依赖 | 阻塞于 |
|------|------|--------|
| 经济系统 | 物品系统 (ItemDefId, ItemProperties) | 物品系统未实现 |
| 战斗系统 | 技能 + 物品 + 生命 | 三个依赖均未实现 |
| 魔法系统 | 技能 + 生命 (Mana, SpiritState) | 🔴 零性能预算 — 额外阻塞 |
| NPC 行动涌现 | 技能 + 生命 | 两个依赖均未实现 |
| NPC 基本需求 | 物品 (ConsumableEffect) | 物品系统未实现 |

---

## 层 3 — 依赖层 2（全部未开始）

| 模块 | 依赖 |
|------|------|
| NPC 认知与智慧 | NPC 行动涌现 + 概念与语言地基 |
| NPC 审美 | NPC 认知 + 物品系统 |
| NPC 生命周期 | NPC 基本需求 + 生命系统 |
| 文化系统 | NPC 认知 + 语言表达 + 权力系统 |
| 信仰系统 | NPC 认知 + 文化系统 |
| 权力系统 | NPC 行动涌现 + 经济系统 |

---

## 层 4 — 表现与交互（全部未开始）

| 模块 | 依赖 |
|------|------|
| 模型动作与物理 | 世界生成 + NPC 行动涌现 + 物品系统 |
| 感官与知觉 | NPC 认知 + 世界生成 |
| 语言表达 | NPC 认知 + 概念与语言地基 |
| 音频系统 | 世界生成 + NPC 行动涌现 + 战斗 |
| 建筑模块 | 世界生成 + 物品系统 |
| 载具系统 | 世界生成/海洋 + 物理 |
| UI/UX | 全部模块（消费 Rust 数据） |

---

## 红色偏离对依赖链的冲击

5 个红色架构偏离全部在层 1 的世界生成中，现已全部修复。它们曾阻塞：

| 偏离 | 直接阻塞 | 状态 |
|------|---------|------|
| 🔴1 DensityField trait 残缺 | 多层密度组合（洞穴/矿脉/建筑地基/NPC编辑/玩家SDF）— 阻塞世界生成 P3-P13 | ✅ 已修复 |
| 🔴2 Seed u32 | 确定性生成不可靠 — 阻塞存档系统（存档重载→世界不一致） | ✅ 已修复 |
| 🔴3 Chunk 128m | LMDB 存储对齐 — 阻塞存档系统 | ✅ 已修复 |
| 🔴4 MC vs Transvoxel | LOD 接缝 — 阻塞大世界无缝漫游 | ✅ 已修复 |
| 🔴5 单层密度 | 阻塞世界生成 P3-P13（同 🔴1） | ✅ 已修复 |

**结论**：层 1 地基已稳固，层 2-4 阻塞解除。可继续推进上层模块。

---

## 当前实现路径

```
✅ 完成: 层 0 (woworld_core + spatial) — 核心类型 + 空间索引
✅ 已修复: 层 1 世界生成 (5 红色偏离全部修复)
⏳ 待启动: 层 1 存档/物品/技能/生命/天气
🔒 阻塞: 层 2-4（依赖层 1 完成）
```

> **关联**: [CONSTITUTION.md](CONSTITUTION.md) · [DEVELOPMENT_STATUS.md](DEVELOPMENT_STATUS.md) · [audit-reports/20250625-code-vs-design/](audit-reports/20250625-code-vs-design/README.md)
