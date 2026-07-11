# 002 — MovementState 与连续移动

> **开发代号**: WoWorld (Wonder World)
> **模块**: 模型动作与物理系统 > 角色控制器 > 002
> **版本**: v1.0
> **日期**: 2026-07-09
> **状态**: 开发规格
> **定位**: 角色连续移动的完整数据模型——Stance/Pace/SpecialMode 三重组合 + MovementProfile TOML + 介质自动切换 + 恢复栈 + 步态接口
> **依赖**: [[../运动学地基/001-质量冲量与涌现运动学|CHG-067 LocomotionMode]] · [[../../生命/004-身体状态与生命过程|Vitals]]
> **关联**: [[001-角色控制器总纲]] · [[003-ActionController与离散动作]] · [[../动画系统/001-动画系统总纲|动画系统]]

---

## 〇、定位

MovementState 是角色的**连续移动状态**——它回答"身体在当前帧以什么姿态、什么速度、在什么介质中移动"。它是 MovementSystem 的核心输入之一。

与 ActionController（离散动作）的关系：MovementState 永远是**背景状态**。ActionController 的 `MovementLock` 是**前景覆盖**。Lock 解除后，MovementState 立刻恢复生效。两套系统不冲突——Lock 优先级始终高于 State。

---

## 一、核心类型

### 1.1 Stance——身体姿态

```rust
/// 持久状态——玩家按键切换或 GOAP 设定。
/// 影响：碰撞体高度、被感知范围、速度上限
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Stance {
    Standing,    // 全高、全速、全暴露
    Crouching,   // 半高、降速 50%、感知暴露 ×0.4
    Prone,       // 最低、降速 80%、感知暴露 ×0.12
}
```

### 1.2 Pace——步速

```rust
/// 每帧由输入/GOAP 重新设定——非持久。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pace {
    Still,       // 原地——方向为零
    Walking,     // 散步——最慢，不耗体力
    Running,     // 慢跑——默认移动速度
    Sprinting,   // 冲刺——最快，持续消耗体力。仅 Standing 可用
}
```

### 1.3 SpecialMode——介质/物理特化

```rust
/// 存在时覆盖 stance 和 pace。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpecialMode {
    Swimming(SwimPace),
    Climbing,
    Airborne(AirState),
    Mounted { anchor: EntityId },
}
```

### 1.4 MovementState——组合类型

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MovementState {
    pub stance: Stance,
    pub pace: Pace,
    pub special: Option<SpecialMode>,
    pub exhaustion_cooldown: f32,  // 体力耗尽冷却（秒）
}

impl MovementState {
    pub fn max_speed(&self, profile: &MovementProfile) -> f32 { /* 查表 */ }
    pub fn acceleration(&self, profile: &MovementProfile) -> f32 { /* 查表 */ }
    pub fn friction(&self, profile: &MovementProfile) -> f32 { /* 查表 */ }
    pub fn stamina_rate(&self, profile: &MovementProfile) -> f32 { /* 正=消耗,负=回复 */ }
    pub fn turn_rate(&self, profile: &MovementProfile) -> f32 { /* °/s */ }
    pub fn is_compatible_with(&self, loco: LocomotionMode, medium: Medium) -> bool { /* 合法性矩阵 */ }
    pub fn is_voluntary_ground(&self) -> bool { self.special.is_none() }
    pub fn gait_params(&self, profile: &GaitProfile, big_five: &BigFive, emotion: &EmotionState, fatigue: f32) -> GaitParams { /* →动画系统 */ }
}
```

**用法**：
- 正常地面：`MovementState { stance: Standing, pace: Walking, special: None }`
- 蹲着走：`MovementState { stance: Crouching, pace: Walking, special: None }`
- 在水中：`MovementState { stance: Standing, pace: Still, special: Some(Swimming(Slow)) }`——stance 和 pace 被忽略
- 挂在船上：`MovementState { stance: Standing, pace: Still, special: Some(Mounted { anchor }) }`

---

## 二、MediaState——介质驱动自动切换

MovementModeSystem 的核心逻辑。无论玩家还是 NPC——介质变化时自动修正 MovementState。

```
入水（Medium::Water + LocomotionMode::Ground → PhysicsBody）：
  当前自愿状态 → push 到 CMovementRecovery 栈
  MovementState.special = Some(Swimming(Slow))

