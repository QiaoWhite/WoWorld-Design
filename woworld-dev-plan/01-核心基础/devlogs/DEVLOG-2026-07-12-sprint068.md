# DEVLOG 2026-07-12 — Sprint-068 V4a 问候/情绪气泡

> **冲刺**: Sprint-068 — V4a 问候/情绪气泡（垂直切片「活着的村庄」第 3/10 步）
> **状态**: ✅ 完成·~1075 tests 全绿·clippy/fmt 零警告·实机验证问候可用
> **提案**: [[../../sprint-proposals/sprint-068-V4a-问候情绪气泡-20260712]]（5 项裁决 D1-D5 + 4 轮审查定稿）

## 做了什么

把 Sprint-061 的**硬编码自言自语气泡**升级为**遭遇驱动的问候/告别 + 数据驱动选句**，且**接既有行动涌现**（barrier-free）。经**8+ 轮设计拷问**（哲学定位/barrier-free/10 处审查缺口/5 项裁决）后编码。

### 产物（代码）

| 文件 | 内容 |
|------|------|
| `core/speech_bubble.rs` | +`SpeechAct{Greeting,Farewell,NeedMutter,EmotionVent}`（无 serde·⊥ BubbleType）+ `BubbleType::from_key` |
| `assets/speech_fragments.toml` | 片段库（问候/告别/需求嘟囔/情绪宣泄）+ `FragmentCondition`（time_of_day/trust/pleasure/extraversion/topic）+ 声明式 `social_effect` |
| `ecs/resources/speech_fragment_registry.rs` | `SpeechFragmentRegistry` — TOML 加载 + 条件过滤 + 概率加权选句（003「片段组合」子集） |
| `ecs/systems/npc/encounter.rs` | `neighbors_within` 原语 + `encounter_system`（迟滞 3m/4m + 首帧播种 + despawn 归因 + XZ 距离） |
| `ecs/resources/encounter_state.rs` | `EncounterState`/`EncounterEvent`（Enter/Leave·per-pair 问候冷却） |
| `ecs/systems/npc/speech_bubble.rs` | 重写——Pass1 遭遇问候/告别（ActionIntent 否决+朝向门+人格 occurrence+单槽仲裁）+ Pass2 数据驱动自言自语；`classify_self_talk` 纯函数 |
| `ecs/systems/npc/social.rs` | 重构调用 `neighbors_within`（行为等价·13 测试守门） |
| `godot/terrain_chunk.rs` | 挂 `encounter_system`（逻辑相位·movement/action_weight 后）+ speech 接线（EncounterState/registry/relations/day_progress） |

### 架构定位（barrier-free）

```
感知层  encounter_system → EncounterEvent（通用·未来战斗/记忆复用）
   ↓
行动涌现  既有 ActionWeight → ActionIntent（Socialize/Fight/Flee 平级竞争）
   ↓
表达层  speech_bubble = 表达渲染器（Socialize→问候；Fight/Flee 否决）
```
**问候是"社交行动的表达"非"语音决策"**——打斗/沉默/单方不回应皆经同一 `ActionCategory` 涌现，零壁障、零语音决策 silo。

## 实机验证 + 修 bug

实机跑发现问候早期正常后**突停**。诊断（加临时计数器 + ActionIntent 直方图）定位 **[ECS-001](../../bugs/ECS/ECS-001-seeksafety-veto-silences-greetings.md)**：`SeekSafety` 被误列入否决集，而安全需求随时间累积让**全部 NPC → SeekSafety** → 全村问候被否决。**修**：否决收窄为 `Fight`/`Flee`（真敌意/逃命；SeekSafety=环境需求≠逃命）。修后 `social_total` 持续涨、问候涌现可见。

诊断脚手架（spawn 硬编码"村庄"+ DIAG 计数器）**已全部回退/移除**——唯一永久改动 = SeekSafety 否决收窄。

## 涌现纪律（A1）

- 词是预写片段（设计 003 规格·非造词），**涌现在「选择」**：人格×情绪×关系×时段→不同候选。
- 问候**非硬映射**——是否/如何问候从 personality×mood + ActionIntent 涌现（外向热情/内向沉默/忙碌无视）。
- 无脚手架/假坐标/占位驱动/语音决策 silo；表达层不改关系（D4·社会效果归 social_system）。

## 诚实边界（探针结论素材）

- **实机世界显荒凉**：needs 累积→全 NPC `SeekSafety`→无目标四散游荡。真因 = 安全需求无满足路径（V3a/防护未做）+ 无 worldgen 聚落 + 无 V2 牵引移动。**这是切片未完成的诚实空白**，探针价值正在暴露它。
- `SeekSafety` 一统全村或指向 🟢 需求权重失衡（安全压过饥饿太快）——超 V4a 范围，记待评估。
- 完整 003 引擎（CompositeTemplate/填槽/LLM）、真语言生成、SpatialGrid 收敛、`social_effect` 施加、记忆形成、非语言联动、LOD 门控——皆记为长线接缝，本冲刺不做。

## 自检门

`cargo build/test/clippy -D warnings/fmt --check` 全绿。~1075 tests（core 401 + ecs 590 + worldgen 58 + atmosphere 26）。

## 关联

- 提案 [[../../sprint-proposals/sprint-068-V4a-问候情绪气泡-20260712]] · Bug [[../../bugs/ECS/ECS-001-seeksafety-veto-silences-greetings]]
- 上游 [[handoff-20260712-sprint067]] · 切片 [[../../02-垂直切片/README]] · [[../../附录E-开发状态]]
