# Sprint-036: ECS Phase 1 生命系统（上）— 死亡→掉落涌现链

> **提案日期**: 2026-07-05
> **提案状态**: ✅ 已完成
> **所属阶段**: Phase 1 — 核心基础
> **所属里程碑**: 1H — 生命系统（首个完整 ECS 模块）
> **后续**: Sprint 037（腐败→消失链 + RegenSystem + Godot 集成）
> **权威设计来源**: `开发文档/01-世界框架/02-生命系统.md`（ECS 视角·优先于 1H）

## 📋 依赖前提检查

| 前置项 | 状态 | 备注 |
|--------|------|------|
| 1A Layer 0 核心类型 | ✅ | EntityId, WorldPos, ItemDefId 就位 |
| 1J ECS 基础设施 | ✅ | Sprint 035 — hecs 0.10 + LodCoordinatorSystem + 6 tests |
| 113 tests | ✅ | 零回归保证 |
| 设计文档 | ✅ | `开发文档/01-世界框架/02-生命系统.md` 是 ECS 权威规格 |

## 🎯 目标（3 个）

### 目标 1: 5 个生命 Component（死亡→掉落相关）

**涉及代码**: `woworld_ecs/src/components/vitals.rs`（新建·~100行）

| Component | 字段 | 字节 | 语义 |
|-----------|------|------|------|
| `Vitals` | hp, max_hp, stamina, hunger, thirst, body_temp, oxygen — 7×f32 | 28 | "这个实体还活着" |
| `DeathCause` | category: DeathCategory, specific: u8, source: Option\<EntityId\> | ~16 | "怎么死的、谁杀的" |
| `Corpse` | death_tick: u64, corpse_temperature: f32 | 16 | "死多久了、尸体状态" |
| `PendingLoot` | _(零字段 tag)_ | 0 | "等待掉落判定" |
| `LootResult` | items: [Option\<ItemDefId\>; 8], count: u8 | ~72 | "掉落了什么" |

**设计依据**:

- `Corpse` 带数据而非零字段 tag——`death_tick` 一处写入多处读取（CorpseDecaySystem / 感官System / 调查System），避免各处重复 `WorldClock - 死亡时间` 计算。`corpse_temperature` 支持涌现式感官线索（"尸体还温热"→凶手不远）。16 字节换数据自足性——值。
- `PendingLoot` 是 **tag 而非 `loot_table_id: u32`**——DeathWatch 不需要知道"掉什么"。LootRoll 自己读 `EntityKind`（或未来的 `SpeciesId`）决定查哪个掉落表。**一个 System 只做一件事。**
- `LootResult` 固定数组 `[Option<ItemDefId>; 8]`——ECS 铁律 2 禁止 `Vec` 内联。≤8 个物品用固定数组：72 字节、一个 cache line、零间接寻址。Handle+Storage 模式留给 Memory（2000条）这种真正的大数据。

> ⚠️ `DecayingRemains` 和 `PendingDespawn` 属于腐败→消失链，留在 Sprint 037。

### 目标 2: 死亡→掉落链条（3 System + CommandBuffer）

**涉及代码**: `woworld_ecs/src/systems/life/`（新建目录·~120行）

```
帧 N:   [Vitals{hp:0}]
        └─ death_watch_system ──→ cmd: remove Vitals
                                      insert Corpse + PendingLoot + DeathCause
        cmd.run_on(&mut world)
        
帧 N+1: [Corpse, PendingLoot, EntityKind]
        └─ loot_roll_system ────→ 读 EntityKind → 查 LootTableRegistry
                                  cmd: remove PendingLoot
                                      insert LootResult
        cmd.run_on(&mut world)
        
帧 N+2: [Corpse, LootResult, Position]
        └─ item_spawn_system ───→ 每物品 cmd.spawn(DroppedItem Entity)
                                  cmd: remove LootResult
        cmd.run_on(&mut world)
```

**一帧延迟是设计特性，不是 bug：**

- System 之间不直接调用——仅通过 Component 拆装通信
- System A 在帧 N 写入的 Component，System B 在帧 N+1 读到
- 未来 rayon 并行时，这个模式保证零竞态
- 这是 Bevy/ECS 社区的标准范式

| System | 触发查询 | CommandBuffer 操作 |
|--------|---------|-------------------|
| `death_watch` | `(Entity, &Vitals)` where hp<=0 | remove Vitals; insert Corpse, PendingLoot, DeathCause |
| `loot_roll` | `(Entity, &Corpse, &PendingLoot, &EntityKind)` | remove PendingLoot; insert LootResult |
| `item_spawn` | `(Entity, &Corpse, &LootResult, &Position)` | spawn DroppedItem×N; remove LootResult |

### 目标 3: LootTableRegistry Resource

**涉及代码**: `woworld_ecs/src/resources/loot_table.rs`（新建·~40行）

```rust
pub struct LootTableRegistry {
    tables: HashMap<u32, LootTable>,
}

pub struct LootTable {
    pub entries: Vec<(ItemDefId, f32)>, // (物品, 权重)
}
```

