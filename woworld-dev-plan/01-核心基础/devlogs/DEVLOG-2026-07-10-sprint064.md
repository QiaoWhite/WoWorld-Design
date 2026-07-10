# DEVLOG: 2026-07-10 — Sprint-064 跳跃腾空积分

> **冲刺**: Sprint-064 — 跳跃（movement_system 垂直/腾空积分）
> **阶段**: Phase 2 — 垂直切片（角色控制器移动细节）
> **前置**: Sprint-063（input_bridge + 玩家实体 Block A0·实机验证走动可行）

## 今日目标
- [x] P1 MovementProfile += gravity/jump_speed（TOML + struct + static default）
- [x] P2 movement_system 腾空积分分支（重力 + 落地）
- [x] P3 jump_launch_system（起跳注入垂直速度 + 置 Airborne）+ Block A0 接线

## 背景（Sprint-063 实机暴露）
跳跃此前**只跑动作状态机、物理零位移**——`movement_system` 是 Sprint-1 grounded 存根，
两条分支都 `pos.y = terrain.height_at()` 强制贴地；且 jump 的 `movement_lock=Full` 冻结。
本冲刺补上垂直积分，让 Space 真正跳起来。

## 做了什么
- **woworld_core** `movement.rs`：`MovementProfile += gravity: f32, jump_speed: f32`（Default 20/7）。
- **movement_profiles.toml**：humanoid（20/7）、wolf（20/6.5）。
- **movement_profile_registry.rs**：static `DEFAULT_PROFILE` 补两字段（const context 无 Default）。
- **movement_system.rs**：新增 **Airborne 分支**（置于 lock 判断之前）——`special==Airborne` 时
  `vel.y -= gravity*dt`、保留水平动量（无空中控制，匹配 jump 的 Hard 承诺）、积分位置、
  落地检测（`vel.y<=0 && new_pos.y<=terrain_y` → 贴地 + `vel.y=0`）。忽略 MovementLock
  （设计 001 §三：PhysicsBody/Airborne 时 MovementState 被忽略）。+4 单测。
- **jump_launch_system.rs**（新）：`action_system` 后、`movement_system` 前运行。边沿检测——
  active 动作 == `id_of("jump")` && `phase==Active` && `special` 为 None → `vel.y=jump_speed`
  + `special=Airborne(Jumping)`。special 变后条件不再满足，天然一次性。+3 单测。
- **Block A0 接线**：`...action(+flush) → jump_launch → movement`。
- **落地恢复**（Airborne→Grounded, special→None）复用既有 `movement_mode_system`
  （`is_walkable` 的 1.0m 落地带 + `pop_compatible`）——无需新代码。

## 关键决策
- **参数数学验证**（Python）：gravity=20, jump_speed=7 → 跳高 1.23m（>1.0m 落地带，能触发
  Airborne）、滞空 700ms（>动作 400ms，动作先完成解锁，落地不卡 Full）。
- **腾空分支优先于 lock**：jump `movement_lock=Full`，但腾空必须无视锁——分支置于 lock 判断前。
- **无空中控制**：保留起跳时的水平动量、不加 move_intent 加速——匹配 jump Hard 承诺（committed jump）。
- **一次性起跳靠状态边沿**：`special.is_none()` 守卫，无需额外 flag。

## 遇到的问题
- **static DEFAULT_PROFILE 漏字段**：const context 不能 `..Default`——两处（TOML struct + static）
  都要显式补。编译错误定位即修。

## 学到的东西
- **落地检测天然分工**：movement_system 管垂直积分 + 贴地，movement_mode_system 管
  Airborne→Grounded 状态转换（既有 `is_walkable` 1m 带）——两系统各司其职，无需耦合。
- **数学铁律**：起跳参数用 Python 先验证跳高/滞空/落地带关系，避免"跳不起来/穿地"。

## 计数
- 968 → **975 tests**（+7：jump_launch 3 + movement_system airborne 4），clippy 零警告，零回归。

## 🔴 实机修复（精读 002/003/CHG-067 后的设计对齐）
用户实机报告"能跳但无法水平移动"。逐字精读设计文档后定位**我初版的 3 处设计偏离**：

| # | 设计文档 | 初版错误 | 修复 |
|---|---------|---------|------|
| 1 | **002 §二**：腾空进入"当前自愿状态→push 恢复栈"，落地 pop_compatible；栈空→{Standing,**Still**} | jump_launch **没 push 恢复栈** → 落地弹空栈→pace 卡 Still→max_speed=0→**无法水平移动**（回归根因） | jump_launch `push_if_voluntary(ms)` 起跳前入栈 |
| 2 | **002 §四**：`Jumping.control_ratio=0.7`（空中可控转向） | 我设 `0.0` | 改 0.7 + movement_system 空中控制（朝输入加速，不超 jump_horizontal_speed，无摩擦保momentum） |
| 3 | **CHG-067 §三**：着地=下降接触（三级着地） | movement_mode_system 用 is_walkable 1m 带判着地 → 上升段 0.1m 处误着地截断跳跃 | 腾空态收紧：`vel.y<=0 && pos.y<=terrain_y+0.15` 才判着地（movement_mode_system 加读 Velocity） |

**架构定位**：完整设计（CHG-067 §二）是 `ImpulseQueue→PhysicsBody→COM 积分`，但那是未动工切片（§十二 Q-A2），`kinematics.rs=SHIM`。SHIM 直设速度可接受，**但必须遵守 002 §二恢复栈契约**（SHIM 层也生效的玩法规则）。

**集成测试** `sprint064_jump.rs`：逐字复刻 `movement_mode→jump_launch→movement` 全弧线——断言升 >1m、落地、**pace 恢复 Running**、落地后 pos.x 前进（水平移动恢复）。

**计数（修复后）**：975 → **977 tests**（+2：recovery-push 单测 + 全弧线集成），clippy 零警告。

## 明日计划
- [ ] 实机验证跳跃（Space 起跳、落地、行走中跳保留动量）
- [ ] （下一次输入冲刺）Godot InputMap 迁移 + 飞行/夺舍统一驻停 + NPC 迁移
