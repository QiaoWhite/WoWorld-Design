# DEVLOG: 2026-07-11 — 第三人称相机 MVP 实现 + 夺舍修复

> **冲刺**: Sprint 065 — 第三人称相机 MVP（A+B+C）+ 夺舍 CC 管线统一
> **参考**: 玩家系统 007 v1.2 · CHG-069
> **状态**: ✅ 1001 tests · clippy 零警告 · build 成功 · 未提交 · 实机验证通过

## 产出概览

将第一人称相机改造为第三人称自由环绕相机系统，并修复夺舍与 CC 管线分裂导致的移动/浮空/跳跃问题。两阶段：相机 MVP 实现 → 四轮 bug 修复 → 夺舍架构统一。

### 新增 (4 文件)

- **`woworld_core/src/camera.rs`** (~250 行) — `smooth_damp`/`smooth_damp_vec3`/`smooth_damp_quat`（Unity SmoothDamp 移植）+ `resolve_camera_arm`（TerrainQuery 射线夹紧）+ 13 单测
- **`woworld_ecs/.../character_facing_system.rs`** (~170 行) — RotationLock→Rotation 落地，smooth_damp_quat 写朝向 + 6 单测
- **`godot/scripts/camera_rig.gd`** (~100 行) — yaw/pitch/zoom/FP 切换/ESC 捕获/门控/FOV kick
- **handoff/devlog** (本文件)

### 修改 (13 文件)

- `woworld_core/src/entity_visual.rs` — `controlled: bool` 字段 + 字面量更新 + 2 单测
- `woworld_core/src/lib.rs` — `pub mod camera;`
- `woworld_ecs/src/components/movement_state.rs` — `CJustLanded` 单帧信号组件
- `woworld_ecs/.../movement_mode_system.rs` — CJustLanded 生命周期（清除+写入+3 单测）
- `woworld_ecs/.../entity_visual.rs` (ECS) — 排除→打标 controlled；测试更新
- `woworld_ecs/.../player_input.rs` — query 加 CMovementState；pace 直写；Auto 实体零化 direction
- `woworld_ecs/.../possess.rs` — possess_entity 添加完整 CC+action 组件束；移除旧 Movement
- `woworld_godot/src/terrain_chunk.rs` — 10 新字段 + 6 #[func] + camera_follow_and_publish + LOD 块 + Block A0 插入 + 化身过滤 + is_block_a0_driving 去夺舍排除 + sync_possessed 反转 + 落地下沉暂禁
- `woworld_godot/src/entity_renderer.rs` — label/bubble 抑制被控实体
- `godot/scripts/player.gd` — 移除相机逻辑（收缩为纯物理脚本）
- `godot/scripts/input_bridge.gd` — camera_transform 来源于 Player→CameraRig
- `godot/scenes/main.tscn` — Camera3D 迁移 + CameraRig 树
- `CLAUDE.md` / `CLAUDE-INTERFACES.md` — 状态更新

## 四轮 Bug 修复历程

### 第一轮（实机初测 — 4 问题）

| Bug | 根因 | 修复 |
|-----|------|------|
| 夺舍后 WASD 不跟相机 | `possess_entity` 缺 CMoveIntent/CMovementControl 等 CC 组件 → player_input_system query 不匹配 NPC | possess_entity 添加 12 个 CC+action 组件 |
| 落地下沉感受不到 | LANDING_DIP_K=0.03 → 5m/s 冲击仅 15cm | 调为 0.08 |
| 无法疾跑 | player_input_system 写 desired_state.pace 但 movement_system 不读——无消费者 | 直写 CMovementState.0.pace |
| 夺舍目标意外 | find_possessable_entities 用 body 位置+朝向而非 Camera3D | 改用 camera_3d_node 真实位置+forward |

### 第二轮（实机二测 — 2 问题）

