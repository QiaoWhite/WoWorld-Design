# Entity 是什么

> **关联**: [[002-Component拆装机制]] · [[001-ECS不是换了皮的POP]]

---

## Entity ≠ 游戏对象

在 ECS 中，**Entity 不是"物"，而是"正在发生的事"**。

这是 ECS 和 OOP 最根本的区别，也是最容易误解的地方。

---

## OOP 的误解

```rust
// ❌ OOP 思维：Entity = 游戏对象
// "这个 Entity 是一个人"
// "人有手（Component）"
// "人能抽烟（Component）"
// "不抽烟的时候，抽烟 Component 暂停（Disabled）"
// "手断了，手 Component 暂停（Disabled）"
// ↑ 这是 Unity DOTS 的错误理解
```

## ECS 的正确理解

```rust
// ✅ ECS 思维：Entity = 正在发生的事

// "这个 Entity 代表'一个 NPC 正在铁匠铺打铁'这件事"
// 细节：
//   - 这个 NPC 是谁（NpcCore Component）
//   - 他在哪（Position Component）
//   - 他在做什么（CraftingTask Component）
//   - 他疲劳了吗（Needs Component）

// 当他打完铁，这件事结束了：
//   - 拆 CraftingTask
//   - 装 CompletedItem
//   - Entity 现在代表"一个 NPC 刚刚打完一把剑"

// 当他去吃饭：
//   - 拆 CompletedItem
//   - 装 Goal{kind: Eat}
//   - Entity 现在代表"一个 NPC 正在去吃饭的路上"
```

---

## "缘来是你"

这是文中引用的佛学概念，恰当地描述了 ECS 的哲学：

> 很多个小细节（Component）构成了一件事情（Entity）。当这些细节都发生之后，这件事情就叫做"完成了"。

反过来理解：
- 移动系统中"坐标发生变化"这个细节 → 出现在 NPC 行走中、也出现在箭矢飞行中、也出现在 UI 动画中
- 同一个细节（Component 类型）出现在不同的事情（Entity）中
- Entity 的"身份"是它所拥有的 Component 的**集合**，不是某个固有的本质

---

## 在 WoWorld 中的应用

### Entity 代表"一件事"

| Entity | 代表的事 | 核心 Component |
|--------|---------|---------------|
| NPC 在打铁 | 一个工匠正在执行锻造任务 | NpcCore + Position + CraftingTask + Needs |
| NPC 在巡逻 | 一个守卫正在巡视区域 | NpcCore + Position + PatrolRoute + Goal |
| 尸体待处理 | 一具尸体等待掉落生成 | Corpse + Position + PendingLoot |
| 物品可拾取 | 一把剑躺在地上等待被发现 | ItemEntity + Position + Pickupable |
| 正在进行对话 | 两个 NPC 在交谈 | Conversation + Participants + Topic |

### 同一件事的变化

```
NPC 铁匠的一天：

  清晨: [NpcCore, Position{home}, Needs{hunger:0.2}, Goal{kind: Work}]
  ↓ GoalResolutionSystem
  出门: [NpcCore, Position{road}, Needs{hunger:0.3}, Goal{kind: Work}, Moving]
  ↓ MovementSystem 到达
  到店: [NpcCore, Position{forge}, Needs{hunger:0.4}, Goal{kind: Work}, CraftingTask]
  ↓ CraftingSystem 完成
  午休: [NpcCore, Position{forge}, Needs{hunger:0.7}, Goal{kind: Eat}, CompletedItem]
  ↓ EatingSystem
  下午: [NpcCore, Position{forge}, Needs{hunger:0.1}, Goal{kind: Work}, CraftingTask]

  ↑ 同一个 Entity 在一天内经历了 6 次"身份变化"
  ↑ 每次变化 = Component 的拆和装
  ↑ 不修改已有 Component 的值（如 State enum），而是换 Component
```

---

## 与 hecs::Entity 的关系

`hecs::Entity` 是一个轻量标识符（本质是 `u64`），不代表任何"物"。

```rust
// Entity 只是一个 ID
let entity: hecs::Entity = world.spawn((Position::default(),));

// 同一个 ID 在不同帧可以有完全不同的 Component 集合
// 帧 N:   [Position, Health, NpcCore]
// 帧 N+1: [Position, Corpse, PendingLoot]
// ↑ Entity 还是那个 Entity（ID 没变），但"代表的事"完全不同了
```

---

## 总结

| OOP 概念 | ECS 概念 |
|---------|---------|
| Object = 数据 + 行为 | Entity = 正在发生的事（一堆 Component 的集合） |
| 对象身份由 Class 决定 | 事情的"身份"由 Component 组合决定 |
| 状态变化 = 修改字段值 | 状态变化 = 换 Component |
| 继承表达"是什么" | Component 组合表达"正在发生什么" |
