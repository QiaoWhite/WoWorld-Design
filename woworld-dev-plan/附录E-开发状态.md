> **旧文件**: DEVELOPMENT_STATUS.md 内容已迁移至此。原文件保留作为历史参考。

# DEVELOPMENT_STATUS.md — WoWorld 全局状态追踪

> **最后更新**: 2026-07-12（Sprint-071 V3a 代谢闭环·1111 tests）
> **维护者**: Claude Code（按 CONSTITUTION.md §7 更新）
> **关联文件**: `CONSTITUTION.md` · `附录D-模块依赖图.md` · `../CLAUDE-INTERFACES.md`

> **活文档**: 本文件每个冲刺结束时更新。是开发流程体系的「当前位置」权威数据源。

---

## 总体状态

| 指标 | 值 |
|------|-----|
| 设计模块总数 | 27 个独立系统 + 1 个子模块（家具与放置物品） |
| 有代码的模块 | **9 / 27**（世界生成、大气氛围、时间、空间索引、植被、生命系统、地形修改编排层、玩家系统 Phase 1、**★模型动作与物理（角色控制器）**） |
| 零代码的模块 | **21 / 27** — 设计完备，待实现 |
| 冻结模块 | **1**（魔法 — 性能预算未建立） |
| Rust workspace | 5 crates, **1111 tests 全绿** (core: 401 + worldgen: 75 + atmosphere: 26 + ecs: 609（600 lib + 9 集成） + godot: 0), cargo clippy 零警告 |
| ECS 架构 | **Phase 0/1/2/3 ✅** — 55 Components + 41 Systems + 600 lib tests。社会×4 + 物品 Phase 2 + 经济 Phase 3 + 玩家 Phase 1 + 对话气泡 MVP + 角色控制器核心三层 + Step 5e 管线集成 + 第三人称相机 MVP + 持续/充能动作运行时 + 遭遇感知层（问候/告别气泡·barrier-free）+ **★Sprint-069 Poisson disc 采集物生成（Bridson 2007·`query_harvestable` 真实返回）** + **★Sprint-070 V2 牵引移动（`goal_resolution_system` 同步填充 `target_pos`·FindFood→植被查询·FindWater→水点搜索）** + **★Sprint-071 V3a 代谢闭环（采集入库→进食消费→hunger/hp 真实变化·`harvest_on_arrival_system` + `consume_system`）** 就位。 |
| Godot 项目 | Godot 4.7 + GDExtension — Transvoxel 完整 + Clipmap LOD 8 层 + Signed Heightfield + 海洋 + 大气 + 昼夜 + LODCoordinator Phase1 + 天气 Phase1 + 经济循环 + 库存系统 + Tab夺舍NPC + NPC对话气泡 + 独立 CameraRig 第三人称相机 + **★BiomeVegetation 初始化（set_vegetation_provider·可采集植被查询在线）** + **★代谢闭环（harvest_on_arrival→库存→consume→真实饥饿循环）** |
| 当前冲刺 | **Sprint-071 V3a 代谢闭环完成**（1111 tests·clippy/fmt 零警告·**待提交**）— `ConsumableEffect` 展开（hunger_restore+hp_restore）+ `resolve_plant_yield()` yield resolver + `ArrivedAtTarget` 组件 + `movement_system` FindFood 到达改造 + `harvest_on_arrival_system`(新) + `consume_system`(新) + WorldDriver Block A2.5/A3.5 接线 + 4 TOML 食物条目·+11 tests。→ 下一步: V3b 市场接真（第 7/10 步·order_creation 读真实 Needs/盈余·双账统一） |
| 最新 CHG | **CHG-069**（2026-07-11·第三人称相机与视角系统·玩家系统007 v1.2·实现已落地）— 前: CHG-067 物理运动学地基（仅设计） |
| 最新交接 | [[woworld-dev-plan/01-核心基础/handoff/handoff-20260712-sprint071]]（2026-07-12·Sprint-071 V3a·1111 tests） |

---

## Phase 映射总览

