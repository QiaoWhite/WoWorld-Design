# 003-Utterance与渲染管道

> **开发代号**: WoWorld (Wonder World)
> **模块**: 概念与语言地基系统
> **文档类型**: 正式开发规格 · Utterance与渲染管道
> **版本**: v1.1 (CHG-057)
> **创建日期**: 2026-06-19
> **父文档**: [[001-概念与语言地基总纲|001-总纲]]

---

## 一、Utterance——结构化话语

### 1.1 核心定义

```rust
// woworld_types/utterance.rs

/// 结构化话语——NPC 语言产出的统一格式
/// 替代裸 String 作为对话/自语/书写的内容载体
#[derive(Clone, Serialize, Deserialize)]
pub struct Utterance {
    pub utterance_id: UtteranceId,            // 唯一标识
    pub content: UtteranceContent,            // 话语内容
    pub speech_act: SpeechAct,                // 言语行为类型
    pub language: LanguageId,                 // 说话者使用的语言
    pub delivery: SpeechDelivery,             // 传递方式
    pub speaker_id: EntityId,                 // 说话者
    pub speaker_culture: CultureId,           // 说话者的文化
    pub emotional_valence: f32,               // 情感基调 -1~1
    pub created_at: GameInstant,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum UtteranceContent {
    /// 有概念内容的话语
    Speech(Vec<UtteranceConcept>),
    /// 有意义的沉默
    Silence(SilenceIntent),
}

/// 话语中的单个概念引用
#[derive(Clone, Serialize, Deserialize)]
pub struct UtteranceConcept {
    pub concept_id: ConceptLocalId,           // 概念 ID
    pub culture_id: CultureId,                // 概念的来源文化
    pub confidence: f32,                      // 说话者对此概念的置信度
    pub role: ConceptRole,                    // 在当前话语中的语义角色
    pub modifiers: Vec<ConceptModifier>,      // 修饰语（肯定的/否定的/时间/数量）
    /// ★ v1.1 (CHG-057): 概念来源的领域签名
    /// None = 未计算。用于修辞检测——TextGenerator 检查相邻概念的 domain_similarity。
    pub domain_sig: Option<u64>,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ConceptRole {
    Subject,      // 主语——"铁匠铺主"
    Object,       // 宾语——"儿子"
    Action,       // 动作——"雇佣"
    Location,     // 地点——"铁匠铺"
    Cause,        // 原因
    Effect,       // 结果
    Attribute,    // 属性
    Relation,     // 关系——"为……工作"
}

#[derive(Clone, Serialize, Deserialize)]
pub enum ConceptModifier {
    Negation,                    // 否定——"不用"
    Temporal(GameInstant),       // 时间——"昨天"
    Completion,                  // 完成体——"了"
    Possession(EntityId),        // 从属——"我的"
    Quantity(u32),               // 数量
}

pub enum SpeechAct {
    Inform,         // 陈述——"儿子去铁匠铺工作了"
    Question,       // 询问
    Request,        // 请求
    Command,        // 命令
    Greet,          // 问候
    Farewell,       // 告别
    Thank,          // 感谢
    Apologize,      // 道歉
    Promise,        // 承诺
    Refuse,         // 拒绝
    Exclaim,        // 感叹
    InternalMonologue, // 内心独白
}
```

### 1.2 概念的语义角色——完整示例

```
原始话语: "我的儿子去铁匠铺工作，我不用给他钱了"

Utterance {
  content: Speech([
    UtteranceConcept {
      concept_id: SON, culture_id: zhongyuan, role: Subject,
      modifiers: [Possession(father_id)],  // "我的"
    },
    UtteranceConcept {
      concept_id: EMPLOYMENT, culture_id: zhongyuan, role: Action,
    },
    UtteranceConcept {
      concept_id: BLACKSMITH_SHOP, culture_id: zhongyuan, role: Location,
    },
    UtteranceConcept {
      concept_id: FINANCIAL_SUPPORT_CEASE, culture_id: zhongyuan, role: Effect,
      modifiers: [Negation, Completion],  // "不用……了"
    },
  ]),
  speech_act: Inform,
  language: zhongyuan_common,
  delivery: Normal,
}
```

