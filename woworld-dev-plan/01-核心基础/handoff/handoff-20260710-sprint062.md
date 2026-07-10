# Handoff: 2026-07-10 — Sprint-062 ActionResolver

> **冲刺**: Sprint-062 — ActionResolver 与输入解析（角色控制器 004）
> **日期**: 2026-07-10
> **阶段**: Phase 2 — 垂直切片
> **冲刺状态**: ✅ 完成（P1-P6 全部达成，965 tests，Block A0 激活）

## 📊 冲刺回顾

### 目标达成
| 目标 | 状态 | 备注 |
|------|------|------|
| 目标 1: 输入契约 + 意图生成 | ✅ | InputAction(~40)/InputState/HotbarConfig + player_input_system |
| 目标 2: 六层映射 + 域过滤 + 释放 | ✅ | action_resolver_system 六层全实现（2/5 用 stub 数据源）+ controls_domain |
| 目标 3: 上下文解析 + 管线激活 | ✅ | resolve_interact_target + ActionWheelData + Block A0 接线 + 3 集成测试 |

### 关键决策
- **数据驱动缓冲/即时路由**：`ActionDef.bufferable` → 时敏入 `CInputBuffer`（下一帧 input_buffer drain），即时直写 `CActionRequestBuf`。零硬编码。
- **域过滤极简**：`ControlMode::controls_domain()` = Auto→∅ / Manual→全6域（不引入 DomainDelegated，属玩家系统 Phase 2）。
- **Movement 域不经 resolver**：`player_input_system` 直写 `CMoveIntent.direction`（相机相对→世界空间），resolver 只管离散动作（004 §五）。
- **stub 数据源 seam**：第二层 `CHeldItem`（装备）、第五层 `CEquippedSkills`（技能）——未挂组件优雅降级，真实系统建好后零改动接线。
- **`desired_state` 前瞻契约**（用户裁决 M1=A）：pace/stance 意图写入但当前无消费者，`direction` 有真实位移效果。pace 生效待 MovementModeSystem 扩展。

## 💾 恢复点（下一会话 AI 必读）

> ⚠️ 下一会话 AI 启动首先读这里。

- **当前冲刺**: Sprint-062 ActionResolver — **P1-P6 全部完成**，未提交
- **最后操作**: P6 收尾——修 CMoveIntent 注释（movement_state.rs:23）+ 同步 001 §四 执行序订正注记 + 本 handoff
- **机械门状态**: build ✅ / test ✅ (965 passed: core 374 + ecs 507 + worldgen 58 + atmosphere 26) / clippy ✅ (规范命令零告警) / fmt ✅
- **git 状态**: **未提交**（工作树含本冲刺全部改动）。待用户确认后提交。
- **Block A0 状态**: **已激活**——`sprint062_actionresolver.rs` 3 集成测试证明 InputAction→ActionRequest→ActionController 端到端跑通（玩家输入移动实体 + 跳跃启动动作 + Auto 模式抑制）。真实游戏中当前无实体挂新组件——仍休眠 no-op，待夺舍迁移/桥接接上。
- **下一步（精确，按优先级）**:
  1. 🥇 **Godot input_bridge.gd**——InputMap → InputState（每帧 begin_frame + press/release + move_direction/camera_transform 填充）→ 让键盘真正驱动 Block A0。这是本冲刺明确留下的下一步。
  2. 🥈 **NPC/玩家迁移**——夺舍时给被控实体挂 CMoveIntent/CInputBuffer/CActionRequestBuf/CActiveAction/ControlModeComponent 等，从旧 npc::movement 迁到新管线。
  3. **pace 消费者**（M1 follow-up）——扩展 MovementModeSystem 读 `CMoveIntent.desired_state`，让 Sprint/蹲行真实影响速度。
  4. **装备/感官系统接线**——填充 CHeldItem/CEquippedSkills/NearbyInteractables 真实数据。
  5. Continuous/Charge 运行时（006）——让 Block 防御/aim_bow 可激活。
