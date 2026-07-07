# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.
用中文和用户交流。
## 项目概述

**WoWorld (Wonder World)** — 与其说是一个游戏，不如说是在以游戏的方式创建另一个"人世"。程序化生成的体素开放世界RPG，独立开发中。

**核心设计哲学**：WoWorld 是一个**故事生成器**，不是一个故事容器。开发者只定义世界的底层规则（人格参数、行动规则、情绪规律、文化传播率），具体的故事由NPC之间的互动以概率和统计的方式自然涌现。每个存档都是一个完全不同的世界。游戏没有"通关"的概念。

**涌现优先原则**：玩家与 NPC 无本质区别——NPC 能正常运作，玩家就只需"夺舍"NPC再优化细节。许多内容从现有原子系统涌现，不轻易建立独立模块：法律从 PrescribeRule+Sanction+Adjudicate+NPC决策+信息传播涌现；名声从分散式 NPC记忆×信息传播×共识涌现。战斗/魔法等细节在开发中边做边迭代，不在开发前过度设计。

**"人世"的关键维度**：
- **完整的人**：NPC 有记忆（上限2000条）、预测、情感、思想、人际关系、欲望、习惯——与"人"打交道是核心
- **历史深度**：玩家进入时世界已运转千百万年，书本和遗迹中保留着先民活过的证据
- **冒险与生活平等**：冒险者、铁匠、商人、政客——每条路都是"主线"，同一套底层系统支撑
- **全球多元文化**：西方剑与魔法为基底，全球文化元素自由融入，文明自行演化
- **无限探索**：25万km²初步世界（≈英国面积），海:陆≈7:3，700m+山峦。Minecraft级无限世界为最终目标

**设计→编码过渡中**：技术方案历经 001→015（v1.0）→016（v2.0）→018（v3.0）→**v4.0（当前权威，经技术栈全量审计升级）** 演进。开发路线图已启动——设计文档与代码脚手架并存。路线图功能由 `附录D-模块依赖图.md` + `附录E-开发状态.md` + 冲刺提案共同承担（旧四轨计划已于 2026-06-25 归档）。

> ⚠️ **重要说明**：WoWorld 处于**设计+编码并行阶段**。设计文档中的伪代码和数据结构示例用于阐明设计理念，实际 Rust 实现可能根据工程发现重构。`woworld/` 已进入**轨A·MC体素+Clipmap LOD+海洋**——5 crate workspace（含 `woworld_ecs`·37 Components+21 Systems+247 tests），核心类型/trait/世界生成(MC+Clipmap+LOD)/大气氛围/海洋渲染/Godot 桥接全部就位。
>
> **仓库构成**：设计文档（`WoWorld-Design/`）+ Rust workspace（`woworld/`）+ 开发治理（`woworld-dev-plan/`）。设计文档用 **Obsidian** 编辑（`[[wikilink]]` 导航），Rust 代码用标准 Cargo 工具链。

**当前规格版本**: v4.0。模块累计 **~26 个独立系统** + 1 个子模块（家具与放置物品）+ 交互配方表系统 + 存档系统 v2.0（CHG-055/056）。★ 2026-06-22 Loop Audit 全量审计完成。★ NPC 认知系统 v1.1（CHG-057/058/059）。★ 开发路线图优化（CHG-060）。★ 轨C 孤儿接口修复（CHG-061）。★ UI/UX 系统创建（CHG-062）。★ 玩家系统新建（CHG-063·6篇~1,448行）。★ 轨A 昼夜循环 + 5群系系统（CHG-064）。★ 宪法 v1.5（已审批生效）。最新 CHG 序列见 `WoWorld-Design/Change/`。

## 快速导航

> 📘 **核心入口**: **CLAUDE.md** = 项目大局观 + 工作约定。**[[CLAUDE-INTERFACES.md]]** = 跨模块契约完整参考。**[[woworld-dev-plan/00-流程总览.md]]** = 开发流程全景图（六阶段 + 9场景SOP + 阶段切换决策树）。开发治理体系详见 `woworld-dev-plan/README.md`。

| 我想… | 去… |
|------|------|
| 理解项目设计哲学 | `WoWorld-Design/Happy Game/欢迎.md` |
| 查找某模块的设计规格 | `开发阶段/<模块名>/README.md` |
| 查看跨模块契约（谁 own 什么概念、trait 签名） | [[CLAUDE-INTERFACES.md]] |
| 查我的模块被谁依赖 | `开发阶段/模块接头总览/<NN>-<模块>/003-变更影响链.md` |
| 看开发流程全景图 | [[woworld-dev-plan/00-流程总览.md]] |
| 查看开发宪法与冲刺工作流规则 | `woworld-dev-plan/CONSTITUTION.md` |
| 查看各模块开发准入状态（🔴🟡🟢） | [[woworld-dev-plan/附录E-开发状态.md]] |
| 查中英术语映射 | `woworld-dev-plan/附录A-术语表.md` |
| 查模块实现依赖图 | [[woworld-dev-plan/附录D-模块依赖图.md]] |
| 查 GPU/CPU/VRAM 性能预算 | [[woworld-dev-plan/附录B-性能预算.md]] |
| 查架构决策记录（为什么这样设计） | [[woworld-dev-plan/附录C-架构决策记录.md]] |
| 新开发者/新设备接入 | [[woworld-dev-plan/README.md]] |
| 看最近的设计变更 | `WoWorld-Design/Change/README.md` |
| 查用户的设计裁决意见 | `WoWorld-Design/Change/hand/` |
| 看历史设计讨论存档 | `参考文档/` |
| 查阅NPC认知v1.1传播审计 | `参考文档/039-NPC认知传播审计-20260622/README.md` |
| 验证技术栈决策 | `开发阶段/技术栈方案/001-WoWorld正式技术栈方案v3.md` |
| 查 ECS 架构设计（Component/System/Resource 定义） | `开发文档/README.md` |
| **写/读 Rust 代码** | `woworld/` — workspace 结构见下方「代码架构」 |
| **构建项目** | `cd woworld && cargo build --workspace` |
| **启动 Godot 编辑器** | `tools/godot/Godot_v4.7-stable_win64.exe woworld/godot/project.godot` |
| **看最新开发日志** | `woworld-dev-plan/01-核心基础/devlogs/DEVLOG-2026-07-08.md` (全天: 物品 Phase 2+经济 Phase 3·737 tests) |
| **看最新交接文档** | `woworld-dev-plan/01-核心基础/handoff/handoff-20260708.md` (物品 Phase 2+经济 Phase 3·737 tests·下一步建议) |
| **🐛 查已知 bug/陷阱** | `woworld-dev-plan/bugs/INDEX.md` — 调试前必须先查 |