### 1.3 UtteranceId——与 ExpressionRef 分离

```rust
/// 瞬时话语的标识——与 ExpressionRef（持久可读物句柄）分离
/// UtteranceId: 仅当次对话有效，单调递增
/// ExpressionRef: 指向 LMDB 中持久化的内容
#[derive(Copy, Clone, Hash, Eq, PartialEq)]
pub struct UtteranceId(u64);
```

| | ExpressionRef | UtteranceId |
|------|--------------|-------------|
| 指向 | LMDB 持久化内容 | 内存瞬时话语 |
| 生命周期 | 永久 | 当次对话 |
| 使用者 | ContentResolver (书籍/碑文) | TopicResolver → TextGenerator |
| 存储 | 8B (carrier_type + local_id) | 8B (单调 ID) |

---

## 二、TextGenerator——三模式统一

### 2.1 三个入口

```rust
// woworld_lang/src/text_generator.rs

impl TextGenerator {
    // ── 入口1: 概念渲染（对话、自语、动态文本）──
    pub fn render_utterance(&self, ctx: &RenderContext) -> String {
        // ① 概念翻译（如果需要）
        let translated = if ctx.speaker_culture != ctx.listener_culture {
            translate_concepts(&ctx.utterance.concepts, ctx.speaker_culture,
                              ctx.listener_culture, ctx.listener_proficiency)
        } else {
            DirectMatch { concepts: ctx.utterance.concepts.clone() }
        };
        // ② 概念 → 参数映射
        let params = concepts_to_params(&translated.concepts, ctx.ui_language);
        // ③ SpeechAct → 选择片段组
        let fragments = self.fragments.select(ctx.utterance.speech_act, &params, ctx.ui_language);
        // ④ 片段组合 + 滤镜管道
        let raw = self.assemble(&fragments, &params);
        // ⑤ proficiency 退化
        let text = self.apply_degradation(raw, ctx.listener_proficiency);
        // ⑥ 简化滤镜（低 proficiency 时）
        if ctx.listener_proficiency < 0.5 {
            self.simplify_for_limited_proficiency(text, ctx.listener_proficiency)
        } else { text }
    }

    // ── 入口2: 模板渲染（书籍、碑文——已有，保留）──
    pub fn generate(&self, content: &ResolvedContent, reader: &ReaderContext) -> ReadResult {
        // 已有逻辑不变——详见 [[../语言表达/003-文本生成引擎|003-文本生成引擎]]
    }

    // ── 入口3: 思考浮现文本（NPC 自语/微表情）──
    pub fn render_thought(&self, fragment: &ThoughtFragment, npc: &NpcSnapshot,
                         ui_language: LanguageId) -> String {
        let utterance = fragment.to_utterance(npc);
        self.render_utterance(&RenderContext {
            utterance: &utterance,
            speaker_culture: npc.culture(),
            listener_culture: npc.culture(),     // 内心独白——说话者和听者是同一个人
            listener_proficiency: 1.0,            // 对自己总是母语
            ui_language,
            familiarity: 1.0,
        })
    }
}
```

### 2.2 RenderContext

```rust
pub struct RenderContext<'a> {
    pub utterance: &'a Utterance,
    pub speaker_culture: CultureId,
    pub listener_culture: CultureId,           // 听者的文化（用于概念翻译）
    pub listener_proficiency: f32,             // 听者对 speaker 语言的 proficiency
    pub ui_language: LanguageId,               // 最终渲染的目标 UI 语言
    pub familiarity: f32,                      // 说话者与听者的熟悉度
    pub speaker_rhetorical_ability: f32,       // ★ v1.1 (CHG-057): 说话者的修辞能力，>0.4 触发修辞化渲染
}
```

### 2.3 简化滤镜

```rust
impl TextGenerator {
    fn simplify_for_limited_proficiency(&self, text: String, proficiency: f32) -> String {
        if proficiency > 0.7 { return text; }
        match proficiency {
            p if p > 0.5 => self.filter_complexity(text, ComplexityLevel::Moderate),
            p if p > 0.3 => self.filter_complexity(text, ComplexityLevel::Simple),
            _ => self.filter_complexity(text, ComplexityLevel::Basic),
            // Basic: 极简句 + 重复关键概念
        }
    }
}
```

