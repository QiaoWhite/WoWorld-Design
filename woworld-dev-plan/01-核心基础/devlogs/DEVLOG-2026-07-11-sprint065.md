# DEVLOG: 2026-07-11 — Sprint-065 持续与充能动作运行时

> **冲刺**: Sprint-065 — 持续与充能动作运行时（角色控制器 006）
> **阶段**: Phase 2 — 垂直切片

## 今日目标
- [x] 相机 MVP 提交推送（前置·`94ea27e`）
- [x] Continuous 动作运行时 + SustainDrain 消耗 + SustainPhase 迁移 + 强制释放
- [x] Charge 动作运行时 + 充能阶梯 + follow-up 帧间接续 + overcharge 三分支
- [x] TOML block/aim_bow 示例 + A3(interrupt_on_move) + M4(coyote 字段) + 集成

## 做了什么

**核心发现（精读收益）**：006 的数据类型（ActionKind/SustainPhase/SustainDrain/ReleaseBehavior/ChargeStage/OverchargeBehavior + ActionDef 全字段）**在 Sprint-062 就已定义**，`ActionLifecycleEvent::ChargeTrigger` 与 `InterruptSource::{MoveInput, InputReleased, VitalDepleted}` 也已存在。缺的纯是**运行时接线**——`action_controller_tick:113` 硬门 `kind == Discrete` 静默丢弃持续/充能动作。整个冲刺是"激活已有类型"，非"新建"，也无需 `FinishReason`（用现成 Completed/Interrupted + InterruptSource）。

**P1**：`CPendingFollowUp(Option<ActionRequest>)`（帧间子动作载体）+ `CInputFeelConfig{coyote_time_secs}`（手感配置）两个纯数据组件。

**P2**：解除 Discrete 硬门——Continuous/Charge 走同一接受路径，`resource_drain_rate` 从 `sustain_drain` 初始化。

**P3/P4（架构关键）**：释放分发全部移入 wrapper（`action_system`）——它是**唯一同时握有 Vitals（强制释放）与 CPendingFollowUp（子动作）的层**。`action_controller_tick` 退化为纯 cancel+accept，释放逻辑抽成纯函数 `dispatch_release`（voluntary + forced 共用，可独立单测）。wrapper 新增 `update_sustain`（消耗 Vitals + SustainPhase 迁移 + overcharge 判定）。

**P5**：
- ActionId 加 `from_key`（const FNV-1a）+ 自定义双模 Deserialize（TOML 字符串键→hash / 整数→直用）。registry 的 `fnv_hash` 委托 `from_key`——单一 hash 源，保证 `action_id = "aimed_shot"` 解析出的 id 与 `[action.aimed_shot]` map-key id 一致（充能接续闭环的前提）。
- block 补 006 字段 + aim_bow(Charge/三阶梯/Penalize) + quick_shot/aimed_shot/full_draw 子动作入 TOML。
- A3：action_system query 补 `Option<&CMoveIntent>`，`interrupt_on_move` 动作遇移动输入→`Interrupted{MoveInput}`。
- M4：coyote_time_system 读 `CInputFeelConfig.coyote_time_secs` 替换硬编码 0.15。
- 集成测试 `sprint065_sustain.rs`：真实 TOML block → 请求接受 → sustain 消耗 → 松键 Complete，走 coyote+stamina+action 多系统管线。

## 遇到的问题
- **006 TOML 枚举标签格式**：006 文档写 `{ kind = "Charged", ... }`（内部标签），但代码 derive 的 serde 是外部标签（变体名作 key）。按 CLAUDE.md「伪代码阐明理念，实现可重构」用外部标签写 TOML，语义不变，TOML 注释说明。
- **ActionId 字符串反序列化**：`ActionId(pub u32)` 原 derive 是 newtype-over-u32，无法解析 `"quick_shot"`。→ 自定义 Visitor 接受 str（FNV）+ int（直用），零风险（审计确认 ActionId 无期望整数的 serde 用法）。
- **clippy `needless_option_as_deref`**：`Option<&mut T>::as_deref_mut()` 返回同类型属冗余。→ `vitals_opt` 只用一次直接移动；`follow_opt` 用三次改 `.as_mut()`（双重引用合法）。

## 设计吻合度审计（编码后·对照 006/003/005/008）
用户要求编码后逐条核验实现与原文档的吻合度/遗漏/增补。发现并**自主修复** 3 处吻合度缺口 + 1 处覆盖缺口：
- **取消窗口**（006 §〇）：持续/充能动作 Active 中取消窗口须"始终开放"——原代码 `cancel_window_open` 在无限 Active 保持 false，block 无法被 dodge 取消。→ Active 臂加 `active_s==0` 分支设窗口开 + 测试。
- **block movement_lock**（006 §三）：TOML 原 `Full`（不可动）→ 改 `Partial`（防御时可慢走）。
- **overcharge 失败原因**：`ContextInvalidated`（外部上下文）→ `SelfStateInvalidated`（自身力竭，更贴 006 语义）。
- **分支覆盖**：Trigger/AutoRelease/ForceCancel 三条实现了但无测试 → 补 4 测试。
- **遗留待用户裁决**（未擅改，已报告）：Q1 持续动作 Recovery 收尾（005 §六 vs 006 §〇 张力 + 复杂度，**用户裁决延后**）；Q2 combat_params（战斗域）；Q3 aim speed 0.5（Partial 硬编码限制）；Q4 Critical{forced_release_in} 单阈值解读；Q5 ActionId 双模 Deserialize（超设计·工程必需）；Q6 TOML 外部标签。**用户"支持建议"——Q1 延后，Q2-Q6 确认。**

## 会话终态
- 机械门：**1026 tests 全绿** · clippy 零警告 · fmt clean · build 成功。
- 提交：`3d5ec72`（Sprint-065 + 审计）→ `ababd79`（handoff 恢复点同步），均**已推送**。工作区干净。
- 相机 MVP `94ea27e` 为本会话前置产出（已推送）。

## 学到的东西
- **先读类型定义再动手**：本冲刺 80% 的"实现"是激活已存在但未接线的类型。若一上来就写新类型会大量重复。精读 `action.rs` 尾部（事件/中断源枚举）省下 FinishReason 等冗余设计。
- **释放分发的层归属**：纯函数 controller 无法访问 Vitals/组件——凡"需要资源判定 + 组件写入"的逻辑必须在 ECS wrapper，纯逻辑抽成 free function 保持可测。
- **单一 hash 源**：跨 crate 的字符串→id 映射，务必让所有路径委托同一函数（`ActionId::from_key`），否则 map-key 与引用 id 悄悄不一致。
- **编码后审计抓真缺口**：机械门全绿 ≠ 吻合设计。对照原文档逐条核验才发现"取消窗口/movement_lock"两处运行时行为与 006 不符——测试全过是因为测试没覆盖那些交互。

## 明日计划（下一会话）
- Sprint-065 已闭环推送。下一冲刺候选：🥇 I1-5 手感系统 mini-sprint（CInputFeelConfig 已就位可扩展）/ 🥈 玩家实体接入 Vitals+Block 键位使持续动作实机可玩 / A2 InterruptSource 语义（战斗冲刺）。
- 触及角色控制器先精读对应 006/008 原文档；I1-5 见 008 手感系统。
