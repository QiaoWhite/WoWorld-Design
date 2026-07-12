# DEVLOG: 2026-07-13 — Sprint-073 V4b 交易气泡

> **冲刺**: Sprint-073 — V4b 交易气泡·成交事件→吆喝气泡·薄出口
> **日期**: 2026-07-13
> **阶段**: Phase 2 探针切片「活着的村庄」·进度 `▓▓▓▓▓▓▓▓░░ 7→8/10`

## 做了什么

将市场撮合成交事件接入 V4a 气泡框架，成交后买卖双方 NPC 头顶冒交易吆喝气泡：

1. **成交事件走 `EventChannel<TradeRecord>`**（设计 `06-跨System通信` §通道3）
   - `market_matching_system` 签名从 `()` 改为 `-> Vec<TradeRecord>`
   - Block A5 推入 `trade_events.send_all(trades)` + `mid_phase_flush`
   - 下帧视觉相位 `trade_bubble_system` 通过 `drain()` 消费（双缓冲跨帧）

2. **`ActiveBubble` 加 `priority: u8` 字段**（设计 `UI与UX系统/005` `SpeechBubbleEvent { priority }`）
   - 三级优先级常量：`PRIORITY_SOCIAL=3` > `PRIORITY_TRADE=2` > `PRIORITY_SELF_TALK=1`
   - `speech_bubble_system` Pass 2 从 `is_some()` 改为 `priority > PRIORITY_SELF_TALK` 比较（行为等价）
   - 仲裁规则：greeting(3) 可抢占一切 / trade(2) 可抢占 self-talk(1)，不覆盖 greeting(3)

3. **新增 `SpeechAct::TradeShout`** + `trade_bubble_system` 薄函数（~60 行）
   - 消费 `EventChannel<TradeRecord>::drain()` → 买卖双方独立选句（per-entity seed·涌现非对称）
   - Player 排除、实体存在性检查、cooldown 约束、TOML 片段库选句

4. **TOML 片段** — 8 条交易吆喝（外向/内向/开心变体）

5. **WorldDriver 接线**
   - `trade_events: EventChannel<TradeRecord>` 字段
   - Block A0 `begin_frame` → Block A5 `send_all+mid_phase_flush` → 视觉相位 `trade_bubble_system`

## 机械门

- **1128 tests 全绿** (+12: core +5, ecs +7)
- clippy 零警告
- fmt 通过
- build 通过

## 关键文件

| 文件 | 改动 |
|------|------|
| `woworld_core/src/speech_bubble.rs` | +PRIORITY_* 常量 + TradeShout 变体 + from_key/default_bubble_type + tests |
| `woworld_ecs/src/resources/speech_bubble_state.rs` | ActiveBubble +priority: u8 |
| `woworld_ecs/src/systems/npc/speech_bubble.rs` | pub(crate) 常量 + Pass 2 priority 仲裁 |
| `woworld_ecs/src/systems/economy/mod.rs` | market_matching_system 返回 Vec<TradeRecord> + trade_bubble_system + tests |
| `woworld_godot/src/terrain_chunk.rs` | trade_events 字段 + EventChannel 接线 |
| `woworld/assets/speech_fragments.toml` | +8 trade_shout 片段 |

## 诚实边界

- `EventChannel` 而非完整 `EventBus`——设计 `06-跨System通信` 的 `EventBus { trade_events }` 的最小实现，仅含 `Vec<TradeRecord>`
- priority 三级制而非完整 `SpeechBubbleEvent` 合同——`emotion` 字段代以 `bubble_type`（MVP 简化·CHG-066）
- 不实现"最大5气泡同时"限制（`UI与UX系统/002`）——村庄 ~3-5 NPC 不触发边界
- 交易吆喝 ≠ 讨价还价对话——后者属 `语言表达/006` DialogueIntent 体系，Phase 3+