- 内置 ≥2 个测试掉落表（Creature 通用表 + Plant 通用表）
- LootRoll 通过 `EntityKind` 查表：Creature→表0, Plant→表1
- 未来 TOML 数据驱动时，替换 HashMap 的填充方式即可——接口不变

## 🏗️ 架构设计逻辑（10 维交叉验证）

### 1. 解耦

```
DeathWatch             LootRoll              ItemSpawn
    │                      │                     │
    │ 只读 Vitals          │ 只读 EntityKind     │ 只读 LootResult
    │ 只写 Corpse+         │ 只查 LootTableReg   │ 只读 Position
    │   PendingLoot+       │ 只写 LootResult     │ 只 spawn 新 Entity
    │   DeathCause         │                     │
    │                      │                     │
    ▼                      ▼                     ▼
  CommandBuffer ──────→ run_on() ──────→ next frame
    
  System 之间零直接调用。零共享状态。仅通过 Component 存在/不存在 通信。
```

### 2. 模块协作

- `DeathCause.source_entity: Option<EntityId>` 预留战斗系统接口——不阻塞本 Sprint
- `LootResult` 使用 `woworld_core::id::ItemDefId`——与物品系统共享类型
- `Position` 复用 Sprint 035 的 Component——不重复定义

### 3. 性能

- 生命 System 不在热路径——死亡是低频事件（分钟~小时级）
- 1000 NPC × DeathWatch 检查 `hp<=0` ≈ 微秒级
- LootRoll 查 HashMap 是 O(1)——但仅对**已死亡**实体运行（query 自动过滤）
- CommandBuffer 批量应用——非逐实体刷新

### 4. 代码品质

- `DeathCategory` 用 Rust enum（6 变体），不是 magic u8
- System 函数签名清晰表达读写意图：`(world: &hecs::World, cmd: &mut CommandBuffer, loot_tables: &LootTableRegistry)`
- 测试每个 System 独立——hecs World 即 fixture

### 5. 数学工具

- `corpse_temperature` 线性衰减：`temp = 37.0 - (ambient - 37.0) * decay_factor * elapsed_hours`（Sprint 037 实现，Component 就位）
- 掉落权重使用加权随机（`rand` crate 的 `WeightedIndex`）

### 6. 模块特色

生命系统的**记忆点**不是死亡瞬间——是死亡之后的涌现：
- 发现一具还温热的尸体 → 凶手在附近（感官 System 读 `corpse_temperature`）
- 古墓中的白骨 → `death_tick` 显示已死 800 年（历史 System）
- 掉落物不是"怪物死亡→爆装备"——是从生物身上自然剥离（皮/肉/骨），铁律：不合理的掉落不存在

### 7. 涌现式交互

```
Vitals.hp=0 → DeathWatch → Corpse+PendingLoot+DeathCause
    │
    ├─→ LootRoll（查表掉落）→ 皮/肉/骨/角 → 玩家拾取 → 经济 System
    ├─→ 感官 System（嗅到尸臭）→ NPC 警觉 → AI 决策
    ├─→ 调查 System（DeathCause.source_entity）→ 法律 System → 追凶
    └─→ CorpseDecay（腐败计时）→ DecayingRemains → Cleanup
    
    所有分支从 Corpse 的 Component 集合中生长——没有中心调度器。
```

### 8. 社区实践

| 参考 | 借鉴 |
|------|------|
| Bevy ECS `Commands` | CommandBuffer 延迟应用——本 Sprint 的基础模式 |
| roguelike 掉落表 | 加权随机掉落——`rand::distributions::WeightedIndex` |
| hecs 0.10 `CommandBuffer` | remove/insert/spawn + run_on——标准 API |

### 9. Godot 4.7

- 生命 System 在 `WorldDriver::process()` 中运行——Godot 每帧调用
- 本 Sprint 无 Godot 侧集成（Sprint 037 添加 `#[func]` 暴露 Vitals）
- process() 阻塞主线程——当前 System 耗时 < 0.1ms，不构成瓶颈

### 10. 玩家体验

- 死亡→掉落链条是**玩家可观察的涌现结果**——杀死生物后下一帧出现掉落物
- 一帧延迟（16ms）肉眼不可见——但架构上保证了后续并行的安全性
- 未来：掉落物出现在尸体旁的地面上（Position 偏移），而非"尸体消失→物品生成"的街机感

## 🧪 研究事项

| 问题 | 级别 | 状态 |
|------|------|------|
| hecs `CommandBuffer::remove::<T>()` API 签名 | 🟡 | ⚠️ 验证——可能与 `insert` 语法不同 |
| hecs `CommandBuffer::spawn()` 返回 Entity | 🟡 | ⚠️ ItemSpawn 需要拿到新 Entity 的 ID |
| `DeathCategory` 6 大分类 + 30 种死因 | 🟡 | 🔴 待回查原文档——先用精简版（6 分类 + 占位 specific） |
| `rand` crate 是否已在依赖树中 | 🟢 | 检查 Cargo.lock——可能需要显式添加 |
| LootRoll 从 EntityKind 映射到 loot_table_id | 🟡 | 硬编码映射表——TOML 驱动留 TODO |

