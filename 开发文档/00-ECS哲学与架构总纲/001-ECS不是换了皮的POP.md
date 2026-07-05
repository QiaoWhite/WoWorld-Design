# ECS 不是换了皮的 POP

> **关联**: [[002-Component拆装机制]] · [[005-调度模型]] · [[006-ECS铁律与陷阱]]

---

## 核心论点

**Bevy 的 `before()` / `after()` 系统依赖，是把 ECS 用成了面向过程编程（POP）的另一个版本。**

---

## POP vs ECS：两种根本不同的思维方式

### POP 思维：把事情拆成步骤

```
"怪物死亡"这件事：
  步骤 1: 判断怪物是否死亡
  步骤 2: 如果死亡，检查掉落表
  步骤 3: 如果掉落，随机选择物品
  步骤 4: 生成物品实体
  步骤 5: 启用拾取交互

  ↑ 每个步骤依赖前一个步骤的结果
  ↑ 顺序不可逆——先随机物品再检查掉落？乱套了
  ↑ 这就是 POP——面向过程编程
```

### ECS 思维：把事情拆成细节

```
"怪物死亡"这件事由以下细节构成：
  DeathWatchSystem:  看到 Health{0} → 拆 Health, 装 Corpse + PendingLoot
  LootRollSystem:    看到 Corpse + PendingLoot → 拆 PendingLoot, 装 LootResult
  ItemSpawnSystem:   看到 Corpse + LootResult → 拆 LootResult, 生成物品 Entity
  PickupSystem:      看到 Pickupable + PlayerNear → 处理拾取

  ↑ 每个 System 只关心自己认识的 Component 组合
  ↑ System 之间互不知道对方存在
  ↑ "死亡→掉落→拾取"的顺序从组件变化中涌现，不是靠调度保证
```

---

## 体检处的比喻

有一个体检机构，负责给人检查视力、听力、血压等项目。

### POP 视角（体检者的视角）

"我先去检查视力 → 然后去听力 → 然后去血压 → 然后……"

你把"体检"拆成了一系列步骤，并为这些步骤安排了顺序。这很自然——因为一个人同时只能做一项检查。

### ECS 视角（体检机构的视角）

- **视力检查部门**：任何人拿着"需要检查视力"的单子来，我就给他检查。检查完，收回单子，发"视力检查结果"。
- **听力检查部门**：任何人拿着"需要检查听力"的单子来，我就给他检查。检查完，收回单子，发"听力检查结果"。
- **血压检查部门**：同上。

每个部门（System）**独立工作**，不知道其他部门的存在。视力部门不关心世界上有没有听力部门。

当所有的检查部门都处理过这个人之后，"体检完成"这件事就从这些细节中**涌现**了——不需要一个"体检流程管理器"来协调顺序。

---

## 应用到游戏开发

### 错误的 ECS（Bevy 式）

```rust
// 系统之间通过 before/after 耦合——这就是 POP
app.add_systems(Update, (
    death_system,
    loot_system.after(death_system),       // ← POP 思维泄漏
    item_spawn_system.after(loot_system),  // ← 步骤依赖
    pickup_system.after(item_spawn_system),
));
```

### 正确的 ECS

```rust
// 系统平等注册，无 before/after
let mut phase1_systems: Vec<SystemFn> = vec![
    death_watch_system,
    loot_roll_system,
    item_spawn_system,
    pickup_system,
    // 顺序无关——每个 System 只对 Component 状态做出反应
];

// 死亡→掉落→拾取 的顺序从 Component 变化中涌现：
// 帧 N:   DeathWatch 看到 Health{0} → 装 Corpse + PendingLoot
// 帧 N+1: LootRoll 看到 Corpse + PendingLoot → 装 LootResult  
// 帧 N+2: ItemSpawn 看到 Corpse + LootResult → 生成物品
// 帧 N+3: Pickup 看到 Pickupable + PlayerNear → 处理拾取
```

---

## 新创意接入：零侵入

在 POP 模型中，如果你想在"死亡"和"掉落"之间插入一个"诅咒判定"：

```rust
// POP：必须修改调度链
app.add_systems(Update, (
    death_system,
    curse_system.after(death_system).before(loot_system), // 修改了调度
    loot_system.after(curse_system),
    // ...
));
```

在 ECS 模型中：

```rust
// ECS：只加一个 System，不改任何已有代码
fn curse_system(world: &hecs::World, cmd: &mut CommandBuffer) {
    for (e, (corpse, loot, pos)) in world.query::<(&Corpse, &LootResult, &Position)>().iter() {
        if is_cursed_zone(pos) { cmd.insert_one(e, CursedItem); }
    }
}
phase1_systems.push(curse_system);
// DeathWatch 不知道 CurseSystem 存在
// LootRoll 不知道 CurseSystem 存在
// ItemSpawn 下帧自然看到 CursedItem 标记
```

---

## 一帧延迟是特性，不是 bug

System A 写入的 Component，System B 在**下一帧**看到。

这听起来像 bug——实际上是 ECS 的核心机制：
- 它消除了 System 之间的顺序耦合
- 它使每个 System 可以独立并行
- 16ms 的延迟在游戏玩法中不可感知（死亡→掉落→拾取的整个链条跨 3 帧 = 50ms，玩家完全看不出来）
- 对于必须在同一帧内完成的操作——两个 System 应该合并为一个 System，或者通过 Component 状态通信而非通过调度

---

## 总结

| POP（Bevy 式 ECS） | ECS（本文档的模型） |
|--------------------|-------------------|
| System 有 before/after 依赖 | System 互不知道对方存在 |
| 顺序由调度保证 | 顺序从 Component 变化中涌现 |
| 新增步骤 = 修改调度链 | 新增 System = 注册一条 |
| "先做 A 再做 B" | "A 做完后 Entity 换了一组 Component，B 自然就能处理" |