---

## 三、跨文化概念翻译

### 3.1 翻译函数

```rust
/// 跨文化概念翻译——纯函数
pub fn translate_concepts(
    source_concepts: &[UtteranceConcept],
    source_culture: CultureId,
    target_culture: CultureId,
    proficiency: f32,
) -> TranslationResult {
    let target_space = CONCEPT_REGISTRY.load(target_culture);
    let mut translated = Vec::new();

    for concept in source_concepts {
        let source_def = CONCEPT_REGISTRY.find(concept.concept_id, source_culture);
        let source_coverage = &source_def.coverage;

        // 在目标文化概念空间中搜索最佳匹配
        let best = target_space.iter()
            .filter_map(|tgt| {
                let overlap = pattern_overlap_coverage(source_coverage, &tgt.coverage);
                if overlap > 0.1 { Some((tgt.local_id, overlap)) } else { None }
            })
            .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

        match best {
            Some((id, overlap)) if overlap > 0.8 => {
                // 情况 A: 直接翻译
                translated.push(TranslatedConcept {
                    concept_id: id,
                    fidelity: overlap,
                    clarification: None,
                });
            }
            Some((id, overlap)) if overlap > 0.3 => {
                // 情况 B: 近似翻译 + 描述补全
                let gap = describe_concept_gap(&source_def, &target_space[id as usize], target_culture);
                translated.push(TranslatedConcept {
                    concept_id: id,
                    fidelity: overlap,
                    clarification: Some(gap),
                });
            }
            _ => {
                // 情况 C: 无对应概念——保留源概念 ID，用描述性短语
                translated.push(TranslatedConcept {
                    concept_id: concept.concept_id,  // 保留源 ID
                    fidelity: 0.0,
                    clarification: Some(generate_descriptive_phrase(&source_def, target_culture)),
                });
            }
        }
    }

    TranslationResult {
        concepts: translated,
        overall_fidelity: translated.iter().map(|c| c.fidelity).sum::<f32>()
                        / translated.len().max(1) as f32,
    }
}
```

### 3.2 翻译示例

```
中原"雇佣" → 兽人文化:
  source_coverage: [Contract + Extract(labor) + Extract(wage), without Compel]
  target_space: "强者权利" (覆盖 Extract + Constrain + Compel) — 重叠 0.45
  翻译: "强者权利（某种特定的劳役契约）" — 信息丢失了 Contract 的"同意"维度

精灵"终身委身" → 中原文化:
  source_coverage: [Contract + Extract(labor) + Extract(wage) + Pledge(lifetime)]
  target_space: "雇佣" — 重叠 0.65 (缺失 Pledge 维度)
  翻译: "雇佣（但带有终身效忠的誓言）"
```

---

## 四、玩家输入进入概念管道

```rust
pub fn player_input_to_utterance(
    input: &PlayerInput,
    player_npc: &NpcData,
    conversation_context: &DialogueContext,
) -> Result<Utterance, NluError> {
    match input {
        PlayerInput::SelectOption { option_index } => {
            // 预设选项 → 已有对应的 Utterance
            conversation_context.get_utterance_for_option(*option_index)
        }
        PlayerInput::JournalReference { entry_id } => {
            // 大日志条目 → 提取概念 → Utterance
            let entry = JOURNAL.get(*entry_id)?;
            entry_to_utterance(entry, &effective_concept_space(player_npc))
        }
        PlayerInput::FreeText { text, language } => {
            // NLU: 自然语言 → 概念
            let concepts = NLU_ENGINE.extract_concepts(
                text, *language, &effective_concept_space(player_npc)
            );
            Ok(Utterance {
                utterance_id: UtteranceId::next(),
                content: UtteranceContent::Speech(concepts),
                speech_act: infer_speech_act(text),
                language: *language,
                delivery: SpeechDelivery::Normal,
                speaker_id: player_npc.entity_id(),
                speaker_culture: player_npc.birth_culture,
                emotional_valence: 0.0,
                created_at: GameInstant::now(),
            })
        }
    }
}
```

---

## 五、语音输出与 CurrentSpeech

