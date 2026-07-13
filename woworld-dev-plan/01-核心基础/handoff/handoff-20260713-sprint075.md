# Handoff: 2026-07-13 — Sprint-075 V6 快照存档

> **会话类型**: 冲刺执行（场景 B）· **日期**: 2026-07-13
> **阶段**: Phase 2 探针切片「活着的村庄」·进度 `▓▓▓▓▓▓▓▓▓▓ 10/10`
> **状态**: ⚠️ 编码完成·存档 bug 待修复·未提交·未 push

## 📊 本会话做了什么

1. **Sprint-074 提交确认**——已在上个会话末尾提交（`3f74561`）
2. **三审计设计审查**——设计文档对照 + 代码现实对照 + 接口契约对照，发现 32 项问题，修正 18 项，5 项呈送用户裁决
3. **用户裁决**——LMDB (heed) · bincode 1.3 · serde 非 optional · 新 `woworld_save` crate · SaveableModule trait 骨架
4. **编码**——创建 `woworld_save` crate（7 新文件·~650 行）+ serde 全量派生（~55 类型）+ WorldDriver 接线（~170 行）+ 控制台命令（~50 行）
5. **两轮审计修正**——代码质量轮 9 项 + 结构性轮 4 项
6. **LMDB 根因修复**——`mdb_env_open()` 接受目录非文件，改为目录方案

### 架构决策（用户裁决）

| 决策 | 选择 |
|------|------|
| 持久化后端 | LMDB (heed 0.22) |
| 序列化格式 | bincode 1.3（不可用 2.x——glam 损坏 bug #547） |
| serde 在 woworld_core | 非 optional |
| 代码位置 | 新 `woworld_save` crate（独立于 Godot，引擎无关） |
| SaveableModule trait | 14/14 方法骨架 + LoadContext 已就位，dispatch 延后至 Phase 2 |
| MVP 范围 | V6 row 明确切除：不做脏增量/迁移/崩溃恢复/多槽/trait dispatch |

## 📦 产物

**新 crate**: `woworld_save` — 7 文件, ~650 行
- `header.rs` — SaveHeader (9/17 字段·MVP 子集)
- `snapshot.rs` — WorldSnapshot + ComponentBag(25 fields) + Entity 收集/恢复 + EntityId 重映射
- `trait_def.rs` — SaveableModule trait(14 methods·object-safe) + LoadContext + DirtySnapshot
- `system.rs` — SaveSystem (heed/LMDB + save/load/list/delete + 原子目录 rename)
- `lib.rs` — 模块导出

**已有 crate 改动**: ~35 文件, ~300 行
- `woworld_core`: serde 非 optional, ~15 类型加 Serialize/Deserialize
- `woworld_ecs`: ~27 Component + ~3 Resource 加 serde, EconomyRegistry 加 `wallet_entries()`
- `woworld_godot`: F3 `save`/`load` 命令, handle_save/handle_load, world_seed 字段

**文档**: DEVLOG, 本 Handoff

## 💾 恢复点（下一会话 AI 必读）

> ⚠️ 下一会话按 `00-流程总览` **场景 B（冲刺执行）** 启动——**先修存档 bug，再实机验证**。

