# 005 — ActionOutcome 与动作结果事件

> **开发代号**: WoWorld (Wonder World)
> **模块**: 模型动作与物理系统 > 角色控制器 > 005
> **版本**: v1.0
> **日期**: 2026-07-09
> **状态**: 开发规格
> **定位**: 动作结果的双层事件体系——生命周期事件（统一） + 效果事件（按域定义）。GOAP/记忆/动画/UI 通过 ActionInstanceId 追溯完整的因果链。
> **依赖**: [[003-ActionController与离散动作]]
> **关联**: [[001-角色控制器总纲]] · [[../../战斗/001-战斗系统总览|战斗系统]] · [[../../NPC活人感模块/NPC活人感开发文档ver2.0|NPC活人感]]

---

## 〇、双层事件体系

| | Layer A: 生命周期事件 | Layer B: 效果事件 |
|---|---|---|
| 谁发出 | ActionController | 各域执行 System |
| 含义 | 动作怎么样了 | 动作产生了什么效果 |
| 粒度 | 所有动作统一 | 每种动作有自己的 payload |
| 消费者 | GOAP/动画/记忆/UI | 伤害管线/物品系统/感官 |
| 举例 | "轻攻击被闪避打断了" | "造成 12 点劈砍伤害，命中左臂" |

---

## 一、ActionInstanceId——追溯链

```rust
pub struct ActionInstanceId(pub u64);  // 单调递增——每次动作执行唯一

// Resource 中
pub struct ActionInstanceCounter(u64);
```

GOAP 在 `Started` 时记录 id，之后所有携带此 id 的效果事件都属于同一个动作。完成因果追溯。

---

## 二、Layer A: ActionLifecycleEvent

```rust
pub enum ActionLifecycleEvent {
    Started    { instance, entity, action_id, params, started_at },
    PhaseChanged { instance, entity, from: ActionPhase, to: ActionPhase },
    Completed  { instance, entity, action_id, total_duration_ms },
    Interrupted { instance, entity, action_id, by: InterruptSource, progress: f32 },
    Failed     { instance, entity, action_id, reason: ActionFailureReason },
    ChargeTrigger { instance, entity, charge_ms, stage },
}

pub enum InterruptSource {
    HigherPriorityAction(ActionId),
    PhysicsKnockback,         // ImpulseQueue 切 PhysicsBody
    Staggered,                // 被轻击硬直
    Death,                    // 致命伤害
    Dying,                    // 濒死——比 Death 先到
    PossessionTakeover,       // 玩家夺舍
    MoveInput,                // interrupt_on_move
    InputReleased,            // 持续动作松键
    VitalDepleted(ResourceType),
    DodgeCancel,
    ParryCancel,
}

pub enum ActionFailureReason {
    TargetOutOfRange, TargetDied,
    InsufficientResource(ResourceType),
    ContextInvalidated, EnvironmentalInterrupt, SelfStateInvalidated,
    PhysicsIncompatible,
}
```

---

## 三、Layer B: 域效果事件

```rust
// 战斗域
pub struct CombatHitEvent {
    pub action_instance: ActionInstanceId,
    pub attacker: EntityId, pub defender: EntityId,
    pub hit_result: HitResult,  // Hit/Blocked/Parried/Dodged/Missed
}

// 制作域
pub struct CraftCompletedEvent {
    pub action_instance: ActionInstanceId,
    pub crafter: EntityId, pub recipe: RecipeId, pub quality: f32,
}

// 魔法域
pub struct SpellCastEvent {
    pub action_instance: ActionInstanceId,
    pub caster: EntityId, pub spell: SpellId, pub effect: SpellEffect,
}

// 交互域
pub struct InteractionEvent {
    pub action_instance: ActionInstanceId,
    pub actor: EntityId, pub target: EntityId, pub result: InteractionResult,
}

// 死亡感知
pub struct NpcObservedDeathEvent {
    pub observer: EntityId, pub corpse: EntityId, pub cause: DeathCause,
}
```

---

## 四、双缓冲事件队列

```rust
pub struct EventChannel<E> {
    write_buf: Vec<E>,
    read_buf: Vec<E>,
}
impl<E> EventChannel<E> {
    pub fn begin_frame(&mut self) { swap; read_buf.clear(); }
    pub fn send(&mut self, event: E) { write_buf.push(event); }
    pub fn read(&self) -> &[E] { &self.read_buf }
    pub fn mid_phase_flush(&mut self) { read_buf.extend(write_buf.drain(..)); }
}
```

帧内时序：
```
begin_frame() → read_buf 清空
Phase 1 早段: ActionController::tick() → send 到 write_buf
mid_phase_flush() → write_buf → read_buf（中段 System 可见）
Phase 1 中段: 战斗/制作 → 读 read_buf → send 效果事件到各自的 write_buf
Phase 1 晚段: GOAP/记忆/动画 → 读所有事件
```

---

## 五、消费者表

| 消费者 | 消费的事件 | 动作 |
|--------|----------|------|
| **GOAP** | `Completed` + `Interrupted` + `Failed` + 效果事件 | 规划反馈——目标达成/重新规划/更新世界模型 |
| **MemorySystem** | `Completed`(memorable) + `Interrupted` + `CombatHit`(重击) | 写入值得记住的事件 |
| **AnimationSystem** | `Started` + `PhaseChanged` + `Completed/Interrupted/Failed` | Layer 0 姿态切换 + Layer 8 中断响应 |
| **UI System** | `Interrupted` + `Failed` + `CombatHit` + `CraftCompleted` | 伤害数字/命中标记/屏幕震动/制作完成 |
| **SensorySystem** | `NpcObservedDeathEvent` | NPC 发现尸体→情绪/报警/搜尸 |

---

## 六、持续动作的 Completed vs Interrupted 修正

持续动作（如 Block）的**正常释放**应产生 `Completed`，而非 `Interrupted`：

```rust
// ActionController 中
if def.active_ms == 0 && current.phase == Active {
    if input_released {
        → Completed { ... total_duration_ms }
    }
}
```

`Interrupted { by: InputReleased }` 只用于不正常的释放（如被击飞时被迫松键——但那时已经是 PhysicsKnockback 中断，不是 InputReleased）。

> **注（阶段维度澄清）**：本节伪代码聚焦**事件类型**（正常释放发 `Completed` 而非 `Interrupted`），为突出这一点省略了 Recovery 步骤。**阶段机以 [[006-持续动作与充能动作]] §〇 为权威**——持续动作松键后仍走短暂 Recovery 收尾（如 block `recovery_ms=100`，此间 cancel_set 内动作可取消），`Completed` 在 Recovery 末尾发出。两文档不矛盾：005 定"发什么事件"，006 定"走哪些阶段"。
> ⚠️ **实现状态**（Sprint-065 审计·2026-07-11）：当前代码松键→立即 `Completed`，**尚未走 Recovery**。因 block/aim_bow 无键位绑定、实机不可触发，零现时影响。Recovery 阶段实现延后至 block 绑键冲刺（需重置 `elapsed` 计时基线）。

---

> **下一篇**: [[006-持续动作与充能动作]]
> **上一篇**: [[004-ActionResolver与输入解析]]
> **父文档**: [[001-角色控制器总纲]]
