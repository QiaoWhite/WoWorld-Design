# DEVLOG 2026-07-12 — Sprint-067: V0+V1 地基与昼夜涌现

> **冲刺**: Sprint-067（垂直切片「活着的村庄」第 1-2/10 步·V0+V1 合并）
> **日期**: 2026-07-12
> **基线**: `1db25a9`（1047 tests）→ **终态 1055 tests 全绿**
> **状态**: ✅ 完成（clippy/fmt 零警告·**未提交**）
> **A1 纪律**: 纯涌现·零脚手架·只实现设计已规定的真实路径

## 一句话

村庄切片进度 `0/10 → 2/10`：V0 库存坐实（验证测试）+ V1 昼夜涌现（`time_modifier` 第 6 因子 + 漫游回落）+ Vf 食物源前置调研，全程守住"调制非门控"哲学红线。

## 编码前审计（计划阶段·撤销一处假冲突）

对照设计文档逐条审计初版计划，修正 7 项、遗留 1 项待用户裁决：

- **假冲突（撤销）**：曾误判"circadian 进 action_weight 违 A1（设计只在需求层）"。核 `NPC活人感开发文档ver2.0.md:1643` 发现权威权重链本含 `* time_modifier(&action, world) // v3: 昼夜修正`——**是设计已规定路径**，只是提前到 Phase-1 子集。与需求层 circadian（§3.4，调 hunger/fatigue urgency，已在 needs.rs）是两个独立因子，不重复计数。
- **R2**：原计划"复用 `circadian_factor` 正弦"错——那波峰值在**日出(0.25)**、午夜中性（代码注释 line 33 误标"正午"），是需求层曲线。action 层改用白昼度 `-cos(dp·TAU)`。
- **R4**：原"无 Desire→Idle"会**每帧振荡**（Desire 被 goal_resolution 即时消费，持续紧迫的 NPC 也有"当帧无 Desire"窗口）→ 改为读 `Needs` 紧迫度判据。
- **F1**：修正断裂文档引用（`03-基本需求系统/004-决策器集成方案.md` 等）。
- **F5**：V0"空库存"不准——`inventory_init_system` 会 `seed_npc_items`。
- **R1（用户裁决）**：Action 层因子遵设计 `time_modifier`——**纯世界时、不含 chronotype**。

## 交付

### 目标 1（V0）库存地基——只加验证测试
审计发现 inventory **已由 `inventory_init_system`（`terrain_chunk.rs:2203`）每帧幂等补挂**（当初"缺 inventory"漏看它）→ 不改 `spawn_npc`。加 1 测试坐实 spawn 形状实体 → 持有 + 读写闭环（`add_item`→`count_item` 读回增量）。

### 目标 2（V1）昼夜涌现
- **`time_modifier(category, day_progress)`**（`action_weight.rs`）：设计 ver2.0 v3「昼夜修正」提前实现。白昼度 `-cos(dp·TAU)`：Rest 夜偏好、Socialize/Explore 日偏好、进食/生存中性。返回 `1.0 ± 0.08`。**连续调制、非门控、不碰 `TimeOfDay::from_progress`**。
- **第 6 乘性因子**：`action_weight_system` 签名加 `day_progress: Option<f32>`，乘链补 `* time_w`（缺省中性 1.0）；`terrain_chunk.rs:2190` 传 `self.clock.day_progress()`。
- **漫游回落（R4）**：`goal_resolution_system` 加第二遍——无 Desire 且当前 Goal 非 Idle 且**最高需求紧迫度 < 阈值**时回落 `Goal::Idle`（修 Goal sticky）。为防振荡，判据读 `Needs`，抽 `needs.rs::evaluate_top_urgency` + 公开 `URGENCY_THRESHOLD` 共用（行为不变重构）。

### 目标 3（Vf spike）
`sprint-proposals/BACKLOG-Vf-食物源落地-20260712.md`：建议路线 B（worldgen 内扩展 `BiomeVegetation`，填 `query_harvestable` stub），路线 A（新建 `woworld_vegetation` crate + 完整 P2.25）拆独立 backlog（前提缺 `woworld_life`）。research-first：Bridson 2007（URL 留存）。复用点/开放问题/代码-设计分歧齐备。**未写任何植被实现代码。**

## 涉及文件

| 文件 | 改动 |
|------|------|
| `woworld_ecs/systems/npc/action_weight.rs` | `time_modifier` fn + 第 6 因子 + 签名 + 4 测试 + 头注更新 |
| `woworld_ecs/systems/npc/needs.rs` | 抽 `evaluate_top_urgency` + 公开 `URGENCY_THRESHOLD`（重构，行为不变） |
| `woworld_ecs/systems/npc/goal.rs` | 漫游回落第二遍 + 3 测试 |
| `woworld_godot/terrain_chunk.rs:2190` | `action_weight_system(..., day_progress)` |
| `woworld_ecs/systems/item/inventory.rs` | V0 spawn→持有→读写闭环测试 |
| `sprint-proposals/BACKLOG-Vf-...md` | 新建 Vf 草案 |

## 自检门（真实输出）

```
cargo build --workspace        → Finished（DLL 已更新）
cargo test --workspace         → 1055 passed; 0 failed（1047 基线 + 8 新增）
cargo clippy --workspace -D warnings → 零警告
cargo fmt --all --check        → 无差异（exit 0）
```
- **A1**：无脚手架/假坐标/占位驱动/平行 trait。
- **哲学反检**：grep「schedule/gate/日程/门控」仅命中"非门控/非日程"反模式声明；`time_modifier` 连续、只吃 `day_progress`、无查表。

## 诚实边界（探针结论前哨）

- V1 是 **Phase-1 单意图架构**（`action_weight` 给当前 Goal 派生的单一类别算权重）——`time_modifier` 缩放该单一 intent 权重。可测"给定 Rest Goal 权重夜>日"；但整村肉眼"夜里睡"依赖 **V3a 代谢闭环**（fatigue 累积→阈值 Desire→Rest→恢复）才完全显现。薄而真。
- V6 存档、Vf 植被、技能仍零代码（切片 §2.1 有意识跳过）。

## 下一步

- **V4a 问候/情绪气泡**（第 3/10 步）：TOML 片段表 + 3m 邻近（复用 `social.rs`）+ 对齐 `DialogueIntentType`。
- 可选：提交本冲刺（代码 + 治理文档）。

> **关联**: [[../handoff/handoff-20260712-sprint067]] · [[../../02-垂直切片/README]] · [[../../sprint-proposals/sprint-067-V0V1-地基与昼夜涌现-20260712]] · [[../../sprint-proposals/BACKLOG-Vf-食物源落地-20260712]]