| Bug | 根因 | 修复 |
|-----|------|------|
| 落地下沉弹跳感 | dip 直接改 rig position，SmoothDamp 0.08s 恢复产生弹性 | **暂禁**（需 dip 叠加到 follow target Y 再启用） |
| 夺舍仍不工作 | possess_entity **未移除旧 Movement 组件** → CC movement_system 的 Without\<Movement\> 过滤 NPC | 加 `cmd.remove_one::<Movement>(entity)` |
| (关联) 无法跳跃 | NPC 缺 CActiveAction/CActionRequestBuf/CInputBuffer/CMovementRecovery | possess_entity 补充完整 action 组件 |

### 第三轮（实机三测 — 仍浮空+固定方向）

**发现根本性架构缺陷**：`is_block_a0_driving()` 在夺舍时返回 `false`（`block_a0_driving && !is_possessing()`），导致 player.gd 的 legacy `_physics_walk` 接管移动——但该代码的 body basis 已被 007 移除鼠标 yaw → 固定世界方向。同时 `_physics_walk` 节点吸附到 `ground_h+1.7` 产生浮空。

三处修复彻底解决：
1. `is_block_a0_driving()` 去 `!is_possessing()` 排除——夺舍也走 Block A0
2. `sync_possessed_position` 从 pre-ECS "节点→ECS" 反转为 post-ECS "ECS→节点"
3. `sync_possessed_position` 调用从 pre-ECS(1434) 移到 post-ECS(2220+)

### 第四轮（实机四测 — ✅ 一切正常）

夺舍后 WASD 跟相机、可跳跃、胶囊贴地、裸玩家不漂移。

## 测试分布

| Crate | 测试数 | 变化 |
|-------|--------|------|
| woworld_core | 392 | +122 (camera.rs 13 + prior) |
| woworld_atmosphere | 26 | — |
| woworld_worldgen | 58 | — |
| woworld_ecs | 520 | +137 (new systems + prior) |
| woworld_godot | 0 | — |
| sprint*/step5e* | 5 | — |
| **合计** | **1001** | **+24 this session** |

## 架构决策记录

### 1. 跟随架构: WorldDriver push（非 CameraRig poll）
ECS 步进后推位置 → 零帧位置延迟（vs CameraRig._physics_process 读取旧帧）。

### 2. 碰撞射线用 Camera3D basis（含 pitch）
rig.basis.z 仅含 yaw——俯视时射线水平，判无碰撞穿地。用 cam basis 的 +Z 列（含 pitch）。

### 3. MIN_ARM 地板 `MIN_ARM.min(desired)`
FP（desired=0）时 arm=0，TP 时地板 0.3m。

### 4. character_facing_system 插在 mid_phase_flush 之后
action_system(1948) 写 rotation_lock → flush(1956) → facing 读新鲜值(1957) → jump_launch(1958)。

### 5. 夺舍并入 CC 管线
旧架构：夺舍节点物理 / 裸玩家 CC 管线——分裂。统一：`is_block_a0_driving()` 不再排除夺舍，同步方向反转（ECS→节点）。

### 6. 化身走 entity_renderer 统一路径
entity_visual_system 从排除改打标 controlled=true → "玩家=NPC"涌现。entity_renderer 抑制 controlled 实体的名字/气泡（更新路径+创建路径）。

### 7. 落地下沉暂禁
直接改 rig Y 产生弹跳感——正确实现需 dip 叠加到 follow target Y，让 SmoothDamp 同时平滑 onset+recovery。待后续冲刺。

## 已知后续项

- `is_ui_capturing()` MVP stub（无模态面板）
- `LANDING_DIP_K` 待实机调参后重新启用
- 夺舍 NPC 无旧组件 idempotent 处理（hecs insert_one 对已有为 no-op——安全）
- 预留: TargetDirection / 越肩偏移 / 遮挡 dither / 震屏 / 游泳坐骑载具相机 / 死亡留机 / 相机存档
