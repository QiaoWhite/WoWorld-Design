# Sprint-035: ECS Phase 0 — hecs 基础设施 + 核心 Component + LodCoordinatorSystem

> **提案日期**: 2026-07-05
> **提案状态**: ✅ 已完成
> **所属阶段**: Phase 1 — 核心基础
> **所属里程碑**: 1J — ECS 基础设施
> **后续**: Sprint 036（生命系统·首个完整 ECS 模块，阻塞于本 Sprint）

## 📋 依赖前提检查

| 前置项 | 状态 | 备注 |
|--------|------|------|
| 1A Layer 0 核心类型稳定 | ✅ | EntityId, WorldPos, LodPrescription 就位 |
| LODCoordinator Phase 2 完成 | ✅ | Sprint 034 已提交，8 步算法完整 |
| 设计文档无歧义 | ✅ | `开发文档/` 42 篇就位，`1J-ECS基础设施.md` 明确 |
| workspace 健康 | ✅ | 107 tests ✅, clippy ✅, build ✅ |

## 🎯 目标（3 个）

### 目标 1: 创建 `woworld_ecs` crate + hecs 依赖

- **验收标准**: `cargo build --workspace` 通过（5 crate），`cargo check` 零错误
- **涉及模块**: `woworld_ecs`（新建 crate）
- **涉及代码**: `woworld/Cargo.toml`（workspace members）+ `woworld/crates/woworld_ecs/`（全部新建）

### 目标 2: 定义核心 Component + EntityId 互转

- **验收标准**: 5 个 Component（Position/Rotation/Velocity/EntityKind/LodLevel）定义完毕，EntityId ↔ hecs::Entity 无损往返，ECS 铁律 8 条全部通过
- **涉及模块**: `woworld_core`（只读·EntityKind 保留），`woworld_ecs`（Component 定义）
- **涉及代码**: `woworld_ecs/src/components/`（transform.rs, entity_kind.rs, lod.rs）, `woworld_core/src/types.rs`（EntityId 添加 to_hecs/from_hecs）

### 目标 3: WorldDriver 集成 + LodCoordinatorSystem

- **验收标准**: Godot 编辑器启动无变化，调试日志确认 ECS World 初始化，现有 107 测试零回归，新增 ≥5 ECS 测试
- **涉及代码**: `woworld_godot/src/terrain_chunk.rs`（WorldDriver + ecs 字段）, `woworld_ecs/src/systems/lod_coordinator.rs`（新建）

## 🧪 研究事项

| 问题 | 级别 | 研究计划 | 结果 |
|------|------|---------|------|
| hecs 0.10 API（World::spawn / query / CommandBuffer） | 🟡 | docs.rs 验证 API 签名 | 冲刺执行中填充 |
| GDExtension 中 hecs::World 的 Send/Sync 兼容性 | 🟡 | `hecs::World: Send + Sync` 验证；确认 WorldDriver 单线程持有 | 冲刺执行中填充 |
| `LodPrescription` → `LodLevel` 迁移策略 | 🟡 | 保留 LodPrescription 在 woworld_core（非 ECS 消费者用），LodLevel 作为 ECS Component 新定义 | 冲刺执行中填充 |

## 📊 决策矩阵

**单一候选，无竞争**。ECS Phase 0 是 1J 的唯一路径——hecs 已是宪法选型，无替代方案。

## 📖 必读文档清单

| 文档 | 路径 | 为什么读 |
|------|------|---------|
| ECS 铁律 8 条 | `开发文档/00-ECS哲学与架构总纲/006-ECS铁律与陷阱.md` | Component 设计不可违背规则 |
| hecs 存储与查询 | `开发文档/00-ECS哲学与架构总纲/004-hecs存储与查询.md` | API 参考 |
| 调度模型 | `开发文档/00-ECS哲学与架构总纲/005-调度模型.md` | 三阶段调度 + SystemMeta |
| ECS 基础设施 | `woworld-dev-plan/01-核心基础/1J-ECS基础设施.md` | 本冲刺任务清单 |
| 类型迁移表 | `开发文档/06-迁移映射/002-类型迁移表.md` | 哪些类型进 ECS、哪些保留 |
| 实现路线图 | `开发文档/06-迁移映射/003-实现路线图.md` | Phase 0-5 全景 |
| LOD 协调器 ECS | `开发文档/05-全局基础设施/03-LOD协调器.md` | LodLevel Component + LodCoordinatorSystem 规格 |