| Phase | 覆盖层 | 涉及模块数 | 代码状态 | 设计状态 |
|-------|--------|----------|---------|---------|
| Phase 1: 核心基础 | Layer 0-1 | 10 模块 | 5/10 🟡 | 10/10 ✅ |
| Phase 2: 垂直切片（探针·活着的村庄） | Layer 1-2 子集 | 10 里程碑（V0/V1/V4a/Vf/V2/V3a/V3b/V4b/V5/V6） | 6/10 🟡 | ✅ 定稿 2026-07-12·A1纯涌现·食物源方案A（见 [[02-垂直切片/README]] §3-§5） |
| Phase 3: 系统完形 | Layer 2-4 | ~17 模块 | 0 🔴 | 17/17 🟡 |
| Phase 4: 世界填充 | — | 0 新模块 | 0 🔴 | — |
| Phase 5: 打磨发布 | — | 0 新模块 | 0 🔴 | — |
| Phase 6: 持续运营 | — | 0 新模块 | 0 🔴 | — |

---

## ECS 迁移状态

> ECS 架构设计文档：`[[../开发文档/]]`（42 篇，~80 Component，~120 System，~30 Resource）
> ECS 实现路线图：`[[../开发文档/06-迁移映射/003-实现路线图]]`

| ECS Phase | 内容 | 状态 | 对应 Dev Phase | 关键交付 |
|-----------|------|------|---------------|---------|
| Phase 0 | hecs 基础设施 + 核心 Component + LodCoordinatorSystem | ✅ Sprint-035 完成 | Phase 1 (1J) | `woworld_ecs` crate, 5 Component, WorldDriver.ecs 字段 |
| Phase 1 | 生命系统（首个完整 ECS 模块） | 🟢 Sprint-036+037 完成 | Phase 1 (1H) | 完整生命周期：Vitals→死亡→掉落→腐败→消失+再生 |
| Phase 2 | NPC 核心（批量 System 迁移） | ✅ Sprint-043~057 完成 | Phase 3 (P3) | 35 Components + 20 Systems: BigFive/Emotion/Needs/Movement/Social/Goal/Cognitive/ActionWeight/Lifecycle/Gender/Aesthetic/Biases/Circadian + 207 tests |
| Phase 3 | 社会系统（懒加载·低频）+ 物品 + 经济 | ✅ Phase 1/2 完成 | Phase 3 | 社会×4(Culture/Economy/Faith/Power) Phase 1 + 物品 Phase 1(ItemCategory/Registry/TOML) + 经济 Phase 2(Market/OrderBook/撮合/需求驱动订单/Pareto钱包/物品持有) |
| Phase 4 | 交互系统（战斗/魔法/物品/技能） | — 阻塞于 Phase 3 | Phase 3 | CombatState/SpellSlots/InventoryHandle |
| Phase 5 | 大规模并行 + 性能调优 | — 阻塞于 Phase 4 | Phase 5 | rayon par_iter(), 1000 Entity benchmark |

### ECS 不变部分

以下模块全程不进入 ECS（保留当前架构）：

| 模块 | 原因 |
|------|------|
| `woworld_worldgen` | 世界生成——纯计算管线，不进 Archetype |
| `woworld_atmosphere` | 大气合成——纯计算 |
| Godot UI | GDScript 侧渲染 |
| Godot 音频渲染 | Godot AudioServer |
| Godot 动画渲染 | Godot AnimationTree |

### ECS 当前进度

- **hecs 依赖**: ✅ hecs 0.10.5
- **Component 定义**: 35 / ~80（18 模块文件——BigFive/Emotion/Needs/Movement/Lifecycle/Cognitive/Aesthetic/Social/Gender/Biases/Goals/ActionWeight/Vitals 等）
- **System 实现**: 20 / ~120（life: 6·npc: 8·lod_coordinator: 1·另有 5 个 NPC 辅助 system）
- **Resource 定义**: 2 / ~30 (LootTableRegistry + SpatialGrid)
- **ECS 测试**: 207
- **341 现有测试**: ✅ 全绿（迁移过程中每步必须保持）

---

## 一、代码模块（Rust Workspace）

### woworld_core — 🟢 稳定

