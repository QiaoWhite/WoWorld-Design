# Handoff: 2026-07-13 — Sprint-073 V4b 交易气泡

> **会话类型**: 冲刺执行（场景 B）· **日期**: 2026-07-13
> **阶段**: Phase 2 探针切片「活着的村庄」·进度 `▓▓▓▓▓▓▓▓░░ 8/10`
> **状态**: ✅ Sprint-073 完成·1128 tests 全绿·clippy/fmt 零警告·待提交

## 📊 本会话做了什么

1. **V4b 提案编写**——两轮设计文档审计修正（EventChannel 对齐 `06-跨System通信`、priority 字段对齐 `UI与UX系统/005`）
2. **V4b 编码**——7 文件改动：`speech_bubble.rs`(core) + `speech_bubble_state.rs` + `speech_bubble.rs`(npc) + `economy/mod.rs` + `terrain_chunk.rs` + `speech_fragments.toml`
3. **测试**——新增 12 tests（core +5: TradeShout/priority · ecs +7: trade_bubble_system 专项）
4. **合规审计**——代码 ↔ 提案逐项核对，零偏离、零意外增补

### 架构决策

**成交事件走 `EventChannel<TradeRecord>`**（设计规定）：`market_matching_system` 改返回 `Vec<TradeRecord>` → WorldDriver Block A5 推入 `trade_events` → `mid_phase_flush` → 下帧视觉相位 `trade_bubble_system` 通过 `drain()` 消费。双缓冲跨帧——≤1 帧滞后，同 EncounterEvent 模式。

**`ActiveBubble` 加 `priority: u8`**（设计规定）：三级常量 `PRIORITY_SOCIAL=3` > `PRIORITY_TRADE=2` > `PRIORITY_SELF_TALK=1`。`speech_bubble_system` Pass 2 隐式 `is_some()` → 显式 `priority > PRIORITY_SELF_TALK` 比较（行为等价）。Pass 1 无条件覆盖保持不变（greeting=3 ≥ 任何现有）。

**`trade_bubble_system` 薄函数**——仅 ~60 行，复用 V4a 全部基础设施（`SpeechBubbleState`·`SpeechFragmentRegistry`·`ActiveBubble`·TOML 片段选择）。

## 📦 产物

**代码**（`woworld/`）：7 文件，+~150 行，+12 tests。

**文档**（`woworld-dev-plan/`）：DEVLOG、本 Handoff、冲刺提案。

## 💾 恢复点（下一会话 AI 必读）

> ⚠️ 下一会话按 `00-流程总览` **场景 B（冲刺执行）** 启动——V4b 已完成，下一步 **V5 旁观工具**。

- **当前阶段**: Phase 2 切片·`8/10`（V0+V1+V4a+Vf+V2+V3a+V3b+V4b ✅）·10 步序列见 [[../../02-垂直切片/README]] §3
- **下一步**: **V5 旁观工具**（第 9/10 步）——需求满足点场景 + 时间加速 + 点击 NPC 看需求/意图/库存/钱包。依赖：V1-V4 全就绪。21 NPC 可视化已在，缺场景编排 + inspect UI
- **机械门状态**: `1128 tests 全绿`（core 405 + worldgen 75 + atmosphere 26 + ecs 613 + integration 9），clippy/fmt 零警告，build 通过。
- **提交状态**: ⚠️ **未提交**。基线 = `11fe1f5`（Sprint-072）。本会话改动待提交。
- **A1 铁律**: 纯涌现，禁脚手架/假坐标/占位驱动/平行 trait/语音决策 silo。
- **关键新增类型/函数**:
  - `PRIORITY_SOCIAL: u8 = 3` / `PRIORITY_TRADE: u8 = 2` / `PRIORITY_SELF_TALK: u8 = 1` — `woworld_core::speech_bubble`
  - `SpeechAct::TradeShout` — 新语义变体（from_key: `"trade_shout"`, default_bubble_type: Normal）
  - `ActiveBubble { priority: u8 }` — 新字段（设计 `UI与UX系统/005`）
  - `trade_bubble_system(world, trade_events, state, fragments, tick, day_progress, player_entity)` — `woworld_ecs::systems::economy`
  - `market_matching_system` 签名: `-> Vec<TradeRecord>`（曾 `()`）
  - `BUBBLE_DURATION_TICKS` / `BUBBLE_COOLDOWN_TICKS` 改为 `pub(crate)`
- **数据流改变**:
  - 成交事件: `market_matching_system` → `EventChannel<TradeRecord>` (Block A5 send_all+flush) → `trade_bubble_system` (下帧视觉相位 drain)
  - 气泡优先级: greeting(3·无条件覆盖) > trade(2·priority 比较覆盖) > self-talk(1·不覆盖高优先级)
  - `speech_bubble_system` Pass 2: `is_some()` → `priority > PRIORITY_SELF_TALK` (行为等价)
- **WorldDriver 新字段**: `trade_events: EventChannel<TradeRecord>` — `begin_frame`(Block A0) · `send_all+mid_phase_flush`(Block A5) · `drain`(视觉相位)
- **TOML 新片段**: 8 条 `trade_shout`——`SpeechFragmentRegistry` 通过 `SpeechAct::from_key("trade_shout")` 加载

## ⚠️ 遗留 / 诚实边界

- **不实现完整 EventBus**：`EventChannel<TradeRecord>` 是 `06-跨System通信` `EventBus { trade_events }` 的最小实现——仅含 trade 事件，不建通用总线。Phase 3 EventBus 就位后迁移
- **priority 三级制**：设计 `UI与UX系统/005` `SpeechBubbleEvent { emotion, priority }` 的 `emotion` 字段代以 `bubble_type`（MVP 简化·CHG-066）
- **不实现"最大5气泡同时"**：设计 `UI与UX系统/002` 规定上限 5——村庄 ~3-5 NPC 不触发边界
- **交易吆喝 ≠ 交易对话**：后者属 `语言表达/006` DialogueIntent 体系，Phase 3+

## 🔗 关联

- **提案**: [[../../sprint-proposals/sprint-073-V4b-交易气泡-20260713]]
- **DEVLOG**: [[../devlogs/DEVLOG-2026-07-13-sprint073]]
- **上游**: [[handoff-20260712-sprint072]]（V3b 市场接真）
- **路线图**: [[../../02-垂直切片/README]] · [[../../附录E-开发状态]]
- **下游**: V5 旁观工具（需求满足点场景 + 时间加速 + NPC inspect UI）
