> **开发代号**: WoWorld (Wonder World)
> **文档类型**: 参考文档 · 讨论大纲
> **创建日期**: 2026-06-17
> **父文档**: [[000-模块大纲与导航]]

---

# 004-ThoughtTrigger与ThoughtFragment

## 待讨论的核心问题

### 1. ThoughtTrigger——6类触发源

| 触发类型 | 来源系统 | 触发条件 |
|----------|---------|---------|
| perceptual | 感官PerceptEntry | 实体→记忆参与者匹配 |
| location | 空间记忆 | 当前位置→记忆location_id |
| emotional | 情绪引擎 | emotion_delta>0.3→同情绪记忆检索 |
| dissonance | 信息传播/语言表达 | 新信息vs已有core_values冲突>0.5 |
| wander | CognitiveTide | mind_wander_drive>0.7+概率 |
| zeigarnik | MentalModel | 不确定模型+未消化反例→反刍触发 |

### 2. 设计关键

- 不主动轮询——被其他系统的正常事件携带触发
- 每个check返回Option<ThoughtTrigger>——大部分时候None
- 纯函数，零副作用

### 3. ThoughtFragment——3级清晰度

- VagueFeeling：模糊感觉，只有情绪色彩+近似domain+心理意象种子
- HalfFormedThought：半成形，几个片段+方向
- ClearThought：清晰命题，可明确表达

### 4. 文本生成

- 对标语言表达系统的片段组合模型（~430片段·~86KB）
- ThoughtTemplate ~200-300条，覆盖常见思考模式
- CognitiveStyle选择表达模式（分析型："如果A那么B..."；直觉型："总觉得..."）
- 内容验证：实体存在性检查（NPC必须认识引用的实体）

### 5. SurfacingModality——6种浮现方式

- InternalOnly：闷在心里→仅微表情变化
- MicroExpression：皱眉/眼神失焦/嘴角微动→Godot动画层
- Muttering：自言自语→玩家3m内Godot字幕
- ActionPause：动作暂停2-5秒→铁匠停手、食客放下筷子
- WritingImpulse：通过SurfacingModality选择媒介（书写/绘画/音乐）→调用已有LifeTrace/审美/物品API
- SpeakingImpulse：产生"得说出来"的冲动→调用DialogueIntent

### 6. 对话中的微思考

- micro_think_during_dialogue()——听→想→答的中间步骤
- 0.3-1.0秒的认知加工
- ~5-15%概率触发（取决于reflective_impulsive）
- 可能产出：ActionPause("等等让我想想")、关联记忆("你这么说让我想起...")、认知失调检测

### 7. 地点触发与自传体记忆

- 对标已有空间认知模型（5层路径模型）
- 空间查询记忆→高emotional_encoding→触发
- 直觉型NPC更容易被地点触发

### 8. 思考产生情绪

- 认知失调检测→焦虑/困惑
- 失调解决→释然/满足
- CreativeLeap→兴奋（高openness）
- 反刍阻塞→挫折
- push到情绪引擎已有external_event_push机制
