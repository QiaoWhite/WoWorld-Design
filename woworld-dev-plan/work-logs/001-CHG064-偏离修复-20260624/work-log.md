# 工作日志 — CHG-064 偏离修复

> **日期**: 2026-06-24
> **编号**: 001
> **关联**: CHG-064, CONSTITUTION v1.4, D01-D04

## 概述

修复 CHG-064（轨A 昼夜循环+5群系系统）中记录的 4 项架构偏离，并进行了重大架构重构。

## 偏离修复

### D01（🔴→✅）：GDScript 时间/天空逻辑重复

**问题**：`time_manager.gd` 在 GDScript 中独立重写了日循环/太阳位置/天空颜色计算，Rust 侧 5 个 `#[func]` 成为僵尸代码。

**修复过程**：
1. 删除 `time_manager.gd`、`terrain_renderer.gd`（死代码）
2. 移除 `main.tscn` 中的 TimeManager 节点
3. `TerrainChunk::process()` 直接操控 Godot 节点（DirectionalLight3D, ProceduralSkyMaterial, WorldEnvironment）
4. 删除 5 个僵尸 `#[func]`+ `RefCell<WorldClock>` + 12 硬编码颜色常量

### D02（🟡→✅）：woworld_core serde 未 feature-gate

**修复**：`serde` 改为 optional，`SurfaceMaterial` 用 `#[cfg_attr(feature = "serde")]` 条件派生。`woworld_worldgen` 启用 `features = ["serde"]`。

### D03（🔴→✅）：WorldClock 推进方式

**已自然解决**：D01 修复中改为 `process(&mut self)` 直接 `self.clock.advance(delta)`，无需 `RefCell`。

### D04（🟡→⬜）：SubMaterialWeight 死代码

**非偏离**：设计文档规定的子材质后续迭代功能，接口已就绪，待渲染管线消费。

## 架构决策

1. **太阳高度角驱动大气曲线**：天空色彩驱动源从 `day_progress`（抽象时间）改为 `sun_elevation`（太阳高度角）。朝霞/晚霞在天文学地平线（elev=0°）精确出现。
2. **锚点数组化**：`TimeCurve` 7 命名字段 + 8 if-else → `AtmosCurve` 通用 `Vec<AtmosAnchor>` 排序数组。增删锚点只需改 TOML，零 Rust 代码变动。
3. **WorldClock 下沉**：`woworld_worldgen → woworld_core`，core 成为时间权威。
4. **新建 woworld_atmosphere**：独立 crate（依赖仅 core），`AtmosphereSynthesizer` + `AtmosCurve` + TOML 数据驱动。

## 代码变更

### 新建
- `woworld/crates/woworld_atmosphere/` — 5 源文件 + 1 TOML 数据文件
  - `src/lib.rs`, `src/resolved_atmosphere.rs`, `src/time_curve.rs`, `src/synthesizer.rs`, `src/traits.rs`
  - `assets/atmos_curve.toml`

### 修改
- `woworld/crates/woworld_core/src/time.rs` — 新增 `WorldClock`（从 worldgen 移入）
- `woworld/crates/woworld_core/src/material.rs` — serde feature-gate
- `woworld/crates/woworld_core/Cargo.toml` — serde optional feature
- `woworld/crates/woworld_worldgen/src/lib.rs` — 移除 `pub mod time`，重导出 `WorldClock`
- `woworld/crates/woworld_worldgen/src/terrain.rs` — 导入路径更新
- `woworld/crates/woworld_worldgen/Cargo.toml` — `woworld_core = { features = ["serde"] }`
- `woworld/crates/woworld_godot/src/terrain_chunk.rs` — 完全重写（process 直接驱动 Godot 节点）
- `woworld/crates/woworld_godot/Cargo.toml` — 新增 `woworld_atmosphere` 依赖
- `woworld/Cargo.toml` — workspace members 新增 `woworld_atmosphere`
- `woworld/godot/scenes/main.tscn` — 移除 TimeManager 节点

### 删除
- `woworld/crates/woworld_worldgen/src/time.rs`
- `woworld/godot/scripts/time_manager.gd`
- `woworld/godot/scripts/terrain_renderer.gd`

## 验证结果

```
cargo check --workspace  ✅
cargo test --workspace   ✅ 57 tests (atmosphere 11 + core 12 + spatial 12 + worldgen 19 + godot 3)
cargo clippy --workspace ✅ 零警告
```

## 关键教训

1. **硬编码是万恶之源**：锚点位置 `const` 与 TOML 数据重复定义 → 改为从数据派生。
2. **物理量驱动优于抽象量驱动**：`sun_elevation` 替代 `day_progress` 后太阳与天空严格同步。
3. **GDScript 铁律不可违反**：`time_manager.gd` 从 200 行 GDScript → 0 行，全部数学公式移入 Rust。
4. **命名字段陷阱**：7 个锚点 = 7 个 struct 字段 + 8 个 if-else → 通用数组循环。
