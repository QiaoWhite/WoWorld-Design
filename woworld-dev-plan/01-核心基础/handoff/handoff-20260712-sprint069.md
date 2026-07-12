# Handoff: 2026-07-12 — Sprint-069 Vf 食物源落地

> **会话类型**: 冲刺执行（场景 B）· **日期**: 2026-07-12
> **阶段**: Phase 2 探针切片「活着的村庄」·进度 `▓▓▓▓░░░░░░ 4/10`
> **状态**: ✅ Sprint-069 完成·1092 tests 全绿·clippy/fmt 零警告·**待推送**

## 📊 本会话做了什么

执行 Sprint-069（Vf）。先经**三轮自审查**（7 问题全部自修——高度采样缺口/种子判别符对齐/Poisson 间距推导/海洋兜底/Godot 桥 gap/splitmix 复用/yield 确定性），再编码，再**编码后审计**（7 维对撞设计 012/010 + 提案·发现 3 项全部自修）。

1. **`HarvestableInfo` 补 `regen_state`**：对齐设计 010 §1.2——枚举已定义，全部 `Full`（消耗是 V3a 的事）。
2. **Poisson disc（Bridson 2007）**：`r/√2` 背景网格·活跃列表·环带 [r,2r]·k=30·确定性 `mix64` 链式 RNG。r=4m（采集物是全集子集·有效间距）。
3. **`query_harvestable` 真实返回**：TC 粒度·群系分类门控·加权产物抽取·`classifier.sample_height()` 地形 y·对齐设计 012 §八 的 `instance_id` 派生。
4. **`BiomeClassifier::sample_height()`**：1 行委托 `WorldNoise`——填补"采集物需地形高度但植被模块无高度采样路径"的缺口。
5. **`mix64` → `pub(crate)`**：复用同 crate 现有 splitmix64·零重复代码（跨 crate 收敛是独立轻活）。

## 📦 产物

**代码**（`woworld/`）：`core/vegetation.rs`（+`regen_state` 字段）、`worldgen/noise_gen.rs`（`pub(crate)`）、`worldgen/biome.rs`（`sample_height`）、`worldgen/vegetation.rs`（+`world_seed`·Poisson disc·`query_harvestable`·群系映射·17 新测试）。
**文档**（`woworld-dev-plan/`）：提案 sprint-069、DEVLOG、本 Handoff。
**待更新**：`附录E`、`02-垂直切片/README` §4 Vf 行。

## 💾 恢复点（下一会话 AI 必读）

> ⚠️ 下一会话按 `00-流程总览` **场景 B（冲刺执行）** 启动——Vf 已完成，下一步 **V2 牵引移动**。

- **当前阶段**: Phase 2 切片·`4/10`（V0+V1+V4a+Vf ✅）·10 步序列见 [[../../02-垂直切片/README]] §3
- **下一步**: **V2 牵引移动**（第 5/10 步）——`Goal.target_pos` 解析到最近可采集食物点（复用 `query_harvestable`）/ 水（`OceanProvider`）。需求**牵引**，非排班。依赖：Vf ✅（食物点存在）+ V1 ✅（昼夜调制）。
- **机械门状态**: `1092 tests 全绿`（core 401 + worldgen 75 + atmosphere 26 + ecs 590 + godot 0），clippy/fmt 零警告，build 通过。
- **提交状态**: ⚠️ **未提交/未推送**。基线 = `d3d882f`（Sprint-068 收尾）。本会话改动待提交。
- **A1 铁律**: 纯涌现，涌现不出的如实呈现空白，禁脚手架/假坐标/占位驱动/平行 trait/语音决策 silo。
- **Vf 哨兵值**: `SpeciesId(1)`=BerryBush·`SpeciesId(2)`=Mushroom·`SpeciesId(3)`=NutTree·`SpeciesId(4)`=Herb·`SpeciesId(5)`=Flower·`SpeciesId(6)`=FiberPlant——MVP 硬编码，待 `PlantSpeciesRegistry` 替换。

## ⚠️ 遗留 / 诚实边界

- **Vf 不建 crate**：`woworld_life`/`woworld_vegetation` 零代码——Vf 群系→产物硬编码，`SpeciesId` 哨兵值。
- **Vf 不实现 VMC 双层**：直接 TC 32m 粒度——VMC（1km）是 P2.25 完整实现的事。
- **Vf 不做 RegenState 消耗/再生**：全部 `Full`——采集→消耗是 V3a 的事。
- **Vf 不做季节连线**：`season_optimal` 恒 true。
- **Godot 桥 `set_vegetation_provider` 从未调用**：方法存在（`terrain_chunk.rs:1959`）但无调用点——V2/V3a 集成时自然接通。
- **splitmix 跨 crate 收敛搁置**：复用同 crate `noise_gen::mix64`·零重复——收敛到 `woworld_core` 是独立轻活（单 CHG）。
- **`HarvestableInfo.instance_id` 保持 `u64`**：设计 010 指定 `PlantInstanceId` newtype——待 `woworld_life` crate 统一引入。

## 🔗 关联

- **提案**: [[../../sprint-proposals/sprint-069-Vf-食物源落地-20260712]]
- **DEVLOG**: [[../devlogs/DEVLOG-2026-07-12-sprint069]]
- **上游**: [[handoff-20260712-sprint068]] · **路线图**: [[../../02-垂直切片/README]] · [[../../附录E-开发状态]]
- **下游**: V2 牵引移动 → V3a 代谢闭环（命门）