出水（TerrainQuery.medium_at → Air + 着地 → Grounded）：
  从 CMovementRecovery.pop_compatible(Grounded, Air) → 恢复自愿状态
  栈空 → MovementState::default_for(Grounded, Air) → {Standing, Still, None}

踩空/滑落（Grounded → PhysicsBody + Air）：
  当前自愿状态 → push
  special = Some(Airborne(Falling { coyote_time_remaining: 0.15s }))

着地恢复（PhysicsBody → Grounded）：
  pop_compatible(Grounded, Air)
```

### SwimPace——介质自动切换不覆盖玩家选择

```rust
pub enum SwimPace {
    Slow,      // 安静、省体力、回复体力
    Fast,      // 耗体力、产生水花（听觉感知）
    Diving,    // 深度可控
    Treading,  // 原地维持浮力
}
```

入水默认 `SwimPace::Slow`。玩家按 Sprint → `SwimPace::Fast`。NPC GOAP 可设 置。

---

## 三、MovementRecoveryStack

嵌套物理过渡的恢复机制。

```rust
/// 最大深度 3（地面/空中/水各一层）
#[derive(Debug, Clone, Default)]
pub struct MovementRecoveryStack {
    stack: SmallVec<[MovementState; 3]>,
}

impl MovementRecoveryStack {
    pub fn push_if_voluntary(&mut self, state: MovementState) { /* 只有 is_voluntary_ground 的才入栈 */ }
    pub fn pop_compatible(&mut self, loco: LocomotionMode, medium: Medium) -> MovementState { /* 跳过不兼容的 */ }
    pub fn clear(&mut self) { /* 死亡/传送时清空 */ }
}
```

**示例**：地面 Sprint → 被击飞 → push(Sprint) → 入水 → push(当前状态)不触发(已在 special)→ 出水 → pop_compatible(Grounded, Air)返回 Sprint → 继续冲刺。

---

## 四、AirState——空中状态五态细分

```rust
pub enum AirState {
    /// 主动跳跃——一定程度的空中控制
    Jumping { control_ratio: f32, height: JumpHeight },
    /// 被动被击飞——几乎不可控，指定恢复时间
    KnockedBack { recoverable_at_secs: f32 },
    /// 踩空——土狼时间窗口
    Falling { coyote_time_remaining: f32 },
    /// 完全坠落——不可控
    Terminal,
    /// 主动滑翔
    Gliding,
}
```

**差异**：`Jumping.control_ratio=0.7`（空中可控转向），`KnockedBack` 几乎不可控，`Falling` 的土狼时间决定能否起跳。

---

## 五、MovementProfile——TOML 数据驱动

```toml
# movement_profiles.toml
[profile.humanoid]
label = "人形生物"
walk_speed = 1.4
run_speed = 3.5
sprint_speed = 5.5
ground_accel = 10.0
sprint_accel = 14.0
ground_friction = 12.0
sprint_friction = 8.0
default_turn_rate = 720.0
sprint_turn_rate = 360.0
air_turn_rate = 180.0
knocked_turn_rate = 30.0
sprint_stamina_rate = 8.0          # 单位/秒
sprint_min_stamina_to_start = 8.0
climb_speed = 0.6
climb_accel = 3.0
climb_friction = 8.0
climb_stamina_rate = 6.0
swim_slow_speed = 1.0
swim_fast_speed = 2.5
dive_speed = 1.5
swim_accel_slow = 3.0
swim_accel_fast = 5.0
swim_friction = 6.0
swim_fast_stamina_rate = 10.0
swim_slow_stamina_rate = -2.0     # 负 = 回复
treading_stamina_rate = 2.0
glide_horizontal_speed = 12.0
glide_vertical_speed = -1.5
glide_accel = 4.0
glide_stamina_rate = 3.0
jump_horizontal_speed = 3.0
knockback_recover_secs = 0.4
coyote_time_secs = 0.15            # ⚠️ 已迁出——见下方注（土狼时间现属 InputFeelConfig / 008，非 MovementProfile）
mounted_accel = 5.0
mounted_friction = 4.0
# ── 垂直积分与骑乘（实现新增，标 Provisional·CHG-067 后并入正式运动学）──
gravity = 20.0                     # 腾空垂直积分重力（m/s²）
jump_speed = 7.0                   # 起跳垂直初速（m/s）——跳跃高度 = jump_speed²/(2·gravity) ≈ 1.225m
mounted_speed = 7.0                # 骑乘态 max_speed 分支

