# 1J — ECS 基础设施

> **状态**: 🟢 Phase 0 完成 — hecs 0.10 就位 + 5 Component + LodCoordinatorSystem
> **所属阶段**: Phase 1 — 核心基础
> **前置依赖**: 1A（Layer 0 核心类型——EntityId、WorldPos、LodPrescription 等值类型）

## 目标

完成 ECS Phase 0——`hecs` 依赖就位 + `woworld_ecs` crate 新建 + 核心 Component 定义 + `WorldDriver` 集成 + 首个 ECS System（`LodCoordinatorSystem`）。

这是后续所有 ECS 模块（生命/NPC/社会/交互）的地基——必须先于任何 ECS-based 模块工作。

## 涉及模块

| 模块 | 设计文档 | 代码 |
|------|---------|------|
| woworld_ecs | `开发文档/00-ECS哲学与架构总纲/`（6 篇）· `开发文档/05-全局基础设施/` | `woworld/crates/woworld_ecs/`（新建） |
| woworld_godot | `开发文档/06-迁移映射/003-实现路线图.md` §Step 0.3 | `woworld/crates/woworld_godot/`（WorldDriver 扩展） |

## 任务清单

### Step 0.1: 新建 `woworld_ecs` crate

- [ ] 创建 `woworld/crates/woworld_ecs/` 目录结构
- [ ] 创建 `Cargo.toml`：依赖 `woworld_core` + `hecs = "0.10"`
- [ ] 创建 `src/lib.rs`：模块导出 + prelude
- [ ] 在 workspace `Cargo.toml` 注册新成员

### Step 0.2: 定义核心 Component

在 `woworld_ecs/src/components/` 下创建：

- [ ] `transform.rs` → `Position(pub Vec3)`, `Rotation(pub Quat)`, `Velocity(pub Vec3)`
- [ ] `entity_kind.rs` → `EntityKind` tag Component（迁移自 `woworld_core::types::EntityKind` 枚举）
- [ ] `lod.rs` → `LodLevel` Component（迁移自 `woworld_core::lod::LodPrescription`，7×u8 子 LOD 维度）

### Step 0.3: WorldDriver 集成

- [ ] 在 `woworld_godot/Cargo.toml` 添加 `woworld_ecs` 依赖
- [ ] 在 `woworld_godot/src/terrain_chunk.rs` 的 `WorldDriver` 结构体中添加 `ecs: hecs::World` 字段
- [ ] 在 `WorldDriver::new()` 中初始化 `hecs::World::new()`
- [ ] 确保 Godot 侧不持有 Entity 引用——仅通过 Rust `#[func]` 查询

### Step 0.4: 首个 ECS System —— `LodCoordinatorSystem`

- [ ] 将现有 `LodCoordinator::compute_lod()` 封装为 `LodCoordinatorSystem`
- [ ] System 写入 Player Entity 的 `LodLevel` Component
- [ ] 消费者 System（如 ClipmapManager）通过读取 `LodLevel` 确定 LOD 行为（单帧延迟——设计特性）

### Step 0.5: EntityId ↔ hecs::Entity 互转

- [ ] 实现 `EntityId::to_hecs(&self) -> hecs::Entity`（`from_bits(self.0)`）
- [ ] 实现 `EntityId::from_hecs(entity: hecs::Entity) -> Self`（`EntityId(entity.to_bits())`）
- [ ] 无损往返测试验证

### Step 0.6: 模型动作与物理系统 + 音频系统适应性调整

- [ ] 确认 5 个不进 ECS 的模块（worldgen / atmosphere / UI / 音频渲染 / 动画渲染）无需修改
- [ ] 确认 `TerrainQuery` / `OceanProvider` / `VegetationProvider` trait 签名无需变更——它们作为 Resource 被 System 消费，不成为 Component

## 验收标准

### 🔧 技术

- [ ] `cargo build --workspace` 通过（5 crate：core + worldgen + atmosphere + **ecs** + godot）
- [ ] `cargo test --workspace` 现有 107 测试全绿（零回归）
- [ ] `cargo clippy --workspace -- -D warnings` 零警告
- [ ] 新增 ECS 测试 ≥5 个：
  - hecs World 创建 + Entity spawn
  - Component 增删（insert / remove / satisfies）
  - LodCoordinatorSystem 基本功能（输入距离 → 输出 LodLevel）
  - EntityId ↔ hecs::Entity 无损往返
  - CommandBuffer 延迟执行

### 📐 设计

- [ ] 所有 Component 满足 ECS 铁律（对照 `开发文档/00-ECS哲学与架构总纲/006-ECS铁律与陷阱.md` 逐条审查）：
  - Component = 纯数据，零方法
  - 无堆数据内联（无 `Vec`/`HashMap`/`String` 字段）
  - 实现了 `'static + Send + Sync`
- [ ] `EntityId::to_hecs()` / `from_hecs()` 往返无损
- [ ] `hecs::World` 仅存在于 `WorldDriver`——不泄漏到 Godot 侧或 `woworld_core`

### 🎮 功能

- [ ] Godot 编辑器启动后场景渲染无变化（ECS 尚未影响渲染管线）
- [ ] 调试日志确认 ECS World 初始化成功

## 关联文档

| 文档 | 用途 |
|------|------|
| `[[../../开发文档/00-ECS哲学与架构总纲/006-ECS铁律与陷阱]]` | Component 设计规则（不可违背） |
| `[[../../开发文档/00-ECS哲学与架构总纲/004-hecs存储与查询]]` | hecs API 参考 |
| `[[../../开发文档/00-ECS哲学与架构总纲/005-调度模型]]` | 三阶段调度设计 |
| `[[../../开发文档/05-全局基础设施/01-核心类型]]` | 不进 ECS 的核心类型清单 |
| `[[../../开发文档/05-全局基础设施/03-LOD协调器]]` | LodLevel Component + LodCoordinatorSystem 规格 |
| `[[../../开发文档/06-迁移映射/003-实现路线图]]` | ECS Phase 0-5 全景路线图 |

## 预估

- **冲刺数**: 1 冲刺（首次 ECS 代码落地，基础设施类任务）
- **风险**: 低——hecs 仅增 2 个传递依赖（ahash + hashbrown），不改变现有代码路径
