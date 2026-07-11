# Handoff: 2026-07-11 — Sprint-065 持续与充能动作运行时

> **冲刺**: Sprint-065 — 持续与充能动作运行时（角色控制器 006）
> **日期**: 2026-07-11
> **阶段**: Phase 2 — 垂直切片
> **冲刺状态**: ✅ 完成（P1-P6 达成 + 设计吻合度审计，1026 tests，clippy 零警告，**已提交推送 3d5ec72**）

## 📊 冲刺回顾

### 目标达成
| 目标 | 状态 | 备注 |
|------|------|------|
| 1: Continuous 运行时 + SustainDrain 消耗 | ✅ | 无限 Active + 消耗 Vitals + SustainPhase(Normal/Overextended/Critical) + 资源耗尽/过久强制释放 + Complete/Trigger |
| 2: Charge 运行时 + 阶梯 + follow-up 接续 | ✅ | 充能时长选阶梯 + CPendingFollowUp 帧间注入 + Overcharge(AutoRelease/Penalize/ForceCancel) |
| 3: TOML 示例 + A3 + M4 + 集成 | ✅ | block/aim_bow+子动作入 TOML + interrupt_on_move + coyote 字段 + 端到端集成测试 |

### 关键决策
- **释放分发归 wrapper**：`action_controller_tick` 移除释放分支，退化为纯 cancel+accept；释放（voluntary+forced）逻辑抽成纯函数 `dispatch_release`，由 `action_system`（唯一握有 Vitals+CPendingFollowUp 的层）调用。
- **无需 FinishReason**：正常松键→`Completed`，强制结束→`Interrupted{VitalDepleted}`，用现成类型。
- **ActionId 双模 Deserialize**：字符串键→FNV hash（`from_key`）/整数→直用。registry `fnv_hash` 委托 `from_key`——单一 hash 源，保证充能子动作引用与 map-key 一致。
- **006 TOML 用外部标签**（serde 默认），非文档示例的内部标签 `{kind=...}`——数据表示按实现，语义不变。
- **不改 Godot 玩家 spawn**：action_system 用 Optional query 项（Vitals/CPendingFollowUp/CMoveIntent），无这些组件的实体走缺省路径零回归。玩家实体接 Vitals+Block 键位延后到 input/战斗冲刺（block/aim_bow 尚无键位绑定，不可触发）。

## 💾 恢复点（下一会话 AI 必读）

> ⚠️ 下一会话 AI 启动首先读这里。

- **本会话产出**：① 相机 MVP 提交推送（`94ea27e`，前置）；② Sprint-065 持续/充能动作运行时完整实现 + 设计吻合度审计（`3d5ec72`，已推送）。
- **当前冲刺**: Sprint-065 — 已完成（P1-P6 + 审计），**已提交推送 `3d5ec72`**。
- **机械门状态**: build ✅ / test ✅ **1026 passed** / clippy ✅ 零警告 / fmt ✅ clean
- **上次提交**: `3d5ec72`（Sprint-065，已推送）。工作区干净（无未提交）。
- **未提交文件**:
  ```
   M woworld/assets/action_registry.toml            (block 补 006 字段 + aim_bow + 3 子动作)
   M woworld/crates/woworld_core/src/action.rs       (ActionId::from_key + 双模 Deserialize + 测试)
   M woworld/crates/woworld_ecs/src/components/action_state.rs   (CPendingFollowUp)
   M woworld/crates/woworld_ecs/src/components/input_state.rs    (CInputFeelConfig)
   M woworld/crates/woworld_ecs/src/resources/action_registry.rs (fnv_hash→from_key + 测试)
   M woworld/crates/woworld_ecs/src/systems/action/action_controller.rs (移释放分支 + dispatch_release)
   M woworld/crates/woworld_ecs/src/systems/action/action_system.rs     (sustain 运行时 + A3)
   M woworld/crates/woworld_ecs/src/systems/input/coyote_time_system.rs  (M4 coyote 字段)
   ?? woworld/crates/woworld_ecs/tests/sprint065_sustain.rs      (集成测试)
   M woworld-dev-plan/附录A-术语表.md / 附录E-开发状态.md
   ?? sprint-065 提案 / DEVLOG-2026-07-11-sprint065 / 本文件
   # 附带（fmt 归一化，非本冲刺逻辑）:
   M camera.rs / character_facing_system.rs / movement_mode_system.rs / possess.rs / terrain_chunk.rs
  ```
