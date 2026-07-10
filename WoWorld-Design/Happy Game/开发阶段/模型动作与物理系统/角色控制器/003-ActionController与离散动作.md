# 003 — ActionController 与离散动作

> **开发代号**: WoWorld (Wonder World)
> **模块**: 模型动作与物理系统 > 角色控制器 > 003
> **版本**: v1.0
> **日期**: 2026-07-09
> **状态**: 开发规格
> **定位**: 承诺中断系统——离散动作的仲裁核心。四级承诺 + 三阶段时间线 + 取消窗口 + 优先级体系。复杂度在 TOML 数据中，不在代码中。
> **依赖**: [[../运动学地基/001-质量冲量与涌现运动学|CHG-067 ImpulseQueue]] · [[../../玩家系统/003-双角色与托管模式|ControlMode]]
> **关联**: [[001-角色控制器总纲]] · [[002-MovementState与连续移动]] · [[004-ActionResolver与输入解析]] · [[../../战斗/005-招式积木与流派生成|战斗·招式积木]]

---

## 〇、设计哲学

传统 FSM 在 WoWorld 的动作空间下不可行——~40 个动作 × 10+ 个游戏系统，显式状态转换是 O(n²) 的维护灾难。**承诺中断系统将复杂度从代码移到数据**：每个动作在 TOML 中声明自己的承诺级别、取消窗口、可被谁取消。核心仲裁逻辑 ~40 行 Rust。

---

## 一、核心类型

### 1.1 ActiveAction——运行时动作

```rust
pub struct ActiveAction {
    pub instance: ActionInstanceId,   // 唯一追溯 ID
    pub action_id: ActionId,
    pub phase: ActionPhase,
    pub commitment: CommitmentLevel,
    pub elapsed: f32,                 // 动作已执行的总时长
    pub cancel_window_open: bool,     // 当前是否在可取消的时间窗口内
    pub resource_drain_rate: f32,     // 当前资源消耗速率（持续动作）
    pub sustain_phase: SustainPhase,  // 持续动作的维持阶段
}
```

### 1.2 ActionPhase——三阶段时间线

```rust
pub enum ActionPhase {
    Windup,    // 前摇——动作已提交，碰撞体未激活，可被极高优先级中断
    Active,    // 生效窗口——碰撞体存活，伤害/效果可命中，仅系统中断可通过
    Recovery,  // 后摇——碰撞体消失，取消窗口内可取消到指定动作
}
```

### 1.3 CommitmentLevel——四级承诺

```rust
pub enum CommitmentLevel {
    None = 0,   // 空闲——任意动作可立即开始
    Soft = 1,   // 软承诺——前摇或后摇中，可被闪避/跳跃/招架取消
    Hard = 2,   // 硬承诺——生效窗口中，只能被物理命中/死亡/系统中断打断
    Locked = 3, // 锁定——仪式/读条中，物理命中也不能打断（极少用）
}
```

### 1.4 ActionPriority——优先级常量

```rust
pub mod ActionPriority {
    pub const RELEASE: u8 = 0;           // 持续动作释放
    pub const CHARGE_TRIGGER: u8 = 5;    // 充能触发子动作
    pub const INTERACT: u8 = 10;         // 交互
    pub const HOTBAR: u8 = 12;           // 热键栏
    pub const ATTACK: u8 = 15;           // 轻/重攻击
    pub const SPECIAL_SKILL: u8 = 18;    // 特殊技能
    pub const JUMP: u8 = 20;             // 跳跃
    pub const BLOCK: u8 = 20;            // 防御
    pub const CLIMB_ENTRY: u8 = 25;      // 挂墙
    pub const CLIMB_TRANSITION: u8 = 28; // Mantle/Detach
    pub const DODGE: u8 = 25;            // 闪避
    pub const PARRY: u8 = 30;            // 招架
    pub const STAGGER_THRESHOLD: u8 = 35;// 硬直打断阈值
    pub const INSTINCT: u8 = 80;         // 本能反应
    pub const EMERGENCY: u8 = 100;       // 系统紧急（死亡处理）
}
```

---

## 二、仲裁逻辑——核心 ~40 行

```rust
impl ActionController {
    pub fn tick(
        &mut self, entity: EntityId, dt: f32,
        requests: &[ActionRequest], registry: &ActionRegistry,
        loco: &LocomotionMode, vitals: &Vitals,
        counter: &mut ActionInstanceCounter,
    ) -> SmallVec<[ActionLifecycleEvent; 4]> {
        // ── 处理释放信号 ──
        if let Some(current) = &self.current {
            if requests.iter().any(|r| r.action_id == current.action_id && r.priority == 0) {
                return self.finish_current(entity, FinishReason::Released);
            }
            // ── 检查取消请求 ──
            for req in requests {
                if req.action_id == current.action_id { continue; }
                if self.can_interrupt(current, req, registry) {
                    self.current = None;
                    break;
                }
            }
        }
        // ── 接受新请求 ──
        if self.current.is_none() {
            if let Some(req) = requests.iter().find(|r| r.priority > 0
                && registry.get(r.action_id).physics_req.is_satisfied_by(loco))
            {
                self.current = Some(ActiveAction::new(counter.next(), req, registry));
                return events_with(ActionLifecycleEvent::Started { ... });
            }
        }
        // ── Tick 阶段推进 ──
        // Windup计时→Active; Active计时→Recovery; Recovery计时→Completed
        // 持续动作(active_ms=0)→无限Active，等待释放
        // ...
    }

    fn can_interrupt(&self, current: &ActiveAction, req: &ActionRequest, def: &ActionDef) -> bool {
        // 规则 1: 取消窗口 + cancel_set（玩家/GOAP 主动取消）
        if current.cancel_window_open && def.cancel_set.contains(&req.action_id) {
            return true;
        }
        // 规则 2: 本能反应始终通过
        if req.source == ActionSource::Instinct { return true; }
        // 规则 3: 系统中断（硬直/击飞/死亡）始终通过
        if req.source == ActionSource::System && req.priority >= ActionPriority::STAGGER_THRESHOLD {
            return true;
        }
        false
    }
}
```

