# DEVLOG: 2026-07-10 — Sprint-062 ActionResolver

> **冲刺**: Sprint-062 — ActionResolver 与输入解析（角色控制器 004）
> **阶段**: Phase 2 — 垂直切片

## 今日目标
- [x] P1 输入契约：InputAction 枚举 + InputState + HotbarConfig
- [x] P2 action_resolver_system 六层映射
- [x] P3 域过滤 + 释放检测 + 最小 player_input_system
- [x] P4 上下文解析：resolve_interact_target + NearbyInteractables/ActionWheelData
- [x] P5 管线激活：Block A0 接线 + 端到端集成测试
- [x] P6 自检 + 交接

## 做了什么
- **woworld_core**：`input.rs` 扩展 InputAction(~40/5组·domain()/is_meta())/InputState(帧快照·was_pressed/released/is_held)/HotbarConfig/HeldItemKind；新 `interact.rs`（InteractKind 12级优先级 + Interactable + resolve_interact_target 纯仲裁）；`player.rs` 补 `ControlMode::controls_domain()`。
- **woworld_ecs**：
  - `action_resolver_system`（六层：1/3/5/6 实全，2/5 用 CHeldItem/CEquippedSkills stub，4 交 interact_context）
  - `player_input_system`（相机相对→世界空间 direction，pace/stance→desired_state）
  - `interact_context_system`（Interact 键 → resolve → 请求/轮盘）
  - `CHeldItem`/`CEquippedSkills` stub 组件；`NearbyInteractables`/`ActionWheelData` 资源壳
  - `ActionRegistry::id_of()`（名→ActionId）
- **woworld_godot**：Block A0 接入 4 新系统（player_input/action_resolver/interact_context + 既有）+ 5 字段（input_state/hotbar/nearby/wheel/game_time_secs）。
- **集成测试** `sprint062_actionresolver.rs`：3 端到端（移动/跳跃启动/Auto抑制）——证明 Block A0 激活。
- **文档**：修 CMoveIntent 注释（PlayerInputSystem 归属）；同步 001 §四 执行序订正注记（对齐 002 §六 D1）。
- **计数**：927 → **965 tests**（+38），clippy 零警告，零回归。

## 遇到的问题
- **CInputBuffer 无填充者**：现有 input_buffer_system 只 drain 不 fill → resolver 补填（时敏动作）。定位后管线自洽。
- **CMoveIntent.desired_state 无消费者**（发现）：pace 意图死写 → 用户裁决 M1=A 保持前瞻契约，direction 已够激活切片。
- **第五层与提案"全实现"不符**（自审发现）：原为 no-op → 加 CEquippedSkills stub 补齐，与第二层对称。
- **ControlMode 仅 Auto/Manual**：manual_domains 无法按 spec 直译 → controls_domain(Auto→∅/Manual→全) 极简实现，不引入 DomainDelegated。

## 学到的东西
- **数据驱动路由 > 硬编码分支**：`ActionDef.bufferable` 决定缓冲/即时，比在 resolver 里枚举"哪些动作可缓冲"干净得多。
- **stub 数据源 seam 模式**：Option<&CHeldItem>/Option<&CEquippedSkills>——未挂组件即优雅降级，真实系统零改动接线。比 trait 抽象轻。
- **纯函数进 core**：resolve_interact_target 无 ECS 依赖 → 放 core，7 个内存候选单测覆盖所有分支，无需构造 World。
- **绞杀者验证靠集成测试而非活实体**：延续上会话 ec33720 教训——Block A0 用 tests/ 镜像验证，不在活 WorldDriver 留浮空测试实体。

## 明日计划
- [ ] Godot input_bridge.gd（InputMap → InputState）——让键盘驱动 Block A0
- [ ] 或 NPC/玩家迁移到新管线
