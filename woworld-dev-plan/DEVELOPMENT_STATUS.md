# DEVELOPMENT_STATUS.md — WoWorld 全局状态追踪

> **最后更新**: 2026-06-28（★ 2026-07-09 追加：CHG-067 运动学地基待实现冲刺，见 §二/§三。其余条目仍为 06-28 状态，与实际进度[Sprint-061/807 tests]有滞后，待整体刷新）
> **维护者**: Claude Code（按 CONSTITUTION.md §7 更新）
> **关联文件**: `CONSTITUTION.md` · `DEPENDENCY_GRAPH.md` · `../CLAUDE-INTERFACES.md`
> **审计基准**: `audit-reports/20250625-code-vs-design/README.md`

---

## 总体状态

| 指标 | 值 |
|------|-----|
| 设计模块总数 | 27 个独立系统 + 1 个子模块（家具与放置物品） |
| 有代码的模块 | **5 / 27**（世界生成、大气氛围、时间、空间索引、**植被**） |
| 零代码的模块 | **22 / 27** — 设计完备，待实现 |
| 冻结模块 | **1**（魔法 — 性能预算未建立） |
| Rust workspace | 6 crates, **139 tests 全绿**, cargo clippy 零警告 |
| Godot 项目 | Godot 4.7 + GDExtension — Transvoxel 完整（常规+过渡）+ Clipmap LOD 6 级 CHG-049 对齐（0.5m-16m voxel, 4km 视野）+ Signed Heightfield (scene_lod 5) + 海洋 + 大气 + 昼夜 |
| 当前冲刺 | Sprint-015 完成（Signed Heightfield — scene_lod 5 远距离渲染）→ 下一步待定 |
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

### woworld_worldgen — 🟡 部分实现（零架构偏离）

程序化世界生成。依赖 woworld_core + noise crate。

| 文件 | 内容 | 状态 |
|------|------|------|
| `noise_gen.rs` | 双层 Perlin 噪声 (continent+detail+mountain) + 气候场 + Worley 3D | ✅ 完整 |
| `biome.rs` | 温度×降水 2D 噪声 → 5 群系硬盒分类 (TOML 数据驱动) | ⚠️ 硬盒分类 vs 设计规定的连续参数场 |
| `density.rs` | DensityField trait + DensityStack + CaveDensity 装饰器 | ✅ Sprint-014 多层密度地基 |
| `terrain.rs` | HeightfieldTerrain — 完整 TerrainQuery trait 实现 | ✅ 完整 |
| `marching_cubes.rs` | 等值面查找表 + MC 参考实现（仅供 Transvoxel 对比测试） | ⚠️ 查找表为 Transvoxel 数学地基；`extract_isosurface` 降级为 pub(crate) |
| `transvoxel.rs` | ★ Transvoxel 完整实现（常规+过渡单元，顶点共享） | ✅ Sprint-011 完成 |
| `transition_tables.rs` | ★ 过渡单元查找表（auto-generated, ~610 行） | ✅ Sprint-011 新增 |
| `terrain_mesh.rs` | 纯 Rust 网格生成 — SH + 高度场 + 共享索引 | ✅ Sprint-015 SH 新增 |
| `clipmap.rs` | ClipmapManager — 6 层 Clipmap LOD CHG-049 对齐 | ✅ Transvoxel (LOD 0-4) + SH (LOD 5) |
| 测试 | 76 tests（Sprint-016 清理 -12 死代码测试） | ✅ 全绿 |

**架构偏离：零（5/5 已修复）**：

| # | 偏离 | 状态 |
|---|------|------|
| ✅1 | DensityField trait 残缺 | Sprint-006 |
| ✅2 | Seed u32 | Sprint-006 |
| ✅3 | Chunk 128m | Sprint-012 |
| ✅4 | MC vs Transvoxel | Sprint-011 |
| ✅5 | 单层密度 vs 11 层 | Sprint-014 — DensityStack + CaveDensity |

**LOD 偏差（vs CHG-049）**：

| #   | 偏差                      | 状态                                                  |
| --- | ----------------------- | --------------------------------------------------- |
| ✅1  | 缺 scene_lod 0 (0.5m)    | Sprint-013                                          |
| ✅2  | LOD 距离带偏移               | Sprint-013                                          |
| 🟡3 | 远距离全 Transvoxel (应为 SH) | **Sprint-015: scene_lod 5 SH ✓ / scene_lod 6-7 仍缺** |
| 🟡4 | 缺 scene_lod 6-7 (4km+)  | 后续——复用 SH 代码路径                                      |
| 🟡5 | 缺 LODCoordinator        | 后续                                                  |
| 🟡6 | 远距离人造结构不可见              | 需灯光烘焙 + 建筑群轮廓系统，等待建筑模块                              |

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
| 模型动作与物理 | 🟡 就绪 | — | 9 层动画栈/四 trait/5 子模块。★ **CHG-067 运动学地基**（质量/冲量/COM/投射物/挂载/攀爬）设计完成，**待实现冲刺** → `sprint-proposals/BACKLOG-物理运动学地基-实现-20260709.md` |
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

### woworld_vegetation — 🟡 部分实现（基础设施就位）

植被覆盖层。依赖 woworld_core + woworld_worldgen。

| 文件 | 内容 | 状态 |
|------|------|------|
| `community.rs` | Shannon 熵优势种筛选（纯函数） | ✅ 完整 — 6 测试 |
| `species.rs` | TOML 物种表 + 高斯适应度计算 | ✅ 完整 — 4 测试 |
| `noise.rs` | VegetationNoise（3 层 Perlin） | ✅ 完整 — 3 测试 |
| `provider.rs` | VegetationStub 存根实现 | ⚠️ 存根 — 所有方法返回默认值 |
| 测试 | 16 tests | ✅ 全绿 |

