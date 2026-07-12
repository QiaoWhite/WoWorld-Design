# Handoff: 2026-07-13 — Sprint-074 V5 旁观工具

> **会话类型**: 冲刺执行（场景 B）· **日期**: 2026-07-13
> **阶段**: Phase 2 探针切片「活着的村庄」·进度 `▓▓▓▓▓▓▓▓▓░ 9/10`
> **状态**: ✅ Sprint-074 完成·1136 tests 全绿·clippy/fmt 零警告·待提交

## 📊 本会话做了什么

1. **提案编写+两轮自审修正**——首轮发现 6 项问题（植被无渲染/ECS 未统一加速/ActionIntent缺失/GrowthNeeds缺失/架构分叉/点击逻辑位置模糊），全部在提案中修正
2. **用户裁决**——时间加速从全局热键移入 F3 控制台命令（调试功能集中管理）
3. **编码**——7 文件改动：`time.rs`(core) + `debug_console.rs` + `terrain_chunk.rs` + `entity_visual.rs` + 提案更新
4. **测试**——新增 9 tests（time_scale ×7 + entity_debug ×2）
5. **实机验证**——`speed 60` 全模拟统一加速，`info` 显示 12 section 完整快照，旁观者点击选中+高亮正常

### 架构决策

**模拟速度走控制台命令**（用户裁决）：`speed <value>` 命令设置 `WorldClock.time_scale`——通过 `ConsoleState.pending_time_scale` → WorldDriver 下帧消费（与 `pending_possess_request` 同模式）。速度档位常量 `[1.0, 10.0, 60.0]` 集中在 `cmd_speed` 函数内。

**`sim_delta` 在 ECS 块内计算**：因 `fn process()` 体量巨大、ECS 系统在深层嵌套块内，`sim_delta` 直接在 ECS 块顶部定义（`let sim_delta = delta * self.clock.time_scale as f64;`）。天气/大气系统在块外——使用内联 `delta * self.clock.time_scale as f64`。

**`entity_debug_system` 签名扩展**：新增 `inventory: Option<&InventoryRegistry>` 参数——console `info` 传 `None`（无 Inventory section），`#[func] get_inspect_data` 传 `Some(&self.inventory_registry)`。类型系统强制调用者显式处理。

## 📦 产物

**代码**（`woworld/`）：7 文件，+~275 行，+9 tests。1136 tests 全绿。

**文档**（`woworld-dev-plan/`）：DEVLOG、本 Handoff、冲刺提案（两轮审计修正）。

## 💾 恢复点（下一会话 AI 必读）

> ⚠️ 下一会话按 `00-流程总览` **场景 B（冲刺执行）** 启动——V5 已完成，下一步 **V6 快照存档**。

- **当前阶段**: Phase 2 切片·`9/10`（V0+V1+V4a+Vf+V2+V3a+V3b+V4b+V5 ✅）·10 步序列见 [[../../02-垂直切片/README]] §3
- **下一步**: **V6 快照存档**（第 10/10 步·最后一站）——bincode 序列化快照 → LMDB → 重载重建。**存档零代码**、首次引入 LMDB、**显式砍到 MVP**（不做 SaveableModule 14 方法/脏增量/迁移/崩溃恢复/多槽）。完成后写探针结论报告
- **机械门状态**: `1136 tests 全绿`（core 412 + worldgen 75 + atmosphere 26 + ecs 614 + integration 9），clippy/fmt 零警告，build 通过。
- **提交状态**: ⚠️ **未提交**。基线 = `ed123dc`（Sprint-073）。本会话改动待提交。
- **A1 铁律**: 纯涌现，禁脚手架/假坐标/占位驱动/平行 trait/语音决策 silo。
- **关键新增类型/函数**:
  - `WorldClock.time_scale: f32` + `set_time_scale(f32)` — `woworld_core::time`
  - `ConsoleState.pending_time_scale: Option<f32>` — `woworld_godot::debug_console`
  - `cmd_speed(args, state, world) -> String` — `woworld_godot::debug_console`（speed 命令）
  - `entity_debug_system(world, entity, inventory: Option<&InventoryRegistry>)` — 签名变更（新增参数）
  - `collect_creature_debug(world, entity, inventory, sections)` — 签名变更·5 个新 section
  - `WorldDriver.spectator_selected_entity: Option<hecs::Entity>` — 旁观者选中状态
  - `#[func] get_selected_entity_bits() -> i64` — Godot 轮询选中实体
  - `#[func] get_inspect_data(entity_bits: i64) -> GString` — JSON 格式快照（供未来面板）
- **数据流改变**:
  - 时间加速: `ConsoleState.pending_time_scale` → WorldDriver 消费 → `clock.set_time_scale()` → `advance()` 内部乘法 + `sim_delta` 乘数注入 ECS/天气
  - 旁观者点击: `process()` 非控制台路径 → `raycast_select` → `spectator_selected_entity` → highlight
  - 检视数据: `entity_debug_system` 现返回 12 section（曾 8）——Action/Wallet/Economy/Growth 为新 Component section，Inventory 为 Registry section
- **ECS 块内 `sim_delta` 位置**: `terrain_chunk.rs` ECS tick 块顶部（`// ── ECS tick — 全 17 System ──────────` 下方第一个 `{` 内）
- **console `speed` 命令**: 支持 `speed 60`（设置）/ `speed 0`（暂停）/ `speed`（帮助）/ `speed presets`（列出档位）

## ⚠️ 遗留 / 诚实边界

- **速度档位硬编码** `[1.0, 10.0, 60.0]`：debug_console 内常量——未来可 TOML 化
- **`get_inspect_data` 手写 JSON**：未引入 serde_json——Godot 面板需 `JSON.parse_string()` 消费
- **无独立 Godot 检视面板 `.tscn`**：控制台 `info` 命令提供同等 12 section 数据——Godot 面板属"好看"非"必需"
- **Goal 3 采集点标记未实现**：植被无 mesh 渲染管线（`BiomeVegetation` 纯数据 Provider）——旁观者通过 NPC `Goal.target_pos` 间接定位食物/水
- **不实现完整 EventBus**：同 Sprint-073——`EventChannel<TradeRecord>` 是最小实现

## 🔗 关联

- **提案**: [[../../sprint-proposals/sprint-074-V5-旁观工具-20260713]]
- **DEVLOG**: [[../devlogs/DEVLOG-2026-07-13-sprint074]]
- **上游**: [[handoff-20260713-sprint073]]（V4b 交易气泡）
- **路线图**: [[../../02-垂直切片/README]] · [[../../附录E-开发状态]]
- **下游**: V6 快照存档（bincode → LMDB → 重载重建·MVP·最后一站）
