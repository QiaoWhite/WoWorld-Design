# 附录C — 架构决策记录（ADR）

> **旧文件**: ARCHITECTURE_DECISIONS.md 内容已迁移至此。原文件保留作为历史参考。

> 每次做出影响跨模块接口或后续冲刺的架构选择时，即时记录。
> 格式：日期 + 冲刺 + 决策 + 原因 + 备选方案 + 波及。
>
> **维护者**: Claude Code（宪法 §4 提交前置检查强制登记）
> **最后更新**: 2026-07-06

---

## ADR 索引

| 编号 | 日期 | 冲刺 | 决策 |
|------|------|------|------|
| ADR-001 | 2026-06-25 | Sprint-002~005 | 标准 Marching Cubes 先行，Transvoxel 延后 |
| ADR-002 | 2026-06-25 | Sprint-004 | ClipmapManager 替代 ChunkManager 为活跃地形系统 |
| ADR-003 | 2026-06-25 | Sprint-006 规划 | 5 红色偏离分三批修复（P0: trait+seed → P1: chunk+transvoxel → P2: 多层密度） |
| ADR-004 | 2026-06-24 | CHG-064 偏离 | 大气合成从 GDScript 迁回 Rust——sun_elevation 物理驱动 |
| ADR-005 | 2026-07-04 | 六阶段流程体系 | 六阶段开发流程体系替代分散治理文件 |
| ADR-006 | 2026-07-05 | ECS 架构规划 | hecs ECS 架构采纳——Archetype SoA 存储，Component 拆装通信 |
| ADR-007 | 2026-07-06 | CHG-065 编排层 | 地形修改编排层——内核不转 ECS，编排层入 ECS |

---

## ADR-001: 标准 Marching Cubes 先行，Transvoxel 延后

**日期**: 2026-06-25
**冲刺**: Sprint-002~005
**决策**: 地形网格生成使用标准 Marching Cubes 算法。Transvoxel（带 transition cell 的 MC 变体）延后至 Sprint-007+。
**原因**: 标准 MC 在单个 LOD 层内产生正确的等值面。Transvoxel 的唯一优势是跨 LOD 层接缝消除——这个优势在当前 4 层 Clipmap 且无跨层混合的阶段不产生实际收益。先验证 MC 管线正确性，再加 Transvoxel。
**备选方案**: 直接实现 Transvoxel（增加 ~40% 初始实现复杂度，推迟 A.6 里程碑 1-2 天——当时冲刺目标优先）
**波及**: `marching_cubes.rs`（当前实现）· `clipmap.rs`（LOD 接缝处可见裂缝——已知限制）· 未来 `transvoxel.rs`（新增文件）

## ADR-002: ClipmapManager 替代 ChunkManager 为活跃地形系统

**日期**: 2026-06-25
**冲刺**: Sprint-004
**决策**: `ClipmapManager`（4 层 LOD clipmap）成为活跃地形加载系统。`ChunkManager`（固定网格 chunk 管理）保留为参考代码但不再使用。
**原因**: Clipmap 以玩家为中心动态调整 LOD——近处 MC 体素、远处高度场。固定网格 ChunkManager 无法支持 1km+ 视野距离。两者共存增加 WorldDriver 复杂度且无收益。
**备选方案**: 改造 ChunkManager 支持 LOD（本质上就是重新发明 Clipmap——不如果断切换）
**波及**: `clipmap.rs`（活跃）· `chunk_manager.rs`（僵尸代码，待标注 `#[allow(dead_code)]` 或删除）· `terrain_chunk.rs`（WorldDriver 消费 ClipmapManager）

## ADR-003: 5 红色架构偏离分三批修复

**日期**: 2026-06-25
**冲刺**: Sprint-006 规划
**决策**: 审计发现的 5 个红色偏离不一次修完——分三批：
- **P0 (Sprint-006)**: DensityField trait 补全 + Seed u32→u64
- **P1 (Sprint-007)**: Chunk 128m→32m + MC→Transvoxel
- **P2 (Sprint-008+)**: 单层密度→11 层可插拔 DensityProvider
**原因**: 一次修 5 个架构级变更的回归风险太高。DensityField trait 是纯接口扩展（低风险），seed 是类型替换（机械性）。chunk 大小变更和 Transvoxel 迁移涉及网格生成管线重写——需要独立冲刺。多层密度架构依赖前两批完成。
**备选方案**: 一次冲刺修全部（高回归风险 + 难以审查——74 个测试可能大量失效）
**波及**: 所有依赖世界生成的后续模块（植被/生命/NPC/建筑）——在 P0+P1 完成前，这些模块的地基不可靠

