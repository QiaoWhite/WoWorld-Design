# ECS-001: SeekSafety 否决静默掐断全村问候

> **模块**: ECS (npc/speech_bubble) · **类型**: 🟡 反直觉陷阱 · **状态**: ✅ 已修复（Sprint-068 同刺引入并修复，未 ship）
> **grep_keys**: greeting,问候,打招呼,speech_bubble,ActionIntent,veto,否决,SeekSafety,安全需求,social_total,needs累积,全村沉默,早期正常后突停

## 症状

实机里 NPC **早期偶尔问候，运行 ~6-10 秒后所有问候彻底停止**（`social_total` 计数器卡死不涨），但遭遇事件（`enter_total`）持续发生。屏幕上只剩"肚子饿了"自言自语，看不到任何问候/告别。

## 误诊路径（⚠️ 禁止重复尝试）

1. ❌ **以为是遭遇稀疏**（NPC 太散不相遇）→ 改 spawn 聚拢成"村庄"。**无效**——`enter_total` 本就在涨，遭遇一直有。
2. ❌ 以为是渲染问题（气泡没渲染）→ 排查 EntityVisual。**无效**——饥饿气泡正常渲染，管线没问题。
3. ❌ 以为是问候冷却（30s per-pair）→ **无效**，冷却过期后仍不问候。
4. ❌ 以为是朝向门/人格 occurrence 概率挡掉 → 概率门是随机的，不会 100% 硬停在某个计数。

**关键鉴别**：`social_total` **精确卡死在某值**（二值全否决），而非零星漏过——指向一个**随时间打开的二值门**，不是概率/稀疏问题。

## 根因

`speech_bubble` 的 `action_vetoes_greeting` 曾把 **`ActionCategory::SeekSafety`** 列入否决集（连同 Fight/Flee）。而 **安全需求（`Needs.safety`）像饥饿一样随时间只涨不消**（满足路径未做）→ `action_weight` 让**全部 NPC** 的主导 `ActionIntent` 收敛到 `SeekSafety`（实测 f600 起 20/20）→ 全村问候被否决。

**反直觉点**：`SeekSafety`（安全需求累积触发的"找个安全地儿"）**≠ 遇袭逃命**。在无威胁世界它只是环境需求，不该压制社交。把它当"逃跑"否决社交，是把 ActionCategory 的**两种语义**（环境需求 vs 主动逃命）混为一谈。

## 修复

`action_vetoes_greeting` 只保留 `Fight`/`Flee`（真实敌意/逃命）：
```rust
pub fn action_vetoes_greeting(cat: ActionCategory) -> bool {
    matches!(cat, ActionCategory::Fight | ActionCategory::Flee)
}
```
未来战斗就位后，`Flee` 自然覆盖"主动逃命"，无需再动此函数。

## 验证方法

- 单测 `speech_bubble::tests::test_action_veto`：断言 `SeekSafety` **不**否决。
- 实机：NPC needs 累积后（全 SeekSafety）问候仍持续（`social_total` 持续涨）。

## 前向教训（通用）

**用 `ActionIntent.category` 门控社交/表达行为时**：ambient 需求（安全/困倦等）会驱动出**看似战斗态但实为环境需求**的 category。只否决**真正主动的**敌意/逃命（Fight/Flee），别把环境需求当敌对状态。否则 needs 一累积，行为静默全失效，且**症状滞后**（早期 needs=0 正常 → 难定位）。
