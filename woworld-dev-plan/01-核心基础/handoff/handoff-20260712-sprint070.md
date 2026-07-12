# Handoff: 2026-07-12 — Sprint-070 V2 牵引移动

> **会话类型**: 冲刺执行（场景 B）· **日期**: 2026-07-12
> **阶段**: Phase 2 探针切片「活着的村庄」·进度 `▓▓▓▓▓░░░░░ 5/10`
> **状态**: ✅ Sprint-070 完成·1100 tests 全绿·clippy/fmt 零警告·**待提交**

## 📊 本会话做了什么

1. **Sprint-069 推送确认**（已推·a91d6df）
2. **Sprint-070 提案编写**（二轮审查·16 项检查·🔴1 架构修正 + 🟡7 细节修正）
3. **Sprint-070 执行**——`goal_resolution_system` 同步填充 `target_pos`（FindFood→植被查询·FindWater→水点搜索）+ WorldDriver 接线（BiomeVegetation 初始化 + Block A4 传参）

### 架构决策

**合并进 goal_resolution_system 而非独立 system**：Block A4 共享 CommandBuffer——分拆方案中独立 system 看不到同批排队的 Goal（1 帧延迟+逻辑不一致）。合并后 Goal 创建时同步填充 target_pos，更简洁。

## 📦 产物

**代码**（`woworld/`）：`ecs/goal.rs`（`goal_resolution_system`+3 参数·`resolve_target_pos`·`find_nearest_water_xz`·+8 tests）+ `godot/terrain_chunk.rs`（`BiomeVegetation` 初始化 + Block A4 接线）
**文档**（`woworld-dev-plan/`）：提案 sprint-070、DEVLOG、本 Handoff。

## 💾 恢复点（下一会话 AI 必读）

> ⚠️ 下一会话按 `00-流程总览` **场景 B（冲刺执行）** 启动——V2 已完成，下一步 **V3a 代谢闭环**。

- **当前阶段**: Phase 2 切片·`5/10`（V0+V1+V4a+Vf+V2 ✅）·10 步序列见 [[../../02-垂直切片/README]] §3
- **下一步**: **V3a 代谢闭环**（第 6/10 步·命门）——最小采集配方（走交互配方表）+ `eat_food` 消费系统 → 采集入库、进食使 `Needs.hunger` 真实下降/回 Vitals。依赖：V0 ✅（库存）+ Vf ✅（植被）+ **V2 ✅（到达目标）**
- **机械门状态**: `1100 tests 全绿`（core 401 + worldgen 75 + atmosphere 26 + ecs 598（589 lib + 9 集成） + godot 0），clippy/fmt 零警告，build 通过。
- **提交状态**: ⚠️ **未提交**。基线 = `a91d6df`（Sprint-069）。本会话改动待提交。
- **A1 铁律**: 纯涌现，涌现不出的如实呈现空白，禁脚手架/假坐标/占位驱动/平行 trait/语音决策 silo。
- **V2 哨兵值**: search_radius=150m（硬编码·待可配置化）。`find_nearest_water_xz` 8×8 极坐标网格扫描。

## ⚠️ 遗留 / 诚实边界

- **V2 不解析 FindRest/FindSafePlace 等**：仅 FindFood + FindWater——其余 GoalType 保持 target_pos=None（漫游）
- **V2 不做寻路**：直线走向目标·体素 A* 是 Phase 3 的事
- **V2 不区分淡水/咸水**：只要 `water_depth > 0` 就认为是"水源"——V3a 的事
- **Godot 集成测试**: 零基础设施——WorldDriver 接线手工实机抽查
- **Goap 设计文档 Gap**: `NPC活人感开发文档ver2.0` 使用 `Goal::SurvivalEat` 风格的 GOAP 目标，当前代码使用简化 `GoalType::FindFood`——垂直切片有意简化

## 🔗 关联

- **提案**: [[../../sprint-proposals/sprint-070-V2-牵引移动-20260712]]
- **DEVLOG**: [[../devlogs/DEVLOG-2026-07-12-sprint070]]
- **上游**: [[handoff-20260712-sprint069]] · **路线图**: [[../../02-垂直切片/README]] · [[../../附录E-开发状态]]
- **下游**: V3a 代谢闭环（命门）
