# Handoff: 2026-07-08 — Sprint-061

> **冲刺**: Sprint-061 — 对话雏形 MVP: NPC 头顶自言自语气泡
> **日期**: 2026-07-08
> **阶段**: Phase 1 — 核心基础
> **冲刺状态**: ✅ 完成

## 📊 冲刺回顾

### 目标达成
| 目标 | 状态 | 备注 |
|------|------|------|
| 目标 1: core 数据合同 + BubbleType 颜色 | ✅ | woworld_core/speech_bubble.rs, EntityVisual +2字段, 6 tests |
| 目标 2: SpeechBubbleState + speech_bubble_system | ✅ | Resource + system(桩化文本/prng错峰/retain防泄漏), 16 tests |
| 目标 3: Godot EntityRenderer 气泡渲染 | ✅ | EntityNode.bubble_label + terrain_chunk 调度接入 |

### 关键决策
- **pull 替代 signal push**: 设计 005 规定 signal push,但会违反宪法 §2。改用现有 pull 管线(entity_visual_system → EntityRenderer::sync),复用名字系统 Label3D+billboard 范式,逻辑全留 Rust。
- **术语消歧(CHG-066)**: 审计发现 "Bark" 被 UI 气泡(眼睛)和音频语声(耳朵)两个正交概念重载。UI 侧改 `speech_bubble`/`BubbleType`,音频侧保留 `BarkType`(12类语声,不变)。
- **调度顺序**: speech_bubble_system 在 entity_visual_system 之前(同帧写→同帧读,避免滞后一帧),不放 ECS Block A1-A5。

## 💾 恢复点（下一会话 AI 必读）

- **当前冲刺**: Sprint-061 — ✅ 完成
- **当前目标**: 全部完成(3/3)
- **最后操作**: 冲刺完成 → commit `842e712` + `fd3853c` 均已 push 到 remote master。用户手动 Godot 验证通过。
- **机械门状态**: build ✅ / test 807 ✅ / clippy 零警告 ✅ / fmt clean ✅
- **提交状态**: ✅ 已 push（`842e712` 主冲刺 + `fd3853c` 日志频率修复）。工作区干净（仅 .obsidian 噪音）。
- **下一步**: 开新冲刺——候选见文末（🥇 NPC-NPC双向对话 / 经济 Phase 4 / 物品 Phase 3）
- **已知陷阱**:
  - ⚠️ 桩化文本硬编码在 `speech_bubble.rs::pick_bubble`。接入 `概念与语言地基/003` TextGenerator 时替换此函数,不要在此堆砌更多短语。
  - ⚠️ `BubbleType` 是渲染颜色分类,不是 005 数据合同的语义类型(那是 `emotion` 字段)。未来由 emotion 驱动取代。
  - ⚠️ 被夺舍 NPC 不冒泡(speech_bubble_system 跳过 player_entity)——这是有意的,退出夺舍时防残留气泡闪现。
  - ⚠️ 音频系统 `BarkType`(12类语声)与 UI `BubbleType` 是两回事。改任一方勿混淆。见 CHG-066。
  - ⚠️ 气泡仅在 render_lod < 2（lod 0/1）且距离 < 50m 且满足触发条件（needs 阈值/情绪极值/goal urgency）时显示,还有 5% 概率门错峰。lod=2 的 NPC 不冒泡（有意）。
  - ⚠️ 气泡外观是**纯文字 + 黑描边**（Label3D，y=2.7），无背景框——这是用户确认的 MVP 选择,非缺陷。带背景框（002 的黑80%底气泡框）需 Sprite3D，是明确延后项。
- **用户验证结果**: ✅ 手动 Godot 验证通过（用户确认"一切正常"）。诊断日志频率过高已修（60→600 帧,`fd3853c`）。

## 🔧 机械门验证

### cargo test
```
core: 292 + worldgen: 58 + atmosphere: 26 + ecs: 431 + godot: 0 = 807 passed; 0 failed
```
(783 → 807, +24: BubbleType 6 + speech_bubble_state 3 + speech_bubble_system 13,减去部分统计口径)

### cargo clippy
```
cargo clippy --workspace -- -D warnings → Finished, 零警告
```

### cargo fmt
```
cargo fmt --all --check → exit 0 (clean)
```

### cargo build
```
cargo build --workspace → DLL 已更新
```

## 📐 设计门验证

### A. 主清单
| # | 检查项 | 状态 |
|---|--------|------|
| 1 | trait 签名与 CLAUDE-INTERFACES.md 一致 | ✅ 无新增 trait |
| 2 | ID/共享类型定义在 woworld_core | ✅ BubbleType 在 woworld_core::speech_bubble |
| 3 | Godot/GDScript 无游戏逻辑 | ✅ 气泡逻辑全在 Rust,Godot 仅 Label3D 表现 |
| 4 | 公开类型已登记 | ✅ BubbleType 在 prelude + CHG-066 |
| 5 | 设计决策已记录 | ✅ CHG-066 + DEVLOG + 本 Handoff |

