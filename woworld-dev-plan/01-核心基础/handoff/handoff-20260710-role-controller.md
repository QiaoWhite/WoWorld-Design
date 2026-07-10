# Handoff — 2026-07-10（角色控制器 Sprint）

> **冲刺**: 角色控制器核心三层实现
> **日期**: 2026-07-10
> **阶段**: Phase 2 — 垂直切片
> **冲刺状态**: ✅ Step 5e 管线集成完成 + 全量审计修复 + A6/D1 裁决（927 tests）

## 📊 冲刺回顾

### 目标达成

| 目标 | 状态 | 备注 |
|------|------|------|
| 目标 1: woworld_core 核心类型 | ✅ | 4 新文件: kinematics/movement/action/input — 30+ 类型 |
| 目标 2: ECS Component + System | ✅ | 8 Component + 3 Resource + EventChannel + 7 System |
| 目标 3: TOML 资产 | ✅ | 3 .toml 文件 |

### 关键决策

- **绞杀者模式**：旧 `systems/npc/movement.rs` 零改动，新 System 通过 `Without<Movement>` 互斥
- MovementRecoveryStack 用固定 `[MovementState; 3] + u8`（零依赖 core）
- `ActionLifecycleEvent`/`MovementLock`/`RotationLock` 放 woworld_core（值类型多 crate 共享）
- `can_interrupt()` 加入 CommitmentLevel 门控（Hard/Locked）
- CoyoteTimeSystem 必须在 MovementModeSystem **之前**运行
- 取消窗口改为基于 `cancel_window_ms` 的计时窗口
- `MovementLockDef` 补全 Free/Partial/Full/Override

## 💾 恢复点（下一会话 AI 必读）

> ⚠️ 这是最重要的段落。下一会话 AI 启动时首先读这里。

- **当前冲刺**: 角色控制器 — **核心三层 + Step 5e 管线集成 + 全量审计 + A6/D1 全部完成，已提交推送**
- **最后操作**: 移除 Step 5e 冒烟测试实体（保留 Block A0 管线）→ commit `ec33720` → push origin/master
- **机械门状态**: build ✅ / test ✅ (927 passed: 358 core + 484 ecs + 58 worldgen + 26 atmosphere + 1 集成) / clippy ✅ 零告警
- **git 状态**: 本会话 3 提交全部推送到 origin/master — `745e51f`(角色控制器特性) → `72742be`(杂项) → `ec33720`(移除冒烟实体)。工作树干净（仅 .obsidian 编辑器状态持续抖动）
- **下一步（精确）**: Step 5e 集成 + 审计 + A6/D1 + 冒烟实体移除 **全部完成并推送**。Block A0 管线就位但**处休眠**（当前无实体挂新组件，绞杀者 no-op）——待下列工作把真实实体接上即激活。候选（按优先级）:
  1. 🥇 **ActionResolver sprint**（004）—— InputAction 枚举 + 六层映射 + 动作轮盘 + ControlMode 域过滤；接通玩家按键→ActionRequest，让 Block A0 真正跑起来
  2. 🥈 **NPC/玩家迁移**—— 给实体挂 CMovementState/CMoveIntent 等新组件，从旧 npc::movement 迁到新 movement_system
  3. **Continuous/Charge 运行时**（006）—— 让 block/aim_bow 可激活
  4. 修延后项 A2/A3/M3/M4/I1-I5（见「已知问题」表）
- **已知陷阱**:
  - ⚠️ CoyoteTimeSystem **必须**在 MovementModeSystem 之前运行——否则 CPrevMovementState 已被覆盖
  - ⚠️ 新 MovementSystem query 必须带 `Without<Movement>`——否则会碰到旧 NPC 的 Movement Component
  - ⚠️ 旧 NPC 没有任何新 Component——MovementSystem 对它们不可见。这是绞杀者模式的预期行为
  - ⚠️ `compute_locomotion_mode` 在 4 个文件中重复定义——不要提取到公共模块，CHG-067 会统一解决
  - ⚠️ `ActionRegistry::load_from_toml()` 用 FNV hash 将 TOML key → ActionId。修改 TOML key 名会改变 ActionId
- **待用户确认**: 无（A6/D1 已裁决执行）

## 🔧 机械门验证

```
cargo build --workspace  ✅
cargo test --workspace   ✅ 927 passed (358 core + 484 ecs + 58 worldgen + 26 atmosphere + 1 集成)
cargo clippy --workspace ✅ 零警告
```

## ⚠️ 已知问题

| # | 问题 | 级别 | 状态/计划 |
|---|------|------|----------|
| 1 | ECS 管线集成 | — | ✅ Step 5e 完成（Block A0，已推送）|
| 2 | Block A0 管线休眠（无实体挂新组件）| — | 预期——绞杀者 no-op，待 ActionResolver/NPC 迁移激活。冒烟实体验证后已删 |
| A2 | InterruptSource 硬编码语义失真（System→全 Staggered，Instinct→全 DodgeCancel） | 🟡 | 战斗/ActionResolver sprint |
| A3 | interrupt_on_move 存而不用（interact 应被移动打断） | 🟢 | ActionResolver sprint |
| A7 | Active→Recovery 切换帧 cancel_window 误开 1 帧 | 🟢 | 随动作系统完善 |
| M3 | 滑翔 glide 速度/加速度字段未接线 | 🟢 | 滑翔/垂直移动 sprint |
| M4 | coyote_time_secs 字段缺失（硬编码 0.15） | 🟢 | 手感系统接线 |
| I1-5 | 缓冲淘汰 / 消费 pop_if 物理重检 / 落地预输入 / 边缘吸附 / 空闲门控 | 🟡 | 004 ActionResolver 手感 |

> ✅ 已修（Step 5e 同期，927 tests）: windup=0 卡死、cancel_set 永不匹配 🔴、movement_lock 泄漏、rotation_lock 丢弃、Crouching×0.5、Treading 体力语义、ledge_snap 键名、3 TOML 解析测试、**A6 EMERGENCY 穿透 Locked**、**D1 文档 002 §六 顺序订正(已 sync)**。详见 [[../devlogs/DEVLOG-2026-07-10-role-controller|DEVLOG §七]]。

## 🚀 下一步候选

| 候选 | 依赖前提 | 优先级 |
|------|---------|--------|
| **ActionResolver sprint（004）** | Block A0 已就位 | 🥇 立即 |
| NPC/玩家迁移到新 MovementSystem | Block A0 已就位 | 🥈 |
| Continuous/Charge 运行时（006）| ActionResolver | 🥉 |