## ADR-004: 大气合成计算从 GDScript 迁回 Rust

**日期**: 2026-06-24
**冲刺**: CHG-064 偏离修复
**决策**: 昼夜渲染的太阳轨道/色板/季节偏移计算从 GDScript (`time_manager.gd`) 完全迁移到 Rust (`woworld_atmosphere` crate)。Godot 侧只保留 shader 参数设置——不保留任何模拟逻辑。
**原因**: GDScript 重复实现了 Rust `WorldTime` 已有的太阳公式——违反宪法 §14.1 边界铁律（Rust 权威原则）。GDScript 中写数学公式（sin/cos/lerp）在 LLM 训练数据中占比极低——幻觉风险高。Rust `AtmosCurve` 以太阳高度角驱动（物理正确），替代 GDScript 硬编码时间→颜色映射。
**备选方案**: 保留 GDScript 原型（短期更快，但每次天气/季节/群系扩展都需要同步维护两套公式——CHG-064 的经验证明这条路不可持续）
**波及**: `woworld_atmosphere/` crate 新建 · `time_manager.gd` 删除（~200 行）· `TerrainChunk::process()` 改为直接操控 Godot 节点

## ADR-005: 六阶段开发流程体系替代分散治理文件

**日期**: 2026-07-04
**决策**: 用层级化的六阶段开发流程文档体系（Phase 1-6 + 附录 A-G）替代分散的治理文件（CONSTITUTION v1.5 / WORKFLOW / 旧 DEVELOPMENT_STATUS 等）。新体系以 AI（Claude Code + Deepseek V4 Pro）为第一读者设计——线性可读、显式指令、场景驱动 SOP。

**原因**:
1. 旧治理文件各自为政，AI 需要在多个文件间跳转才能形成完整图景
2. 缺少从「当前状态」到「发布」的完整路线图
3. 跨会话上下文丢失严重，恢复成本高
4. 设计文档变更后执行计划无同步更新机制
5. 没有应对 Vibe Coding 常见问题的内建协议（代码腐化/忽略行业方案/跳过质量门）

**备选方案**:
- 在旧文件上增量修补（不解决根本的结构问题——文件间关系靠人工记忆）
- 用单一巨型文件（对 AI 不友好——token 消耗大，无法按场景精准读取）

**波及**:
- CONSTITUTION.md → 重写为精简版（仅不可违背原则）
- WORKFLOW.md → 拆解吸收到 00-流程总览.md + README.md
- DEPENDENCY_GRAPH.md → 附录D
- DEVELOPMENT_STATUS.md → 附录E（新增 Phase 映射列）
- GLOSSARY.md → 附录A
- PERFORMANCE_BUDGET.md → 附录B
- ARCHITECTURE_DECISIONS.md → 附录C（本文件）
- ONBOARDING.md → 拆解吸收到 README.md + 00-流程总览.md
- 新增: 附录F（冲刺模板）、附录G（质量检查清单）
- 新增: 每阶段独立文件夹（01-核心基础/ ~ 06-持续运营/），含 README + 里程碑 + handoff + devlogs

**关键设计决策**（9 场景 SOP / 三重重构节奏 / 研究先行协议 / 偏差升级协议 / 三件套文档体系）详见 `00-流程总览.md` 和 `CONSTITUTION.md`（v2.0）。

---

## ADR-006: hecs ECS 架构采纳

**日期**: 2026-07-05
**决策**: 游戏逻辑层采纳 ECS（Entity Component System）架构，使用 `hecs` crate 作为 Archetype SoA 存储后端。System 之间通过 Component 拆装通信，不使用 before/after 顺序依赖。

**原因**:
1. 100+ NPC 全模拟需要 SoA 缓存友好内存布局——Archetype 模式天然支持
2. 20+ 系统需要无冲突并行——Component 级别读写声明可实现编译期并行安全验证
3. 新创意接入 = 新 Component + 新 System，不改已有代码——符合涌现优先哲学
4. hecs 选择理由：轻量（仅 ahash + hashbrown 两个传递依赖），无内置调度器（不需要 before/after），变更检测和 CommandBuffer 内置
5. 现在改架构 = 改规划。以后改 = 重写代码

**备选方案**:
- bevy_ecs：功能完整但捆绑调度器（强制 before/after 思维模式），依赖树 ~15+ crate——过重
- 自建 ECS：完全控制但 ~2000+ 行维护负担，且 Archetype 迁移、变更检测、并行迭代均需从零实现
- 不采用 ECS（维持当前 SoA 面向对象架构）：短期最快，但 20+ 系统交互复杂度会在 Phase 3 失控

