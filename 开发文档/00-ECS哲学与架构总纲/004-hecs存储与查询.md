# hecs 存储与查询

> **关联**: [[005-调度模型]] · [[006-ECS铁律与陷阱]]

---

## 为什么是 hecs

`hecs` 是 Rust 生态中最轻量的 Archetype ECS 存储库——**只做存储，不做调度**。

| 对比 | hecs | bevy_ecs | 自建 ECS |
|------|------|----------|---------|
| 存储模型 | Archetype SoA | Archetype SoA | 需自己实现 |
| 变更检测 | ✅ `Changed<T>` | ✅ `Changed<T>` | 需自己实现 |
| 指令缓冲 | ✅ `CommandBuffer` | ✅ `Commands` | 需自己实现 |
| 并行迭代 | ✅ `par_iter()` | ✅ | 需自己实现 |
| 调度器 | **无**（我们不需要） | 绑 Bevy Schedule | 需自己实现 |
| 依赖树 | ~3 个轻量 crate | ~15+ crate | 0 |
| 代码行数 | 0（库） | 0（库） | ~2000+ |

---

## Archetype SoA 存储原理

Archetype = 相同 Component 集合的 Entity 共享一张稠密表。

```
Archetype [Position, Health, NpcCore]:      Archetype [Position, Corpse, PendingLoot]:
┌────┬──────────┬────────┬─────────┐       ┌────┬──────────┬────────┬─────────────┐
│ ID │ Position │ Health │ NpcCore │       │ ID │ Position │ Corpse │ PendingLoot │
├────┼──────────┼────────┼─────────┤       ├────┼──────────┼────────┼─────────────┤
│ 1  │ (0,0,0)  │ {100}  │ {…}     │       │ 5  │ (10,0,5) │ {…}    │ {…}         │
│ 2  │ (5,0,3)  │ {80}   │ {…}     │       │ 8  │ (-2,0,1) │ {…}    │ {…}         │
│ 3  │ (1,0,9)  │ {50}   │ {…}     │       └────┴──────────┴────────┴─────────────┘
└────┴──────────┴────────┴─────────┘
       ↑ 同一 Archetype 的所有 Entity 在同一张表中
       ↑ Component 列连续存储 = CPU 缓存友好
```

**优势**：
- 查询 "所有有 Position + Health 的实体" = 一次线性扫描 Archetype [Position, Health, …]
- 不需要跨表 Join（对比 Sparse Set ECS）
- 迭代 = 连续内存扫描（最快的内存访问模式）

---

## 查询语法

```rust
// 单 Component 查询
for (entity, pos) in world.query::<&Position>().iter() { }

// 多 Component 查询（只匹配同时拥有的 Entity）
for (e, (pos, health)) in world.query::<(&Position, &Health)>().iter() { }

// 可变查询
for (e, (pos, health)) in world.query::<(&Position, &mut Health)>().iter() {
    health.hp -= 1;  // &mut 允许修改
}

// 带 Entity ID 的查询
for (entity, (pos, health)) in world.query::<(&Position, &Health)>().iter() {
    // entity 是 hecs::Entity
}

// 变更检测（只迭代被修改过的实体）
for (e, health) in world.query::<Changed<&Health>>().iter() { }

// 并行查询
use hecs::ParallelIterator;
for (e, (pos, health)) in world.query::<(&Position, &mut Health)>().par_iter() {
    // rayon 并行
}
```

---

## CommandBuffer 用法

```rust
use hecs::CommandBuffer;

fn my_system(world: &hecs::World, cmd: &mut CommandBuffer) {
    for (e, health) in world.query::<&Health>().iter() {
        // 记录操作——不立即执行
        cmd.insert_one(e, NewComponent { ... });
        cmd.remove_one::<OldComponent>(e);
        cmd.despawn(e);  // 标记删除
    }
    // 此时所有操作还在 buffer 里，未应用
}

// System 全部执行完后，统一 flush：
cmd.run_on(&mut world);
// 此时所有操作才真正执行
```

---

## 与现有代码的集成

```rust
// WorldDriver 中
use hecs::World;

pub struct WorldDriver {
    // ECS 存储——替代未来的 HashMap<EntityId, NpcData>
    ecs: hecs::World,
    
    // 现有的、不进 ECS 的——全部保留
    terrain: HeightfieldTerrain,
    ocean: HeightfieldOcean,
    clock: WorldClock,
    atmosphere: AtmosphereSynthesizer,
    weather_driver: SimpleWeatherDriver,
    lod_layers: [Option<LodLayer>; 8],
    // ...
}
```

`hecs::World` 只是一个字段——不替代架构，只替代 `HashMap`。
