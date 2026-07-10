# Handoff: 2026-07-10 — Sprint-063 input_bridge + 玩家实体 Block A0 激活

> **冲刺**: Sprint-063 — input_bridge 桥接 + 玩家实体走 Block A0（角色控制器垂直切片闭环）
> **日期**: 2026-07-10
> **阶段**: Phase 2 — 垂直切片
> **冲刺状态**: ✅ 完成（P1-P4 全达成，968 tests，键盘可驱动玩家实体经 Block A0）

## 📊 冲刺回顾

### 目标达成
| 目标 | 状态 | 备注 |
|------|------|------|
| P1 桥接 API | ✅ | 6 `#[func]` + `InputAction::from_code`（37 变体，+3 测试） |
| P2 input_bridge.gd | ✅ | 边沿 press/release + WASD + 相机变换，process_priority=-100 |
| P3 玩家实体 CC + 渲染翻转 | ✅ | 13 组件配方（生成时挂，不挂旧 Movement）+ ECS Position→节点 |
| P4 自检 + 审计 + 交接 | ✅ | 机械门全绿 + 设计一致性审计（4 修复 + 4 用户裁决） |

### 关键决策
- **玩家=NPC 哲学落地**：玩家实体走同一套 Rust 角色控制器（001 总纲）。CC 组件在**生成时**挂载，非夺舍时挂（合 012 §〇"装两个拆两个"）。
- **渲染权威 = ECS**（001 §196/§349，CharacterBody3D 已移除）：Block A0 后 `Position → Godot 节点`；player.gd 退为纯相机骨架（`is_block_a0_driving()` 真时 `_physics_process` 提前返回，鼠标环顾仍在 `_input`）。
- **切片边界**：仅**裸玩家实体**走 Block A0；夺舍 NPC（无 CC 组件）保持 legacy player.gd 节点驱动。绞杀者 `Without<Movement>` 隔离。
- **`Pace::Running` 非 Still**：pace 无消费者（M1=A），`max_speed(Standing,Still)=0` 会锁死移动——初值即生效值。
- **飞行 = Godot 节点权威调试旁路**：G 键切 `block_a0_driving`，飞行时节点→ECS 同步。

## 💾 恢复点（下一会话 AI 必读）

> ⚠️ 下一会话 AI 启动首先读这里。

- **当前冲刺**: Sprint-063 — **P1-P4 全部完成**。⚠️ **尚未 commit**（工作树含本冲刺全部改动 + `.obsidian` 噪声）。
- **机械门状态**: build ✅ / test ✅ (968 passed: core 377 + ecs 507[503 unit+4 integration] + worldgen 58 + atmosphere 26) / clippy ✅ 规范命令零告警 / fmt ✅ 干净
- **改动文件**: `woworld_core/src/input.rs`（from_code+3测试）· `woworld_godot/src/terrain_chunk.rs`（6 func+配方+渲染同步+夺舍修复）· `godot/scripts/input_bridge.gd`（新）· `godot/scripts/player.gd`（让权+G同步）· `godot/scenes/main.tscn`（InputBridge 节点）· 本 devlog/handoff
- **激活状态**: 键盘 → InputState → Block A0 → 玩家实体 Position → 相机移动。**未实机试玩验证**（下一步应启 Godot 确认 WASD 走动/Space 跳跃启动/相机贴地）。

- **下一步（按优先级）**:
  1. 🥇 **实机试玩验证**：`../tools/godot/Godot_v4.7-stable_win64_console.exe godot/project.godot`——确认 WASD 经 Block A0 走动、相机贴地跟随地形、Space 触发 jump 动作（日志 CActiveAction）、G 飞行旁路正常、Tab 夺舍/F 退出无相机瞬移。
  2. 🥈 **下一次输入冲刺**（用户已确认延后，见下"待用户确认已裁决"）：
     - 迁 **Godot InputMap**（可重映射，合 004 原文"InputMap →"）+ 动作名 `StringName` 直传替换 `from_code`（单一事实源）。
     - 飞行/夺舍统一 **"外部权威驻停 + 落点重播种"** 机制（复用 `set_bare_player_manual`）。
  3. **NPC 迁移到 Block A0**（独立冲刺）：GOAP→CMoveIntent（010 的 12 行为 + PathFollowing），给 NPC 生成时挂 CC 配方 + 移除旧 Movement → 夺舍变纯 PlayerTag 交换（012 §八 终态）。

- **已知陷阱**:
  - ⚠️ `from_code` 整数编码与 `input_bridge.gd` 的 KEY_BINDINGS 是**两侧手工同步的表**——改一处漏一处静默失配。下冲刺迁 InputMap 后此耦合消除。
  - ⚠️ 玩家实体 `Pace` 恒 Running（pace 无消费者）——Sprint/Walk 修饰键写入 desired_state 但不影响速度，方向为零才停。pace 生效待 MovementModeSystem 扩展。
  - ⚠️ `is_block_a0_driving()` = `block_a0_driving && !is_possessing()`——飞行 OR 夺舍任一即让 player.gd 接管节点。
  - ⚠️ 眼高 = 地表 +1.7m（Camera 子节点偏移；实体 Position=脚点）。比旧 3.4m 低——这是修正非回归。stance 派生眼高留待后续。
  - ⚠️ 夺舍 NPC 仍走 legacy（不经 Block A0）——预期边界，NPC 迁移前如此。

- **待用户确认（本冲刺已裁决）**:
  - ✅ NPC legacy 路径 → **接受为过渡态**，独立冲刺迁移。
  - ✅ 眼高 1.7m → **接受**（真实值，旧 3.4m 是 artifact）。
  - ✅ 飞行统一机制 → **延后**至下一次输入冲刺。
  - ✅ InputMap 迁移 → **延后**至下一次输入冲刺。

## 🔧 机械门验证
```
cargo build --workspace                    ✅ Finished
cargo test --workspace                     ✅ 968 passed
cargo clippy --workspace -- -D warnings    ✅ 零警告
cargo fmt --all --check                    ✅ 干净
```

## 📐 设计门（关键项）
| # | 检查 | 状态 |
|---|------|------|
| 玩家=NPC 统一控制器 | 玩家实体挂同一 CC 配方走 Block A0（001） | ✅ |
| 夺舍轻量装拆 | possess 仍只 PlayerComponent+ControlModeComponent（012 §〇） | ✅ |
| 渲染权威 ECS | Position → Transform3D，CharacterBody3D 退纯壳（001 §196/349） | ✅ |
| GDScript §14.1 | input_bridge 纯输入转发，零游戏逻辑 | ✅ |
| ID 类型归属 | InputAction/from_code 在 woworld_core，Godot 侧不重复定义 | ✅ |
| 每 func/映射 ≥1 测试 | from_code +3 测试（basic/payload/unknown） | ✅ |

## ⚠️ 已知问题
| # | 问题 | 级别 | 计划 |
|---|------|------|------|
| 1 | 原始按键绕过 InputMap（无重映射） | 🟡 | 下一次输入冲刺迁 InputMap |
| 2 | from_code 两侧手工同步表 | 🟡 | 同上，动作名直传消除 |
| 3 | 飞行空跑 Block A0（丢弃） | 🟢 | 下冲刺统一驻停机制 |
| 4 | pace 恒 Running（无消费者） | 🟡 | MovementModeSystem 扩展读 desired_state |
| 5 | 夺舍 NPC 走 legacy | — | NPC 迁移独立冲刺 |
| 6 | 未实机试玩验证 | 🔴 | 下一步立即启 Godot |
