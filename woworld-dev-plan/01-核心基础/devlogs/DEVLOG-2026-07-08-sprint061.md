# DEVLOG: 2026-07-08（Sprint-061 对话气泡 MVP）

> **冲刺**: Sprint-061 — 对话雏形 MVP: NPC 头顶自言自语气泡
> **阶段**: Phase 1 — 核心基础

## 今日目标
- [x] core 数据合同 + BubbleType 颜色
- [x] SpeechBubbleState Resource + speech_bubble_system
- [x] Godot EntityRenderer 气泡渲染集成
- [x] 术语消歧(审计发现) + 文档收尾

## 做了什么

### 规划(三轮探查 + 决策矩阵)
- 交接推荐"对话雏形"为 🥇。派 5 个探查代理并行核实:桥接架构、BarkEvent 契约、ECS 调度、NPC 状态、godot-rust API。
- 决策矩阵测算四候选:对话雏形 46 ≈ 经济P4 46 > 物品P3 30 > 玩家P2 21。用户选定对话雏形。
- MVP 三项范围确认:单NPC自言自语 + Label3D 纯文字描边 + 5类颜色。

### 实现(pull 管线,非 signal push)
- **架构决策**:设计文档 005 规定 signal push 到 GDScript,但会违反宪法 §2。改用现有 pull 管线(entity_visual_system → EntityRenderer::sync),复用名字系统的 Label3D + billboard 范式。
- `woworld_core::speech_bubble`: `BubbleType`(Normal/Emotion/Ambient/Quest/Damage) + `color()`。EntityVisual 加 `bubble_text`/`bubble_color`。
- `woworld_ecs`: `SpeechBubbleState` Resource(BubbleSlot/ActiveBubble,跨帧 duration+cooldown)+ `speech_bubble_system`(pick_bubble 纯函数从 needs/emotion/goal 生成桩化文本,prng 错峰,retain 防泄漏)。
- `woworld_godot`: EntityNode 加 bubble_label(y=2.7,黑描边,width 300 autowrap)。terrain_chunk 加 bubble_state 字段 + 在 entity_visual_system 前调 speech_bubble_system。

## 遇到的问题

### 🔴 术语冲突(审计发现,关键)
用户要求审计计划。我直接精读权威设计文档(不依赖规划代理二手转述——它们对 BarkEvent 字段给了互相矛盾的报告),发现:
- **`BarkType` 命名冲突**:我原计划的 UI `BarkType`(5类)与 `音频系统/007` 权威的 `BarkType`(12类语声)将来必在 core 真冲突。
- **深层问题**:"Bark" 在项目里被两个正交概念重载——UI 文字气泡(眼睛看)vs 音频语声(耳朵听)。不是"同一概念不同抽象层",派生原则不适用。
- **我的偏差**:① 抢占了音频系统的 `BarkType` 坑位;② `Combat` 变体是脑补,文档原文是"伤害反应"→ 应为 `Damage`;③ 把渲染颜色表当数据合同类型。

**解决**:UI 侧全改 `speech_bubble`/`BubbleType` 词根 + `Damage`。开 CHG-066 记录消歧,同步 5 个 UI 文档("Bark 气泡"→"对话气泡",`BarkEvent`→`SpeechBubbleEvent`,加消歧注)。音频 `BarkType` 保持不变。

### 调度顺序陷阱(验证代理纠正)
草稿想把 speech_bubble_system 放进 Block A1,但 entity_visual_system 在 process 顶部(line 1193)、ECS Block 在 1683+。放 A1 会导致气泡滞后一帧。改为在 entity_visual_system 前独立调用。

## 学到的东西
- **审计必须回到一手源**:两个规划探查代理对同一个 BarkEvent 结构给了矛盾报告(一个 emotion/priority,一个 bark_type/world_position)。只有亲自读设计文档才定案。二手转述不可全信。
- **"复用现有名字"是命名冲突信号**:当新类型想叫一个已被别的模块钦定的名字时,先 grep 全设计文档确认所有权,别急着占坑。
- godot 0.5.3 Label3D 描边 API:`set_outline_size(i32)` + `set_outline_modulate(Color)`,outline ≈ font_size/3 防糊。

## 机械门
- 测试 807 全绿(783→807, +24):core 292 / ecs 431 / worldgen 58 / atmosphere 26
- clippy 文档门零警告 · build DLL 更新 · fmt clean

## 明日计划
- [ ] 手动 Godot 验证气泡渲染(启动看 NPC 冒泡、跟随、50m 裁剪、夺舍无残留)
- [ ] 下一冲刺候选:NPC-NPC 双向对话 / 物品 Phase 3 / 经济 Phase 4