- **已知陷阱**:
  - ⚠️ Block A0 执行序：`player_input → coyote → stamina → movement_mode → input_buffer → action_resolver → interact_context → action(+flush) → movement`。action_resolver **必须在** input_buffer 后（001 Phase 0 序）——即时直写、时敏入缓冲下一帧 drain（1 帧跳跃缓冲延迟，预期行为）。
  - ⚠️ `CActionRequestBuf` 被 input_buffer/action_resolver/interact_context/action **四系统顺序写**——Block A0 非并行，无竞争。设计门 #10 顺序写例外（已文档化）。
  - ⚠️ `player_input_system` 在 Block A0 最先——它只写 `CMoveIntent`，不碰 CPrevMovementState，与 coyote 无序依赖。
  - ⚠️ `ActionRegistry::id_of(key)` 用 FNV hash——resolver 靠动作名字符串（"jump"/"dodge"/...）映射 ActionId。改 TOML key 名会改 ActionId。
  - ⚠️ 第二层 `aim_draw`/`parry`、无 TOML def → ActionController Failed 拒绝（弓/招架暂无效），待战斗/射箭 sprint 补。
- **待用户确认**:
  - 设计文档 001 已修改（开发阶段/）→ 需询问是否跑 `/woworldidea-design sync`（本次为模块内执行序注记，无跨模块接口 delta——sync 实质 no-op，但按规则须问）。
  - 是否提交本冲刺改动。

## 🔧 机械门验证

```
cargo build --workspace  ✅ Finished
cargo test --workspace   ✅ 965 passed (core 374 + ecs 507 + worldgen 58 + atmosphere 26 + 集成)
cargo clippy --workspace -- -D warnings  ✅ 零警告
cargo fmt --all --check  ✅ 干净
```
> ⚠️ `cargo clippy --workspace --all-targets` 报 npc/movement.rs·npc/social.rs 的**既有** test-code lint（本冲刺未触碰这两文件）——非本冲刺回归，规范门（无 --all-targets）零警告。

## 📐 设计门（关键项）

| # | 检查 | 状态 |
|---|------|------|
| trait/ID 签名一致 | InputAction/InputState/Interactable/HeldItemKind 等值类型入 woworld_core | ✅ |
| ID 类型不在消费 crate 重复 | ActionId 仍 core 唯一，id_of() 只是 hash 包装 | ✅ |
| Godot 侧无游戏逻辑 | terrain_chunk 仅编排系统调用 + 资源字段，无公式 | ✅ |
| Component 纯数据 | CHeldItem/CEquippedSkills 纯数据（get() 是查询非逻辑）| ✅ |
| 每 System ≥1 测试 | player_input(6)/action_resolver(10)/interact_context(3) | ✅ |
| System writes 交集 | CActionRequestBuf 多系统**顺序**写（非并行）——例外已文档化 | ✅ |
| hecs::World 不泄漏 Godot | 仅 WorldDriver 持有 | ✅ |

## ⚠️ 已知问题

| # | 问题 | 级别 | 计划 |
|---|------|------|------|
| 1 | `CMoveIntent.desired_state` 无消费者——pace/stance 意图死写 | 🟡 | M1=A 裁决：前瞻契约，pace 消费者留待移动/手感 sprint |
| 2 | 第二层 aim_draw/parry 无 TOML def → Failed | 🟢 | 战斗/射箭 sprint 补 def |
| 3 | 动作轮盘 icon(IconId) 省略、disabled_reason 恒 None | 🟢 | UI sprint（IconId 类型 + 资源置灰） |
| 4 | Block A0 真实游戏仍休眠（无实体挂组件）| — | 预期——待 input_bridge/夺舍迁移激活 |
| 5 | §八 HUD/音频输入反馈 | 🟢 | Godot UI sprint（Failed 事件已就位）|

## 🚀 下一步候选

| 候选 | 依赖前提 | 优先级 |
|------|---------|--------|
| Godot input_bridge.gd（InputMap→InputState）| Block A0 就位 | 🥇 立即 |
| NPC/玩家迁移到新管线 | Block A0 就位 | 🥈 |
| pace 消费者（MovementModeSystem 读 desired_state）| — | 🥉 |
| Continuous/Charge 运行时（006）| — | |

**建议**: 🥇 input_bridge.gd——让键盘真正驱动已激活的 Block A0，是把垂直切片从"测试可跑"变成"手玩可动"的最后一环。
