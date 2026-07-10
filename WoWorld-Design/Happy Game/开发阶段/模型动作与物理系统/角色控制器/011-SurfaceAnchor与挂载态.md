# 011 — SurfaceAnchor 与挂载态

> **开发代号**: WoWorld (Wonder World)
> **模块**: 模型动作与物理系统 > 角色控制器 > 011
> **版本**: v1.0
> **日期**: 2026-07-09
> **状态**: 开发规格
> **定位**: 挂载态完整设计——三种挂载类型 + 骑乘战斗 + 坐骑 AI + 脱离速度继承。将 CHG-067 的 SurfaceAnchor trait 焊回控制器层。
> **依赖**: [[../运动学地基/001-质量冲量与涌现运动学|CHG-067 SurfaceAnchor trait]] · [[../空间查询与物理/001-空间查询与物理系统|空间查询]]
> **关联**: [[001-角色控制器总纲]] · [[002-MovementState与连续移动]] · [[003-ActionController与离散动作]] · [[../../战斗/011-潜行伏击与骑乘战斗|战斗·骑乘]]

---

## 〇、三种挂载类型

| | 骑乘 | 甲板 | 巨兽表面 |
|---|---|---|---|
| 姿势 | Riding——骨盆锚定鞍骨 | Standing——甲板上走动 | Standing——不平坦活体面 |
| WalkableArea | 无（固定在鞍上） | 甲板多边形 | 巨兽背部局部面 |
| 行动限制 | RiderActionSet 限制 | 自由走动+攻击/施法 | 同甲板 + grip 检定 |
| 脱离 | 主动下 + 被击飞 | 跳下 + 走过头 | 被甩下来 + 滑下 |
| 锚点 AI | 坐骑自主 AI（会受惊） | 船/载具（可能有船长 AI） | 巨兽自主 AI（会甩背） |

统一 `SurfaceAnchor` trait——差异在数据配置中。

---

## 一、AnchorBehavior + RiderActionSet

```rust
pub enum AnchorBehavior {
    Mount { saddle_bone: BoneId, can_spook: bool, rider_actions: RiderActionSet },
    Deck { surface_friction: f32, sway_amplitude: f32 },
    CreatureSurface { surface_deformation: f32, can_buck: bool },
}

pub struct RiderActionSet {
    pub can_melee: bool, pub can_ranged: bool,
    pub can_cast: bool, pub can_use_items: bool, pub can_swap_weapons: bool,
}
```

---

## 二、CMountedState

```rust
pub struct CMountedState {
    pub anchor: EntityId,
    pub anchor_behavior: AnchorBehavior,
    pub local_position: Vec3,       // 锚点局部空间中的位置
    pub local_yaw: f32,             // 锚点局部空间中的朝向
    pub posture: MountedPosture,    // Riding/Standing/Sitting/Clinging
    pub inherited_velocity: Vec3,   // 用于脱离时速度继承
}

impl CMountedState {
    pub fn world_transform(&self, anchor: &dyn SurfaceAnchor) -> Mat4 {
        anchor.anchor_transform(self.anchor) * local_mat4(self)
    }
    pub fn detach_velocity(&self, anchor: &dyn SurfaceAnchor) -> Vec3 {
        anchor.velocity_at(self.anchor, self.world_position(anchor))
    }
}
```

---

## 三、挂载/脱离动作链

```toml
[action.mount]       # Windup500/Active100/Recovery200 → Locomotion=Attached, special=Mounted
[action.dismount]    # Windup300/Active150/Recovery200 → 继承anchor速度→loco=Grounded或PhysicsBody
[action.emergency_dismount]  # Windup100/Active50/Recovery300 → loco=PhysicsBody(Jumping)+anchor速度
```

**脱离时的速度继承**——`vel = anchor.velocity_at(骑手位置) + 主动跳跃速度`。玩家不会"停在空中"然后掉下来。

---

## 四、MountedMovementSystem

```rust
fn apply_mounted_movement(transform, intent, mounted, anchor, dt) {
    match mounted.anchor_behavior {
        Mount => {
            // 骑手在鞍上——骨盆锚定，local_position 不变
            // MoveIntent.direction 路由到坐骑的 ActionRequestBuf
            transform = anchor.anchor_transform × local_mat4(mounted);
        }
        Deck => {
            // 甲板上走动——local_position 按 intent.direction 更新
            // 检查 walkable_local 包含新位置；边界→滑动停止
            // 添加甲板摇晃效果（sway_amplitude）
            transform = anchor.anchor_transform × new_local_mat4;
        }
        CreatureSurface => {
            // 同 Deck + 表面起伏 + grip 检定（如果 can_buck）
        }
    }
}
```

---

## 五、骑乘输入路由

```rust
// 骑手 MoveIntent.direction → 路由到坐骑的 ActionRequestBuf
fn mount_input_routing_system(player_query, mut mount_query) {
    if move_state.special == Mounted { anchor } {
        mount_buf.push(ActionRequest {
            action_id: MountMoveCommand,
            params: MountCommand { direction, speed: sprint ? Gallop : Walk },
            priority: MOUNT_COMMAND,
        });
    }
}
```

坐骑有自己的 MovementState + ActionController——`MountMoveCommand` 被仲裁为坐骑的 pace 和方向。

---

## 六、坐骑 AI——涌现式"不听话"

坐骑是独立的 NPC——有 GOAP/情绪/本能。骑手的命令进入坐骑的 ActionRequestBuf：

- 坐骑饿了→GOAP 目标"找草吃"→拒绝 Gallop 命令（优先级判断）
- 坐骑被狼吓到→`Flee` 目标覆盖骑手的任何命令
- 坐骑累了→Stamina 耗尽→Gallop 被 StaminaGateSystem 降级

**涌现记忆点**：这不是一个"坐骑服从度=0.7"的数字——是活的动物的自主行为。

---

## 七、骑乘战斗——坐骑速度加成

骑手攻击命中时——`charge_bonus = mount_velocity.length() × 0.3` → 追加到伤害和冲量。涌现式——骑马冲锋比步行伤害高，不是因为硬编码"骑乘伤害×2"。

---

## 八、多人甲板

船上有船长+乘客×2。每人独立的 `CMountedState`（同一个 anchor=船实体）：
- **船长**——在驾驶位上，输入控制船
- **乘客 1**——甲板上自由走动，可攻击/施法/交互
- **乘客 2**——坐在长椅上，local_position 固定
- **所有人**——每帧 `world_transform = 船 × local`，船摇晃→乘客自然摇晃

---

## 九、被击飞脱离

当骑手被击飞（`ImpulseQueue` 命中）时，脱离坐骑是涌现的——不需要显式"脱离"指令：

```
帧 N:   CombatHitEvent { caused_knockback: true }
帧 N+1: ImpulseSystem drain → 骑手 delta_v = impulse / mass
        → 骑手 CMountedState.detach_velocity(anchor) = 坐骑在骑手位置的世界速度
        → 骑手最终速度 = anchor_velocity + knockback_delta_v
        → LocomotionMode: PhysicsBody
        → MovementState.special: Airborne(KnockedBack)
        → CMovementRecovery: push(挂载前的地面状态)
帧 N+1: 坐骑 GOAP: "骑手掉下来了→停下/逃跑/攻击敌人"（独立决策）
```

**速度继承**确保骑手不会"突然停在半空中"然后掉下来——而是带着坐骑的动量继续飞。

---

> **下一篇**: [[012-夺舍过渡]]
> **上一篇**: [[010-NPC移动行为]]
> **父文档**: [[001-角色控制器总纲]]