- **当前阶段**: Phase 2 切片·`10/10`（全部 10 步编码完成·V0+V1+V4a+Vf+V2+V3a+V3b+V4b+V5+V6 ✅）
- **🐛 已知 bug**: `save` 命令报 `os error 2`——根因已定位（LMDB `mdb_env_open` 接受目录路径非文件路径），已改为目录方案（`{name}.woworld/` 目录内含 `data.mdb` + `lock.mdb`），**实机验证 pending**
- **机械门状态**: `1147 tests 全绿`（core 412 + worldgen 75 + atmosphere 26 + ecs 614 + save 11 + integration 9），4 ignored（LMDB 测试），clippy/fmt 零警告，build 通过。
- **提交状态**: ⚠️ **未提交**。基线 = `3f74561`（Sprint-074）。本会话所有改动待提交。
- **A1 铁律**: 纯涌现，禁脚手架/假坐标/占位驱动/平行 trait/语音决策 silo。
- **关键新增类型/函数**:
  - `woworld_save::SaveHeader` — magic + global_version + save_uuid + world_name + world_seed + timestamp + game_tick + save_name
  - `woworld_save::WorldSnapshot` — 顶层快照结构（header + clock + entities + inventory + relations + economy_wallets + 计数器）
  - `woworld_save::ComponentBag` — 25 个 `Option<Component>` field
  - `woworld_save::ClockData` — accumulator + tuning 常量（WorldTime 重建）
  - `woworld_save::SaveSystem` — `new(path)` / `save(snapshot, name)` / `load(name)` / `list_saves()` / `delete_save(name)`
  - `woworld_save::SaveableModule` trait — 14 方法（4 必覆 + 10 默认）+ `Send + Sync`
  - `woworld_save::LoadContext` — `{ txn, create_txn }`
  - `woworld_save::snapshot::collect_entities(ecs) -> Vec<EntitySnapshot>`
  - `woworld_save::snapshot::restore_entities(ecs, snapshots) -> HashMap<old_bits, new_entity>`
  - `WorldDriver.save_system: SaveSystem` — 在 `init()` 中 `SaveSystem::new("saves")`
  - `WorldDriver.world_seed: u64` — 在 `ready()` 中存储
  - `WorldDriver.handle_save(name)` / `WorldDriver.handle_load(name)` — 一体式 save/load 方法
  - `ConsoleState.pending_save_name/pending_load_name: Option<String>` — F3 命令→WorldDriver 桥梁
  - `EconomyRegistry::wallet_entries() -> impl Iterator<Item = (EntityId, WalletSnapshot)>`
- **数据流改变**:
  - Save: F3 `save x` → `ConsoleState.pending_save_name` → WorldDriver 下帧 `handle_save("x")` → 收集 entities/registries → `WorldSnapshot` → `SaveSystem::save()` → LMDB 目录 `saves/x.woworld/`
  - Load: F3 `load x` → `ConsoleState.pending_load_name` → WorldDriver 下帧 `handle_load("x")` → `SaveSystem::load()` → 清空 ECS → restore_entities（old_bits→new_entity 映射表）→ remap 所有 registry key → 重建 clock/计数器/派生状态
- **EntityId 重映射**: `HashMap<u64, hecs::Entity>` — 加载时遍历旧 entity bits → 新 entity handle。所有 `HashMap<EntityId, ...>` registry 必须经过此映射表转换
- **EconomyRegistry 加载**: `handle_load` 中先 `= EconomyRegistry::new()` 清空旧状态，再逐一 `set_wallet()`——防止旧市场/订单簿/ID 计数器泄漏
- **trait 现状**: SaveableModule trait 14/14 方法已定义，但 `handle_save`/`handle_load` 完全不调用 trait——是一体式 WorldDriver 方法。V6 row 明确切除此范围

## ⚠️ 遗留 / 诚实边界

- **🐛 存档 bug**：LMDB `open()` 路径问题已定位→目录方案已实现，**实机验证 pending**
- **市场/订单簿不保存**——加载后 `= EconomyRegistry::new()` 重建。MVP 仅钱包余额持久化
- **`"saves"` 相对路径**——SaveSystem API 支持参数注入，当前 WorldDriver 传硬编码字符串。Godot `OS.get_user_data_dir()` 集成延后
- **trait dispatch 未接线**——V6 row 明确切除。SaveableModule + LoadContext 骨架已就位，Phase 2 重构
- **SaveHeader 9/17 字段**——format_name/save_uuid/world_name 已补；其余 8 字段（save_kind/save_status/world_summary 等）延后至 Phase 2
- **崩溃恢复/迁移/多槽**——V6 row 明确切除

## 🔗 关联

- **DEVLOG**: [[../devlogs/DEVLOG-2026-07-13-sprint075]]
- **上游**: [[handoff-20260713-sprint074]]（V5 旁观工具）
- **路线图**: [[../../02-垂直切片/README]] · [[../../附录E-开发状态]]
- **下游**: 修复存档 bug → 实机验证 → 探针结论报告 → commit + push