**三种打断规则**覆盖所有情况：玩家有意取消（规则 1）、身体自动反应（规则 2）、物理强制中断（规则 3）。

---

## 三、ActionRegistry——TOML 动作注册表

```toml
# action_registry.toml
[action.light_attack]
name = "轻攻击"
category = "Combat"
priority = 15
commitment = "Hard"
windup_ms = 120
active_ms = 100
recovery_ms = 250
cancel_set = ["heavy_attack", "special_skill"]
cancel_window_ms = 120
bufferable = true
buffer_window_ms = 150
physics_req = "Grounded"
movement_lock = "Full"
rotation_lock = "TargetDirection"

[action.dodge]
name = "闪避"
category = "Combat"
priority = 25
commitment = "Hard"
windup_ms = 0
active_ms = 350
recovery_ms = 100
cancel_set = ["dodge", "jump"]
cancel_window_ms = 100
bufferable = true
buffer_window_ms = 200
physics_req = "Grounded"
movement_lock = "Override"
rotation_lock = "InputDirection"

[action.interact]
name = "交互"
category = "Interaction"
priority = 5
commitment = "Soft"
windup_ms = 0
active_ms = 100
recovery_ms = 0
cancel_set = []
cancel_window_ms = 0
bufferable = false
physics_req = "Grounded"
movement_lock = "Full"
rotation_lock = "TargetDirection"
interrupt_on_move = true
```

---

## 四、辅助枚举

### 4.1 PhysicsRequirement——物理约束

```rust
pub enum PhysicsRequirement {
    Grounded,     // 必须在地面——Block/Dodge/LightAttack
    NotInWater,   // 地面或空中——AimBow
    Any,          // 任何 LocomotionMode——ChannelSpell/EmergencyDismount
    InWater,      // 必须在水中——Dive
    NotAirborne,  // 不能在被击飞/完全坠落中——大部分自愿动作
}
```

### 4.2 ActionSource——动作来源

```rust
pub enum ActionSource {
    Player,          // 玩家按键
    GOAP,            // NPC GOAP 决策
    Instinct,        // 本能层——不受 ControlMode 影响
    System,          // 系统中断——Stagger/Knockback/Death
    ChargedAction,   // 充能动作触发的子动作
}
```

优先级链：`Instinct(80) > Player(15-30) > GOAP(同Player级别) > System(35+仅中断)`

### 4.3 ActionParams——紧凑参数

```rust
pub struct ActionParams {
    pub target: Option<EntityId>,
    pub position: Option<Vec3>,
    pub data: u32,  // 由 action_id 解释（item_slot/spell_id/recipe_id/charge_power…）
}
```

16 bytes——栈上分配。不膨胀枚举。

---

## 五、与 ImpulseQueue 的交互

CHG-067 定义了定向冲量队列。命中瞬间——战斗/魔法**同时**做两件事：

1. `ImpulseQueue.push(Impulse { target, impulse })` → ImpulseSystem drain → Δv = impulse/mass → `LocomotionMode` 切 `PhysicsBody`
2. `SpatialEventBus.push(WeaponImpact { … })` → 感官通知（NPC 听见/看见）

ActionController 不写入 ImpulseQueue——它只**响应**物理结果。当 MovementSystem 切到 `PhysicsBody` 时，ActionController 在下一 tick 检测到 → `Interrupted(PhysicsKnockback)`。

---

## 六、本能层与 ControlMode 的关系

战斗三层模型的本能层（InstinctLayerSystem）产生的 ActionRequest 的 `source = Instinct`。系统始终接受这类请求——不受 ControlMode 影响。

```
DomainDelegated { Combat: Manual }:
  玩家按闪避 → source=Player → 仲裁通过
  GOAP 想闪避 → GoapIntentDispatchSystem: Combat∈manual_domains → 不发
  敌人来袭 → InstinctLayerSystem → source=Instinct → 仲裁始终通过
```

**Dead NPC 不会本能闪避**——`Without<CDead>` 门控阻止了 InstinctLayerSystem 为死亡实体产出请求。

---

## 七、Combo 链

本模块只提供 `cancel_set` + `cancel_window` **机制**。Combo 树（轻→中→重连击链）由战斗系统基于此机制构建。详见 [[../../战斗/005-招式积木与流派生成|招式积木与流派生成]]。

---

## 八、ECS 组件

```rust
// woworld_ecs
pub struct CActiveAction(pub Option<ActiveAction>);
pub struct CActionRequestBuf(pub SmallVec<[ActionRequest; 4]>);
pub struct CMovementControl { pub movement_lock: MovementLock, pub rotation_lock: RotationLock };
```

---

> **下一篇**: [[004-ActionResolver与输入解析]]
> **上一篇**: [[002-MovementState与连续移动]]
> **父文档**: [[001-角色控制器总纲]]
