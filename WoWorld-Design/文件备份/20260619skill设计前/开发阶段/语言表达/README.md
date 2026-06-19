# 语言表达模块——模块索引

> **开发代号**: WoWorld (Wonder World)
> **模块**: 语言表达系统 v1.0
> **定位**: 地基模块之四——统一 WoWorld 中所有语言文字表达的基础设施
> **依赖**: woworld_types（纯类型 crate）→ lang_expression（行为 crate）→ 各业务模块实现 trait

---

## 模块概述

**语言表达模块**负责 WoWorld 中一切"可被阅读、聆听、言说的内容"的统一管道。它不拥有内容——它让内容被感知。

WoWorld 的文字/书写/语言/对话此前分散在 10+ 个模块中，各自独立重复发明同一件事。本模块建立统一的语言表达基础设施，消除碎片化，严格遵守 Rust 解耦原则。

### 三层 Crate 架构

```
woworld_types/          ← 纯数据类型 + trait 定义（零行为，零业务依赖）
lang_expression/        ← 行为实现（只依赖 woworld_types，不依赖任何业务模块）
各业务模块              ← 依赖 woworld_types，实现 ContentResolver trait
主循环/Godot 集成层     ← 唯一的依赖聚合点
```

---

## 文档索引

| 编号 | 文档 | 核心内容 |
|------|------|---------|
| **001** | [语言表达系统总纲](001-语言表达系统总纲.md) | 三层 Crate 架构、模块边界、依赖方向、ExpressionRef 设计哲学、与其他模块关系图 |
| **002** | [语言与文字系统](002-语言与文字系统.md) | LanguageId/ScriptId/LanguageFamily 定义、谱系 DAG、方言 sigmoid 渐变、literacy 模型、Proficiency |
| **003** | [文本生成引擎](003-文本生成引擎.md) | TextGenerator、片段组合架构（~430 片段）、参数命名约定（~50 标准参数）、条件筛选、滤镜、性能预算 |
| **004** | [内容解析接口](004-内容解析接口.md) | ContentResolver trait、LocatableReadable trait、ExpressionRegistry 注册表、carrier_type 范围分配、各模块注册规范 |
| **005** | [对话系统——核心机制](005-对话系统-核心机制.md) | Conversation（多参与者 2→1000+）、Participant/ParticipantRole、TurnMode（FreeForm/Moderated/Speech/Ritual）、TurnAllocator、DialogueIntent（NPC 主动 5 种驱动）、ConversationTopic + TopicSelector + TopicResolver、DialogueContext 依赖注入、PlayerInput 四种统一 |
| **006** | [对话系统——社交层](006-对话系统-社交层.md) | PhaticLayer（5 类 ~210 片段）、PersonalityFilter、SocialField 群体动力学（惯性/极化/从众/SensoryMapping）、RelationshipFilter |
| **007** | [自然语言理解与输入](007-自然语言理解与输入.md) | InputInterpreter 三层回落（L1 关键词 + L2 嵌入 + L3 LLM）、KeywordIndex 自动构建、LLM 可插拔增强、语音/打字/选项互操作 |
| **008** | [跨模块接口与数据合同](008-跨模块接口与数据合同.md) | 全部 trait 汇集、各模块 impl 规范、LMDB key 约定、性能预算汇总、安全边界、ReliabilityHint、完整类型汇总、版本路线 |
| **009** | [信息传播系统](009-信息传播系统.md) | 五传播通道（口头/书信/信鸽/魔法/公告）、失真算子（ScaleDistortion/DetailLoss/EmotionalAmplification/AttributionShift/Moralization）、欺骗与谎言检测、谣言生命周期（爆发→稳态→衰减→传说→消亡）、NPC间对话渲染（玩家可见/部分可见/仅察觉/不可见）、悄悄话/密谋模式、偷听检测与后果 |
| **010** | [非语言表达与语言联动](010-非语言表达与语言联动.md) | NonVerbalSignal 数据模型（面部/手势/姿态/视线/空间/触觉）、对话中非语言信号生成引擎、NPC 感知对方非语言→DialogueIntent 修正、文化差异（手势含义映射/沟通规范/敬语/打断/沉默/视线接触）、Godot 渲染集成 |
| **011** | [LLM增强层](011-LLM增强层.md) | 装饰器架构·场景粒度开关(19场景)·LlmBackend trait(本地5+云端6+Mock)·LlmBackendRegistry多后端管理+故障转移·LlmRequest/LlmResponse通用格式·NPC自主对话意愿(NpcDialogueWillingness六维)·PersonalityPromptBuilder·旅伴对话(多人+景色+时事)·灵活可闻半径(EffectiveAudibleRadius六因子)·沉默模型·多人LLM·NPC间LLM策略(LlmPriority五级)·对话取消全时机(12种)·安全边界 |
| **012** | [语音输出接口](012-语音输出接口.md) | VoiceProfile(NPC音色身份·物理+个性)·VoiceEmotionModulation(情绪→音高/语速/音量/颤抖)·VoicePacket(Rust→Godot统一语音包·5种delivery·5级priority)·TtsEngine trait(5种后端:系统/本地AI/云端API/预录制/无)·TtsConfig(玩家开关·音量·字幕模式)·VoiceManager·各管道集成 |

---

## 版本路线

| 版本 | 内容 |
|------|------|
| **v1.0** | 语言系统 + ExpressionRef + ContentResolver + TextGenerator + ExpressionRegistry + 对话核心（Conversation/TurnMode/TopicSelector/PlayerInput）+ PhaticLayer + SocialField + L1 NLU |
| **v1.1** | 信息传播系统（五通道+失真+谣言生命周期+NPC间对话渲染+悄悄话/密谋）+ 非语言表达联动 + 对话→记忆消化 + 对话中断/恢复 + 文化沟通规范 + Writeable trait + ConversationSnapshot |
| **v1.2** | L2 嵌入向量匹配 + Godot 语音集成规范 + SensoryMapping 实现 |
| **v2.0** | L3 LLM 集成 + LLM增强层 + 语音输出接口 |

---

> **关联**: [[001-语言表达系统总纲]] · [[../技术栈方案/001-WoWorld正式技术栈方案v3|技术栈方案 v3.0]] · [[../NPC活人感模块/NPC活人感开发文档ver2.0|NPC 活人感 ver2.0]] · [[../历史/001-历史系统总纲|历史系统]] · [[../魔法/01-基础层/001-魔法总纲|魔法系统]] · [[../物品系统/001-物品系统总纲|物品系统]] · [[../技能系统/001-技能系统总纲|技能系统]]
