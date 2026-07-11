# Handoff: 2026-07-12 — Sprint-067 V0+V1 地基与昼夜涌现

> **会话类型**: 冲刺执行（场景 B）
> **日期**: 2026-07-12
> **阶段**: Phase 2 探针切片「活着的村庄」·进度 `▓▓░░░░░░░░ 2/10`
> **状态**: ✅ Sprint-067 完成·1055 tests 全绿·clippy/fmt 零警告·**未提交**

## 📊 本会话做了什么

执行 Sprint-067（V0+V1 合并）。**先对着设计文档逐条审计规划**（撤销一处假冲突 + 修 R2/R4/F1/F5 + 用户裁决 R1），再编码：

1. **V0 库存**：审计发现 inventory 已由 `inventory_init_system` 幂等补挂 → 降级为验证测试（不改 `spawn_npc`）。
2. **V1 昼夜**：`time_modifier`（设计 ver2.0 v3「昼夜修正」）作 `action_weight` 第 6 因子（纯世界时·白昼度曲线）+ 漫游回落（读 Needs 紧迫度防振荡）。
3. **Vf spike**：食物源落地 BACKLOG 草案（路线 B·Bridson 参考·不写实现代码）。

## 📦 产物

**代码**（`woworld/`）：
- `crates/woworld_ecs/src/systems/npc/action_weight.rs` — `time_modifier` + 第 6 因子 + 4 测试
- `crates/woworld_ecs/src/systems/npc/needs.rs` — 抽 `evaluate_top_urgency` + `pub URGENCY_THRESHOLD`
- `crates/woworld_ecs/src/systems/npc/goal.rs` — 漫游回落 + 3 测试
- `crates/woworld_godot/src/terrain_chunk.rs:2190` — 传 `day_progress`
- `crates/woworld_ecs/src/systems/item/inventory.rs` — V0 读写闭环测试

**文档**（`woworld-dev-plan/`）：新建 `sprint-proposals/BACKLOG-Vf-食物源落地-20260712.md`；更新 sprint-067 提案、`02-垂直切片/README` §4+进度、`附录E`、本 DEVLOG。

## 💾 恢复点（下一会话 AI 必读）

> ⚠️ 下一会话按 `00-流程总览` **场景 B（冲刺执行）** 启动——V0+V1 已完成，下一步 V4a。

- **当前阶段**: Phase 2 切片「活着的村庄」·`2/10`（V0+V1 ✅）·10 步序列见 [[../../02-垂直切片/README]] §3
- **下一步**: **V4a 问候/情绪气泡**（第 3/10 步）——桩串外移 TOML 片段表（按 `SpeechAct`）+ 3m 邻近触发（复用 `systems/npc/social.rs`）+ 对齐 `DialogueIntentType`；气泡走 `UtteranceId` 瞬时车道、**不碰 `ExpressionRef`**。执行前逐字精读 `开发阶段/语言表达/` + 复核 §4 现状校准。
- **机械门状态**: `1055 tests 全绿`（core 398 + worldgen 58 + atmosphere 26 + ecs 573 + godot 0），clippy/fmt 零警告，build 通过（DLL 已更新）。
- **未提交/未推送**: 本会话所有改动（`woworld/` 代码 + `woworld-dev-plan/` 文档）**均未提交**。承接基线 `1db25a9`。可选：提交本冲刺（代码 + 治理文档）。
- **A1 铁律**: 纯涌现，涌现不出的如实呈现空白，禁脚手架/假坐标/占位驱动/平行 trait。

## ⚠️ 遗留 / 诚实边界（探针前哨）

- **V1 单意图架构限制**：`time_modifier` 只缩放当前 Goal 单一 intent 的权重；整村肉眼"夜里睡"依赖 **V3a 代谢闭环**（第 6/10 步）才完全显现。V1 独立可见成果偏薄但真实。
- **Vf 是独立中等 worldgen 冲刺**（Poisson P2.25），非接线——BACKLOG 已 honestly-scoped，V2/V3a 前置。
- **NPC 旧 `Movement` vs 玩家新 CC 管线**两套移动——债，留玩家化身切片评估合并。
- **代码-设计分歧**：`HarvestableInfo{instance_id:u64, 无 regen_state}` 与 `生命/010` 不一致（记入 Vf BACKLOG 开放问题）。

## 🔗 关联

- **上游**: [[handoff-20260712-planning]]（规划会话·10 步序列定稿）
- **DEVLOG**: [[../devlogs/DEVLOG-2026-07-12-sprint067]]
- **提案**: [[../../sprint-proposals/sprint-067-V0V1-地基与昼夜涌现-20260712]] · [[../../sprint-proposals/BACKLOG-Vf-食物源落地-20260712]]
- **路线图**: [[../../02-垂直切片/README]] · [[../../附录E-开发状态]]