**波及**:
- `woworld/` workspace：新建 `woworld_ecs` crate（`woworld_core` + `hecs 0.10`）
- `woworld_ecs/src/components/`：定义 ECS Component 类型（~80 Component 规划中，Phase 0 首批 5 个）
- `woworld_godot/src/terrain_chunk.rs`：`WorldDriver` 新增 `ecs: hecs::World` 字段
- 现有 107 个测试：每步迁移后必须全绿
- 不移入 ECS 的模块（5 个）：woworld_worldgen、woworld_atmosphere、Godot UI、Godot 音频渲染、Godot 动画渲染
- 核心类型保留（EntityId、WorldPos、SurfaceMaterial 等）：作为基础类型用于 Component 字段，不成为 ECS Component
- 调度模型：Phase 0（输入/LOD·唯一合理例外）+ Phase 1（游戏逻辑·无顺序·rayon 并行）+ Phase 2（Godot 同步）
- 架构约束：Component = 纯数据零方法，大堆数据不进 Component 内联，实体删除走标记+统一清理
- 设计文档：ECS 架构设计见 `开发文档/`（42 篇，与 `WoWorld-Design/` 并行）

**关联文档**: [[../开发文档/00-ECS哲学与架构总纲/006-ECS铁律与陷阱]] · [[../开发文档/06-迁移映射/003-实现路线图]]

---

## ADR-007: 地形修改编排层——内核不转 ECS，编排层入 ECS

**日期**: 2026-07-06
**冲刺**: CHG-065 地形修改编排层
**决策**: 地形数据（高度场、密度场、海洋）保持为 ECS Resource，不进入 Archetype 存储。地形修改的**生命周期管理**（请求验证、dirty 传播、重 mesh 调度、NPC 感知通知）走 ECS Component + System + Event 编排。

**原因**:
1. 业界一致：Veloren（MMO 体素 RPG·SPECS ECS）将 TerrainGrid 作为 ECS Resource，Chunk 有独立的稀疏存储结构；bevy_voxel_world 使用两层架构（procedural 函数 + persistent HashMap）
2. 地形是"世界级单例"——不是"有生命周期和状态变化"的实体。ECS 的 Archetype 优化对地形无益
3. 地形查询的主要开销是密度场计算（5-20 倍于 mesh 提取本身），ECS 不加速此过程
4. 修改编排层（ModificationBatch 积累 → DirtyChunkQueue 标记 → VoxelChunk 重提取 → SpatialEventBus 通知）天然适合 ECS 事件驱动 + System 调度模式
5. Copy-on-Write（`Arc<HashMap>` 原子交换）提供零锁读取——一次 Transvoxel 提取做 ~36K 密度采样，`RwLock` 不可接受

**关键类型**（`woworld_core::edit_terrain`）：
- `EditDensitySnapshot/Builder` — 3D 稀疏密度修改（CoW）
- `EditHeightfieldSnapshot/Builder` — 2D 表面高度投影（CoW）
- `ModificationBatch` — 帧内批量修改积累（Veloren BlockChange 模式）
- `DirtyChunkQueue` — Chunk 脏标记 + 两级索引
- `EditDensityLayer` — `DensityProvider` 桥接，插入 VoxelChunk 临时栈

**2D/3D 分裂修复**：所有 9 个 `TerrainQuery` 方法（`height_at`、`normal_at`、`density_at`、`terrain_raycast`、`is_walkable`、`surface_material_at`、`medium_at`、`sample_vertex`、`sample_horizon`）均修改为优先查 EditHeightfield/EditDensity，回退噪声。

**波及**:
- 新建 `woworld_core/src/edit_terrain.rs`：~600 行 + 9 测试
- 修改 `woworld_core/src/density.rs`：`material_at()` + `find_surface_y()`
- 修改 `woworld_worldgen/src/terrain.rs`：EditTerrain 集成 + 所有查询方法
- 修改 `woworld_worldgen/src/ocean.rs`：EditHeightfield 集成
- 修改 `woworld_godot/src/terrain_chunk.rs`：VoxelChunk EditDensityLayer 集成
- 待后续：ECS systems 实现、WorldDriver 帧同步、Clipmap 重生成、SpatialEventBus、持久化接口

**关联文档**: [[../../WoWorld-Design/Change/CHG-065-地形修改编排层-20260706|CHG-065]] · [[../开发文档/01-世界框架/01-世界生成|ECS 世界生成架构]]
