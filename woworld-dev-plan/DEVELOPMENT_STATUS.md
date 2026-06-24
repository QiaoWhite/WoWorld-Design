# DEVELOPMENT_STATUS.md — WoWorld 全局状态追踪

> **最后更新**: 2026-06-25
> **维护者**: Claude Code（按 CONSTITUTION.md §7 更新）
> **关联文件**: `CONSTITUTION.md` · `DEPENDENCY_GRAPH.md` · `../CLAUDE-INTERFACES.md`
> **审计基准**: `audit-reports/20250625-code-vs-design/README.md`

---

## 总体状态

| 指标 | 值 |
|------|-----|
| 设计模块总数 | 27 个独立系统 + 1 个子模块（家具与放置物品） |
| 有代码的模块 | **4 / 27**（世界生成、大气氛围、时间、空间索引） |
| 零代码的模块 | **23 / 27** — 设计完备，待实现 |
| 冻结模块 | **1**（魔法 — 性能预算未建立） |
| Rust workspace | 5 crates, **74 tests 全绿**, cargo clippy 零警告 |
| Godot 项目 | Godot 4.7 + GDExtension — MC 体素 + Clipmap LOD + 海洋 + 大气 + 昼夜 |
| 当前冲刺 | Sprint-006 待启动（地基修复） |
| 最新 CHG | CHG-064（2026-06-24）— 轨A 昼夜循环 + 5群系系统 |

---

## 一、代码模块（Rust Workspace）

### woworld_core — 🟢 稳定

核心类型 + trait 定义。仅 glam 依赖。引擎无关。

| 文件 | 内容 | 状态 |
|------|------|------|
| `types.rs` | WorldPos, EntityId, EntityKind(5), SpatialEntity, SpatialEvent, ScentSource, AcousticTag, TerrainHit, Aabb | ✅ 完整 |
| `id.rs` | ItemDefId, ItemEntId, SkillId, ProfessionTagId, ChunkCoord | ✅ 完整 |
| `spatial.rs` | TerrainQuery(9方法), EntityIndex(6方法), SpatialEventBus(3方法), VisibilityQuery(2方法) | ✅ 完整 |
| `material.rs` | SurfaceMaterial(21变体), Medium(4变体) | ✅ 完整 |
| `time.rs` | WorldTime, WorldClock, TimeOfDay — 昼夜循环权威定义 | ✅ 完整 |
| 测试 | 12 tests | ✅ 全绿 |

### woworld_spatial — 🟢 稳定

空间索引实现。依赖 woworld_core。

| 文件 | 内容 | 状态 |
|------|------|------|
| `entity_index.rs` | GridEntityIndex — 稀疏网格实体索引 | ✅ 完整 |
| `visibility.rs` | DdaVisibility — DDA 射线可见性查询 | ✅ 完整 |
| `event_bus.rs` | RingEventBus — 环形缓冲区事件总线 | ✅ 完整 |
| 测试 | 12 tests | ✅ 全绿 |

### woworld_worldgen — 🟡 部分实现（5 个架构偏离）

程序化世界生成。依赖 woworld_core + noise crate。

| 文件 | 内容 | 状态 |
|------|------|------|
| `noise_gen.rs` | 双层 Perlin 噪声 (continent+detail+mountain) + 气候场 | ✅ 完整 |
| `biome.rs` | 温度×降水 2D 噪声 → 5 群系硬盒分类 (TOML 数据驱动) | ⚠️ 硬盒分类 vs 设计规定的连续参数场 |
| `density.rs` | DensityField trait + HeightfieldDensity | 🔴 trait 残缺 — 缺 material_at/priority |
| `terrain.rs` | HeightfieldTerrain — 完整 TerrainQuery trait 实现 | ✅ 完整 |
| `marching_cubes.rs` | Marching Cubes 等值面提取 | 🔴 标准 MC vs 设计规定的 Transvoxel |
| `terrain_mesh.rs` | 纯 Rust 网格生成 (引擎无关) | ✅ 完整 |
| `chunk_manager.rs` | ChunkManager — 保留但未使用 (已被 clipmap 替代) | ⚠️ 僵尸代码 — 待清理或标注 |
| `clipmap.rs` | ClipmapManager — 4 层 Clipmap LOD | ⚠️ 4 层 vs 设计规定的 8 层 |
| 测试 | 39 tests | ✅ 全绿 |

**5 个红色架构偏离（阻塞后续开发）**：

| # | 偏离 | 设计规定 | 影响 | 计划 |
|---|------|---------|------|------|
| 🔴1 | DensityField trait 残缺 | trait 需 material_at(pos)→u8 + priority()→u8 | 多层密度组合不可能 | Sprint-006 |
| 🔴2 | Seed 类型 u32 | u64 + stage/chunk hash 派生 | 确定性生成不可靠 | Sprint-006 |
| 🔴3 | Chunk 大小 128m | 32m（LMDB 存储 + Clipmap tile 基本单元） | 持久化/Clipmap 对齐 | Sprint-007 |
| 🔴4 | 标准 Marching Cubes | Transvoxel（LOD 过渡需 transition cell） | LOD 接缝无法消除 | Sprint-007+ |
| 🔴5 | 单层密度 (HeightfieldDensity) | 11 层 L0-L10 可插拔 DensityProvider | 洞穴/矿脉/地基/NPC编辑/玩家SDF | Sprint-008+ |