## 📊 决策记录

| 决策 | 选项 A | 选项 B | 选择 | 理由 |
|------|--------|--------|------|------|
| Corpse 数据 | 零字段 tag | `{death_tick, corpse_temp}` | **B** | 涌现数据源·一处写入多处读取·16B 换解耦 |
| PendingLoot | `{loot_table_id: u32}` | 零字段 tag | **B** | DeathWatch 不应知道掉落表·LootRoll 自决 |
| LootResult | 固定数组 [8] | Handle+Storage | **A** | ≤8 物品·72B cache line·铁律 2 合规 |
| 腐败链 | 本 Sprint 一起做 | Sprint 037 | **B** | 语义独立·本 Sprint 聚焦死亡→掉落 |
| CommandBuffer | 从 Sprint 036 用 | 直接 World 操作 | **A** | 一帧延迟是涌现基石·未来并行安全 |
| 掉落表来源 | TOML 文件 | 硬编码 HashMap | **B** | TOML 驱动是物品 System 的事——现在过度工程 |

## 📖 必读文档清单

| 文档 | 路径 | 为什么读 |
|------|------|---------|
| 生命系统 ECS 设计 | `开发文档/01-世界框架/02-生命系统.md` | Component+System 权威规格 |
| Component 拆装哲学 | `开发文档/00-ECS哲学与架构总纲/002-Component拆装机制.md` | 拆装是核心机制，不是实现细节 |
| ECS 铁律 | `开发文档/00-ECS哲学与架构总纲/006-ECS铁律与陷阱.md` | 8 条不可违背规则 + 铁律 4 标记延迟清理 |
| 实现路线图 §阶段1 | `开发文档/06-迁移映射/003-实现路线图.md` | Phase 1 全景定位 |
| 类型迁移表 | `开发文档/06-迁移映射/002-类型迁移表.md` | ItemDefId 等类型的 ECS 角色 |

## 📋 任务清单

### Step 1.1: 生命 Component

- [ ] 创建 `woworld_ecs/src/components/vitals.rs`
- [ ] `Vitals` — 7×f32（hp, max_hp, stamina, hunger, thirst, body_temp, oxygen）
- [ ] `DeathCategory` 枚举 — Violent / Disease / Senescence / Environmental / SpiritExhaustion / Volition
- [ ] `DeathCause` — category + specific(u8) + source(Option\<EntityId\>)
- [ ] `Corpse` — death_tick(u64) + corpse_temperature(f32)
- [ ] `PendingLoot` — 零字段 tag
- [ ] `LootResult` — items([Option\<ItemDefId\>; 8]) + count(u8)
- [ ] 在 `components/mod.rs` 注册 `pub mod vitals;`
- [ ] ECS 铁律 8 条逐条审查

### Step 1.2: 死亡→掉落 System

- [ ] 创建目录 `woworld_ecs/src/systems/life/`
- [ ] `death_watch.rs` — 查询 `&Vitals`，hp<=0 → cmd 拆/装
- [ ] `loot_roll.rs` — 查询 `(&Corpse, &PendingLoot, &EntityKind)`，查表 → cmd
- [ ] `item_spawn.rs` — 查询 `(&Corpse, &LootResult, &Position)`，spawn → cmd
- [ ] `mod.rs` — 模块导出
- [ ] 每个 System ≥1 个单元测试

### Step 1.3: LootTableRegistry

- [ ] 创建 `woworld_ecs/src/resources/` 目录
- [ ] `loot_table.rs` — LootTable + LootTableRegistry
- [ ] `mod.rs` — 模块导出 + 在 `lib.rs` 注册
- [ ] 内置 ≥2 个测试掉落表

### Step 1.4: WorldDriver 集成

- [ ] WorldDriver 添加 `loot_tables: LootTableRegistry`
- [ ] `process()` 中：构造 CommandBuffer → 依次调用 3 System → `cmd.run_on(&mut self.ecs)`
- [ ] spawn Creature 测试 Entity（带 Vitals{hp:0}）→ 验证涌现链

### Step 1.5: 全量验证

- [ ] `cargo build --workspace` 通过
- [ ] `cargo test --workspace` — 113 回归 + ≥5 新增 **≥118 tests 全绿**
- [ ] `cargo clippy --workspace -- -D warnings` 零警告
- [ ] "死亡→掉落"链条集成测试：Frame N spawn + set hp=0 → Frame N+2 断言物品 Entity 存在
- [ ] ECS 铁律 8 条全量审查通过

## 预估

- **冲刺数**: 1
- **风险**: 🟡 中等 — CommandBuffer API 首次深度使用；`DeathCategory` 30 种细分死亡原因待回查
- **代码量**: ~350 行（Component ~100 + System ~120 + Resource ~40 + 测试 ~80 + 集成 ~20）
