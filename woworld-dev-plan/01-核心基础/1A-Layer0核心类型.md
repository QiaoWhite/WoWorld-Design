# 1A — Layer 0 核心类型

> **状态**: ✅ 完成
> **所属阶段**: Phase 1 — 核心基础
> **前置依赖**: 无

## 目标

完成 `woworld_core` crate——所有 ID 类型、空间查询 trait、共享数据结构的唯一定义地。仅依赖 glam。**ECS Component 不在此定义**——由 `woworld_ecs` crate 承载（见 [1J](1J-ECS基础设施.md)）。

## 涉及模块

| 模块 | 设计文档 | 代码 |
|------|---------|------|
| woworld_core | `CLAUDE-INTERFACES.md`（27 契约段） | `woworld/crates/woworld_core/` |

## 任务清单

- [x] WorldPos, EntityId, EntityKind(5 种), SpatialEntity, SpatialEvent, ScentSource, AcousticTag, TerrainHit, Aabb
- [x] ItemDefId, ItemEntId, SkillId, ProfessionTagId, ChunkCoord — 所有 ID 类型
- [x] TerrainQuery trait (9 方法) — 地形采样 + 材质查询 + 视线测试
- [x] EntityIndex trait (6 方法) — 空间范围查询 + 最近邻
- [x] SpatialEventBus trait (3 方法) — 事件发布/订阅/查询
- [x] VisibilityQuery trait (2 方法) — 可见性 + 遮挡
- [x] SurfaceMaterial (21 变体) + Medium (4 变体)
- [x] OceanProvider trait (6 方法) — 海平面/水深/水下检测
- [x] VegetationProvider trait — 植被覆盖查询
- [x] WorldTime, WorldClock, TimeOfDay — 昼夜循环权威定义
- [x] SaveableModule trait — 持久化接口
- [x] DensityStack — 分层密度场架构

## 验收标准

### 🔧 技术
- [x] `cargo build -p woworld_core` 通过
- [x] `cargo test -p woworld_core` 41 tests 全绿
- [x] `cargo clippy -p woworld_core -- -D warnings` 零警告
- [x] 仅依赖 glam（零其他依赖）

### 📐 设计
- [x] 所有 trait 签名与 `CLAUDE-INTERFACES.md` 一致
- [x] 所有 pub 类型已登记到术语表
- [x] 引擎无关——不依赖 Godot 任何类型

### 🎮 功能
- [x] 被 woworld_worldgen, woworld_atmosphere, woworld_godot 成功消费
