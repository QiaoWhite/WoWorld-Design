# DEVLOG 2026-07-11 — 第三人称相机与视角系统（设计）

> **会话**: 2026-07-11
> **类型**: 设计冲刺（无代码——纯文档产出）
> **产出**: 玩家系统 007（v1.2）+ CHG-069 + 全套治理同步
> **方法**: /grill-me 逐项决策 → 三阶段质量保证循环 → 两轮审计 → /woworldidea-design sync

---

## 一、做了什么

Sprint-063/064 后玩家仍是**纯第一人称隐形相机骨架**。本会话为"玩家可见化身 + 第三人称相机"做设计 pass，产出正式开发规格 **玩家系统 007**（v1.2，15 节）。

### 探查（代码 grounding，非臆测）
- `player.gd` / `main.tscn`：FP 相机是 Player 子节点 +1.7m；鼠标 yaw 转 body。
- `input_bridge.gd`：传 `player.global_transform` 作 camera_transform。
- `player_input_system`：用 `camera_transform.x_axis/-z_axis` 转 WASD。
- `terrain_chunk.rs`：玩家为 ECS 实体（`player_ecs_entity`，含 EntityKind/LodLevel/CMovementControl/Rotation/CMoveIntent）；`entity_visual_system` **刻意排除玩家**（但 Player 节点无 mesh → 从未真正渲染）；`sync_bare_player_render`（ECS↔节点）；夺舍 `sync_possessed_position`（节点→ECS）。
- `kinematics.rs`：`RotationLock` 五态**已定义**；`movement_system._apply_rotation_lock` **是空 stub**。
- `spatial.rs`：`TerrainQuery::terrain_raycast` **存在**；`types.rs` `TerrainHit.distance` **存在**。
- `lod.rs`：**`CameraState{position:DVec3,forward:DVec3,fov_radians:f32}` 已存在**，`LodCoordinator::prescribe` 已按帧消费它；`PlayerAttention` 为独立结构。
- `008-手感系统`：`turn_smoothing.smooth_time=0.1` 已定义（手感是输入-输出延迟，不含相机 juice）。

## 二、关键设计决策（11 条，详见 CHG-069）

1. **自由环绕 + 角色朝移动方向**（WoW/原神范式）。
2. **架构 P**：化身走 entity_renderer 统一路径（EntityVisual +`controlled` 字段），玩家=NPC 单路径。
3. **独立顶层 CameraRig 节点**，读被控实体 ECS Position（非挂 Player/mesh 下）。
4. **相机碰撞用 Rust `terrain_raycast`**，非 SpringArm3D（地形无 Godot 碰撞体，守 CHG-033）。
5. 落地既有 `RotationLock`（填 movement_system 空 stub）：玩家默认 InputDirection，战斗 CameraForward。
6. `camera_transform` 来源 body→CameraRig（载重修复，逻辑不变只换源）。
7. **复用 `lod::CameraState`**（DVec3/radians），相机为生产者喂 LodCoordinator。
8. 相机手感 MVP：落地下沉（暴露 `CJustLanded`）+ 疾跑 FOV（钳 [60,120]）。
9. 两相更新定序：CameraRig(-200) yaw → InputBridge(-100) 转发 → WorldDriver(0) 跟随位。
10. 目标不连续（夺舍/瞬移）SNAP 而非 SmoothDamp。
11. 相机输入暂用原始键，InputAction 登记变体、接线延后 InputMap 冲刺。

## 三、质量保证循环纠出的实质问题

| 阶段 | 纠正 |
|------|------|
| 审计 v1.0→v1.1 | 漏 CameraState/LOD 契约；误写"退休 sync_bare_player_render"（会回归）；yaw 帧内定序；转向参数重复；相机 juice 无人认领 |
| 自我纠错 | **#3 谬误撤销**（`get_player_position()` 两模式都对，无需新 accessor 作 correctness）；#4 硬编码键与既有 InputMap 债一致，非新违规 |
| 审计 v1.1→v1.2 | **F1 重复定义 CameraState 且类型错**（应复用 lod 的 DVec3/radians）；F2 PlayerAttention 归属错；O1 目标不连续须 SNAP；O2 落地信号须显式暴露；O3 FOV kick 越界；O4 破坏 `test_visual_system_excludes_player` |

## 四、治理同步（/woworldidea-design sync）

- 新建 **CHG-069**；玩家系统 README +007；接头总览 28-玩家系统（出口/入口/变更日志）；CLAUDE-INTERFACES +CHG-069 契约章节；CLAUDE.md +CHG-069/指针。
- 跳过 S0 stash（避免撤回未提交 007）；未跑子代理重扫描（新增生产者 + 复用既有类型，冲突已在审计逐一证实）。

## 五、机械门

- 无代码改动——**build/test/clippy 不适用**（纯设计文档冲刺）。
- 文档一致性自检通过（出口↔文档↔INTERFACES 三处表述统一；CHG 五处 wikilink 闭环）。

## 六、下一步

进入**实现冲刺**（按 007 §十三 MVP 清单）。详见 handoff-20260711-camera-design。