### woworld_atmosphere — 🟡 部分实现

大气与氛围系统。依赖 woworld_core + serde + toml。

| 文件 | 内容 | 状态 |
|------|------|------|
| `time_curve.rs` | AtmosCurve — TOML 驱动的颜色/亮度时间曲线 | ✅ 完整 |
| `synthesizer.rs` | AtmosphereSynthesizer → ResolvedAtmosphere (35 floats) | ✅ 完整 |
| `resolved_atmosphere.rs` | ResolvedAtmosphere — PackedFloat32Array 输出 | ✅ 完整 |
| `traits.rs` | BiomeAtmosQuery, WeatherAtmosQuery, SeasonAtmosQuery | ⚠️ 身份存根 (passthrough, 无实际调制) |
| 测试 | 11 tests | ✅ 全绿 |

### woworld_godot — 🟡 部分实现

GDExtension 桥接层。cdylib → Godot 4.7。

| 文件 | 内容 | 状态 |
|------|------|------|
| `lib.rs` | WoWorldExtension GDExtension 入口 | ✅ 完整 |
| `terrain_chunk.rs` | WorldDriver GodotClass — ClipmapManager + 昼夜 + 大气 | ✅ 完整 |
| `ocean.rs` | OceanPlane GodotClass — Gerstner 波海洋 (6 波叠加, 800m²) | ✅ 完整 |
| 测试 | 0 tests（测试已迁移至 woworld_worldgen） | — |

---

## 二、设计模块准入等级

> 等级定义（宪法 §7）：🔴 冻结（不可编码）· 🟡 就绪（可编码，接口变更需审批）· 🟢 稳定（自由迭代）
>
> ★ 新增标记：`[代码]` = 已有部分代码 · `[设计]` = 仅设计文档

### 全局基础（3 个）

| 模块 | 等级 | 代码 | 备注 |
|------|------|------|------|
| 技术栈方案 | 🟡 就绪 | — | v4.0 权威方案 |
| 模块接头总览 | 🟡 就绪 | — | 102 文件/~6,100 行。12/27 模块时间戳待更新 |
| 存档系统 | 🟡 就绪 | — | v2.0 (CHG-055/056)。LMDB 方案完整 |

### 世界框架（4 个）

| 模块 | 等级 | 代码 | 备注 |
|------|------|------|------|
| 世界生成 | 🟡 部分实现 | ✅ 5 crates | 15 阶段管线仅完成 P0+P2。5 个红色偏离待修复 |
| 生命 | 🟡 就绪 | — | Vitals/Mana/DeathCause(30种6类) 契约完整 |
| 历史 | 🟡 就绪 | — | AetherImprint/KnowledgeSeed 契约完整 |
| 天气与季节系统 | 🟡 就绪 | — | WeatherSample/Markov 6-state 契约完整 |

### NPC 核心（2 个 + 7 子模块）

| 模块 | 等级 | 代码 | 备注 |
|------|------|------|------|
| NPC活人感模块 | 🟡 就绪 | — | v2.0 总规格。~8,000 行设计规格 |
| ↳ 03-基本需求系统 | 🟡 就绪 | — | 已审核 vs CHG-027 |
| ↳ 04-进阶需求系统 | 🟡 就绪 | — | ERG 挫折回归模型已定义 |
| ↳ 05-审美与艺术 | 🟡 就绪 | — | AestheticSignal(6 dims)+AestheticTaste SoA |
| ↳ 06-认知与智慧系统 | 🟡 就绪 | — | v1.1 (CHG-057/058/059)。PatternExpression 数学地基 |
| ↳ 07-生命周期系统 | 🟡 就绪 | — | v1.0 (CHG-041)。AgeClock/Gompertz/InfantDependency |
| ↳ 08-NPC行动涌现 | 🟡 就绪 | — | v1.0 (CHG-042)。3 层原子架构(35+~40+~25) |
| 概念与语言地基 | 🟡 就绪 | — | v1.0 (CHG-044)。3 层模型 |

### 社会系统（4 个）

| 模块 | 等级 | 代码 | 备注 |
|------|------|------|------|
| 经济系统 | 🟡 就绪 | — | v1.0。OrderBook/Market/NpcEconomicState |
| 权力系统 | 🟡 就绪 | — | v1.0。17 PowerAtoms/PowerTopology/Legitimacy |
| 文化系统 | 🟡 就绪 | — | v1.0。CultureCoreParams(10)/Voronoi 屏障/4 路径演化 |
| 信仰系统 | 🟡 就绪 | — | v1.0。FaithTheology(10)/实践先于教义 |