- **下一步**: 提交推送 Sprint-065 → 待用户审核 → 下一冲刺候选（见 §🚀）。
- **已知陷阱**:
  - ⚠️ Vitals 现由 `action_system`（sustain drain）+ regen + age **顺序写**（不同 Block，非并行）——铁律 #10 的文档化例外。新增写 Vitals 的 System 若进 par_iter 需重新审计。
  - ⚠️ block/aim_bow 在 TOML 就位但**无键位绑定**，实机不可触发；玩家实体**无 Vitals**，block 不消耗——这些是 input/战斗冲刺的接入项，非缺陷。
  - ⚠️ Mana-sustain 动作不消耗（Vitals 无 mana 字段·魔法冻结）——`drain_vitals` 中 Mana→false。

## 🔧 机械门验证

### cargo test（真实输出）
```
TOTAL PASSED: 1026   (相机 MVP 已提交部分不计；本冲刺净 +21 运行时/组件 +4 审计修复覆盖)
```
（core 393 + worldgen 58 + atmosphere 26 + ecs 548 + 集成 1 + godot 0 = 1026）

### cargo clippy
```
Finished `dev` profile ... CLIPPY EXIT 0（零警告）
```

### cargo fmt --check
```
FMT CLEAN
```

### cargo build --workspace
```
Finished `dev` profile [unoptimized + debuginfo] target(s)（.dll 已更新）
```

## 📐 设计门验证（15 项）

### A. 主清单
| # | 检查项 | 状态 |
|---|--------|------|
| 1 | trait 签名与 CLAUDE-INTERFACES.md 一致 | ✅ 未改任何跨模块 trait；新增均为函数/组件 |
| 2 | ID 类型定义在 woworld_core | ✅ `ActionId::from_key` 加在 woworld_core，非消费 crate |
| 3 | 无 Godot/GDScript 游戏逻辑 | ✅ 未触碰 Godot/GDScript |
| 4 | 公开类型登记术语表 | ✅ 附录A 追加 11 条（ActionKind/ReleaseBehavior/.../CPendingFollowUp/CInputFeelConfig 等） |
| 5 | 架构决策记录 | 🟡 释放分发归 wrapper + ActionId 双模 hash 记于本 handoff §关键决策（未单独立 ADR，属实现级） |

### B. ECS 铁律
| # | 检查项 | 状态 |
|---|--------|------|
| 6 | Component 纯数据零方法 | ✅ CPendingFollowUp/CInputFeelConfig 纯数据 |
| 7 | 无堆数据内联 | ✅ CPendingFollowUp(Option<ActionRequest>)——ActionRequest 全栈上，无 Vec/String |
| 8 | Component 'static+Send+Sync | ✅ 纯值类型 |
| 9 | Entity 删除标记+统一清理 | ✅ 仅置 active_action.0=None，不迭代中 despawn |
| 10 | System writes 无交集（并行） | ✅ Vitals 由 action_system/regen/age **顺序**写（不同 Block 非并行），文档化例外 |
| 11 | hecs::World 仅在 WorldDriver | ✅ 未泄漏 |
| 12 | 每 System 至少 1 测试 | ✅ action_system 10 测试 / coyote M4 测试 |

### C. 架构边界
| # | 检查项 | 状态 |
|---|--------|------|
| 13 | GDScript 无独立模拟参数 | ✅ 未触碰 |
| 14 | #[func] 有调用 | ✅ 未增 #[func] |
| 15 | GDScript 无数学公式 | ✅ 未触碰 |

