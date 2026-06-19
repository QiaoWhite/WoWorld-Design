> **变更编号**: CHG-019
> **日期**: 2026-06-13
> **关联**: [[../Happy Game/开发阶段/语言表达/README|语言表达系统 v2.0]] · [[../Happy Game/开发阶段/语言表达/011-LLM增强层|LLM增强层]] · [[../Happy Game/开发阶段/语言表达/012-语音输出接口|语音输出接口]]
> **前序变更**: [[CHG-018-语言表达系统v1.1完善-信息传播非语言联动记忆消化-20260613|CHG-018（语言表达 v1.1）]]

# CHG-019 -- LLM增强层与语音输出接口 v2.0 创建

## 概述

为语言表达模块创建 **LLM增强层**（011）和 **语音输出接口**（012），将语言表达系统升级至 v2.0。LLM增强层是模板管道的可选装饰器--玩家启用时提供 LLM 增强输出，未启用/不可用时无缝回退模板。语音输出接口（TTS）是语言表达管道的输出端，与 STT 输入端对称，不依赖 LLM。

**核心原则**：
- LLM 是锦上花，模板永远是 fallback
- 场景粒度开关--玩家按 19 个场景自由选择
- 后端无关--本地部署（5种）和云端 API（6种）统一抽象
- NPC 自主决定对话深度--不由系统设信任硬阈值
- TTS 与 LLM 无关--模板管道同样可用语音输出

## 变更内容

### 新建文件

| 文件 | 所属模块 | 核心内容 |
|------|---------|---------|
| 011-LLM增强层.md | 语言表达 | 装饰器架构·场景粒度开关(19场景)·LlmBackend trait(本地5+云端6+Mock)·LlmBackendRegistry多后端管理+故障转移·LlmRequest/LlmResponse通用格式·NPC自主对话意愿(NpcDialogueWillingness六维)·PersonalityPromptBuilder·旅伴对话(多人+景色+时事)·灵活可闻半径(EffectiveAudibleRadius六因子)·沉默模型·多人LLM·NPC间LLM策略(LlmPriority五级)·对话取消全时机(12种)·安全边界 |
| 012-语音输出接口.md | 语言表达 | VoiceProfile(NPC音色身份·物理+个性)·VoiceEmotionModulation(情绪到音高/语速/音量/颤抖)·VoicePacket(Rust到Godot统一语音包·5种delivery·5级priority)·TtsEngine trait(5种后端:系统/本地AI/云端API/预录制/无)·TtsConfig(玩家开关·音量·字幕模式)·VoiceManager·各管道集成 |

### 一、LLM增强层关键设计

#### 1.1 场景粒度开关（LlmSceneConfig）

20 个独立场景开关 + master_switch 总开关：

| 场景分组 | 场景 | 说明 |
|---------|------|------|
| 对话场景（7个） | deep_dialogue / normal_dialogue / combat_dialogue / phatic_greetings / npc_to_npc_dialogue / travel_companion_chat / multi_party_dialogue | 深度对话/普通对话/战斗对话/快速问候/NPC间对话/旅伴对话/多人对话(3+) |
| 文本场景（3个） | book_text_polish / inscription_text / letter_writing | 书籍润色/铭文碑文/NPC写信 |
| 叙事场景（3个） | journal_narrative / battle_narrative / historical_retelling | 大日志叙事化/战斗叙事/历史口述 |
| 角色场景（3个） | npc_background_story / npc_diary / bard_composition | NPC背景故事/NPC日记/吟游诗歌 |
| 世界场景（4个） | tree_reading / ruin_description / weather_atmosphere / item_history | 读树/遗迹探索/天气描写/物品历史 |

#### 1.2 LLM 后端抽象（LlmBackend trait）

```
本地部署 5 种：LocalOllama / LocalLlamaCpp / LocalVllm / LocalTextGenWebui / LocalOpenAiCompatible
云端 API 6 种：CloudOpenAi / CloudAnthropic / CloudGroq / CloudTogether / CloudOpenRouter / CloudCustom
特殊：Mock（测试）
```

多后端管理（LlmBackendRegistry）：场景路由 -> 默认后端 -> 故障转移（第一个健康的）。

LlmRequest/LlmResponse 后端无关通用格式--不绑定任何特定 LLM。

#### 1.3 NPC 自主对话意愿（NpcDialogueWillingness）

六维模型，不设系统信任硬阈值：

| 维度 | 公式 |
|------|------|
| 个性驱动 | openness x 0.30 + extraversion x 0.20 + agreeableness x 0.10 |
| 情绪驱动 | (0.15 - arousal_penalty + pleasure_bonus).max(0.0) |
| 关系驱动 | trust x 0.20（连续值，非阈值） |
| 话题驱动 | 亲身经历/专业领域高兴趣 |
| 情境驱动 | 旅途中+0.10，危险中-0.20 |
| 文化驱动 | 直接性+表达性 |