核心类型 + trait 定义。仅 glam 依赖。引擎无关。空间查询 trait、植被 trait、LOD 类型、天气类型均在此定义——不拆分独立 crate。**ECS Component 不在此定义**——由 `woworld_ecs` crate 承载（见 ECS Phase 0 / 1J）。

| 文件 | 内容 | 状态 |
|------|------|------|
| `types.rs` | WorldPos, EntityId, EntityKind(5), SpatialEntity, SpatialEvent, ScentSource, AcousticTag, TerrainHit, Aabb | ✅ 完整 |
| `id.rs` | ItemDefId, ItemEntId, SkillId, ProfessionTagId, ChunkCoord | ✅ 完整 |
| `spatial.rs` | TerrainQuery(9方法), EntityIndex(6方法), SpatialEventBus(3方法), VisibilityQuery(2方法) | ✅ 完整 |
| `material.rs` | SurfaceMaterial(21变体), Medium(4变体) | ✅ 完整 |
| `time.rs` | WorldTime, WorldClock, TimeOfDay — 昼夜循环权威定义 | ✅ 完整 |
| `density.rs` | DensityStack — 分层密度场 + LayerPriority 排序 + ★CHG-065 material_at/find_surface_y | ✅ 完整 |
| `edit_terrain.rs` | ★CHG-065 地形修改编排层 — EditDensity/EditHeightfield CoW, ModificationBatch, DirtyChunkQueue, EditDensityLayer | ✅ 完整 |
| `ocean.rs` | OceanProvider trait (6方法) — 海平面/水深/水下检测 | ✅ 完整 |
| `vegetation.rs` | VegetationProvider trait + PlantCommunitySnapshot + 植被类型枚举 | ✅ 完整 |
| `lod.rs` | LodPrescription(7×u8) + distance_to_scene_lod/char_lod + LodCoordinator trait | ✅ Sprint-033 新增 |
| `weather_types.rs` | WeatherState(6), Season(4) — 天气/季节枚举定义 | ✅ Sprint-033 新增 |
| `camera.rs` | ★ 相机工具 — smooth_damp 家族 + resolve_camera_arm（第三人称相机 MVP·CHG-069） | ✅ 完整 |
| 测试 | 393 tests (time + density + lod + weather_types + camera + action(ActionId::from_key) + inventory/equipment/economy/culture/faith/power/bootstrap 等) | ✅ 全绿 |

### woworld_worldgen — 🟡 部分实现（零架构偏离）

程序化世界生成。依赖 woworld_core + noise crate。

| 文件 | 内容 | 状态 |
|------|------|------|
| `noise_gen.rs` | 双层 Perlin 噪声 (continent+detail+mountain) + 气候场 + Worley 3D | ✅ 完整 |
| `biome.rs` | 温度×降水 2D 噪声 → 5 群系硬盒分类 (TOML 数据驱动) | ⚠️ 硬盒分类 vs 设计规定的连续参数场 |
| `cave.rs` | CaveDensity — 3D Worley 洞穴密度层 | ✅ 完整 |
| `terrain.rs` | HeightfieldTerrain — 完整 TerrainQuery trait 实现 | ✅ 完整 |
| `transvoxel.rs` | ★ Transvoxel 完整实现（常规+过渡单元，顶点共享）+ winding 运行时检测 | ✅ Sprint-033 MC绕序修复 |
| `transition_tables.rs` | ★ 过渡单元查找表（auto-generated） | ✅ 完整 |
| `tri_table_data.rs` | Marching Cubes 三角剖分查找表 (256 条目) | ✅ 完整 |
| `terrain_mesh.rs` | 纯 Rust 网格生成 — SH + 高度场 + 共享索引 | ✅ 完整 |
| `clipmap.rs` | ClipmapManager — 8 层 Clipmap LOD CHG-049 对齐 | ✅ Transvoxel (LOD 0-4) + SH (LOD 5-7) |
| `ocean.rs` | HeightfieldOcean — OceanProvider trait 实现 (Gerstner 波) | ✅ 完整 |
| 测试 | 49 tests | ✅ 全绿 |

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
| ✅3 | 远距离全 Transvoxel (应为 SH) | **Sprint-015: LOD 5-7 SH ✓** |
| ✅4 | 缺 scene_lod 6-7 (4km+)  | **Sprint-033: LOD 6-7 距离修正 (6:4-10km, 7:10-15km) ✓** |
| 🟡5 | LODCoordinator Phase 2+  | **Phase 1 完成 (Sprint-033)** — Steps 2-8 (约束/级联/VRAM/帧预算/滞后) 待后续 |
| 🟡6 | 远距离人造结构不可见              | 需灯光烘焙 + 建筑群轮廓系统，等待建筑模块                              |