### 交互/表现/建造/辅助（13 个）

| 模块 | 等级 | 代码 | 备注 |
|------|------|------|------|
| 战斗 | 🟡 就绪 | — | 三层模型(本能→节奏→战略)/半自动 |
| **魔法** | **🔴 冻结** | — | **零性能预算 — 预算建立前不可编码** |
| 物品系统 | 🟡 就绪 | — | ItemDefId/Assembly/Enchantment/CraftingRecipe |
| 技能系统 | 🟡 就绪 | — | SkillId(5分类)/XP公式/天赋三层/教学四路径 |
| 语言表达 | 🟡 就绪 | — | ExpressionRef/Conversation/信息传播 5 通道 |
| 模型动作与物理 | 🟡 就绪 | — | 9 层动画栈/四 trait/5 子模块 |
| 音频系统 | 🟡 就绪 | — | SoundFootprint/AudioQuery(30 methods) |
| 感官与知觉系统 | 🟡 就绪 | — | PerceptBatch/4 查询 trait/PerceptualCache |
| 建筑模块 | 🟡 就绪 | — | ComponentFamily/WFC 2.5D/BuildingGenerator |
| 载具系统 | 🟡 就绪 | — | 5 动力类型/L1-L3 半自动控制 |
| 大气与氛围系统 | 🟡 部分实现 | ✅ woworld_atmosphere | 3/4 调制层为身份存根 |
| 小精灵系统 | 🟡 就绪 | — | v1.0 (CHG-052) |
| 玩家系统 | 🟡 就绪 | — | ★ CHG-063。6篇~1,448行。玩家=NPC+I/O适配层 |

### 设计补全待办（Track B/C 遗留）

| 模块 | 优先级 | 说明 |
|------|--------|------|
| 名声系统 | 高 | 涌现式名声（NPC记忆×信息传播×共识）。6 文件引用，零设计文档 |
| 法律与秩序 | 高 | 分析结论：不需要独立模块。法律从 PowerAtoms+NPC 决策涌现。需接口修补文档 |
| 魔法性能预算 | 高 | 2,328 行设计规格，零性能预算 — 轨 C 最高风险项 |
| 采矿/农业/地下城/死亡传承/教程 | 中 | 按需排期 |
| 饮食/服饰/娱乐 | 低 | 按兴趣推进 |

---

## 三、当前冲刺

**Sprint-006 待启动** — 地基修复：DensityField trait + Seed u64（详见 `sprint-proposals/`）

**上次冲刺历史**：

| Sprint | 日期 | 目标 | 状态 |
|--------|------|------|------|
| Sprint-005 | 2026-06-25 | A.6 里程碑收尾（地形尺度 + 海洋 + 玩家调优） | ✅ 完成 |
| Sprint-004 | 2026-06-25 | Clipmap LOD 4 层 | ✅ 完成 |
| Sprint-003 | 2026-06-25 | MC 体素提取 | ✅ 完成 |
| Sprint-002 | 2026-06-25 | Chunk 分块 + WorldDriver | ✅ 完成 |

---

## 四、已知问题追踪

### 红色架构偏离（阻塞后续）— 详见 §一 woworld_worldgen

🔴1 DensityField trait · 🔴2 Seed u32 · 🔴3 Chunk 128m · 🔴4 MC vs Transvoxel · 🔴5 单层密度

### 轨 C 遗留（设计债务）

| # | 项 | 状态 |
|---|----|------|
| 1-5 | 5 个孤儿接口所有权冲突 | 未修复（CHG-047 Phase 2） |
| 6 | 魔法性能预算缺失 | 未修复 — 最高风险 |
| 7 | 世界生成 5 篇文档 v1.0→v2.1 | 未修复 |
| 8 | 模块接头总览 README 46→60 行 + 12 模块时间戳更新 | 未修复 |

### 治理待办

- [ ] 宪法 v1.4 用户审批（自 v1.1 起待审批）
- [ ] 根目录 `session-handoff.md` 归档（旧格式，已过时）
- [ ] `chunk_manager.rs` 僵尸代码标注（保留但未使用）

---

## 五、最近交接摘要

| 文件 | 内容 |
|------|------|
| [handoff-20260625-003.md](handoff/handoff-20260625-003.md) | ★ 最新 — Sprint-002~005 全量推进（MC+Clipmap+海洋） |
| [handoff-20260624-002.md](handoff/archived/handoff-20260624-002.md) | CHG-064 偏离修复 + 架构重构 |
| [handoff-20260623-001.md](handoff/archived/handoff-20260623-001.md) | 元冲刺·宪法 v1.1 建立 |

---

> **关联**: [CONSTITUTION.md](CONSTITUTION.md) · [DEPENDENCY_GRAPH.md](DEPENDENCY_GRAPH.md) · [audit-reports/20250625-code-vs-design/](audit-reports/20250625-code-vs-design/README.md)