willingness 驱动 LLM 回应深度，而非调用开关：
- >0.7: "Elaborate freely, share personal stories."
- >0.4: "Engage but don't overshare."
- >0.15: "Keep responses short and guarded."
- <0.15: 不调用 LLM--模板足够

#### 1.4 旅伴多人对话

四人触发源（环境景色/时事流言/沉默时间/随机自发）+ SpeakUrge 竞争 + TravelUtteranceStyle（自言自语/对全体/对特定/续前）。

非LLM模式必须工作--multi_travel_turn() 不依赖 LLM，始终产出 ConversationTopic，TopicResolver 用模板生成文本。

#### 1.5 灵活可闻半径（EffectiveAudibleRadius）

> **[CHG-030 所有权迁移]**: effective_audible_radius() 的声学公式定义后续迁至音频模块。本处为消费侧引用。

六因子公式替代固定米数：

```
effective_radius = base_radius
    x intentional_factor    (Normal=1.0, Loud=1.5, Whisper=0.15, Conspiracy=0.08)
    x env_noise_factor      (Silent=1.5, Market=0.5)
    x terrain_factor        (OpenPlain=1.3, Forest=0.7, Canyon=1.5)
    x weather_factor        (雨x0.8, 风x0.7, 雷x0.5)
    x culture_factor        (1.0 + personal_space x 0.1)
    x personality_factor    (1.0 + (extraversion-0.5) x 0.3)
```

### 二、语音输出接口关键设计

#### 2.1 VoiceProfile -- NPC 语音身份

> **[CHG-030 所有权迁移 -- 2026-06-18]**: VoiceProfile 的声学参数定义已迁移至音频模块（005-语音管道）。本模块保留 TTS 渲染接口。当前权威定义参见音频模块 005。

每个 NPC 有一个 VoiceProfile，生成时确定：

| 参数类别 | 字段 | 来源 |
|---------|------|------|
| 物理音色 | base_pitch / pitch_variance / timbre / base_speed | 种族 x 性别 x 年龄 |
| 个性风格 | expressiveness / breathiness / roughness | 大五人格派生（extraversion x 0.7 + openness x 0.3 等） |

#### 2.2 语音情绪修饰（VoiceEmotionModulation）

每轮发言时基于 NPC 当前情绪动态调整：

| 参数 | 公式 |
|------|------|
| pitch_shift | joy x 2.0 - sadness x 3.0 + anger x 1.5 |
| speed_multiplier | 1.0 + anger x 0.3 + fear x 0.5 - sadness x 0.3 |
| volume_multiplier | 1.0 + anger x 0.4 - fear x 0.4 - sadness x 0.2 |
| tremor | fear x 0.6 + sadness x 0.3 |
| breathiness_add | sadness x 0.3 |

#### 2.3 TTS 后端抽象（TtsEngine trait）

五种后端，与 LLM 无关--模板管道同样可用：

```
System:      系统 TTS（Windows SAPI / macOS / Linux）
LocalAI:     本地 AI（piper-tts / Coqui / XTTS）
CloudApi:    云端 API（ElevenLabs / Azure / OpenAI TTS）
PreRecorded: 固定短语预录音
None:        无 TTS--仅文字
```

#### 2.4 语音优先级（VoicePriority）

五级排队+打断仲裁：

| 优先级 | 场景 | 行为 |
|--------|------|------|
| Critical | 战斗警告/求救 | 打断当前语音 |
| High | 回应玩家的对话 | 排队在前 |
| Normal | 一般对话 | 正常排队 |
| Ambient | NPC间闲聊 | 排队在后 |
| Background | 背景旁白 | 最后播放 |

### 三、LLM 安全边界

- LLM 永远不能直接修改游戏状态--只能产出文本
- ActionLibrary 封闭--LLM 只能选预定义行动 ID
- EntityValidator--引用必须存在
- PersonalityValidator--不违反核心人格
- ContentFilter--拦截不当内容
- 全部 LLM 输出记录日志
- 模板永远作为 fallback--LLM 失败/超时/被拒绝 -> 无缝回退

## 新增跨模块接口契约

