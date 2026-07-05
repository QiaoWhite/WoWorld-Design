# 跨 System 通信协议

> **关联**: [[001-ECS不是换了皮的POP]] · [[02-空间查询]] · [[../00-ECS哲学与架构总纲/002-Component拆装机制]]

---

## 三通道模型

System 之间通过三种通道通信——**没有直接的函数调用依赖**。

| 通道 | 机制 | 延迟 | 适用场景 |
|------|------|------|---------|
| Component 拆装 | System A 装 Component → System B 下帧读取 | 1 帧 (16ms) | 状态变化（死亡/装备/目标切换） |
| Resource 读写 | System A 写 Resource → System B 同帧或下帧读取 | 0-1 帧 | 全局状态（时钟/天气/输入） |
| Event 队列 | System A push Event → System B poll Event | 1 帧 | 一次性事件（受到伤害/完成交易） |

---

## 通道 1: Component 拆装（主要通道）

```
System A: DeathWatchSystem
  → cmd.insert(Corpse)  // 记录操作

帧末: CommandBuffer flush  → Corpse 正式装上

下一帧:
System B: LootRollSystem
  → query::<(&Corpse, &PendingLoot)>()  → 查到了！
```

**这是 ECS 中最核心的通信方式——90% 的 System 间通信都应通过此通道。**

---

## 通道 2: Resource 读写

```rust
// System A: WeatherDriverSystem
fn weather_driver(world: &hecs::World, weather: &mut WeatherState) {
    weather.current = WeatherState::Rainy;  // 直接修改 Resource
}

// System B: MovementSystem（同帧或下帧）
fn movement(world: &hecs::World, weather: &WeatherState) {
    let speed_mod = if weather.current == WeatherState::Rainy { 0.8 } else { 1.0 };
}
```

**适用**：全局单例——一个 System 写，多个 System 读。天然无冲突。

---

## 通道 3: Event 队列

```rust
struct EventBus {
    damage_events: Vec<DamageEvent>,
    trade_events: Vec<TradeEvent>,
    death_events: Vec<DeathEvent>,
}

// System A: CombatSystem
fn combat(world: &hecs::World, events: &mut EventBus) {
    events.damage_events.push(DamageEvent { target, amount: 50 });
}

// System B: AudioSystem（下帧）
fn audio(world: &hecs::World, events: &EventBus) {
    for event in &events.damage_events {
        play_hit_sound(event.target);
    }
}

// 帧末: EventBus.clear() —— 每个事件存活一帧
```

**适用**：一次性事件——通知方不关心谁来消费，消费方不关心谁产生的。

---

## 反模式：System 之间直接调用

```rust
// ❌ 反模式：System A 调用 System B
fn system_a(world: &hecs::World) {
    // ...
    system_b(world);  // 直接调用！破坏了 System 的独立性
}

// ✅ 正确：通过 Component 通信
fn system_a(world: &hecs::World, cmd: &mut CommandBuffer) {
    cmd.insert_one(entity, NeedsProcessing);  // 装标记
}
fn system_b(world: &hecs::World) {
    for (e, _) in world.query::<&NeedsProcessing>().iter() {
        // B 在下帧自然处理
    }
}
```
