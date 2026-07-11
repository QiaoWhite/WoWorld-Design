# Handoff: 2026-07-11 — 第三人称相机与视角系统（设计完成，待实现）

> **会话**: 2026-07-11
> **类型**: 设计冲刺（无代码）
> **阶段**: Phase 2 — 角色控制器垂直切片 → 玩家可见化身 + 第三人称相机
> **状态**: ✅ 设计规格完成（玩家系统 007 v1.2 + CHG-069 + 全套治理同步）。**代码未开始**。

## 💾 恢复点（下一会话 AI 必读）

> ⚠️ 下一会话 AI 启动首先读这里。

- **本会话产出**: **纯设计**——玩家系统 007（第三人称相机）v1.2 + CHG-069 + 治理同步。**无任何 Rust/Godot 代码改动**。
- **机械门**: 不适用（无代码）。上一次代码状态仍是 Sprint-064 的 **977 tests passed**（已提交推送 `af7d8fa`）。
- **git**: 本会话改动**未提交**。工作树见下方"改动清单"。旧代码 `af7d8fa` 已在远端。
- **下一步**: 进入 **007 实现冲刺**（按 007 §十三 MVP 清单）。建议先按下方"实现顺序建议"分解，写冲刺提案 → 审批 → 编码。

## 📐 设计权威（先精读）

1. **`WoWorld-Design/Happy Game/开发阶段/玩家系统/007-第三人称相机与视角系统.md`（v1.2）** — 主规格，15 节。
2. `WoWorld-Design/Change/CHG-069-第三人称相机系统创建-20260711.md` — 决策 + 影响 + 参数速查 + 核验项。
3. `CLAUDE-INTERFACES.md` §CHG-069 — 跨模块契约表。

## 🎯 实现顺序建议（007 §十三 MVP → 冲刺分解）

按依赖顺序，可切 2-3 个子冲刺：

**子冲刺 A — 可见化身 + 独立相机骨架（无碰撞/手感）**
1. `EntityVisual` 加 `controlled: bool`（`woworld_core::entity_visual`）。
2. `entity_visual_system`：删排除 → 改打标 `controlled = (player_entity==entity)`；**更新 `test_visual_system_excludes_player`**（断言 len==2 + controlled==true）。
3. `entity_renderer`：`controlled` → 抑制头顶名字/气泡；近距(<1.0m)/FP → 隐藏胶囊（近距/FP 标记由 WorldDriver 传入）。
4. main.tscn：移除 Player 下 Camera3D；新建顶层 `CameraRig(Node3D)→PitchArm(Node3D)→Camera3D(current)`。
5. `WorldDriver.get_camera_target()`（读 `player_ecs_entity` ECS Position）；CameraRig 每帧跟随（先刚性，后加平滑）。
6. `player.gd` 收缩：移除鼠标环顾/相机；保留飞行/legacy/夺舍物理。**不退休 `sync_bare_player_render`**。

**子冲刺 B — 旋转/移动接线 + 碰撞**
7. CameraRig yaw/pitch（priority -200，鼠标累积 + clamp TP±80/FP±89）+ 鼠标捕获/ESC/门控迁入。
8. `input_bridge.gd`：`input_set_camera_transform(rig.global_transform)`（改自 player）。
9. `character_facing_system`（ECS Phase 1，player_input_system 之后）：落地 RotationLock，slerp 写 Rotation，玩家休息态 Free→InputDirection，复用 008 `turn_smoothing`。
10. `resolve_camera_arm`（`woworld_core` 纯函数 + 单测）；`WorldDriver.camera_resolve_arm()`；相机碰撞夹紧臂长。
11. 缩放（[0,8]m）+ FP/TP 切换（隐藏阈值 1.0m）。
12. 目标不连续 SNAP（实体变更/位移>5m）。

**子冲刺 C — 视点契约 + 手感**
13. `publish_camera_state()` → 填 `lod::CameraState`（DVec3/radians）；接入 `LodCoordinator::prescribe` 调用点。
14. `CJustLanded{impact_speed}`（movement_mode_system 着地分支写入）→ 落地下沉。
15. 疾跑 FOV kick（钳 [60,120]）。
16. `smooth_damp` / `smooth_damp_vec3` / `smooth_damp_quat` 助手（Unity SmoothDamp 移植）。

## ⚠️ 实现期核验项（设计已标注，编码时必查——不臆测）

1. **`LodCoordinator::prescribe` 当前是否已按帧接入 Godot 循环、喂 stub 还是真值**（`lod.rs` + terrain_chunk）。若已按帧 → 本模块须即时供真实 CameraState；若未 → 本模块产出为首个真实输入源。
2. **`action_system` 的 `rotation_lock` 复位时机**（`action_system.rs:146` 动作结束复位 Free）——确认与 facing_system 的"玩家休息态 Free→InputDirection"规则不冲突。

## 🔑 关键陷阱（新会话防重踩）

- **GD-002**：`camera_pos` uniform 必须取真实 Camera3D 世界位（`viewport.get_camera_3d().get_global_position()`）——rig 结构变了仍适用，改相机高度必复发。
- **地形无 Godot 碰撞体**：相机碰撞**只能** Rust `terrain_raycast`，**别用 SpringArm3D**（会穿地）。
- **CameraState 已存在**：`woworld_core::lod::CameraState`（DVec3/radians）——**复用，别新造** f32/度 版本。
- **不退休 `sync_bare_player_render`**：`get_player_position()` 消费方（距离裁剪/飞行/夺舍锚点）依赖它；相机另经 `get_camera_target()` 读 ECS。
- **两相定序**：CameraRig(-200) 先更 yaw，WASD 方向才不差帧；跟随位在 WorldDriver(0) ECS 步进后更。
- **绞杀者 `Without<Movement>`**：玩家/CC 实体不挂旧 Movement——facing_system 走新管线实体。

## 📂 本会话改动清单（未提交）

**新增**:
- `WoWorld-Design/Happy Game/开发阶段/玩家系统/007-第三人称相机与视角系统.md`（v1.2）
- `WoWorld-Design/Change/CHG-069-第三人称相机系统创建-20260711.md`
- `woworld-dev-plan/01-核心基础/devlogs/DEVLOG-2026-07-11-camera-design.md`
- 本文件

**修改**:
- `CLAUDE.md`（CHG 序列 + 指针）· `CLAUDE-INTERFACES.md`（+CHG-069 契约章节）
- `玩家系统/README.md`（+007 索引）
- `模块接头总览/28-玩家系统/{001-接口出口,002-接口入口,000-变更日志}.md`
- `.obsidian/workspace.json`（编辑器噪声）

## 🧭 后续路线（007 §十三 预留，非本次）
锁定目标相机(TargetDirection) · 越肩偏移 · 受击震屏/顿帧 · 遮挡 dither · 游泳/坐骑/载具相机 · 死亡留机 · 换角色过场 · 相机状态存档 · InputAction 管线接线。