### B. ECS 铁律合规
| # | 检查项 | 状态 |
|---|--------|------|
| 6 | Component = 纯数据 | ✅ 本冲刺无新 Component(用 Resource) |
| 7 | 无堆数据内联 Component | ✅ ActiveBubble.text 是 String,但存 Resource(SpeechBubbleState)非 Component,合规 |
| 8 | 'static + Send + Sync | ✅ |
| 9 | Entity 删除走标记 | ✅ 本冲刺不删 entity;retain 剔除 despawn 的 slot |
| 10 | System writes 无交集 | ✅ speech_bubble_system 独占写 SpeechBubbleState |
| 11 | hecs::World 仅 WorldDriver | ✅ |
| 12 | 每个 System ≥1 测试 | ✅ speech_bubble_system 13 tests |

### C. 架构边界审计
| # | 检查项 | 状态 |
|---|--------|------|
| 13 | 双权威检测 | ✅ 气泡状态唯一权威 SpeechBubbleState |
| 14 | 僵尸代码检测 | ✅ 新增函数均有调用 |
| 15 | GDScript 无数学公式 | ✅ player.gd 未改 |

## 📁 文件变更

### 新建 (5)
```
woworld/crates/woworld_core/src/speech_bubble.rs
woworld/crates/woworld_ecs/src/resources/speech_bubble_state.rs
woworld/crates/woworld_ecs/src/systems/npc/speech_bubble.rs
WoWorld-Design/Change/CHG-066-对话气泡术语消歧-20260708.md
woworld-dev-plan/01-核心基础/devlogs/DEVLOG-2026-07-08-sprint061.md
```

### 修改 (代码 7)
```
woworld_core/src/lib.rs                    (+speech_bubble 模块 +prelude)
woworld_core/src/entity_visual.rs          (EntityVisual +bubble_text/bubble_color +3测试构造)
woworld_ecs/src/resources/mod.rs           (+speech_bubble_state)
woworld_ecs/src/systems/npc/mod.rs         (+speech_bubble)
woworld_ecs/src/systems/entity_visual.rs   (+bubble_state 参数 +填充 +7测试调用)
woworld_godot/src/entity_renderer.rs       (EntityNode.bubble_label + create_node + sync)
woworld_godot/src/terrain_chunk.rs         (bubble_state 字段 + speech_bubble_system 调用)
```

### 修改 (设计文档 6 — 术语消歧)
```
UI与UX系统/002-HUD与常驻界面.md            (§Bark气泡→对话气泡 + 消歧注)
UI与UX系统/005-跨模块接口与性能预算.md      (BarkEvent→SpeechBubbleEvent + 消歧注 + 预算措辞)
UI与UX系统/001-信息架构与设计哲学.md        (3处 Bark→对话气泡)
UI与UX系统/003-对话与交互界面.md            (1处 Bark气泡→对话气泡)
UI与UX系统/README.md                        (5处 Bark→对话气泡)
woworld-dev-plan/附录E-开发状态.md          (状态表 + 冲刺历史 + 交接摘要 + 语言表达行)
```

## 🖐 手动验证清单（✅ 用户已执行,确认"一切正常"）

用 `_console.exe` 启动 Godot 看 stdout:
1. NPC 头顶陆续自发浮现彩色气泡(饿=蓝灰"肚子饿了…"、开心=黄"今天心情不错"等),错峰不刷屏。
2. 气泡跟随 NPC 移动、面向相机、超 50m 消失。
3. 黑描边使白/黄字在任意地形背景可读。
4. Tab 夺舍某 NPC → 该 NPC 无自言自语气泡;F 退出 → 无残留旧气泡闪现。

## ⚠️ 已知问题
| # | 问题 | 级别 | 计划 |
|---|------|------|------|
| 1 | 桩化文本硬编码,内容单调 | 🟢 MVP 有意 | 接入 TextGenerator(概念驱动)后替换 |
| 2 | 无 0.3s alpha 淡出(硬 show/hide) | 🟢 MVP 有意 | 方案已备(ActiveBubble+alpha),按需实现 |
| 3 | 无 NPC-NPC 双向对话 | 🟢 MVP 范围外 | 下一冲刺候选 |
| 4 | 无同屏5个排队/屏幕边缘箭头/点击转注意力 | 🟢 MVP 范围外 | 延后 |
| ✅ | 诊断日志每秒刷屏 | 已修 | `fd3853c` 频率 60→600 帧 |
| — | godot-rust API v4.6/runtime v4.7 strict + pdb 重命名 warning | 🟢 无害 | godot 0.5 既有,非本冲刺,不改 |

## 🚀 下一步候选
| 候选 | 内容 | 优先级 |
|------|------|--------|
| NPC-NPC 双向对话 | AmbientConversation + ConversationVisibility 4级 + 玩家旁观 | 🥇 承接本冲刺 |
| 物品 Phase 3 | Assembly + ItemEntId 迁移(横切,建议拆两冲刺) | 🥈 |
| 经济 Phase 4 | 行为经济学接线 + 分层定价(自包含,低风险) | 🥈 |
| 玩家 Phase 2 | DomainDelegated(需先做 Action 词汇表设计冲刺) | 🥉 |

**建议**: NPC-NPC 双向对话——气泡渲染管线已就位,加对话内容源即可让世界"对话起来"。或经济 Phase 4(自包含低风险,行为函数已写好只差接线)。