---

## 三、当前冲刺

**下一个冲刺**: 多层密度 L0-L10 — 最后一个架构偏离（🔴5），解锁洞穴/矿脉/地基/NPC 编辑/玩家 SDF。详见最新交接摘要 `handoff/handoff-20260701-019.md`。

**待触发冲刺队列（防遗漏 backlog）**：

| 冲刺 | 状态 | 暂缓依据 | 提案 |
|------|------|---------|------|
| ★ 物理运动学地基·实现 | 🟡 待触发 | CHG-067 Q-A2（设计已定，只留文档） | `sprint-proposals/BACKLOG-物理运动学地基-实现-20260709.md` |

**本次会话冲刺历史**：

| Sprint | 日期 | 目标 | 状态 |
|--------|------|------|------|
| Sprint-015 | 2026-06-28 | ★ Signed Heightfield — scene_lod 5 远距离渲染 | ✅ 完成 |
| Sprint-014 | 2026-06-28 | ★ 多层密度 L0-L10 — DensityStack + CaveDensity | ✅ 完成 |
| Sprint-013 | 2026-06-28 | ★ LOD 重构 — CHG-049 6 级对齐 + scene_lod 0 (0.5m) | ✅ 完成 |
| Sprint-012 | 2026-06-28 | ★ Chunk 128m→32m + LOD 5 级全 Transvoxel (2048m) | ✅ 完成 |
| Sprint-011 | 2026-06-25 | ★ Transvoxel 过渡单元 + L1 Transvoxel 化 | ✅ 完成 |
| Sprint-010 | 2026-06-25 | Transvoxel 常规单元提取（顶点共享） | ✅ 完成 |
| Sprint-009 | 2026-06-25 | 植被 P2.25 基础设施（trait + Shannon 熵 + 物种表） | ✅ 完成 |
| Sprint-008 | 2026-06-25 | Async 后台 mesh 生成（rayon + mpsc） | ✅ 完成 |
| Sprint-007 | 2026-06-25 | 性能修复（poll 帧预算 + 合并查询 + 海洋着色） | ✅ 完成 |
| Sprint-006 | 2026-06-25 | 地基修复（DensityField trait + Seed u64） | ✅ 完成 |
| Sprint-005 | 2026-06-25 | A.6 里程碑收尾（地形尺度 + 海洋 + 玩家调优） | ✅ 完成 |
| Sprint-004 | 2026-06-25 | Clipmap LOD 4 层 | ✅ 完成 |
| Sprint-003 | 2026-06-25 | MC 体素提取 | ✅ 完成 |
| Sprint-002 | 2026-06-25 | Chunk 分块 + WorldDriver | ✅ 完成 |

---

## 四、已知问题追踪

### 红色架构偏离（阻塞后续）— 详见 §一 woworld_worldgen

🔴1 DensityField trait · 🔴2 Seed u32 · 🔴3 Chunk 128m · 🔴4 MC vs Transvoxel · 🔴5 单层密度

> ✅1-4 已修复（Sprint-006, 011, 012）。仅剩 🔴5 单层密度。

### 轨 C 遗留（设计债务）

| # | 项 | 状态 |
|---|----|------|
| 1-5 | 5 个孤儿接口所有权冲突 | 未修复（CHG-047 Phase 2） |
| 6 | 魔法性能预算缺失 | 未修复 — 最高风险 |
| 7 | 世界生成 5 篇文档 v1.0→v2.1 | 未修复 |
| 8 | 模块接头总览 README 46→60 行 + 12 模块时间戳更新 | 未修复 |

### 治理待办

- [ ] 宪法 v1.4 用户审批（自 v1.1 起待审批）
- [x] `session-handoff.md` 根目录旧格式清理 + 交接文档集中化 → ✅ 2026-07-01 完成
- [x] `chunk_manager.rs` 删除 → ✅ Sprint-016 退役

---

## 五、最近交接摘要

| 文件 | 内容 |
|------|------|
| [handoff-20260704-024.md](01-核心基础/handoff/handoff-20260704-024.md) | ★ 最新 — Sprint-029~032（Transvoxel + 海洋 + VoxelChunk 修复）|
| [handoff-20260704-023.md](01-核心基础/handoff/handoff-20260704-023.md) | Sprint-028 完成 |
| [handoff-20260704-022.md](01-核心基础/handoff/handoff-20260704-022.md) | Sprint-027 准备 |
| [handoff-20260703-021.md](01-核心基础/handoff/handoff-20260703-021.md) | Sprint-026 |
| [handoff-20260702-020.md](01-核心基础/handoff/handoff-20260702-020.md) | Sprint-025 |
| [handoff-20260701-019.md](01-核心基础/handoff/handoff-20260701-019.md) | Sprint-020~024（GPU-driven clipmap + Floating Origin）|
| [archived/handoff-20260629-016.md](01-核心基础/handoff/archived/handoff-20260629-016.md) | Sprint-018（性能卡顿修复）|
| [archived/handoff-20260628-015.md](01-核心基础/handoff/archived/handoff-20260628-015.md) | Sprint-017 就绪 |
| [archived/handoff-20260628-014.md](01-核心基础/handoff/archived/handoff-20260628-014.md) | Sprint-016 就绪 |

---

> **关联**: [CONSTITUTION.md](CONSTITUTION.md) · [DEPENDENCY_GRAPH.md](DEPENDENCY_GRAPH.md) · [audit-reports/20250625-code-vs-design/](audit-reports/20250625-code-vs-design/README.md)