## ⚠️ 已知问题
| # | 问题 | 级别 | 计划 |
|---|------|------|------|
| 1 | 玩家实体无 Vitals + block/aim_bow 无键位 → 实机不可触发 | 🔵 结构性 | input/战斗冲刺接入 bundle（Vitals+CPendingFollowUp+键位） |
| 2 | Mana-sustain 不消耗（Vitals 无 mana） | 🔵 结构性 | 魔法解冻 / Vitals 补 mana 字段 |
| 3 | Penalize overcharge 的 accuracy_loss 仅数值不建模 | 🟢 参数 | 战斗/瞄准冲刺（准星表现） |
| 4 | 相机会话 5 文件 fmt 归一化（非本冲刺逻辑） | 🟢 | 随本冲刺提交 |

## 🔍 设计吻合度审计（2026-07-11·对照 006/003/005/008）

**已修复（吻合度缺口）**：
- **取消窗口**（006 §〇）：持续/充能动作 Active 中取消窗口须始终开放——原代码 `cancel_window_open` 在无限 Active 保持 false，block 无法被 dodge 取消。→ 修：Active 臂 `active_s==0` 分支设 `cancel_window_open = cancel_window_ms>0` + 测试 `test_held_block_cancellable_by_dodge`。
- **block movement_lock**（006 §三）：TOML 原 `"Full"`（不可动），006 规定 `"Partial"`（防御时可慢走）。→ 改为 Partial。
- **ForceCancel 失败原因**：原 `ContextInvalidated`（外部上下文），改 `SelfStateInvalidated`（自身力竭，更贴 overcharge 语义）。
- **分支覆盖**：补 Trigger 释放 / AutoRelease / ForceCancel 三条此前无测试的路径（+4 测试→1026）。

**待用户确认（需设计裁决，未擅改）**：
- ⚠️ **Q1 持续动作 Recovery**：006 §〇 表列持续动作 Recovery="有（松键后短暂收尾）"，block recovery_ms=100；但当前松键→立即 Completed，**跳过 Recovery**。005 §六 又显示松键直接 Completed（无 Recovery 步）——两文档张力。修复需重置 recovery 计时基线（elapsed 在无限 Active 已累积），非平凡。block 尚不可触发，零现时影响。**建议延后到 block 绑键时**。
- ⚠️ **Q2 combat_params 遗漏**：006 §三 block 含 stability/damage_reduction——属战斗域数据（ActionDef 无此字段），故未纳入。确认属战斗冲刺范畴。
- ⚠️ **Q3 aim_bow movement_lock_speed=0.5**：006 §四 要求半速慢走，但 `MovementLockDef::Partial` 的 speed_cap 硬编码 1.0（Sprint-1 TOML 限制，非本冲刺引入）。确认延后到 Partial speed 可配置。
- ⚠️ **Q4 SustainPhase::Critical{forced_release_in} 恒 0.0**：数据仅一个 critical 阈值，无"进入 Critical→倒计时→强制"的两段窗口，故到阈值即强制（forced_release_in=0）。确认此解读。
- ⚠️ **Q5 ActionId 双模 Deserialize（超设计增补）**：006 用字符串键 `action_id="quick_shot"`，但 ActionId 是 u32——新增 `from_key`(FNV) + str/int Visitor 是工程必需。确认接受此增补。
- ⚠️ **Q6 006 TOML 外部标签**：`[action.aim_bow.release_behavior.Charged]` 用 serde 默认外部标签，非 006 示例的内部标签 `{kind="Charged"}`。确认接受（TOML 已注释）。

## 🚀 下一步候选
| 候选 | 依赖前提 | 优先级 |
|------|---------|--------|
| A: I1-5 手感系统 mini-sprint（缓冲淘汰/pop_if 物理重检/落地预输入/边缘吸附/空闲门控） | CInputFeelConfig 已就位可扩展 | 🥇 |
| B: 玩家实体接入 Vitals + Block 键位（使持续动作实机可玩） | input_bridge InputMap | 🥈 |
| C: A2 InterruptSource 语义（System→非全 Staggered / Instinct→非全 DodgeCancel） | 战斗系统中断上下文 | 🥉 |
| D: M3 滑翔 glide 字段接线 | 垂直移动子系统 | — |

**建议**: 🥇 A（I1-5 手感）——`CInputFeelConfig` 已建，手感项是自洽 mini-sprint，可复用本冲刺组件；或 🥈 B 让防御/瞄准实机可玩（更有体感回报但依赖 input 绑定）。