[profile.wolf]
label = "狼（四足）"
walk_speed = 1.8
run_speed = 6.0
sprint_speed = 12.0
climb_speed = 0.0               # 不能攀爬岩壁
# ...

[profile.spider]
label = "蜘蛛（八足，天生攀爬）"
walk_speed = 1.2
climb_speed = 3.0               # 攀爬快——墙面如平地
climb_stamina_rate = 1.0        # 天然攀爬——几乎不累
default_turn_rate = 1080.0      # 八足支撑——原地转极快
# ...
```

---

## 六、StaminaGateSystem

体力耗尽时的强制降级——独立关注点，与 MovementModeSystem 解耦。

```rust
fn stamina_gate_system(vitals: &Vitals, state: &mut MovementState) {
    if state.pace == Pace::Sprinting && vitals.stamina < SPRINT_MIN_STAMINA {
        state.pace = Pace::Running;
    }
    if state.pace == Pace::Sprinting && vitals.stamina <= 0.0 {
        state.pace = Pace::Walking;
        state.exhaustion_cooldown = 1.5;  // 1.5 秒内禁止冲刺
    }
    // Crouching/Prone 不消耗额外体力——不拦截
}
```

**执行顺序**：CoyoteTimeSystem（读上一帧真实状态）→ StaminaGateSystem（体力门控——先降级 pace）→ MovementModeSystem（介质切换 + 恢复栈）→ MovementSystem（消费最终状态）。

> ⚠️ **顺序订正（2026-07-10 Step 5e 集成）**：原设计为 `MovementModeSystem → StaminaGateSystem`。实现发现 **StaminaGateSystem 应先于 MovementModeSystem**——否则 MovementModeSystem 在踩空帧把**过期的 Sprinting** 压入恢复栈，落地弹出后闪 1 帧再被重新降级。让 StaminaGate 先降级 pace，恢复栈快照到的即为降级后状态。另 **CoyoteTimeSystem 必须最先**——它读 `CPrevMovementState`，而 MovementModeSystem 会在帧末覆盖该组件。实际管线顺序见 `WorldDriver::process` Block A0。

---

## 七、GaitParams——步态参数接口

MovementState 生成 9 个连续的步态参数，供动画系统（9 层动画栈 Layer 0-2）消费。

```rust
/// 步态参数——全部 0.0-1.0 范围
pub struct GaitParams {
    pub hip_sway: f32,
    pub stride_length: f32,
    pub arm_swing: f32,
    pub bounce: f32,
    pub forward_lean: f32,
    pub rhythm_regularity: f32,
    pub foot_drag: f32,
    pub shoulder_stability: f32,
    pub gaze_level: f32,
}

impl MovementState {
    pub fn gait_params(&self, profile: &GaitProfile, big_five: &BigFive, emotion: &EmotionState, fatigue: f32) -> GaitParams {
        let base = profile.base_gait(self);               // 从 MovementState 选基础模板
        let pm = big_five.gait_modulation();              // 人格调制（9 值）
        let em = emotion.gait_modulation();               // 情绪调制（9 值）
        let fm = fatigue.gait_fatigue_factor();           // 疲劳缩放（1 值→乘到全部 9 值）
        base.blend(pm).blend(em).scale(fm)
    }
}
```

**涌现式步态**：BigFive 高神经质 → `rhythm_regularity=0.6`（走路节奏不规律）。愤怒 → `stride_length=0.9`, `arm_swing=0.85`（大步快走）。疲劳 → `foot_drag=0.6`, `forward_lean=0.4`（拖着脚走）。

---

## 八、非人形身体兼容

接口预留——延后实现。

```rust
impl RaceTraits {
    /// 🟡 延后：该种族是否支持某个姿态
    pub fn supports_stance(&self, stance: Stance) -> bool { /* 人形全true; 蛇Crouching=false; */ }
    /// 🟡 延后：该种族的默认姿态
    pub fn default_stance(&self) -> Stance { Stance::Standing }
    /// 🟡 延后：该种族在某个姿态下的参数覆盖
    pub fn stance_params(&self, stance: Stance) -> StanceParams { /* 蜘蛛Prone→speed×0.8(不是0.2) */ }
}
```

MovementModeSystem 在设置 Stance 前检查兼容性——不兼容的姿态被自动修正。

---

## 九、StanceTransition——姿态切换时长

```rust
impl StanceTransition {
    pub const STAND_CROUCH_MS: u32 = 250;
    pub const CROUCH_PRONE_MS: u32 = 400;
    pub const STAND_PRONE_MS: u32 = 350;

