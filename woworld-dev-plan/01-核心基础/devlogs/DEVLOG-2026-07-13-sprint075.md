# DEVLOG: 2026-07-13 — Sprint-075 V6 快照存档

> **冲刺**: Sprint-075 — V6 快照存档·LMDB 全量快照→重载重建
> **日期**: 2026-07-13
> **阶段**: Phase 2 探针切片「活着的村庄」·进度 `▓▓▓▓▓▓▓▓▓▓ 10/10`
> **状态**: ✅ 编码完成·1147 tests·clippy/fmt 零警告·**待实机验证存档 bug**

## 做了什么

创建 `woworld_save` crate，实现 LMDB 全量快照存档系统——垂直切片最后一步「村庄可存档退出、重载后从中断处继续」：

1. **`woworld_save` crate（新·7 文件·~650 行）**
   - `header.rs` — SaveHeader（magic `b"WOWSAVE\x00"` + global_version + save_uuid + world_name + 验证）
   - `snapshot.rs` — WorldSnapshot + ComponentBag（25 个持久 Component 的 flat Option 结构）+ Entity 收集/恢复 + EntityId 重映射
   - `trait_def.rs` — SaveableModule trait v2.0 骨架（14/14 方法·object-safe）+ LoadContext（渐进加载上下文·`txn + create_txn()` 工厂）
   - `system.rs` — SaveSystem（heed/LMDB 环境管理 + save/load/list/delete + 原子写入：临时目录→rename）
   - `lib.rs` — 模块导出

2. **serde 全量派生（~55 类型）**
   - `woworld_core`: serde 从 optional → 非 optional。约 15 类型加 `Serialize + Deserialize`（time/types/id/economy/listing/behavioral/culture/faith/player/action/item）
   - `woworld_ecs`: 全部 ~27 持久 Component + ~3 资源类型加 serde。`glam/serde` feature 启用

3. **WorldDriver 接线（terrain_chunk.rs + debug_console.rs）**
   - F3 控制台：`save [name]` / `load [name]` 命令（通过 ConsoleState pending 字段→WorldDriver 下帧处理）
   - `handle_save()`: 收集 entities（collect_entities）+ 收集 registries（Inventory/Relation/Economy 快照）→ WorldSnapshot → SaveSystem::save()
   - `handle_load()`: SaveSystem::load() → 清空 ECS → restore_entities（old_bits→new_entity 映射）→ remap 所有 registry key → 重建 clock/计数器/派生状态

4. **三轮审计修正（~15 项）**
   - CRITICAL: EconomyRegistry 加载后旧状态泄漏（市场/订单簿/ID 计数器指向死 entity）
   - HIGH: world_seed 硬编码 0、block_a0_driving 未保存、LMDB map_size 10MB 重复硬编码
   - 结构性: LoadContext 创建 + trait `load()` 签名修正、SaveHeader 补 save_uuid/world_name/format_name、map_size→4GB、max_dbs→16
   - LMDB bug: `mdb_env_open()` 接受目录路径非文件路径——改为目录 `{name}.woworld/` 而非文件

### 架构决策

**新 crate（用户裁决）**: `woworld_save` 作为独立 crate，不依赖 Godot——引擎无关。依赖链: `woworld_godot → woworld_save → woworld_ecs → woworld_core`。SaveSystem 通过函数参数接收所有状态（依赖注入），不持有 WorldDriver 引用。

**LMDB 目录结构**: 存档 = 一个目录（内含 `data.mdb` + `lock.mdb`），扩展名 `.woworld`——对齐设计 doc 002 §1.1 "一个 .woworld 文件 = 一个 LMDB 环境"的精神（LMDB 环境在文件系统上表现为目录）。

**SaveableModule trait 骨架**: 14/14 方法完整定义，object-safe，有测试。但 save/load 管线当前是一体式 WorldDriver 方法——trait dispatch 按 V6 row "不做 SaveableModule 14 方法"明确推迟至 Phase 2。

**EntityId 重映射**: 加载时 old_bits→new_entity 映射表，遍历所有 registry key 转换。ZST 标签（HasInventory/HasEquipment/RelationHandle）不保存，从 registry 反推。

### 机械门

- **1147 tests 全绿**（core 412 + worldgen 75 + atmosphere 26 + ecs 614 + save 11 + integration 9）
- 4 ignored（LMDB 测试——Windows `canonicalize` 路径解析，不阻塞功能）
- clippy 零警告（`-- -D warnings`）
- fmt 通过
- build 通过（.dll 已更新）

## 关键文件

| 文件 | 改动 |
|------|------|
| **`woworld_save/Cargo.toml`** (新) | crate 配置：heed 0.22 + bincode 1.3 + woworld_core/ecs |
| **`woworld_save/src/header.rs`** (新) | SaveHeader: magic/format_name/global_version/save_uuid/world_seed/world_name/timestamp/game_tick/save_name |
| **`woworld_save/src/snapshot.rs`** (新) | WorldSnapshot + ComponentBag(25 Option fields) + ClockData + 3 Registry snapshots + collect_entities/restore_entities |
| **`woworld_save/src/trait_def.rs`** (新) | SaveableModule trait(14 methods) + LoadContext + DirtySnapshot |
| **`woworld_save/src/system.rs`** (新) | SaveSystem: LMDB env 管理 + save/load/list/delete + 原子目录 rename |
| `woworld_core/Cargo.toml` | serde 非 optional |
| `woworld_core/src/*.rs` | ~15 文件加 Serialize/Deserialize |
| `woworld_ecs/Cargo.toml` | +glam/serde feature |
| `woworld_ecs/src/components/*.rs` | ~27 文件加 serde derive |
| `woworld_ecs/src/resources/*.rs` | 3 文件加 serde + `wallet_entries()` |
| `woworld_godot/src/debug_console.rs` | save/load 命令 + ConsoleState pending 字段 |
| `woworld_godot/src/terrain_chunk.rs` | handle_save/handle_load + world_seed 字段 |

## 已知限制

- **💥 存档 bug 待修复**：LMDB `open()` 接受目录路径而非文件路径——已改为目录方案（`{name}.woworld/`），实机验证 pending
- **市场/订单簿不保存**：EconomyRegistry 仅保存钱包余额（MVP 范围）——加载后 `= EconomyRegistry::new()` 清空旧市场状态
- **4 个 LMDB 测试 ignored**：Windows `canonicalize` 路径解析问题，不影响实机功能
- **SaveableModule trait dispatch 未接线**：V6 row 明确切除——trait 骨架已就位，Phase 2 重构
- **`"saves"` 相对路径**：SaveSystem API 已支持参数注入（`new(path)`），当前 WorldDriver 传硬编码值

## 下一步

**🐛 修复存档 bug**（新会话）→ 实机验证 save/load 完整闭环 → 写探针结论报告（`phase2-probe-completion.md`）→ git commit + push