### woworld_atmosphere — 🟡 部分实现

大气与氛围系统。依赖 woworld_core + serde + toml。

| 文件 | 内容 | 状态 |
|------|------|------|
| `time_curve.rs` | AtmosCurve — TOML 驱动的颜色/亮度时间曲线 | ✅ 完整 |
| `synthesizer.rs` | AtmosphereSynthesizer → ResolvedAtmosphere (17 参数) + resolve_with_weather() | ✅ 完整 |
| `resolved_atmosphere.rs` | ResolvedAtmosphere — PackedFloat32Array 输出 | ✅ 完整 |
| `traits.rs` | BiomeAtmosQuery, WeatherAtmosQuery, SeasonAtmosQuery | ✅ Weather/Season 查询接口就位 (Sprint-033) |
| `weather.rs` | SimpleWeatherDriver (简化 Markov 6-state) + SimpleSeasonProvider | ✅ Sprint-033 新增 — ⚠️ 架构债：待升级为连续物理参数驱动 |
| 测试 | 17 tests | ✅ 全绿 |

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

| 模块 | Phase | 等级 | 代码 | 备注 |
|------|-------|------|------|------|
| 技术栈方案 | P1 | 🟡 就绪 | — | v4.0 权威方案 |
| 模块接头总览 | P1 | 🟡 就绪 | — | 102 文件/~6,100 行。12/27 模块时间戳待更新 |
| 存档系统 | P1 | 🟡 就绪 | — | v2.0 (CHG-055/056)。LMDB 方案完整 |

### 世界框架（4 个）

| 模块 | Phase | 等级 | 代码 | 备注 |
|------|-------|------|------|------|
| 世界生成 | P1 | 🟡 部分实现 | ✅ 4 crates（woworld_ecs 规划中） | 15 阶段管线仅完成 P0+P2。5 个红色偏离已修复（Sprint-006/011/012/014） |
| 生命 | P1 | 🟡 就绪 | — | Vitals/Mana/DeathCause(30种6类) 契约完整 |
| 历史 | P3 | 🟡 就绪 | — | AetherImprint/KnowledgeSeed 契约完整 |
| 天气与季节系统 | P1 | 🟡 部分实现 | ✅ woworld_core + woworld_atmosphere | WeatherState/Season + SimpleWeatherDriver (Sprint-033) |

### NPC 核心（2 个 + 7 子模块）

| 模块 | Phase | 等级 | 代码 | 备注 |
|------|-------|------|------|------|
| NPC活人感模块 | P3 | 🟡 就绪 | — | v2.0 总规格。~8,000 行设计规格 |
| ↳ 03-基本需求系统 | P3 | 🟡 就绪 | — | 已审核 vs CHG-027 |
| ↳ 04-进阶需求系统 | P3 | 🟡 就绪 | — | ERG 挫折回归模型已定义 |
| ↳ 05-审美与艺术 | P3 | 🟡 就绪 | — | AestheticSignal(6 dims)+AestheticTaste SoA |
| ↳ 06-认知与智慧系统 | P3 | 🟡 就绪 | — | v1.1 (CHG-057/058/059)。PatternExpression 数学地基 |
| ↳ 07-生命周期系统 | P3 | 🟡 就绪 | — | v1.0 (CHG-041)。AgeClock/Gompertz/InfantDependency |
| ↳ 08-NPC行动涌现 | P3 | 🟡 就绪 | — | v1.0 (CHG-042)。3 层原子架构(35+~40+~25) |
| 概念与语言地基 | P3 | 🟡 就绪 | — | v1.0 (CHG-044)。3 层模型 |

### 社会系统（4 个）