    pub fn duration(from: Stance, to: Stance) -> u32 { /* 查表 */ }
}
```

姿态切换期间——MovementLock: Full（不可移动）→ 过渡动画播放 → 完成后恢复 Free。

---

## 十、组合合法性矩阵

| Pace / Stance | Standing | Crouching | Prone |
|---------------|----------|-----------|-------|
| Still | ✅ | ✅ | ✅ |
| Walking | ✅ | ✅ | ✅ |
| Running | ✅ | ✅ | ❌ |
| Sprinting | ✅ | ❌ | ❌ |

Crouching 不能冲刺（蹲着跑不快）。Prone 只能匍匐慢速（`max_speed × 0.2`）。

---

## 十一、与 ActionController 的交互

MovementSystem 每帧读三个 flags：

```rust
pub enum MovementLock {
    Free,                    // 正常——读 MovementState 的速度曲线
    Partial { speed_cap },   // 减速上限——防御/瞄准中可慢走
    Full,                    // 原地不动——摩擦减速到零
    Override(Vec3),          // 动作接管——闪避/跳跃的强制位移
}

pub enum RotationLock {
    Free,                 // 不强制——保持上一帧朝向
    InputDirection,       // 面朝摇杆方向
    CameraForward,        // 面朝镜头（战斗/防御默认）
    TargetDirection,      // 面朝锁定目标
    Locked,               // 完全锁住
}
```

ActionController 设置 Lock → MovementSystem 执行 → Lock 解除后 MovementState 自动恢复。

---

## 十二、ECS 组件

```rust
// woworld_ecs
pub struct CMovementState(pub MovementState);        // 由 MovementModeSystem 写入
pub struct CMovementRecovery(pub MovementRecoveryStack);  // 介质变迁的恢复栈
pub struct CMoveIntent { pub direction: Vec3, pub camera_transform: Mat4, pub desired_state: Option<MovementState> };  // 由 ActionResolver 或 GOAP 写入
pub struct CMovementControl { pub movement_lock: MovementLock, pub rotation_lock: RotationLock };  // 由 ActionController 写入
```

---

## 十三、关键参数表

| 参数 | 值 | 来源 |
|------|-----|------|
| 地面加速度 | 10.0 m/s²（人形行走） | `movement_profiles.toml` |
| 冲刺最高速度 | 5.5 m/s（人形） | 同上 |
| 冲刺体力消耗 | 8.0 单位/秒 | 同上 |
| 土狼时间 | 0.15s | ⚠️ 现属 `CInputFeelConfig.coyote_time_secs`（M4 迁出 MovementProfile）；当前 `movement_mode_system` 硬编码 0.15，读配置待 I1-5 手感冲刺接线 |
| 恢复栈容量 | 3 层 | `SmallVec<[MovementState; 3]>` |
| 恢复栈溢出 | FIFO——满栈 push 丢弃 stack[0]（保护最近层） | `MovementRecoveryStack::push`（实现新增语义） |
| 可行走坡度上限 | 45°（`MAX_WALKABLE_SLOPE_COS = 0.707`） | `movement_system`（实现新增参数，陡坡阻挡水平位移） |
| 重力 / 起跳初速 | 20.0 m/s² / 7.0 m/s（跳高 ≈1.225m） | `gravity` / `jump_speed`（Provisional·CHG-067） |
| 介质切换延迟 | 无——同帧切换 | MovementModeSystem |
| 体力冷却 | 1.5s（体力耗尽后禁止冲刺） | StaminaGateSystem（常量 `EXHAUSTION_COOLDOWN`，非 profile 字段） |
| 冲刺起步最低体力 | 8.0 | `sprint_min_stamina_to_start`（profile 有此字段，当前系统用同值常量 `SPRINT_MIN_STAMINA`，未读 profile） |

---

> **下一篇**: [[003-ActionController与离散动作]]
> **上一篇**: [[001-角色控制器总纲]]
> **父文档**: [[001-角色控制器总纲]]