```rust
// CurrentSpeech 修正——ExpressionRef → UtteranceId
pub struct CurrentSpeech {
    pub utterance_id: UtteranceId,     // ★ 改——指向结构化 Utterance
    pub started_at: GameInstant,
    pub word_count: u16,               // TextGenerator 渲染后填入
    pub words_per_second: f32,         // 从 VoiceProfile 派生
    pub delivery: SpeechDelivery,
}

// 完整链路: Utterance → TextGenerator → String → VoicePacket → TTS
// 音频模块只轮询 CurrentSpeech，不感知 Utterance 的内容
```

---

## 六、沉默的意义

```rust
pub enum SilenceIntent {
    Thinking,                // 正在思考——停顿
    WaitingForResponse,      // 在等对方说话
    RefusingToAnswer,        // 拒绝回答——有意的沉默
    EmotionalOverwhelm,      // 情绪激动说不出话
    CulturalSilence,         // 文化常规的对话间隙
    Disinterest,             // 不在乎不开口
}

// 有意的沉默 = Utterance { content: Silence(RefusingToAnswer), speech_act: Refuse }
// TextGenerator 渲染: "[NPC 沉默不语，移开了视线]"
```

---

## 七、修辞化渲染 ★ v1.1 新增 (CHG-057)

> **设计裁决**: 修辞 = Creative Leap（认知侧跨域类比）+ TextGenerator 修辞化渲染模式（语言侧）。`rhetorical_ability` 从已有参数涌现，零新技能。

### 7.1 rhetorical_ability —— 纯函数

```rust
/// 修辞能力从已有属性派生——不新增技能
pub fn rhetorical_ability(npc: &NpcData) -> f32 {
    // 找到类比的能力：智慧 + 抽象思维 + 认知灵活性
    let find_analogies = npc.mental.wisdom * 0.5
                       + npc.cognitive_style.abstract_concrete * 0.3
                       + npc.cognitive_style.rigid_flexible * 0.2;
    // 表达类比的能力：魅力 + 语言熟练度
    let express_analogies = npc.mental.charisma * 0.6
                          + npc.language_proficiency * 0.4;

    (find_analogies * 0.6 + express_analogies * 0.4).clamp(0.0, 1.0)
}
```

### 7.2 修辞触发与渲染

TextGenerator 在 `render_utterance()` 中，当 `rhetorical_ability > 0.4` 且话语包含域相似度低的相邻概念时触发：

```rust
fn render_utterance(&self, ctx: &RenderContext) -> String {
    let mut output = self.assemble_fragments(ctx);
    
    if ctx.speaker_rhetorical_ability > 0.4 {
        // 检测跨域概念对
        for w in ctx.utterance.concepts().windows(2) {
            if let (Some(sig_a), Some(sig_b)) = (w[0].domain_sig, w[1].domain_sig) {
                let ds = domain_similarity_raw(sig_a, sig_b);
                if 0.1 < ds && ds < 0.4 && rng.gen_bool(ctx.speaker_rhetorical_ability * 0.4) {
                    output = self.apply_rhetorical_device(output, &w[0], &w[1], ds, ctx);
                }
            }
        }
    }
    
    output
}
```

### 7.3 修辞装置

| 装置 | 触发条件 | 示例 |
|------|---------|------|
| **明喻 (simile)** | 教学 SpeechAct + 两个概念来自不同域 | "淬火就像冬天的冷风吹过热铁" |
| **暗喻 (metaphor)** | Inform/Exclaim + rhetorical_ability > 0.6 | "我们是守护人界的盾牌" |
| **类比解释 (analogy)** | WisdomSharing + 听者技能 < 讲者技能 | "锻造好比烹饪——火候对了才成" |

修辞的**认知内容**来自 Creative Leap 的跨域结构匹配（已有），修辞的**语言形式**来自 TextGenerator 的修辞渲染模式（新增）。零新 crate。零新 trait。

---

> **下一文档**: [[004-语言与文字系统]] · [[012-词汇库与构词系统|012-词汇库与构词]]（概念→词汇如何映射）
> **关联**: [[001-概念与语言地基总纲|001-总纲]] · [[../语言表达/003-文本生成引擎|003-文本生成引擎]] · [[../音频系统/005-语音管道|005-语音管道]]
