# Handoff: 2026-07-11 — Sprint-063+064（input_bridge + 玩家 Block A0 + 跳跃）

> **会话**: 2026-07-10 夜 ~ 2026-07-11
> **冲刺**: Sprint-063（input_bridge + 玩家实体走 Block A0）+ Sprint-064（跳跃腾空积分）
> **阶段**: Phase 2 — 角色控制器垂直切片
> **状态**: ✅ 两冲刺完成，**均已实机验证通过**（走动 + 跳跃）。977 tests。

## 💾 恢复点（下一会话 AI 必读）

> ⚠️ 下一会话 AI 启动首先读这里。

- **机械门**: build ✅ / test ✅ **977 passed** / clippy ✅ 零警告 / fmt ✅ 干净
- **实机验证**: ✅ WASD 经 Block A0 走动、相机贴地、透明已修（GD-002）、**跳跃升 ~1.2m 弧线 + 落地能走**（用户已确认）
- **⚠️ git 未提交**：本会话全部改动**尚未 commit/push**（工作树见下"改动清单"）。新会话若要提交，先跑一遍机械门再 commit。最近提交仍是 `05c8b12`（Sprint-062）。
- **下一步（用户指定方向）**: 🥇 **玩家可见化身 + 第三人称相机**——进入 plan 模式设计时被打断，尚未产出计划。详见下方"下一步：第三人称"。

## 📦 本会话做了什么

### Sprint-063 — input_bridge + 玩家实体 Block A0（垂直切片键盘可驱动）
- `woworld_core::input`：`InputAction::from_code(code,payload)`（Godot↔Rust 整数传输，37 变体）。
- `woworld_godot/terrain_chunk.rs`：WorldDriver 6 个 `#[func]`（input_begin_frame/press/release/set_move/set_camera_transform/is_block_a0_driving/set_block_a0_driving）+ `transform3d_to_mat4`；`ready()` 给玩家实体挂 13 组件 CC 配方（不挂旧 Movement）；渲染权威翻转 `sync_bare_player_render`（ECS Position→节点）；夺舍驻停 `set_bare_player_manual` + 退出重播种。
- `godot/scripts/input_bridge.gd`（新）：每帧边沿 press/release + WASD + 相机变换，process_priority=-100。挂 main.tscn InputBridge 节点。
- `player.gd`：`is_block_a0_driving()` 真时 `_physics_process` 提前返回（退为纯相机骨架）；G 飞行同步 `set_block_a0_driving`。

### Sprint-064 — 跳跃腾空积分
- `MovementProfile += gravity(20)/jump_speed(7)`（movement.rs + movement_profiles.toml humanoid/wolf + static DEFAULT_PROFILE）。
- `movement_system.rs`：Airborne 分支（重力积分 + 空中控制 control_ratio + 落地贴地，忽略 MovementLock）。
- `jump_launch_system.rs`（新）：jump 动作 Active 且非腾空 → push 恢复栈 + vel.y=jump_speed + Airborne(Jumping{0.7})。
- `movement_mode_system.rs`：着地判定对腾空态收紧（读 Velocity，`vel.y<=0 && pos.y<=terrain+0.15` 才着地，防上升段误着地）。
- Block A0 接线：`...action(+flush) → jump_launch → movement`。
- 集成测试 `sprint064_jump.rs`：全弧线（升>1m/落地/pace 恢复 Running/落地能走）。

### 实机修复（本会话中用户反馈驱动）
- **GD-002（已记入 bugs/INDEX.md）**：浮点原点 shader `camera_pos` 误用 body 位置（差 1.7m 眼高）→ 体素地形渲染偏高→低相机看穿透明。修：改用真实 Camera3D 世界位置（`viewport.get_camera_3d().get_global_position()`）。相机 near 1.0→0.2。
- **跳跃回归（"能跳但不能水平移动"）**：精读 002/003/CHG-067 后定位 3 处设计偏离——① jump_launch 没 push 恢复栈→落地弹空栈得 Still→max_speed=0（回归根因）；② control_ratio 0.0 应为 0.7（002 §四）；③ movement_mode 用 1m 带误着地截断跳跃。全部按设计修正。