| 模块 | Phase | 等级 | 代码 | 备注 |
|------|-------|------|------|------|
| 经济系统 | P3 | 🟢 Phase 2 | Market/OrderBook撮合+Pareto钱包+需求驱动订单+物品持有+交易执行+EMA价格 | v1.0。Phase 2 完成（624 tests）。延后: 市场监管/货币管道/借贷 |
| 权力系统 | P3 | 🟢 Phase 1 | PowerAtom(17)+PowerSource(8)+Legitimacy(5因子)+PowerQuery | v1.0。Phase 1 完成 (18 tests)。延后: PowerTopology |
| 文化系统 | P3 | 🟢 Phase 1 | CultureCoreParams(10)+6推导类型+CultureQuery | v1.0。Phase 1 完成 (99 tests)。延后: 空间模型/演化 |
| 信仰系统 | P3 | 🟢 Phase 1 | FaithTheology(10)+ReligiousMotivation(9)+FaithQuery | v1.0。Phase 1 完成 (36 tests)。延后: 传播/节日 |

### 交互/表现/建造/辅助（13 个）

| 模块 | Phase | 等级 | 代码 | 备注 |
|------|-------|------|------|------|
| 战斗 | P3 | 🟡 就绪 | — | 三层模型(本能→节奏→战略)/半自动 |
| **魔法** | P3(冻结) | **🔴 冻结** | — | **零性能预算 — 预算建立前不可编码** |
| 物品系统 | P1 | 🟢 Phase 1 | ItemCategory(44)+ItemProperties(28)+ItemRegistry+TOML+ItemQuery trait | v1.0。Phase 1 完成（+33 tests）。延后: Assembly/装备/背包/附魔 |
| 技能系统 | P1 | 🟡 就绪 | — | SkillId(5分类)/XP公式/天赋三层/教学四路径 |
| 语言表达 | P4 | 🟡 就绪 | 气泡数据驱动 (BubbleType+SpeechAct + speech_bubble_system + TOML 片段库 FragmentCondition + 遭遇驱动问候/告别) | ExpressionRef/Conversation/信息传播 5 通道。★ Sprint-061 渲染(CHG-066) → ★ Sprint-068 桩串外移 TOML(003 片段组合子集)+接遭遇/ActionIntent 涌现；完整 TextGenerator/CompositeTemplate 待后续 |
| 模型动作与物理 | P4 | 🟡 部分实现 | ✅ 角色控制器核心三层 + Step 5e 集成 (927 tests) | 9 层动画栈/四 trait/6 子模块。★ **角色控制器** MovementSystem+ActionController+手感系统+管线集成(Block A0)完成。**CHG-067 运动学地基**暂缓 per Q-A2 |
| ├ 角色控制器 | P2 | 🟢 核心+管线+离散/持续/充能运行时 | ✅ 核心三层 + Step 5e + 006 运行时 | 13 篇开发规格（2,731 行）。核心三层 + Block A0 管线 + ActionResolver(004) + **★006 持续/充能运行时(dispatch_release/SustainDrain/充能阶梯/CPendingFollowUp·Sprint-065)**·1026 tests。延后: A2 中断语义/M3 滑翔/I1-5 手感/玩家实体接 Vitals+键位。★ CHG-067 消费者 |
| 音频系统 | P4 | 🟡 就绪 | — | SoundFootprint/AudioQuery(30 methods) |
| 感官与知觉系统 | P3 | 🟡 就绪 | — | PerceptBatch/4 查询 trait/PerceptualCache |
| 建筑模块 | P4 | 🟡 就绪 | — | ComponentFamily/WFC 2.5D/BuildingGenerator |
| 载具系统 | P4 | 🟡 就绪 | — | 5 动力类型/L1-L3 半自动控制 |
| 大气与氛围系统 | P1 | 🟡 部分实现 | ✅ woworld_atmosphere | 3/4 调制层为身份存根 |
| 小精灵系统 | P3 | 🟡 就绪 | — | v1.0 (CHG-052) |
| 玩家系统 | P2 | 🟢 Phase 1 | ControlMode + ActionDomain + 夺舍NPC + Tab/F切换 + possess命令 | ★ Sprint-060。CHG-063 设计就位。Phase 1 完成 (783 tests)。延后: DomainDelegated/双角色/角色创建 |

