# DEVLOG: 2026-07-10 — Sprint-063 input_bridge + 玩家实体 Block A0 激活

> **冲刺**: Sprint-063 — input_bridge 桥接 + 玩家实体走 Block A0（角色控制器垂直切片闭环）
> **阶段**: Phase 2 — 垂直切片
> **前置**: Sprint-062（ActionResolver·Block A0 接线但休眠·965 tests）

## 今日目标
- [x] P1 feed_input 桥接 `#[func]` API + `InputAction::from_code` 映射
- [x] P2 input_bridge.gd 输入采集（begin_frame + WASD + 边沿 press/release + 相机变换）
- [x] P3 玩家实体生成时挂 Block A0 全组件配方 + 渲染权威翻转（ECS Position → 节点）
- [x] P4 自检 + 设计文档一致性审计 + 交接

## 做了什么
- **woworld_core** `input.rs`：新增 `InputAction::from_code(code, payload) -> Option<InputAction>`——Godot↔Rust 传输约定（37 离散变体全映射，MoveDirection 故意排除；payload 供 SpecialSkill/HotbarSlot；未知 code→None 优雅降级）。+3 单测。
- **woworld_godot** `terrain_chunk.rs`：
  - WorldDriver 新增 6 个 `#[func]`：`input_begin_frame` / `input_press` / `input_release` / `input_set_move` / `input_set_camera_transform` / `is_block_a0_driving` / `set_block_a0_driving`。
  - `transform3d_to_mat4` 辅助（Godot 行主序 Basis → glam 列主序 Mat4）。
  - `ready()` 给玩家实体挂角色控制器 13 组件配方（复刻集成测试 `spawn_player`，**不挂旧 `Movement`**——绞杀者合格）。
  - 渲染权威翻转：`sync_bare_player_render(post_ecs)`——driving 时 ECS Position → 节点（post-ECS），飞行时节点 → ECS（pre-ECS）。
  - 夺舍交互修复：`is_block_a0_driving` 排除夺舍态；退出夺舍从节点重播种裸玩家 Position；夺舍期把裸玩家 `set_bare_player_manual(false)` 驻停避免无形漂移。
  - `block_a0_driving: bool` 字段（默认 true；G 飞行切 false）。
- **Godot** `input_bridge.gd`（新建）：`process_priority=-100` 先于 WorldDriver 消费；每帧 begin_frame → 相机变换 → WASD 移动 → 按键边沿 press/release；控制台开启时释放全部 held。挂入 main.tscn（InputBridge 节点）。
- **player.gd**：`is_block_a0_driving()` 真时 `_physics_process` 提前返回（退为纯相机骨架，鼠标环顾仍在 `_input`）；G 键切飞行同步 `set_block_a0_driving`。
- **计数**：965 → **968 tests**（+3），clippy 零警告，fmt 干净，零回归。

## 遇到的问题（审计中发现并解决）
- **🔴 `Pace::Still` → 玩家完全无法移动**：`max_speed(Standing, Still)` 落 `_ => 0.0`，而 pace 无消费者（M1=A）→ 初值即生效值。改 `Pace::Running` 对齐 proven 配方。
- **🔴 夺舍 getter 冲突**：夺舍 NPC 无 CC 组件需 player.gd 驱动节点，getter 必须 `&& !is_possessing()` 否则夺舍瘫痪。
- **🔴 退出夺舍相机瞬移**：裸玩家永久持 PlayerComponent → 从节点重播种 Position 修复。
- **🟢 夺舍期裸玩家无形漂移**：驻停（Auto）消除。

## 学到的东西
- **审计漏看设计原文的代价**：004 明写 "Godot InputMap → input_bridge.gd"，我却用原始按键——绕过了 InputMap 的玩家重映射。已定为下冲刺迁移项（见交接"下一步"）。
- **strangler `Without<Movement>` 是增量迁移的既定机制**：legacy NPC 与新管线并存不是瑕疵，是设计。NPC 迁移独立成冲刺，不塞进本切片。
- **脚点驱动 + 相机偏移解耦**：实体 Position=地形表面（脚）→ 节点原点 → Camera 子节点 +1.7m 眼高，是干净的第一人称标准（旧 3.4m 是 body 悬空 artifact）。
- **"外部权威驻停"可统一飞行与夺舍**（下冲刺重构点）。

## 实机试玩修复（首次手玩暴露）
- **🔴 GD-002：LOD0 体素透明看穿**——浮点原点 shader 的 `camera_pos` uniform 用了 body 位置
  而非真实 Camera3D（差 eye height 1.7m）→ 体素地形渲染偏高 → 低眼高相机被吞没露背面。旧
  +3.4m 悬浮相机恰好掩盖。修：`camera_pos` 改用 `viewport.get_camera_3d().get_global_position()`
  （voxel + clipmap 一并）。**已记入 `bugs/INDEX.md` GD-002**（反直觉陷阱，新会话易复发）。
  诊断靠临时 `[CamDiag]` 日志（feetY/terrainY/eyeY，证 Δ=0 排除卡地）——已移除。
- **相机 near** 1.0 → 0.2（第一人称标准值，配合低眼高）。
- **验证结论**：Block A0 移动正确（脚点贴地、行走地形跟随）；透明消失。
- **待续（非本冲刺回归）**：跳跃无腾空——`movement_system` Sprint-1 存根无垂直积分 + jump
  `movement_lock=Full` 冻结贴地；需实现 Airborne 垂直积分（001 §三 / CHG-067）。夺舍 Tab/F
  = 延后的 legacy 路径，随 NPC 迁移冲刺一并修。

## 明日计划（下一次输入冲刺）
- [ ] 迁 Godot InputMap（可重映射）+ 动作名直传替换 from_code（单一事实源）
- [ ] 飞行/夺舍统一"外部权威驻停 + 落点重播种"机制
- [ ] （更远）NPC 迁移到 Block A0：GOAP→CMoveIntent（010），夺舍变纯 PlayerTag 交换