## 文档结构

所有设计文档在 `WoWorld-Design/` 目录下，使用 **Obsidian** 编辑，大量使用 `[[wikilink]]` 交叉引用。文档语言为中文。

### `Happy Game/` — 核心设计文档
- `欢迎.md` — 项目总览 | `想法/WoW World/总设计草稿.md` — 总设计文档
- `想法/` — 概念设计/脑暴（`#草稿` = 早期文档，允许矛盾模糊未决）
- `开发阶段/` — 正式开发规格（权威规格）。~25 个独立系统 + 1 个深化子模块（[[WoWorld-Design/Happy Game/开发阶段/物品系统/家具与放置物品/README|家具与放置物品]]），按层级组织：

| 层级 | 包含模块 | 角色 |
|------|---------|------|
| **全局基础** | 技术栈方案、模块接头总览、存档系统 | 决策依据、接口枢纽、持久化契约 |
| **世界框架** | 世界生成、生命、历史、天气与季节系统 | 物理和生物基底 |
| **NPC 核心** | NPC活人感模块（含需求/审美/认知/生命周期/行动涌现）、概念与语言地基 | NPC 完整心智 |
| **社会系统** | 经济系统、权力系统、文化系统、信仰系统 | 涌现式社会结构 |
| **交互/表现/建造/辅助** | 战斗、魔法、物品系统（含家具与放置物品）、技能系统、语言表达、模型动作与物理系统、音频系统、感官与知觉系统、大气与氛围系统、建筑模块、载具系统、小精灵系统、UI/UX系统、玩家系统 | 交互方式、视听表现、物理改造、玩家辅助 |

### 关键架构关系