### 设计补全待办（Track B/C 遗留）

| 模块 | 优先级 | 说明 |
|------|--------|------|
| 名声系统 | 高 | 涌现式名声（NPC记忆×信息传播×共识）。6 文件引用，零设计文档 |
| 法律与秩序 | 高 | 分析结论：不需要独立模块。法律从 PowerAtoms+NPC 决策涌现。需接口修补文档 |
| 魔法性能预算 | 高 | 2,328 行设计规格，零性能预算 — 轨 C 最高风险项 |
| 采矿/农业/地下城/死亡传承/教程 | 中 | 按需排期 |
| 饮食/服饰/娱乐 | 低 | 按兴趣推进 |

---

---

## 三、近期冲刺

**下一个冲刺**: **V3b 市场接真**（垂直切片「活着的村庄」第 7/10 步——order_creation 读真实 Needs/盈余·双账统一）。上一冲刺 **Sprint-071（V3a 代谢闭环）✅ 完成**（1111 tests），交接见 [[01-核心基础/handoff/handoff-20260712-sprint071]]。

**待触发冲刺队列（防遗漏 backlog）**：

| 冲刺 | 状态 | 暂缓依据 | 提案 |
|------|------|---------|------|
| ★ 物理运动学地基·实现 | 🟡 待触发 | CHG-067 Q-A2（设计已定，只留文档） | `sprint-proposals/BACKLOG-物理运动学地基-实现-20260709.md` |
| ★ Vf 食物源落地（P2.25 采集植被） | 🟡 待触发（V2/V3a 前置） | Sprint-067 spike 已探底·建议路线B | `sprint-proposals/BACKLOG-Vf-食物源落地-20260712.md` |

**冲刺历史**：

| Sprint | 日期 | 目标 | 状态 |
|--------|------|------|------|
| Sprint-068 | 2026-07-12 | ★ V4a 问候/情绪气泡 — SpeechAct(core) + TOML 片段库(FragmentCondition 富条件+概率加权) + neighbors_within 原语(social 重构) + 遭遇感知层(迟滞/播种/despawn/朝向门) + 问候/告别接既有 ActionIntent 涌现(Fight/Flee 否决·barrier-free) + 实机验证 + 修 ECS-001(SeekSafety 否决陷阱)·~1075 tests | ✅ 完成 |
| Sprint-067 | 2026-07-12 | ★ V0+V1 垂直切片地基 — V0 库存验证测试(审计:已由 inventory_init_system 幂等补挂) + V1 time_modifier 昼夜第6因子(=设计 ver2.0 v3·纯世界时·白昼度曲线) + 漫游回落(读 Needs 紧迫度防振荡) + Vf 食物源 BACKLOG spike·1055 tests | ✅ 完成 |
| Sprint-066 | 2026-07-11 | ★ 手感系统运行时 I1-4 — 缓冲淘汰/物理重检/落地预输入(实机激活)/边缘吸附 + coyote-jump 玩家组件接线(休眠)·1047 tests | ✅ 完成 |
| Sprint-065 | 2026-07-11 | ★ 持续/充能动作运行时（006）— 解除 Discrete 硬门 + SustainDrain 消耗 + SustainPhase 迁移 + ReleaseBehavior 分发(dispatch_release) + 充能阶梯 follow-up + block/aim_bow TOML + A3(interrupt_on_move) + M4(coyote 字段)·1026 tests | ✅ 完成 |
| 相机 MVP | 2026-07-11 | ★ 第三人称相机 MVP — 独立 CameraRig + SmoothDamp 跟随 + terrain_raycast 碰撞 + character_facing_system + CJustLanded + 夺舍 CC 管线统一（CHG-069·1001 tests·实机验证） | ✅ 完成 |
| Sprint-062~064 | 2026-07-10~11 | ★ 角色控制器垂直切片 — ActionResolver + input_bridge + 玩家 Block A0 + 跳跃（977 tests） | ✅ 完成 |
| Sprint-061 | 2026-07-08 | ★ 对话雏形 MVP — BubbleType + speech_bubble_system + NPC自言自语气泡 + 术语消歧(CHG-066) | ✅ 完成 |
| Sprint-060 | 2026-07-08 | ★ 玩家系统 Phase 1 — ControlMode + 夺舍NPC + Tab/F切换 + possess命令 | ✅ 完成 |
| Sprint-033 | 2026-07-05 | ★ MC绕序修复 + LODCoordinator Phase1 + 天气Phase1 + PBR法线修复 | ✅ 完成 |
| Sprint-032 | 2026-07-04~06 | VoxelChunk LOD 0 7轮修复（MC绕序+统一wy+biome材质） | ✅ 完成 |
| Sprint-031 | 2026-07-04~05 | 性能优化3轮 + 海洋视觉 + VoxelChunk LOD 0 替换 | ✅ 完成 |
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