## 🎯 下一步：玩家可见化身 + 第三人称相机（用户指定）

**现状**：纯第一人称（Camera3D 是 Player 子节点 +1.7m）。玩家实体是**隐形相机骨架**（main.tscn Player 下无 MeshInstance，只有 Camera3D + CollisionShape3D）；只有 NPC 有胶囊（entity_renderer）。

**设计欠规格**：`玩家系统/006-玩家专属I-O适配层.md` §角色与视角只列 3 个输入意图——`CameraRotate`（镜头旋转）/`CameraZoom`（缩放）/`FirstPersonToggle`（第一/第三人称切换）。**无轨道臂/镜头碰撞/越肩偏移规格**。按 CONSTITUTION §5 需先设计 pass（研究 Godot `SpringArm3D` + 镜头碰撞）。

**实现要点（未定，待设计）**：
1. **可见化身**（前置）：给玩家实体渲染胶囊/模型。选项：复用 entity_renderer（但它排除 player_ecs_entity）/ Player 节点加 MeshInstance 子节点。FP 时需隐藏自身 mesh。
2. **第三人称相机**：Godot `SpringArm3D`（镜头碰撞自动拉近）→ Camera3D。鼠标控 yaw（pivot/body）+ pitch（SpringArm）。CameraZoom=臂长。FirstPersonToggle=臂长→0 + 显隐化身。
3. **⚠️ 移动方向调和**：input_bridge 当前传 **Player body** transform 作 camera_transform（player_input_system 据此转世界方向）。第三人称下移动应相对**轨道相机 yaw**，非 body。需改 camera_transform 来源（传相机/pivot yaw）或让 body yaw 跟随相机。
4. **GD-002 契约保持**：camera_pos uniform 仍须取真实 Camera3D 世界位置——SpringArm 拉近相机时 camera_pos 也随之变，`viewport.get_camera_3d()` 自动正确。

**下一会话建议**：进 plan 模式 → Explore（相机/avatar 现状 + 玩家006/UI-UX 设计）→ 研究 Godot SpringArm3D 方案 → 出设计+实现计划 → 审批 → 实现。

## 📂 改动清单（未提交）
**修改**：`bugs/INDEX.md` · `movement_profiles.toml` · `input.rs` · `movement.rs` · `movement_profile_registry.rs` · `movement/mod.rs` · `movement_mode_system.rs` · `movement_system.rs` · `terrain_chunk.rs` · `main.tscn` · `player.gd` · `.obsidian/workspace.json`(噪声)
**新增**：`jump_launch_system.rs` · `tests/sprint064_jump.rs` · `input_bridge.gd`(+.uid) · `GD-002-*.md` · `DEVLOG-2026-07-10-sprint063.md` · `DEVLOG-2026-07-10-sprint064.md` · `handoff-20260710-sprint063.md` · 本文件

## 🔑 关键陷阱（新会话防重踩）
- **GD-002**：浮点原点 camera_pos 必须用真实 Camera3D 位置，非 body（改相机高度时必复发）。
- **Pace::Still = max_speed 0**：任何路径把玩家 pace 变 Still（如恢复栈弹空）→ 无法移动。跳跃靠恢复栈 push/pop 保 pace。
- **绞杀者 `Without<Movement>`**：玩家/CC 实体不挂旧 Movement；旧 20 NPC 挂 Movement，不走 Block A0（NPC 迁移仍待做）。
- **夺舍 Tab/F 仍 legacy**：NPC 无 CC 组件，夺舍不经 Block A0。NPC 迁移独立冲刺。
- **InputMap 未迁**：input_bridge 用原始按键（绕过玩家重映射），下一次输入冲刺迁 InputMap（合 004 原文）。