- **技能系统 ↔ 物理原子层**: 正交维度——AgentSnapshot 连续参数决定**身体能不能执行**（force_check），SkillEntry 追踪**脑子练过多少次**（用进废退）。汇合点：复合原子 execute() 同时消费两者，`execution_noise_std` 签名 f32——零耦合。技能是事后记录标签，不是事前门控规则——门槛从物理涌现（MaterialProperties × noise）。
- **ID 类型所有权**: 所有 ID 类型（`ItemDefId`·`SkillId`·`EntityId`·`ProfessionTagId`·`ChunkCoord`）统一定义在 `woworld_core`（仅 glam 依赖）。TOML 数据文件为中性数据——各消费模块通过 `include_str!()` 平等加载。
- **交互配方表**: 物品获取的物理路径（采集/拆解/屠宰）统一走 TOML 配方表——`(EntityKind, tool_tags) → (composite_atom, yield_resolver, xp)`。配方表和 `CraftingRecipe` 是两张独立表——采集不需要 SkillRequirement。
- **世界生成 ←→ 存档系统**: ★ P13 不再写 LMDB——输出纯内存 `ValidatedWorldState`，持久化由存档系统接管。`SaveableModule::snapshot_dirty() → SaveSystem → LMDB` 是唯一持久化路径。详见 `开发阶段/存档系统/README` 或 [[CLAUDE-INTERFACES.md#CHG-055]]。

### `WoWorld-Design/Change/` — 设计变更追踪

> ⚠️ **Change 文件夹约定**：编号大的覆盖编号小的。以 `开发阶段/` 实际内容为权威。最新 CHG 序列（053-063）详见 `WoWorld-Design/Change/README.md`。

近期关键变更：**CHG-053**（Godot 4.7·12子系统）→ **CHG-054**（世界生成 v2.1）→ **CHG-055/056**（存档系统 v1.0→v2.0）→ **CHG-057**（NPC认知 v1.1·PatternExpression数学地基）→ **CHG-058**（NPC认知系统自审修正）→ **CHG-059**（NPC认知v1.1全模块传播审计）→ **CHG-060**（开发路线图优化·四轨重定义·孤儿接口修复）→ **CHG-061**（轨C孤儿接口修复·CHG-063前置）→ **CHG-062**（UI与UX系统创建）→ **CHG-063**（玩家系统新建·6篇~1,448行·28-玩家系统登记）→ **CHG-065**（地形修改编排层·~800行代码+50测试·内核不转ECS编排层入ECS）。

近期冲刺：**Sprint 031-033**（性能优化+LOD+天气+PBR法线）→ **Sprint 035-057**（ECS Phase 0-2·生命·NPC人格·BigFive·行为链·Godot可视化·21 NPC）→ **Sprint 058**（Gompertz死亡·社交深度·地形移动·审计修复·381 tests）→ **物品 Phase 1 + 经济 Phase 2**（ItemCategory/Registry/TOML + Market/OrderBook撮合/Pareto钱包/需求驱动订单·624 tests）→ **物品 Phase 2**（PersonalInventory/装备/Assembly stub·705 tests）→ **经济 Phase 3**（NeedCategory/Urgency/ListingType/Partial fill/Scarcity bonus/Needs连接·737 tests）。

**`WoWorld-Design/Change/hand/`** — 用户直接设计反馈。修改涉及的设计决策时，需检查此目录是否有相关意见。

### `WoWorld-Design/开发路线图/` — 路线图指针

旧版"四轨并行 Week 1-9+"路线图（2026-06-22）已于 2026-06-25 归档至 `参考文档/044-开发路线图归档-v1-20260625/`。当前 `开发路线图/README.md` 为简短指针，路线图功能由以下三文件共同承担：

| 文件 | 角色 |
|------|------|
| `woworld-dev-plan/附录D-模块依赖图.md` | 地图——层 0→1→2→3→4 实现顺序 |
| `woworld-dev-plan/附录E-开发状态.md` | 当前位置——每个模块 🔴🟡🟢 + WIP + Phase 映射 |
| `woworld-dev-plan/sprint-proposals/` | 下一步——冲刺提案 |

流程全景见 [[woworld-dev-plan/00-流程总览.md]]。

### `参考文档/` — 参考性设计文档
按 `NNN-简短描述-YYYYMMDD` 格式组织。001-015 已归档至 `第一部分-设计演进存档-20260610/`。活跃文档 031-041 覆盖技术栈审计、各系统设计探讨、引擎分析、**世界生成重构跨模块补充需求(037)**、**法律涌现与执法机制分析(035)**、**玩家游玩内容全貌设计(036)**。详见 `参考文档/README.md`。

## 技术栈（v4.0 权威方案）

> 📘 **权威技术方案**: `开发阶段/技术栈方案/001-WoWorld正式技术栈方案v3.md`（文件名为v3，内容已升级为v4.0）。仅关键决策列于下方，完整细节见该文档。

- **引擎**: Godot 4.7 | **模拟语言**: Rust (stable 1.80+) — GDExtension 集成。引擎评估结论：保持 Godot+Rust，Rust 模拟核心保持引擎无关以降低迁移成本（详见 [[参考文档/032-Bevy引擎切换可行性分析-20260618/README|Bevy分析]]）
- **体素**: 自建 Transvoxel（Rust 侧）→ Godot ArrayMesh。垂直稀疏Chunk + Clipmap LOD。分层密度场(L0-L10 + L6.5植被)。3km切换Signed Heightfield
- **海洋**: Gerstner程序化波，`OceanProvider` trait (woworld_core · 6方法) 预留FFT升级。海:陆≈7:3
- **画面**: 3D低多边形 + Toon 2-tone cel渲染。512²面部图集+shader合成驱动表情。★ CHG-053
- **NPC AI**: GOAP（9%）+ 概率行为树（90%）+ 可选LLM增强（1%）。SoA + rayon并行。分层模拟 L1-L4
- **战斗**: 半自动——玩家AI = NPC AI = 同一套Rust代码。三层模型（本能→节奏→战略）。详见 `开发阶段/战斗/`
- **动画**: 9层可叠加动画栈——Rust CPU批量骨骼矩阵(≤0.5ms) → Godot GPU skinning。涌现式步态(9参数从BigFive派生)
- **世界生成**: 编排器模式——15阶段管线(P0-P13 + P2.25)。Bootstrap哲学+五Pass混合构造(含同性/多配偶/非婚生社会关系初始化)+KnowledgeSeed事件驱动传播。★ P13 输出纯内存 ValidatedWorldState，持久化由存档系统接管。详见 `开发阶段/世界生成/`（15篇·v2.2）。★ CHG-065 新增运行时地形修改编排层（`woworld_core::edit_terrain`）
- **数据库**: LMDB — 单文件 + 多 named_db。全量快照 + 脏数据增量。详见 `开发阶段/存档系统/`
- **物理架构**: ⚠️ CHG-033 — 仅玩家保留PhysicsServer3D，其余全部Rust侧空间查询(TerrainQuery/EntityIndex/SpatialEventBus/VisibilityQuery四trait)
- **LOD架构**: 场景8层(0-7)×角色5层(0-4)双层体系 + LODCoordinator 8步冲突解决 + 7维LodPrescription。统一距离带覆盖12km。
- **ECS**: hecs 0.10 (Archetype SoA 存储)——Component 拆装通信，无 before/after 调度。三阶段调度（Phase 0: 输入/LOD → Phase 1: 游戏逻辑·并行 → Phase 2: Godot 同步）。详见 [[开发文档/]]
- **架构**: Rust模拟核心 → GDExtension → Godot客户端（渲染/UI/音频/输入/玩家物理）
- **硬件目标**: GTX 1660 SUPER 6GB VRAM（理论估算，待原型RenderDoc验证）
- **Mod**: TOML数据驱动——调节涌现乘数，无脚本引擎
- **平台**: Windows / Linux / macOS

## 开发命令（woworld/ Rust workspace）

项目已进入轨A·GPU-Driven Clipmap+海洋+LODCoordinator+天气+**ECS Phase 0 已启动**。Rust workspace 含 5 crate，**737 个测试全部通过**。

```bash
cd woworld

# 编译整个 workspace（含 GDExtension 动态库）
cargo build --workspace

# 编译 release（启用 LTO + 单 CGU + panic=abort，用于性能测试）
cargo build --release --workspace

# 快速检查（不生成二进制，比 build 快）
cargo check --workspace

# 运行所有测试
cargo test --workspace           # 5 crates，全部 737 个测试

# 运行单个 crate 的测试
cargo test -p woworld_worldgen

# 运行单个测试（按名称过滤）
cargo test -p woworld_worldgen test_density_below_terrain

# Clippy lint（Rust 最佳实践检查）
cargo clippy --workspace -- -D warnings

# 格式化
cargo fmt --all

# 启动 Godot 编辑器（Windows — 必须在 woworld/ 目录下执行）
../tools/godot/Godot_v4.7-stable_win64.exe godot/project.godot

# 调试 GDExtension 加载问题 — 使用 _console.exe 变体（显示 stdout/stderr）
../tools/godot/Godot_v4.7-stable_win64_console.exe godot/project.godot

# 完整验证序列（修改 Rust 代码后跑一遍）
cargo check --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo build --workspace
```
> ⚠️ `cargo build --workspace` 不可省略——Godot 加载的 `.dll` 只在 build 时更新。

### GDExtension 调试备忘

从 Sprint 031-033 实战中提炼的常见陷阱与诊断流程：

- **Rust 改完 Godot 没变化**：`cargo build --workspace` 才会更新 `.dll`，`cargo check` 不够。用 `_console.exe` 启动 Godot 确认 GDExtension 加载成功。
- **网格渲染碎片化诊断顺序**（Sprint 032 教训——不要猜测，按顺序排查）：① NaN/Inf 检测 → ② 索引 OOB 检测 → ③ 法线采样验证 → ④ 面剔除（cull_back vs cull_disabled）→ ⑤ ArrayMesh 格式 → ⑥ 相邻 chunk 边界对齐。**先收集数据，再形成假说**。
- **Shader 调试技巧**：用 `unshaded→SPECULAR=0→shadow→ambient→normalize` 逐层剥离定位光照问题（Sprint 033 PBR 法线修复 10 轮诊断法）。
- **Release vs Debug**：`[profile.dev] opt-level=1`（非 0）用于日常开发——比 debug 快，比 release 编译快。Release mode 仅用于性能验证。
- **Godot 编辑器缓存**：修改 `.gdshader` 后有时需重启编辑器才能生效。
- **VoxelChunk vs Clipmap 材质一致性**：两者必须用相同的材质分类路径（biome 分类器），不能一个用纯高度分类一个用群系分类——否则 LOD 边界有色差（Sprint 032-G 根因 1）。

> **当前状态**（2026-07-08）：`cargo check --workspace` 通过。`cargo test` **737 个测试全部通过**, clippy 零警告。★ 四大社会系统 Phase 1 全部完成 (Culture/Economy/Faith/Power)。★ **物品系统 Phase 1+2** 完成 (ItemCategory/Registry/TOML + PersonalInventory/装备/Assembly stub)。★ **经济系统 Phase 2+3** 完成 (Market/OrderBook/Partial fill/Pareto钱包/需求驱动订单/NeedCategory/Urgency/ListingType/Scarcity bonus/Bootstrap/Needs连接)。GPU-Driven Clipmap 8 层 LOD + Gerstner 海洋 + 昼夜循环 + 5 群系系统 + OceanProvider trait + Transvoxel 骨架 + **LODCoordinator Phase2（完整8步算法）** + **天气系统 Phase1** 就位。**ECS 架构——`woworld_ecs` crate 已就位（42 Components + 28 Systems + 383 tests）**。★ **CHG-065 地形修改编排层已就位**（`woworld_core::edit_terrain`）。最新状态见 `woworld-dev-plan/01-核心基础/devlogs/`。

### 测试分布

| Crate | 测试数 | 说明 |
|-------|--------|------|
| `woworld_core` | 270 | culture + economy + faith + power + item + time + density + lod + weather_types + edit_terrain + inventory + equipment + assembly + listing + bootstrap |
| `woworld_worldgen` | 58 | biome + cave + clipmap + noise_gen + ocean + terrain + transvoxel + vegetation |
| `woworld_atmosphere` | 26 | time_curve + synthesizer + weather |
| `woworld_ecs` | 383 | 42 Components + 28 Systems (life/npc/lod + culture/economy/faith/power + item/inventory + needs) |
| `woworld_godot` | 0 | cdylib 不便于单元测试——已迁移至 worldgen/ecs |

> 详细模块状态见 [`附录E-开发状态.md`](woworld-dev-plan/附录E-开发状态.md)。

## 代码架构（woworld/ Rust workspace）

```
woworld/
├── Cargo.toml                  # workspace 清单（resolver="2", edition=2021, 5 crates）
├── crates/
│   ├── woworld_core/           # 核心类型 + trait 定义（仅 glam 依赖，ECS Component 不在此定义）
│   │   └── src/
│   │       ├── lib.rs          #   模块导出 + prelude
│   │       ├── types.rs        #   WorldPos, EntityId, EntityKind(5种), SpatialEntity,
│   │       │                   #     SpatialEvent, ScentSource, AcousticTag, TerrainHit, Aabb
│   │       ├── id.rs           #   ItemDefId, ItemEntId, SkillId, ProfessionTagId, ChunkCoord
│   │       ├── edit_terrain.rs #   ★ CHG-065 地形修改编排层 — EditDensity/EditHeightfield CoW
│   │       │                   #     存储, ModificationBatch, DirtyChunkQueue, EditDensityLayer
│   │       ├── spatial.rs      #   4 大 trait: TerrainQuery(9方法), EntityIndex(6方法),
│   │       │                   #     SpatialEventBus(3方法), VisibilityQuery(2方法)
│   │       ├── material.rs     #   SurfaceMaterial(21变体), Medium(4变体)
│   │       ├── density.rs      #   DensityStack — 分层密度场 + LayerPriority 排序
│   │       │                   #     ★ CHG-065 新增 material_at() 材质组合 + find_surface_y()
│   │       ├── ocean.rs        #   OceanProvider trait (6方法) — 海平面/水深/水下检测
│   │       ├── time.rs         #   WorldTime, WorldClock, TimeOfDay — 昼夜循环权威定义
│   │       ├── vegetation.rs   #   VegetationProvider trait + PlantCommunitySnapshot + 植被类型
│   │       ├── lod.rs          #   LodPrescription + LodCoordinator — 场景8层×角色5层距离映射
│   │       └── weather_types.rs#   WeatherState, Season — 天气/季节枚举 + 调试快捷键 1-6
│   ├── woworld_worldgen/       # GPU-Driven Clipmap 世界生成 (8 级 LOD + 海洋 + 洞穴)
│   │   └── src/
│   │       ├── lib.rs          #   导出 HeightfieldTerrain, BiomeClassifier, TerrainMeshData
│   │       ├── noise_gen.rs    #   双层 Perlin 噪声 (continent+detail+mountain) + 气候场 + 3D Worley
│   │       ├── biome.rs        #   温度×降水 2D 噪声 → 5 群系硬盒分类 (TOML 数据驱动)
│   │       ├── terrain.rs      #   HeightfieldTerrain — 完整 TerrainQuery trait 实现
│   │       ├── terrain_mesh.rs #   纯 Rust 网格生成 (引擎无关) — 顶点/索引/法线
│   │       ├── cave.rs         #   CaveDensity — 3D Worley 洞穴密度层
│   │       ├── ocean.rs        #   HeightfieldOcean — OceanProvider trait 实现 (Gerstner 波)
│   │       ├── clipmap.rs      #   GPU-Driven Clipmap — 8 层 LOD 环形网格 + Heightmap 纹理
│   │       ├── transvoxel.rs   #   Transvoxel 提取 — 密度场 → 过渡网格曲面
│   │       ├── transition_tables.rs # Transvoxel 过渡查找表 (4096 条目)
│   │       └── tri_table_data.rs    # Marching Cubes 三角剖分查找表 (256 条目)
│   │                           #   LOD 速查: L0(0-30m·TV·0.5m) → L4(500-1500m·TV·8m)
│   │                           #             L5(1500-4000m·SH·16m) → L7(7000-10000m·SH·64m)
│   ├── woworld_atmosphere/     # 大气与氛围系统 — 17 参数合成 (CHG-064)
│   │   ├── assets/
│   │   │   └── atmos_curve.toml#   时间曲线数据 (锚点/插值)
│   │   └── src/
│   │       ├── lib.rs          #   模块导出
│   │       ├── traits.rs       #   BiomeAtmosQuery, WeatherAtmosQuery, SeasonAtmosQuery (trait stub)
│   │       ├── time_curve.rs   #   AtmosCurve — TOML 驱动的颜色/亮度时间曲线
│   │       ├── synthesizer.rs  #   AtmosphereSynthesizer → ResolvedAtmosphere (17 字段)
│   │       └── resolved_atmosphere.rs # ResolvedAtmosphere — PackedFloat32Array 输出
│   └── woworld_godot/          # GDExtension 桥接（cdylib → Godot 4.7）
│       └── src/
│           ├── lib.rs          #   WoWorldExtension GDExtension 入口
│           ├── terrain_chunk.rs#   WorldDriver GodotClass — 8 层 LodLayer + ShaderMaterial
│           │                   #   Vertex Shader Camera-Relative Floating Origin
│           ├── voxel_chunk.rs  #   VoxelChunk GodotClass — Transvoxel 网格生成 + 顶点色
│           └── ocean.rs        #   Ocean GodotClass — Gerstner 波海洋渲染
├── godot/                      # Godot 4.7 项目
│   ├── project.godot           #   引擎配置（Forward+, GodotPhysics3D, MSAA 2x）
│   ├── WoWorld.gdextension     #   GDExtension 配置（Win/Linux/macOS 动态库路径）
│   ├── scenes/main.tscn        #   主场景（TerrainChunk + Player + DirectionalLight3D + Camera3D）
│   └── scripts/
│       └── player.gd           #   玩家控制器 — WASD + 鼠标环顾 + Space 跳跃 + G 键飞行
└── assets/                     # TOML 数据文件（群系、物品等 — 尚未填充）
```

### Crate 依赖链

```
woworld_core (glam 0.28)
  ├── woworld_worldgen (woworld_core, glam, noise 0.9)
  ├── woworld_atmosphere (woworld_core, serde, toml)
  └── woworld_godot (woworld_core, woworld_worldgen, woworld_atmosphere, godot 0.5)
```

### 架构原则

- **`woworld_core` — 最少依赖**：所有 ID 类型（`ItemDefId`, `SkillId`, `EntityId`, `ProfessionTagId`）、空间查询 trait（`TerrainQuery`, `EntityIndex`, `SpatialEventBus`, `VisibilityQuery`）、植被 trait（`VegetationProvider`）、LOD 类型（`LodPrescription`, 场景8层×角色5层距离映射）、天气类型（`WeatherState`, `Season`）、共享数据结构均在此定义。仅依赖 `glam`（SIMD 向量运算）。引擎无关，不依赖 Godot。**ECS Component 不在此定义**——由 `woworld_ecs` 承载。
- **`woworld_ecs` — ECS Component + System 定义（✅ 已就位·40 Components + 27 Systems + 345 tests）**：所有 ECS Component 和 System 在此定义。依赖 `woworld_core`（值类型 + trait）+ `hecs`（Archetype SoA 存储）。17/17 Systems 接入 Godot 主循环，21 NPC 可视化。★ 社会系统 Phase 1 + 物品 Phase 1 + 经济 Phase 2 已就位。
- **`woworld_worldgen` — GPU-Driven Clipmap 世界生成**：双层 Perlin 噪声高度场 + 5 群系硬盒分类（温度×降水 TOML 数据驱动）+ `TerrainQuery` trait 完整实现。8 层 Clipmap LOD（L0-L4 Transvoxel 0.5-8m 体素 + L5-L7 Signed Heightfield 16-64m 间距）。GPU-Driven 架构——网格启动时生成一次，Vertex Shader 通过 heightmap 纹理采样完成 Y 轴位移，运行时零 CPU mesh 修改。DensityStack 分层密度（高度场 + 3D Worley 洞穴层）。`OceanProvider` trait 实现（Gerstner 程序化波 + 海深色变）。纯 Rust 网格生成器（`terrain_mesh.rs`，引擎无关）+ Transvoxel 过渡网格（`transvoxel.rs`）。依赖 `woworld_core` + `noise` crate。
- **`woworld_atmosphere` — 大气与氛围系统**：合成 `ResolvedAtmosphere`（17 参数：天空色/雾/环境光/曝光/太阳色等），输出 `PackedFloat32Array` 供 Godot shader 消费。时间曲线优先——群系/天气/季节调制预留 trait stub。依赖仅 `woworld_core`（WorldTime, WorldPos）。引擎无关。
- **`woworld_godot` — 薄桥接层**：Rust 类型 ↔ Godot GDExtension API 的转换。不包含游戏逻辑。编译为 `cdylib`，由 Godot 运行时动态加载。包含 WorldDriver GodotClass（8 层 LodLayer + Vertex Shader Camera-Relative Floating Origin）、VoxelChunk GodotClass（Transvoxel 网格 + 顶点色专用 shader）和 Ocean GodotClass（Gerstner 波海洋渲染）。
- **Godot 项目 — 纯表现层**：渲染、UI、音频、输入、玩家物理（仅玩家保留 `PhysicsServer3D`——其他全部 Rust 侧空间查询）。
- **数据流**：`TOML 数据文件` → 各 crate 通过 `include_str!()` 平等加载 → `woworld_godot`（桥接层）→ Godot 场景树。
- **godot-rust 版本**：`godot` crate 0.5.x（GDExtension API）。

### 关键跨层契约（代码侧）

> 完整契约见 [[CLAUDE-INTERFACES.md]]。以下为代码层面的关键约束：

| 契约 | 说明 |
|------|------|
| ID 类型所有权 | 所有 ID 类型统一定义在 `woworld_core`（仅 glam 依赖）——各消费 crate 平等引用 |
| TOML 数据加载 | TOML 为中性格式——各消费模块通过 `include_str!()` 平等加载，无单点解析 |
| 空间查询 | 仅玩家保留 Godot `PhysicsServer3D`，NPC/生物/载具全部走 Rust 侧四 trait |
| 持久化 | `SaveableModule::snapshot_dirty() → SaveSystem → LMDB`——世界生成 P13 不直接写磁盘 |
| 引擎无关 | `woworld_core` 不依赖 Godot——降低未来引擎迁移成本（Bevy 分析见参考文档 032） |

## 关键目录与文件规则

### `想法/` vs `开发阶段/` — 两级文档体系

| 目录 | 性质 | 规则 |
|------|------|------|
| `想法/` | 概念设计/脑暴 | 可带 `#草稿` 标签，表示**允许矛盾、模糊、未决**——是思考过程而非最终结论 |
| `开发阶段/` | 正式开发规格 | **权威规格**。必须清晰、完整、无内部矛盾。优先以此为准 |

### README ≥60 行约定

所有模块根目录 README.md 必须 ≥60 行（CHG-048 规定，2026-06-22 Loop Audit 全量达标）。README 模板：模块定位 + 文档索引 + 架构速览 + 关键参数表 + 性能预算 + 消费模块导航。子模块 README（如 `06-认知与智慧系统/`、`08-NPC行动涌现与分类/`）同样适用此门槛。

### `.canvas` 文件 — ⚠️ 禁止文本编辑

Obsidian 画布文件（`.canvas`）是**二进制 JSON**——用 Edit/Write 工具编辑会损坏文件。只能通过 Obsidian 修改。

### `文件备份/` — ⚠️ 禁止修改

`WoWorld-Design/文件备份/` 下的所有目录（如 `20260618/`、`20260619skill设计前/` 等）是**历史快照**，用于回溯设计演进。**绝不能修改或删除**这些文件。

## 文件格式

| 格式 | 用途 | 注意事项 |
|------|------|----------|
| `.md` | 所有设计文档（主要格式） | — |
| `.canvas` | Obsidian画布文件 | ⚠️ 见上方规则 |
| `.svg` | Mermaid导出图表 | — |
| `.png` | 粘贴的图片资产 | — |
| `.base` | Obsidian模板文件 | 用于创建新文档的模板 |

## 文档元数据约定（Frontmatter）

所有设计文档在开头使用 `> ` 块引用格式标注元数据（非 YAML frontmatter）：

```markdown
> **开发代号**: WoWorld (Wonder World)
> **引擎**: Godot 4.7
> **开发者**: 独立游戏开发者（Solo）
```

## 项目配置

- **开发宪法系统**: `woworld-dev-plan/` — CONSTITUTION.md（元规则层·v2.0 精简版）+ 附录E-开发状态.md（全局状态）+ 附录D-模块依赖图.md（依赖图）+ sprint-proposals/ + handoff/。定义冲刺工作流和决策规则。流程全景见 [[woworld-dev-plan/00-流程总览.md]]。★ 2026-07-04 起，治理文件已重组为六阶段开发流程体系（详见 woworld-dev-plan/README.md）。
- **Obsidian**: `.obsidian/` 目录包含工作区配置，无需手动编辑
- **.gitignore**: `.claude/` 和 `woworld/target/` 已加入 gitignore
- **Git远程**: `git@github.com:QiaoWhite/WoWorld-Design.git` (SSH)
- **可复用 Skill**: `/woworld-tech-stack-audit` — 技术栈全量审计 8 阶段方法论。仅手动调用，不自动触发
- **可复用 Skill**: `/woworldidea-design` — 设计文档同步引擎（铁匠学徒）。sync(改后同步)/impact(改前预览)/gate(新模块闸门)/audit(旧文件修复)。详见 `.claude/skills/woworldidea-design/SKILL.md`
- **可复用 Skill**: `/woworld-loop-audit` — 设计文档自动审计循环系统。手动触发，最多 10 轮。全量轻扫（硬错误自动修复）+ 热点模块 16 维深度审计（软问题报告）。子代理工厂并行审计 + state.json 跨轮进度持久化。适用项目重大变更、里程碑审计、定期保鲜检查。详见 `.claude/skills/woworld-loop-audit/SKILL.md`

### `.claude/` 配置（自动化 Hook）

- **`settings.local.json`**: 唯一的 settings 文件（无 `settings.json`）——权限允许列表 + 两个自动化 hook：
  - **PostToolUse** (Write/Edit 触发): 调用 `.claude/hooks/posttooluse-detect.sh` — 检测到修改 `开发阶段/` 下文件时写入标记（`.claude/temp/modified_files.txt`）
  - **Stop** (回合结束触发): 调用 `.claude/hooks/stop-remind.sh` — 读取标记文件，若30分钟内有改动则在下轮注入同步提醒（`/woworldidea-design sync` 的兜底机制）
- **`audit-reports/`**: Loop Audit 输出目录，按 `YYYYMMDD-NNN/` 组织，含各模块审计报告 + `FINAL-SUMMARY.md` + `state.json`（跨轮进度）
- **`scheduled_tasks.lock`**: 计划任务（CronCreate 持久化任务）的锁文件。持久化任务数据写至 `.claude/scheduled_tasks.json`
- **仓库根目录 `task_plan.md` / `findings.md` / `progress.md`**: planning-with-files 工作流文件，由 Claude 自动维护，不应手动编辑

### 持久记忆系统

项目记忆存储于 `~/.claude/projects/C--Entertainment-GAME-DEV-The-Development/memory/`，由 `MEMORY.md` 索引。每个记忆一个 `.md` 文件（带 frontmatter: name/description/metadata）。分类：`user`（用户偏好）、`feedback`（反馈纠正）、`project`（项目事实）、`reference`（外部引用）。涉及跨会话需要记住的项目决策、用户反馈、非代码库可推导的背景时写入。

## 跨模块接口契约

> 📘 契约详情见 [[CLAUDE-INTERFACES.md]]。修改跨模块概念时**必须同步维护它**。

**冲突修正原则**：不删除原有设计。通过建立正确的派生/引用/映射关系消除冲突。两个模块定义同一概念的不同抽象层时——建立派生关系而非强制合并。有疑问时先与用户确认，不要从根上削减原有设计。

## 模块接头总览

> 📘 位于 `开发阶段/模块接头总览/`（00~26 共 27 个模块文件夹 + 变更追踪 + 孤儿接口汇总）。**定位**: 介于 CLAUDE-INTERFACES.md（契约宪法）和具体模块文档之间的接口地图。设计新模块时先查可消费接口；修改旧模块时查 `003-变更影响链.md` 看波及谁。

**保鲜协议**: 修改模块文档→同步更新接头条目。每模块维护 000-变更日志。新 CHG 必须包含"接头总览更新"章节。每 5 个 CHG 后检查时间戳。
> ⚠️ **保鲜延迟声明** (2026-06-22): 关键模块接头已更新至最新，剩余模块时间戳待后续增量更新。

---

## 工作约定

- **禁止自主循环审查**：除非用户手动触发（如 `/woworld-loop-audit` 或明确要求"重复直到没问题"），否则**不要**进入一轮又一轮的自我审查和纠错循环。发现问题→提出方案→执行修复→报告结果，一轮完成。不要自主反复自我批判。
- **冲刺工作流**：编码工作按 `woworld-dev-plan/CONSTITUTION.md` 定义的冲刺循环推进——Claude 自主提出冲刺提案（按 `附录F-冲刺模板.md`）→ 用户审批 → 执行 → 自检（`附录G-质量检查清单.md` 机械门+设计门+五层防御SOP）→ 用户审核 → 交接摘要（Handoff 模板）→ 下一个冲刺。触及新模块时第一步逐字精读 `开发阶段/` 原文档。编码中遭遇文档矛盾/模糊/不可行时，按宪法 §3 立刻停止并请示，不可自行填补设计空白。

- 所有新设计文档使用 `.md` 格式，放在对应的 `想法/` 或 `开发阶段/` 子目录下
- 使用 Obsidian wikilink 语法 `[[路径/文件名]]` 引用其他文档。**跨模块引用必须加 `[[]]`**——这是用户明确要求，方便 Obsidian 导航
- 新文档跟随 `> ` 块引用 frontmatter 风格
- Godot 项目代码在 `woworld/` 目录下（Rust workspace + Godot 项目，GPU-Driven Clipmap + 海洋）
- **设计变更**：对多个文档的结构性修改，在 `WoWorld-Design/Change/` 按 `CHG-XXX-简短描述-YYYYMMDD.md` 创建变更文档。CHG文档之间及CHG与参考文档之间用 `[[]]` 交叉引用
- **用户设计反馈**：`WoWorld-Design/Change/hand/` 目录存储用户对具体设计问题的直接裁决。涉及已有模块的修改时，先检查是否有相关用户意见
- **参考文档**：在 `参考文档/` 中创建 `NNN-简短描述-YYYYMMDD` 格式子文件夹，内部文档从 001 编号
- **技术决策**：以 `开发阶段/技术栈方案/` 为权威依据。所有 001-016 为历史演进存档，017 为测试方法论，018 已正式迁移至开发阶段
- **规划文件**：项目根目录的 `task_plan.md`、`findings.md`、`progress.md` 为 planning-with-files 工作流文件，用于追踪任务进度和设计决策。这些文件由 Claude 自动维护，不应手动编辑
- **修改后必须自检**：完成跨模块修改后，重新审计所涉及的模块间接口——确保没有引入新冲突
- **设计文档同步提醒（hook兜底）**：当本轮对话中修改了 `开发阶段/` 下的任何 `.md` 文件后，必须在下一轮对话开始前主动询问用户："检测到设计文档修改，需要运行 `/woworldidea-design sync` 进行同步吗？"。这是 hook 失效时的兜底规则，不得跳过
- **Skill 调用约定**：`/woworldidea-design` 为纯手动调用 skill。用户需显式输入命令。被动提醒仅通过 hook 或本规则触发，不自动执行任何 skill 命令
- **开发流程文档体系**: 六阶段（Phase 1-6）+ 附录 A-G + 9 场景 AI 标准作业流程。AI 启动时必须按 `woworld-dev-plan/00-流程总览.md` 的场景 SOP 执行。详见 `woworld-dev-plan/README.md`
- **🧪 研究先行**: 🟠🔴 级新问题必须先搜索行业方案再编码。4 步研究协议（识别→搜索→对比→记录）+ URL 引用可追溯。详见 `CONSTITUTION.md §5`
- **🧹 三重重构**: 🟢冲刺末轻量整理 → 🟡里程碑重构 → 🔴阶段深度清理（硬门禁，不通过不进下一阶段）。详见 `附录G-质量检查清单.md §🧹`
- **🔄 偏差升级**: 同目标 2 冲刺未完成→调整时间线 / 3 次→重评估（用户裁决）/ 2 次重评估→重规划（用户主导）。详见 `CONSTITUTION.md §7`

### ⚠️ 数学计算铁律

> **任何涉及数学计算时，必须用 `python -c "…"` 执行，禁止 LLM "心算"。** 包括但不限于：公式求值、参数验证、数值比较、概率推算、单位换算、范围边界检查。计算结果需标注来源（Python 执行输出）。设计文档中的关键公式应附 Python 验证脚本。

### 🐛 调试匹配协议（强制）

> **遇到任何 bug/异常行为/渲染问题，第一步不是排查代码——是查找 bug 索引。** 这防止 LLM 新会话遗忘导致已修复的 bug 被重复排查（历史上 terrain cracks 试了 15 次、Transvoxel y-z stride swap 花了 5.5h）。

1. **查索引**：`grep "症状关键词" woworld-dev-plan/bugs/INDEX.md`
2. **命中** → 打开对应条目 → 读"误诊路径"——列出的方案**禁止重复尝试**
3. **命中** → 读"验证方法"确认是否同一个 bug
4. **症状不完全匹配** → 也要先排除已知 bug 再形成新假说
5. **修复完成后** → 前向判断准入标准（LLM 新会话能否快速定位？不能→写新条目 + 更新 INDEX.md + 必要时在代码中标记 `// BUG:XX-NNN`）

### 🏥 用户反馈分诊协议

> **收到用户问题反馈时，必须先分诊再行动。禁止默认"立刻修复"。**

| 类别 | 判定标准 | 动作 |
|------|---------|------|
| 🔴 **回归** | 之前可工作、现在坏了。测试失败/编译错误/crash。 | 立刻修。 |
| 🟡 **已知偏离** | 已在 `附录E-开发状态.md` 或冲刺提案中追踪。 | 引用追踪 ID。不重复调查。 |
| 🟢 **参数调优** | 数值/视觉效果不满意，不是正确性错误。根因不依赖未完成的重构。 | 由用户决定现在修或延后。附成本估算。 |
| 🔵 **结构性依赖** | 真正的修复依赖未来冲刺中计划的重构/新模块。 | 确认追踪状态 → 标注哪个 Sprint 会解决 → **不现在修**。 |

**分诊回复格式**：

```
> **分诊**: [🔴🟡🟢🔵] — [一句话判定]
> **如果现在修**: 影响 ~N 文件, 预计 ~X-Y 分钟, 阻塞风险: [无 / 依赖 Sprint-N]
> **建议**: [现在修 / 记入已知问题 / 等待 Sprint-N 自然解决]
```

**原则**：用户有权在任何分诊后说"不，现在修"——分诊是建议，不是否决。

## 常用操作

| 操作 | 方式 |
|------|------|
| 创建设计变更 | 在 `WoWorld-Design/Change/` 创建 `CHG-XXX-描述-YYYYMMDD.md` |
| 修改后同步接口 | `/woworldidea-design sync`（同步所有关联文件） |
| 改前预览影响 | `/woworldidea-design impact` |
| 新模块脚手架 | `/woworldidea-design gate`（创建模块目录和接头文件） |
| 旧文件修复 | `/woworldidea-design audit` |
| 全量设计审计 | `/woworld-loop-audit`（最多10轮，硬错误自动修复 + 热点模块深度审计） |
| 技术栈全量审计 | `/woworld-tech-stack-audit`（8阶段3层嵌套，手动触发） |
| ECS 架构合规检查 | 对照 [[开发文档/00-ECS哲学与架构总纲/006-ECS铁律与陷阱]] 逐条审查 |
| 新建模块 | 8步标准流程（见下方"新模块设计标准工作流程"完整说明） |

## 新模块设计标准工作流程

创建或重大扩展一个模块时，遵循8步流程（详见 `WoWorld-Design/Change/` 中 CHG-025~028 实践）：

1. **参考文档草稿** → 2. **跨模块审计**（本模块为Owner，其他模块冲突以本模块为准）→ 3. **正式开发规格**（极尽清晰完整，自审偏移/遗漏/冲突）→ 4. **修改关联文档** → 5. **审查**（内部一致性+跨文档对齐+规格漂移）→ 6. **CHG 文档**（在 `WoWorld-Design/Change/` 创建）→ 7. **更新 CLAUDE.md + [[CLAUDE-INTERFACES.md]]** → 8. **更新模块接头总览**（新模块接头文件夹 + 受影响模块条目）
