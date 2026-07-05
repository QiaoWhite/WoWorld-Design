# Component 拆装机制

> **关联**: [[001-ECS不是换了皮的POP]] · [[003-Entity是什么]] · [[006-ECS铁律与陷阱]]

---

## 拆装是核心，不是实现细节

在 ECS 中，**Component 的增删（拆装）不是"动态组件管理"的实现细节——它是 ECS 的核心哲学机制。**

---

## 拆装 = Entity 的身份变化

```
Entity 在帧之间：

  帧 N 进入:   [Health{hp:100}, Position, NpcCore, CombatState]
  ↓ DeathWatchSystem 看到 Health.hp == 0
  帧 N 离开:   [Corpse, Position, PendingLoot]
  
  ↑ "活着的战士" → "待处理的尸体"
  ↑ Entity 的身份通过 Component 集合的变化来表达
  ↑ 不是"改了一个状态字段的值"
  ↑ 是"换了一整套 Component"
```

---

## 为什么不把状态存成 enum 字段

```rust
// ❌ OOP 思维：用 enum 表示状态
struct NpcState {
    state: NpcStateEnum,  // Alive, Dead, Corpse, Decayed...
    health: u32,
    loot_table: Option<LootTable>,
    // 问题：状态越多，字段越多，匹配越复杂
}

// ✅ ECS 思维：用 Component 的存在/不存在 表示状态
// Entity 有 Health → 活着
// Entity 有 Corpse → 死了
// Entity 有 Corpse + PendingLoot → 待掉落
// Entity 有 DecayingRemains → 腐烂中

// 好处：
// - 每个 System 只匹配自己关心的组合
// - 新状态 = 新 Component，不改已有代码
// - Component 组合天然表达了"这是什么"
```

---

## 拆装的三种形式

### 1. 拆（Remove）

System 判定某条件满足 → 移除 Component

```rust
fn death_watch_system(world: &hecs::World, cmd: &mut CommandBuffer) {
    for (e, health) in world.query::<&Health>().iter() {
        if health.hp == 0 {
            cmd.remove_one::<Health>(e);          // 拆
            cmd.remove_one::<CombatState>(e);     // 拆
            cmd.remove_one::<Goal>(e);            // 拆
        }
    }
}
```

### 2. 装（Insert）

System 判定某条件满足 → 添加 Component

```rust
fn need_evaluation_system(world: &hecs::World, cmd: &mut CommandBuffer) {
    for (e, needs) in world.query::<&Needs>().iter() {
        if needs.hunger > 0.8 {
            cmd.insert_one(e, Desire { kind: DesireKind::Eat, urgency: 0.9 });  // 装
        }
    }
}
```

### 3. 换（Remove + Insert 组合）

拆掉一组 Component，装上另一组——Entity 身份彻底改变

```rust
fn death_watch_system(world: &hecs::World, cmd: &mut CommandBuffer) {
    for (e, health) in world.query::<&Health>().iter() {
        if health.hp == 0 {
            // 拆掉"活着的"所有 Component
            cmd.remove_one::<Health>(e);
            cmd.remove_one::<CombatState>(e);
            cmd.remove_one::<Goal>(e);
            cmd.remove_one::<Desire>(e);
            // 装上"死了的" Component
            cmd.insert_one(e, Corpse);
            cmd.insert_one(e, PendingLoot);
        }
    }
}
```

---

## Archetype 迁移的成本认知

每次拆装触发 Archetype 迁移——Entity 的组件数据在内存中移动位置。

| 操作 | 成本 | WoWorld 100 NPC 下的影响 |
|------|------|------------------------|
| 装 1 个 Component | ~搬家 1 次 | 可忽略 |
| 拆 1 个 Component | ~搬家 1 次 | 可忽略 |
| 装 N 个 + 拆 M 个 | ~搬家 1 次（批量） | 可忽略 |

在 100 个 NPC 的规模下，即使每帧有 20 个 Entity 经历拆装，成本也远小于 0.1ms。**不是瓶颈。**

但高频切换（每秒 60 次以上的拆装）应该用标签 Component 或 enum 字段代替——详见 [[006-ECS铁律与陷阱#铁律 3]]。

---

## CommandBuffer：拆装的执行机制

拆装操作不是立即生效的——它们写入 `CommandBuffer`，在 System 迭代完成后统一应用。

```rust
// System 执行期间：
for (e, health) in world.query::<&Health>().iter() {
    if health.hp == 0 {
        cmd.insert_one(e, Corpse);  // 记录操作，不是立即执行
        cmd.remove_one::<Health>(e); // 记录操作，不是立即执行
    }
}
// 此时 Corpus 还没装上，Health 还没拆掉

// CommandBuffer flush 之后：
cmd.run_on(&mut world);
// 此时所有记录的拆装操作才真正生效
```

**这就是为什么 System 之间没有"我先装、你后读"的依赖——装的操作在帧末才生效，读的操作在帧初就完成了。跨 System 的状态传递自然发生在下一帧。**
