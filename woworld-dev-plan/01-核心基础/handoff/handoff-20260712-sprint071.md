# Handoff: 2026-07-12 — Sprint-071 V3a 代谢闭环

> **会话类型**: 冲刺执行（场景 B）· **日期**: 2026-07-12
> **阶段**: Phase 2 探针切片「活着的村庄」·进度 `▓▓▓▓▓▓░░░░ 6/10`
> **状态**: ✅ Sprint-071 完成·1111 tests 全绿·clippy/fmt 零警告·**待提交**

## 📊 本会话做了什么

1. **V3a 计划编写**——三轮审计（自审→双 agent 深度审计→终稿）。对照 `生命004`·`物品007`·`NPC活人感ver2.0`·`NPC行动涌现001`·`010-NPC移动行为` 逐项审计，12 项设计偏差全部记录在案。
2. **V3a 编码**——`ConsumableEffect` 展开 + `resolve_plant_yield` + `ArrivedAtTarget` + `movement` 改造 + `harvest_on_arrival_system`(新) + `consume_system`(新) + WorldDriver 接线 + 4 TOML 食物条目。
3. **审计修复**——发现 consume_system 和 need_evaluation 共享 CommandBuffer 导致"先吃再判断"排序失效。修复：consume 独立 Block A3.5 立即 flush。

### 架构决策

**harvest 独立系统（非 movement 内）**：`movement_system` 到达后仅插入 `ArrivedAtTarget` 标记（ZST）。新 `harvest_on_arrival_system` 在 Block A2.5 读取标记→采集→入库。movement 签名不变，关注点分离。

**consume 独立 cmd**：Block A3.5 独立 CommandBuffer 立即 flush，确保 Block A4 的 `need_evaluation` 看到更新后 hunger。避免同帧"吃了还饿"的假 Desire。

**`satisfy_goal` 分化**：
- FindFood → harvest+consume（本冲刺改造）
- FindWater → 保留直接 `thirst -= 0.5`（MVP 简化）
- FindRest/FindSafePlace 等 → 保留原行为（Phase 3+）

## 📦 产物

**代码**（`woworld/`）：core(item+vegetation) + ecs(goal+movement+harvest+metabolism+registry) + godot(terrain_chunk) + assets(test_items.toml)。12 文件，+~400 行，+11 tests。

**文档**（`woworld-dev-plan/`）：DEVLOG、本 Handoff、更新 附录E。

## 💾 恢复点（下一会话 AI 必读）

> ⚠️ 下一会话按 `00-流程总览` **场景 B（冲刺执行）** 启动——V3a 已完成，下一步 **V3b 市场接真**。

- **当前阶段**: Phase 2 切片·`6/10`（V0+V1+V4a+Vf+V2+V3a ✅）·10 步序列见 [[../../02-垂直切片/README]] §3
- **下一步**: **V3b 市场接真**（第 7/10 步）——`order_creation` 读**真实** Needs/库存盈余（弃 seed-random）+ 统一 Wallet↔registry、item_holdings↔InventoryRegistry 双账。依赖：V3a ✅（代谢闭环·食物可被消费）
- **机械门状态**: `1111 tests 全绿`（core 401 + worldgen 75 + atmosphere 26 + ecs 600 + integration 9），clippy/fmt 零警告，build 通过。
- **提交状态**: ⚠️ **未提交**。基线 = `9367408`（Sprint-070）。本会话改动待提交。
- **A1 铁律**: 纯涌现，禁脚手架/假坐标/占位驱动/平行 trait/语音决策 silo。
- **关键新增类型**:
  - `ConsumableEffect { is_consumable, hunger_restore, hp_restore }` — woworld_core::item
  - `PlantYield { item_def_id, hunger_restore, hp_restore }` — woworld_core::vegetation
  - `ProductCategory::resolve_plant_yield() → Option<PlantYield>` — yield resolver
  - `ArrivedAtTarget { goal_type, target_pos }` — ecs component (goal.rs)
  - `harvest_on_arrival_system` — ecs system (harvest.rs)
  - `consume_system` — ecs system (metabolism.rs)
- **系统调度顺序**（关键）:
  ```
  A1: needs_decay
  A2: movement_system (FindFood→ArrivedAtTarget)
  A2.5: harvest_on_arrival (独立cmd, flush)
  A3: age_system
  A3.5: consume_system (独立cmd, flush)  ← ★ 必须在 need_evaluation 之前
  A4: need_evaluation → goal_resolution → action_weight → ...
  ```

## ⚠️ 遗留 / 诚实边界

- **HARVEST 复合原子未实现**：直接 `add_item()` 入库。标注 `V3a shortcut`。Phase 3 接入物理原子管线。
- **`ingest_food()` 仅 hunger+hp**：6 个 Vitals 字段仅实现 2 个。元素瓶颈/moisture/stamina/temp 全部 Phase 3。
- **FindWater/Rest/SafePlace 未改造**：仍走原 `satisfy_goal` 魔法。
- **消费阈值 0.5 硬编码**：`const HUNGER_CONSUME_THRESHOLD: f32 = 0.5`。
- **ProductCategory→ItemDefId 硬编码**：`resolve_plant_yield()` match 语句。Phase 3 迁移到 TOML 交互配方表。
- **设计-code 方向偏差**: `Needs.hunger: 0=满足→1=缺乏` vs 设计 `Vitals.hunger: 0=饿→1=饱`。全 ECS 一致使用代码方向。Phase 3 统一。

## 🔗 关联

- **计划**: `~/.claude/plans/fancy-moseying-stallman.md`（三轮审计终稿）
- **DEVLOG**: [[../devlogs/DEVLOG-2026-07-12-sprint071]]
- **上游**: [[handoff-20260712-sprint070]]（V2 牵引移动）
- **路线图**: [[../../02-垂直切片/README]] · [[../../附录E-开发状态]]
- **下游**: V3b 市场接真（order_creation 读真实 Needs/盈余·双账统一）