## 🔌 外部 API 预验证清单

| API | 来源 | 文档验证 | 状态 |
|-----|------|---------|------|
| `hecs::World::new()` | hecs 0.10 | docs.rs/hecs | ✅ |
| `hecs::World::spawn(tuple)` | hecs 0.10 | docs.rs/hecs | ✅ |
| `hecs::World::query::<&T>()` | hecs 0.10 | docs.rs/hecs | ✅ |
| `hecs::CommandBuffer` | hecs 0.10 | docs.rs/hecs | ✅ |
| `hecs::Entity::to_bits()` | hecs 0.10 | docs.rs/hecs | ✅ |
| `hecs::Entity::from_bits(u64)` | hecs 0.10 | docs.rs/hecs (unsafe) | ⚠️ 需验证 |
| `hecs::Component` trait | hecs 0.10 | docs.rs/hecs | ✅ `Send + Sync + 'static` |

## ⚠️ 需用户裁决的事项

无。本冲刺为基础设施——不改变任何现有行为，不破坏任何 API，仅新增 crate + Component + 一个 System。

## 📋 任务清单

### Step 0.1: 新建 `woworld_ecs` crate

- [ ] 创建 `woworld/crates/woworld_ecs/Cargo.toml`（依赖 `woworld_core` + `hecs = "0.10"`）
- [ ] 创建 `woworld/crates/woworld_ecs/src/lib.rs`（模块导出 + prelude）
- [ ] 在 workspace `Cargo.toml` 添加 `"crates/woworld_ecs"` 成员

### Step 0.2: 定义核心 Component

- [ ] `components/transform.rs` → `Position(pub Vec3)`, `Rotation(pub Quat)`, `Velocity(pub Vec3)`
- [ ] `components/entity_kind.rs` → `EntityKind` tag Component
- [ ] `components/lod.rs` → `LodLevel` Component（7 维 u8）
- [ ] `components/mod.rs` → 模块导出
- [ ] ECS 铁律逐条审查（纯数据·无堆·Send+Sync·零 unwrap）

### Step 0.3: EntityId ↔ hecs::Entity

- [ ] `EntityId::to_hecs(&self) -> hecs::Entity`（`from_bits`）
- [ ] `EntityId::from_hecs(entity: hecs::Entity) -> Self`
- [ ] 无损往返测试

### Step 0.4: WorldDriver 集成

- [ ] `woworld_godot/Cargo.toml` 添加 `woworld_ecs` 依赖
- [ ] `WorldDriver` 添加 `ecs: hecs::World` 字段
- [ ] `WorldDriver::new()` 中 `hecs::World::new()` + spawn Player Entity

### Step 0.5: LodCoordinatorSystem

- [ ] `systems/lod_coordinator.rs` → `lod_coordinator_system(world, cmd, camera, clock)`
- [ ] `systems/mod.rs` → 模块导出
- [ ] 现有 `LodCoordinator::compute_lod()` 逻辑封装进 System

### Step 0.6: 验证

- [ ] `cargo build --workspace` 通过
- [ ] `cargo test --workspace` 107 现有测试零回归
- [ ] 新增 ≥5 个 ECS 测试（hecs World 创建/Entity spawn/Component 增删/LodCoordinatorSystem/EntityId 往返/CommandBuffer）
- [ ] `cargo clippy --workspace -- -D warnings` 零警告
- [ ] Godot 编辑器启动验证（`_console.exe`）

## 预估

- **冲刺数**: 1
- **风险**: 🟢 低——hecs 仅增 3 个传递依赖，不改现有代码路径，LodPrescription 保留不动
- **代码量**: ~200-300 行新增（Component 定义 ~80 + EntityId 互转 ~20 + WorldDriver 集成 ~30 + LodCoordinatorSystem ~60 + 测试 ~80）
