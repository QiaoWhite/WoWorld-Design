# Handoff: 2026-07-11 — 第三人称相机 MVP 完成 + 夺舍修复

> **会话**: 2026-07-11
> **类型**: 实现冲刺（完整 A+B+C + 四轮 bug 修复 + 夺舍架构统一）
> **状态**: ✅ 1001 tests · clippy 零警告 · build 成功 · **实机验证通过** · 未提交

## 💾 恢复点（下一会话 AI 必读）

> ⚠️ 下一会话 AI 启动首先读这里。

- **本会话产出**: 第三人称相机的完整 Rust+Godot 实现，从第一人称改造为自由环绕第三人称。含四轮实机 bug 修复，最终实机验证通过（自由环绕/可见化身/缩放/FP/碰撞/转向/冲刺/跳跃/夺舍正常工作）。
- **机械门**: ✅ cargo check · ✅ cargo test --workspace **1001 tests** · ✅ cargo clippy 零警告 · ✅ cargo build --workspace (.dll 已更新)
- **git**: 本会话改动**未提交**。无远端推送。
- **上次提交**: `af7d8fa` (Sprint-063+064, 977 tests)——已在远端。

## 📁 改动文件清单

**新增**:
- `woworld/crates/woworld_core/src/camera.rs` (smooth_damp 家族 + resolve_camera_arm + 13 单测)
- `woworld/crates/woworld_ecs/src/systems/movement/character_facing_system.rs` (RotationLock→Rotation + 6 单测)
- `woworld/godot/scripts/camera_rig.gd` (CameraRig 输入 + FOV kick)
- `woworld-dev-plan/01-核心基础/devlogs/DEVLOG-2026-07-11-camera-mvp.md`
- 本文件

**修改**:
- `woworld_core/src/entity_visual.rs` — `controlled: bool` 字段
- `woworld_core/src/lib.rs` — `pub mod camera;`
- `woworld_ecs/src/components/movement_state.rs` — `CJustLanded` 组件
- `woworld_ecs/.../movement_mode_system.rs` — CJustLanded 清除+写入
- `woworld_ecs/.../movement/mod.rs` — 注册 character_facing_system
- `woworld_ecs/.../entity_visual.rs` — 排除→打标 controlled; 测试重命名
- `woworld_ecs/.../player_input.rs` — query 加 CMovementState; pace 直写; Auto 零化 direction
- `woworld_ecs/.../possess.rs` — possess_entity 完整 CC+action 组件; 移除旧 Movement
- `woworld_godot/src/terrain_chunk.rs` — 10 新字段 + 6 #[func] + camera_follow_and_publish + LOD 块 + Block A0 插入 + 化身过滤 + is_block_a0_driving + sync_possessed 反转
- `woworld_godot/src/entity_renderer.rs` — controlled 抑制名字/气泡
- `godot/scripts/player.gd` — 移除相机逻辑
- `godot/scripts/input_bridge.gd` — camera_transform 源改 CameraRig
- `godot/scenes/main.tscn` — Camera3D 迁移 + CameraRig 树
- `CLAUDE.md` — 状态行/测试数/camera.rs 架构条目/devlog+handoff 指针
- `CLAUDE-INTERFACES.md` — CHG-069 实现状态更新 + LOD 核验确认

## 🧪 实机核验清单（已验证通过 ✓）

1. ✅ 鼠标自由环绕 yaw/pitch
2. ✅ WASD 相机相对移动
3. ✅ 可见自身化身胶囊（TP 距离）
4. ✅ 滚轮缩放 [0,8]m；arm<1.0m 化身隐藏
5. ✅ V 键 FP/TP 切换；FP 化身隐藏
6. ✅ 相机碰撞不穿地（terrain_raycast）
7. ✅ 走动时角色面朝移动方向（character_facing_system）
8. ✅ Shift 疾跑 FOV +7°
9. ✅ 夺舍后 WASD 跟相机 / 可跳跃 / 胶囊贴地
10. ✅ ESC 释放/捕获鼠标；F3 控制台冻结相机
11. ✅ GD-002 camera_pos 取真实 Camera3D 世界位

## 🔑 关键架构变动（新会话注意）

1. **CameraRig 独立于 Player**：Camera3D 迁出 Player，CameraRig 为顶层节点。WorldDriver 在 ECS 步进后 `set_global_position` 推动位置。CameraRig.gd 只处理输入。
2. **player.gd 已收缩**：不再有鼠标环顾/相机/Camera3D 子节点——只含 G 飞行 + legacy 走 + 夺舍锚点。
3. **input_bridge.gd 读 CameraRig**（非 Player）作为 camera_transform 来源。
4. **entity_visual_system 不再排除玩家**——改为打标 `controlled=true`。entity_renderer 据此抑制名字/气泡，WorldDriver 据此过滤近距/FP 化身。
5. **夺舍并入 CC 管线**：`is_block_a0_driving()` 不再排除夺舍；`sync_possessed_position` 从 pre-ECS "节点→ECS" 反转为 post-ECS "ECS→节点"。
6. **`character_facing_system`** 插在 Block A0 的 `mid_phase_flush` 之后（1956→1957）。
7. **LOD CameraState** 从硬编码 70°+body-yaw 改为相机真实位/朝向/FOV——回退路径保留。
8. **落地下沉已暂禁**（注释说明正确实现方向）——待后续冲刺实机调参后启用。

## 🐛 已知陷阱

- **GD-002 已保护**：camera_pos uniform 用 `get_viewport().get_camera_3d()`——Camera3D 迁入 CameraRig 后自动跟随。
- **地形无 Godot 碰撞体**：相机碰撞全凭 Rust terrain_raycast——SpringArm3D 对地形无效。
- **`sync_bare_player_render` 未退休**：保持并存；相机另行经 `get_camera_target()` 读 ECS。
- **`is_ui_capturing()` 是 false stub**：无模态面板——归 UI/UX 冲刺。
- **`LANDING_DIP_K` 待调**：落地下沉暂禁，正确实现需 dip 叠加到 follow target Y。
- **DebugConsole 相机路径已更新**：`../Player/Camera3D` → `../CameraRig/PitchArm/Camera3D`。

## 📐 设计参数速查

| 参数 | 值 | 来源 |
|------|-----|------|
| pivot 眼高 | 1.5 m | §IV.1 |
| arm 默认/范围 | 4.0 / [0, 8] m | §VII.1 |
| follow_smooth_time | 0.08 s | §IV.2 |
| zoom_smooth_time | 0.12 s | §VII.1 |
| SNAP 阈值 | 5 m | §IV.4 |
| MIN_ARM | 0.3 m | §VIII.2 |
| HIDE_ARM_THRESHOLD | 1.0 m | §VII.2 |
| 鼠标灵敏度 | 0.003 rad/px | player.gd 原有 |
| turn_smooth_time | 0.1 s | input_feel.toml |
| pitch clamp TP/FP | ±80°/±89° | §V.1 |
| FOV 范围 | [60, 120] | §XI.2 |
| Sprint FOV kick | +7° | §XI.2 |
| collision margin | 0.3 m | §VIII.2 |

## 🔮 下一冲刺建议

按 007 §十三 预留清单：
- RotationLock::TargetDirection + 锁定目标相机（战斗）
- 越肩偏移
- 实体/植被遮挡 dither
- 受击震屏 / 命中顿帧
- 游泳/坐骑/载具相机
- 死亡留机
- 相机视图存档持久化
- 落地下沉重新启用（调参后）