| 概念 | 权威 Owner | 消费方 | 关键约定 |
|------|-----------|--------|---------|
| LlmSceneConfig（19场景开关） | **语言表达** 011 | 全部 | 19场景独立开关+master_switch。LLM装饰器模式--包裹模板管道，失败/关闭/不可用 -> 无缝回退模板 |
| LlmBackend trait | **语言表达** 011 | 无（玩家配置） | 统一本地(5种)+云端(6种)+Mock。LlmBackendRegistry多后端管理+场景路由+故障转移。LlmRequest/LlmResponse后端无关通用格式 |
| NpcDialogueWillingness（六维） | **语言表达** 011 | NPC | 个性/情绪/关系/话题/情境/文化六维--不设系统信任硬阈值。willingness驱动LLM回应深度而非调用开关 |
| multi_travel_turn()（旅伴对话） | **语言表达** 011 | NPC | 四人触发源(环境景色/时事流言/沉默时间/随机自发)+SpeakUrge竞争+TravelUtteranceStyle(自言自语/对全体/对特定/续前)。非LLM模式必须工作 |
| EffectiveAudibleRadius | **语言表达** 011 --> **[CHG-030] 音频模块** | NPC/环境 | 六因子(有意控制/环境噪音/地形/天气/文化/个性)。替代固定米数。后续所有权迁至音频模块 |
| VoiceProfile（语音身份） | **语言表达** 012 --> **[CHG-030] 音频模块** | NPC | 物理(种族/性别/年龄到音高/音色/语速)+个性(外向性到表现力, 宜人性到粗砺度, 神经质到气声)。生成时确定。后续所有权迁至音频模块 |
| VoiceEmotionModulation | **语言表达** 012 --> **[CHG-030] 音频模块** | NPC | 五参数(音高偏移/语速/音量/颤抖/气声)从当前情绪派生。VoicePacket统一Rust到Godot语音包。后续所有权迁至音频模块 |
| TtsEngine trait（5种后端） | **语言表达** 012 | Godot | System/LocalAI/CloudAPI/PreRecorded/None五种。与LLM无关--模板管道同样可用。TtsConfig玩家开关+字幕模式 |
| VoicePriority（五级） | **语言表达** 012 --> **[CHG-030] 音频模块** | Godot | Critical打断/High/Normal/Ambient排队/Background。多NPC同时说话时高优先先播。后续所有权迁至音频模块 |

## 对现有模块的影响

| 受影响模块 | 影响类型 | 说明 |
|-----------|---------|------|
| **NPC 活人感** | 新增消费 | NpcDialogueWillingness 从NPC个性/情绪/关系字段派生。LlmEnhancer stub 预留 |
| **信息传播系统**（语言表达 009） | 新增消费 | 旅伴对话的时事触发源消费 InformationPropagationQuery |
| **对话系统**（语言表达 005） | 新增集成 | multi_travel_turn() 与 TurnAllocator 协作。对话取消全时机补充 |
| **文本生成引擎**（语言表达 003） | 装饰关系 | LLM 装饰器包裹模板管道--不修改模板代码。失败回退无缝 |
| **Godot 渲染层** | 新增消费 | VoicePacket 渲染 + TTS 播放 + VoiceManager 队列管理 |
| **音频模块**（后续 CHG-030） | 所有权迁入 | VoiceProfile / VoiceEmotionModulation / VoiceManager / effective_audible_radius() 后续迁至音频模块 |
| **技术栈方案** | 性能预算 | LLM调用延迟预算（深度对话<3s / 普通对话<2s / 战斗对话<1s）。成本~$0.20/游戏日（GPT-4o参考） |

## CHG-013 至 CHG-018 契约兼容

- 不修改 ExpressionRef 8B句柄设计
- 不修改 ContentResolver trait 注册模式
- 不修改 TextGenerator 片段组合架构
- 不修改 Conversation/TurnMode/DialogueIntent 核心模型
- 不修改 LanguageId/ScriptId 定义
- 不修改信息传播五通道（011 消费信息传播管道，不修改）
- 不修改非语言表达数据模型
- 不修改对话-记忆消化管道
- 模板永远是 fallback--LLM 失败/关闭/不可用 -> 无缝回退

## 性能与成本预算

| 指标 | 值 | 说明 |
|------|-----|------|
| 每游戏日 LLM 调用量 | 5-38 次 | 取决于场景开关，玩家可控 |
| Token/游戏日 | ~10K-50K | -- |
| 成本/游戏日 | ~$0.20 (GPT-4o参考) | 本地模型=零费用 |
| 成本/月 | ~$6.00 | 玩家可设硬上限 |
| 深度对话延迟 | <3s | 云端高质量模型 |
| 普通对话/旅伴延迟 | <2s | 本地快速模型 |
| 战斗对话延迟 | <1s | 本地极快模型 |
| NPC间对话延迟 | <5s | 本地模型 |
| 背景故事/日记 | 异步 | 云端最高质量 |

## 文档交叉引用

- [[../Happy Game/开发阶段/语言表达/README|语言表达系统 README]]
- [[../Happy Game/开发阶段/语言表达/011-LLM增强层|011-LLM增强层]]
- [[../Happy Game/开发阶段/语言表达/012-语音输出接口|012-语音输出接口]]
- [[CHG-017-语言表达系统v1.0创建-20260613|CHG-017（语言表达 v1.0）]]
- [[CHG-018-语言表达系统v1.1完善-信息传播非语言联动记忆消化-20260613|CHG-018（语言表达 v1.1）]]
- [[CHG-030-音频系统v1.0创建-20260618|CHG-030（音频系统--后续所有权迁移）]]

---

> **注**: 本文档于 2026-06-20 (CHG-047 Phase 1 基础设施清理) 追溯创建--模块设计工作于 2026-06-13 完成。VoiceProfile / VoiceEmotionModulation / VoiceManager / effective_audible_radius() 的声学定义所有权已于后续 CHG-030 迁至音频模块。