> 🐛 **运行时 bug → [`bugs/INDEX.md`](bugs/INDEX.md)** 为权威源。调试前必须先查 bug 索引。本节仅保留架构偏离（已修复·历史记录）和设计债务。

### 红色架构偏离（阻塞后续）— 详见 §一 woworld_worldgen

🔴1 DensityField trait · 🔴2 Seed u32 · 🔴3 Chunk 128m · 🔴4 MC vs Transvoxel · 🔴5 单层密度

> ✅ **全部 5 个已修复**（Sprint-006, 011, 012, 014）。零架构偏离。

### 轨 C 遗留（设计债务）

| # | 项 | 状态 |
|---|----|------|
| 1-5 | 5 个孤儿接口所有权冲突 | 未修复（CHG-047 Phase 2） |
| 6 | 魔法性能预算缺失 | 未修复 — 最高风险 |
| 7 | 世界生成 5 篇文档 v1.0→v2.1 | 未修复 |
| 8 | 模块接头总览 README 46→60 行 + 12 模块时间戳更新 | 未修复 |

### 治理待办

- [x] `session-handoff.md` 根目录旧格式清理 + 交接文档集中化 → ✅ 2026-07-01 完成
- [x] `chunk_manager.rs` 删除 → ✅ Sprint-016 退役
- [x] 宪法 v2.0（精简版）→ ✅ 2026-07-04 生效
- [x] Bug 追踪知识库 → ✅ 2026-07-07 — 见 [`bugs/`](bugs/)

---

## 五、最近交接摘要

| 文件 | 内容 |
|------|------|
| [handoff-20260708-sprint061.md](01-核心基础/handoff/handoff-20260708-sprint061.md) | ★ 最新 — Sprint-061（对话气泡MVP·BubbleType+speech_bubble_system·术语消歧CHG-066·807 tests）|
| [handoff-20260708-late.md](01-核心基础/handoff/handoff-20260708-late.md) | Sprint-060（玩家Phase1·ControlMode+夺舍NPC·783 tests）|
| [handoff-20260706-031.md](01-核心基础/handoff/handoff-20260706-031.md) | Sprint-056/057（PRNG清理+ECS-Godot可视化·21 NPC·341 tests）|
| [handoff-20260706-030.md](01-核心基础/handoff/handoff-20260706-030.md) | Sprint-043~055（NPC人格·BigFive·行为链·15 Sprints）|
| [handoff-20260705-029.md](01-核心基础/handoff/handoff-20260705-029.md) | Sprint-034~042（LODCoordinator P2+ECS P0+生命+天气+NPC需求链）|
| [handoff-20260705-028.md](01-核心基础/handoff/handoff-20260705-028.md) | Sprint-036/037（ECS Phase 1 生命系统）|
| [handoff-20260705-027.md](01-核心基础/handoff/handoff-20260705-027.md) | Sprint-035（ECS Phase 0 基础设施）|
| [handoff-20260705-026.md](01-核心基础/handoff/handoff-20260705-026.md) | Sprint-034（LODCoordinator Phase 2 完整8步算法）|

---

> **关联**: [CONSTITUTION.md](CONSTITUTION.md) · [DEPENDENCY_GRAPH.md](DEPENDENCY_GRAPH.md) · [audit-reports/20250625-code-vs-design/](audit-reports/20250625-code-vs-design/README.md)
