> **开发代号**: WoWorld (Wonder World)
> **引擎**: Godot 4.6.2
> **模块**: 06-NPC认知与智慧系统
> **文档类型**: 正式开发规格 · MentalModel与智慧积累
> **版本**: v1.0
> **创建日期**: 2026-06-17
> **父文档**: [[001-认知与智慧系统总纲]]

---

# 003-MentalModel与智慧积累

## 目录

- [一、MentalModel数据结构](#一mentalmodel数据结构)
- [二、归纳：从记忆中提取信念](#二归纳从记忆中提取信念)
- [三、微消化（micro_digest）](#三微消化micro_digest)
- [四、宏消化（macro_digest）](#四宏消化macro_digest)
- [五、种子继承：NPC出生时脑子里有什么](#五种子继承npc出生时脑子里有什么)
- [六、创造性飞跃（creative_leap）](#六创造性飞跃creative_leap)
- [七、Shareability：信念的传播边界](#七shareability信念的传播边界)
- [八、跨代传递：信念如何在NPC之间流动](#八跨代传递信念如何在npc之间流动)
- [九、MindAttribution：Theory of Mind](#九mindattribution-theory-of-mind)
- [十、BeliefHistory：信念演变里程碑](#十beliefhistory信念演变里程碑)
- [十一、泛化质量与过度拟合](#十一泛化质量与过度拟合)

---

## 一、MentalModel数据结构

### 1.1 定义

```rust
/// 一个MentalModel代表NPC持有的一个"世界如何运作"的信念
/// 对标：Knowledge（~64条个人知识）
/// 上限：20条，约1.6KB
#[derive(Clone, Serialize, Deserialize)]
pub struct MentalModel {
    pub id: MentalModelId,                    // 唯一标识
    pub domain: MentalModelDomain,            // 信念所属领域
    pub pattern: PatternExpression,           // "当X发生时，Y倾向于跟着发生"
    pub confidence: f32,                      // 贝叶斯置信度 0-1
    pub supporting_count: u16,               // 正面实例计数
    pub counter_count: u16,                   // 反面实例计数
    pub supporting_diversity: f32,            // 支持证据的情境多样性 (0=全同, 1=高度多样)
    pub emotional_charge: f32,                // 信念的情绪附着强度
    pub emotional_valence: f32,               // 正面或负面信念 (-1=负面, 1=正面)
    pub source: ModelSource,                  // 从哪来的
    pub source_age: f32,                      // 游戏日——持有多久了
    pub formed_at: GameInstant,
    pub last_activated: GameInstant,          // 最后一次被记忆/思考触发的时间
    pub last_tested: GameInstant,             // 最后一次被验证的时间
    pub activation_threshold: f32,            // 惰性遗忘门槛 (0.3~0.95)
    pub shareability: Shareability,           // 传播边界
    pub hypothesis: bool,                     // 是否为"假说"（来自creative_leap的初始低置信模型）
    pub abandoned: bool,                      // 是否已被放弃（保留用于叙事）
}

pub enum MentalModelDomain {
    Combat, Crafting, Magic, Aesthetics, Construction,
    Trade, Politics, Social, Weather, Navigation,
    Cooking, Farming, AnimalHusbandry, Medicine,
    Education, Philosophy,  // 共17个domain
}

pub enum ModelSource {
    SelfDerived,        // 自己从记忆中归纳
    SelfRefined,        // 消化后的——已经忘了是从哪学的
    TaughtByNpc(NpcId), // 某个NPC教的
    ReadFromBook(WorkId), // 从书中读到
    Inherited,          // 从父母/文化继承（初始种子）
}

pub enum Shareability {
    Public,                 // 随便说
    InGroup { group: GroupFilter },  // 只对特定群体说
    SpecificTarget { target: NpcId }, // 只对特定NPC说
    Private,                // 绝不说
}
```

### 1.2 容量管理

对标记忆系统2000条上限的硬cap。MentalModel上限20条：

```rust
/// 当MentalModel达到上限时的策略
/// 优先级（从低到高）：
///   1. abandoned=true的模型 → 先删
///   2. confidence<0.15且counter>supporting → 删除
///   3. 最久未激活的模型 → 压缩为BeliefMilestone → 删除
///   4. 全部压缩后仍超出上限 → 旧的合并，保留20条
fn manage_mental_model_capacity(models: &mut ArrayVec<MentalModel, 20>, history: &mut BeliefHistory) {
    if models.len() >= 20 {
        // 第一步：删除已放弃的
        models.retain(|m| !m.abandoned);
        
        // 第二步：删除极低置信度的
        models.retain(|m| m.confidence > 0.15 || m.emotional_charge > 0.7);
        
        // 第三步：压缩最后激活的模型到history
        while models.len() > 20 {
            let oldest = models.iter().enumerate()
                .min_by_key(|(_, m)| m.last_activated)
                .map(|(i, _)| i);
            if let Some(idx) = oldest {
                let model = models.remove(idx);
                history.entries.push(BeliefMilestone::from_archived_model(&model));
            }
        }
    }
}
```

---

## 二、归纳：从记忆中提取信念

### 2.1 触发条件

```rust
/// 在记忆编码后自动检查
/// 对标：AestheticTaste一年一度成熟
pub fn try_induce_pattern(
    new_memory: &EventMemory,
    memory_store: &MemoryStore,
    existing_models: &[MentalModel],
    cognitive_style: &CognitiveStyle,
    knowledge: &Knowledge,
    rng: &mut Pcg64,
) -> Option<MentalModel> {
    // 前置条件：同一domain下已有≥3条高impact记忆
    let domain = new_memory.event_type.domain();
    let domain_memories: Vec<_> = memory_store
        .query_by_domain(domain)
        .filter(|m| m.impact_score > 0.3)
        .take(10)  // 最多扫描10条
        .collect();
    
    if domain_memories.len() < 3 { return None; }
    
    // 搜索共享模式
    let pattern = search_shared_pattern(&domain_memories, cognitive_style);
    
    // 避免重复：已有相同模式的模型
    if existing_models.iter().any(|m| m.pattern.overlaps(&pattern, 0.7)) {
        return None;
    }
    
    // 归纳
    let n = domain_memories.len() as f32;
    let initial_confidence = 1.0 - 1.0 / (1.0 + n);  // 拉普拉斯平滑
    let supporting_diversity = compute_diversity(&domain_memories);
    
    Some(MentalModel {
        id: MentalModelId::new(),
        domain,
        pattern,
        confidence: initial_confidence,
        supporting_count: n as u16,
        counter_count: 0,
        supporting_diversity,
        emotional_charge: avg_emotional_charge(&domain_memories),
        emotional_valence: avg_valence(&domain_memories),
        source: ModelSource::SelfDerived,
        source_age: 0.0,
        formed_at: GameInstant::now(),
        last_activated: GameInstant::now(),
        last_tested: GameInstant::now(),
        activation_threshold: 0.3,
        shareability: Shareability::InGroup { group: GroupFilter::Family },
        hypothesis: false,
        abandoned: false,
    })
}
```

### 2.2 专家直觉

```rust
/// 高技能等级NPC在相关domain更容易归纳
pub fn expert_intuition_bonus(skill_level: u32) -> f32 {
    if skill_level < 70 { return 0.0; }
    (skill_level - 70) as f32 / 30.0  // 70→0.0, 85→0.5, 100→1.0
}
```

---

## 三、微消化（micro_digest）

### 3.1 定义

**微消化**：每记忆编码时触发。检查新记忆是否支持/反对现有MentalModel，贝叶斯更新置信度。

对标已有模式：审美系统的`digest_aesthetic_experience()`。

### 3.2 算法

```rust
pub fn micro_digest(
    new_memory: &EventMemory,
    models: &mut [MentalModel],
    biases: &CognitiveBiases,
    style: &CognitiveStyle,
    emotion: &EmotionState,
) -> Vec<MentalModelEvent> {
    let mut events = Vec::new();
    
    for model in models.iter_mut()
        .filter(|m| !m.abandoned)
        .filter(|m| m.domain == new_memory.event_type.domain()) 
    {
        // 1. 一致性判断
        let consistency = evaluate_consistency(new_memory, model);  // -1 到 1
        
        // 2. 确认偏误调制（对标已有记忆系统的Valence Distortion过滤器——情绪扭曲感知）
        let weighted_consistency = if consistency > 0.0 {
            consistency * (1.0 + biases.confirmation_bias * 0.5)  // 放大支持
        } else {
            consistency * (1.0 - biases.confirmation_bias * 0.3)  // 削弱反对
        };
        
        // 3. 负面偏误调制
        let weight = if weighted_consistency < 0.0 {
            1.0 + biases.negativity_bias * 1.5  // 负面证据2-3倍权重
        } else {
            1.0
        };
        
        // 4. 近因加权
        let temporal_weight = biases.recency_weight;
        
        // 5. 贝叶斯更新
        let likelihood = sigmoid(weighted_consistency * weight * temporal_weight * 5.0);
        model.confidence = (model.confidence * likelihood) 
                         / (model.confidence * likelihood 
                          + (1.0 - model.confidence) * (1.0 - likelihood));
        
        // 6. 更新计数
        if weighted_consistency > 0.0 { 
            model.supporting_count += 1; 
        } else { 
            model.counter_count += 1; 
        }
        
        // 7. 更新支持证据多样性（新记忆的情境特征→多样性信息熵更新）
        model.supporting_diversity = update_diversity_ema(
            model.supporting_diversity, 
            compute_context_novelty(new_memory, model),
            0.1  // EMA α
        );
        
        // 8. 情绪附着（EMA）
        model.emotional_charge = model.emotional_charge * 0.95 
                               + new_memory.emotional_encoding.intensity() * 0.05;
        
        // 9. 更新最后触发时间
        model.last_activated = GameInstant::now();
        model.last_tested = GameInstant::now();
        
        // 10. 检查是否产出事件
        if model.confidence > 0.85 && model.confidence - previous_confidence > 0.2 {
            events.push(MentalModelEvent::ConfidenceSurge { model_id: model.id });
        }
        if model.counter_count > model.supporting_count * 1.5 && model.confidence > 0.6 {
            events.push(MentalModelEvent::Undermined { model_id: model.id });
        }
    }
    
    events
}
```

---

## 四、宏消化（macro_digest）

### 4.1 定义

**宏消化**：每7天随`SelfNarrative::reflect()`触发。处理需要长尺度时间才能发生的深层认知变化。

### 4.2 算法

```rust
pub fn macro_digest(
    models: &mut [MentalModel],
    self_narrative: &SelfNarrative,
    style: &CognitiveStyle,
    current_day: GameDay,
) -> Vec<MentalModelEvent> {
    let mut events = Vec::new();
    
    for i in 0..models.len() {
        if models[i].abandoned { continue; }
        
        // 1. 计算消化深度
        let depth = macro_digestion_depth(&models[i], style, self_narrative);
        
        // 2. source转移：消化足够深→外来模型变成"自己的"
        if depth > 0.8 
           && models[i].source != ModelSource::SelfRefined 
           && models[i].source != ModelSource::SelfDerived 
        {
            let days_since_acquired = (current_day - models[i].formed_at).0 as f32;
            if days_since_acquired > 30.0 && models[i].confidence > 0.7 {
                let old_source = models[i].source.clone();
                models[i].source = ModelSource::SelfRefined;
                events.push(MentalModelEvent::SourceShifted { 
                    model_id: models[i].id,
                    from: old_source,
                });
            }
        }
        
        // 3. 边界扩展：高openness+高抽象→给高confidence模型附加条件/例外
        if style.rigid_flexible > 0.6 && models[i].confidence > 0.8 {
            maybe_extend_model_boundaries(&mut models[i], style, self_narrative);
        }
        
        // 4. 放弃：证据长期负面且情绪已经冷淡
        if models[i].confidence < 0.1 
           && models[i].counter_count > models[i].supporting_count * 3
           && models[i].emotional_charge < 0.3 
        {
            models[i].abandoned = true;
            events.push(MentalModelEvent::Abandoned { 
                model_id: models[i].id,
                reason: "evidence erosion".into(),
            });
        }
    }
    
    // 5. 模型融合：检测可合并的模型对
    let merges = detect_merge_candidates(models, style);
    for (a_idx, b_idx, fused_model) in merges {
        models[a_idx].abandoned = true;
        models[b_idx] = fused_model;
        events.push(MentalModelEvent::Merged { 
            from: [models[a_idx].id, models[b_idx].id],
            into: models[b_idx].id,
        });
    }
    
    events
}

/// 消化深度——NPC将外部模型内化的程度
fn macro_digestion_depth(
    model: &MentalModel,
    style: &CognitiveStyle,
    self_narrative: &SelfNarrative,
) -> f32 {
    let base = model.confidence * 0.4                                    // 越信越消化
             + (model.supporting_count as f32 / 20.0).min(1.0) * 0.2    // 实证越多越消化
             + style.reflective_impulsive * 0.2                          // 反思型消化更深
             + style.rigid_flexible * 0.2;                               // 灵活型消化更快
    
    // 与core_values冲突时→消化被阻碍
    let value_conflict = self_narrative.check_value_conflict(&model.pattern);
    base * (1.0 - value_conflict * 0.5)
}
```

---

## 五、种子继承：NPC出生时脑子里有什么

### 5.1 设计原则

对标已有模式：信仰系统的`ChildFaithProfile`（CHG-025）——`parent1 × 0.7 + parent2 × 0.7 + community × 0.05`。

### 5.2 算法

```rust
/// NPC出生/创建时的初始MentalModel种子
/// 对标：ChildFaithProfile::inherit_faith_profile()
pub fn inherit_initial_models(
    parents: Option<(&NpcData, &NpcData)>,     // 父母（可能不存在）
    settlement_wisdom: &SettlementWisdomAggregate, // 聚落常识
    culture: &CultureCoreParams,                   // 文化基线
    cognitive_style: &CognitiveStyle,
) -> ArrayVec<MentalModel, 20> {
    let mut seeds = ArrayVec::new();
    
    // 1. 父母传递
    if let Some((parent1, parent2)) = parents {
        let inheritance_factor = 0.5 + culture.individualism * 0.3;  // 个人主义文化→继承因子更低
        let responsibility_weights = compute_care_weights(parent1, parent2);
        
        for model in parent1.mental_models.iter()
            .chain(parent2.mental_models.iter())
            .filter(|m| m.confidence > 0.6)
            .filter(|m| m.shareability != Shareability::Private)  // 私密模型不遗传
            .take(8)  // 最多从父母继承8条
        {
            let parent_confidence = model.confidence * inheritance_factor;
            if parent_confidence > 0.3 {
                seeds.push(MentalModel {
                    id: MentalModelId::new(),
                    confidence: parent_confidence,
                    source: ModelSource::Inherited,
                    // ... 其他字段重置
                    ..MentalModel::clone_from_parent(model)
                });
            }
        }
    }
    
    // 2. 聚落常识
    for model in settlement_wisdom.top_models(0.5).iter().take(6) {
        seeds.push(MentalModel {
            confidence: model.confidence * 0.6,
            source: ModelSource::Inherited,
            // ...
        });
    }
    
    // 3. 文化基线——文化隐含的世界观
    // 对标：CultureCoreParams → CommunicationNorms
    for cultural_model in derive_cultural_baseline_models(culture).iter().take(4) {
        seeds.push(MentalModel {
            confidence: 0.4 + cognitive_style.rigid_flexible * 0.2,
            source: ModelSource::Inherited,
            // ...
        });
    }
    
    // 上限20条
    seeds.truncate(20);
    seeds
}
```

---

## 六、创造性飞跃（creative_leap）

### 6.1 机制

**创造性飞跃不是魔法——是跨界结构模板匹配。** 对标已有审美系统的`Embellish`原子（在已有框架上创新）和睡眠跨界关联。

```rust
pub fn creative_leap(
    models: &[MentalModel],
    memory: &MemoryStore,
    knowledge: &Knowledge,
    cognitive_style: &CognitiveStyle,
    tide: &CognitiveTide,
    association_looseness: f32,  // 从EmbodiedCognitionModifiers来
    rng: &mut Pcg64,
) -> Option<MentalModel> {
    // 前置条件（7个条件的AND）:
    // 1. 漫游驱动高（心智在自由漫游）
    // 2. 开放性高（愿意接受新可能）
    // 3. 抽象思维强
    // 4. 认知负载低
    // 5. 有跨domain经验（支持多样性高）
    // 6. 认知压力不太高
    // 7. 有一定的未满足的探索欲（stagnation > 0.3）
    
    if tide.mind_wander_drive() < 0.7 { return None; }
    if cognitive_style.abstract_concrete < 0.6 { return None; }
    if cognitive_style.rigid_flexible < 0.5 { return None; }
    if cognitive_style.openness() < 0.5 { return None; }
    
    // 跨界结构匹配
    // "淬火让刀变硬" (Crafting domain)
    // × "玻璃吹制的快速冷却也改变硬度" (Crafting domain)
    // → "分段淬火：先油后水"
    //
    // 算法：随机选取两个不同domain的模型→计算pattern结构的向量相似度
    // 相似度>阈值→合成新模型
    
    let pairs = sample_cross_domain_pairs(models, 5, rng);  // 最多检查5对
    
    for (model_a, model_b) in &pairs {
        let structural_similarity = compute_pattern_similarity(
            &model_a.pattern, &model_b.pattern
        );
        
        if structural_similarity > 0.6 
           && rng.gen_bool((structural_similarity - 0.5) * 0.3) 
        {
            return Some(MentalModel {
                id: MentalModelId::new(),
                domain: model_a.domain,  // 创新产物的domain通常是第一个模型的domain
                pattern: synthesize_pattern(model_a, model_b, structural_similarity),
                confidence: 0.15 + structural_similarity * 0.15,  // 0.15-0.30
                hypothesis: true,  // 标记为假说——消化中特殊对待
                source: ModelSource::SelfDerived,
                // ...
                shareability: Shareability::InGroup { group: GroupFilter::Family },
            });
        }
    }
    
    None
}
```

### 6.2 与睡眠洞察的关系

- **清醒飞跃**（creative_leap）：前置条件严格，概率低，产出confidence=0.15-0.3
- **睡眠洞察**（sleep_cognitive_processing中的跨界关联）：门槛更低（睡眠中关联松弛），概率更高，产出confidence=0.1-0.2

两者的产出走相同的验证管道（MentalModelTest Intent→micro_digest→宏消化）。

---

## 七、Shareability：信念的传播边界

### 7.1 定义

对标已有模式：语言表达的`DeceptionIntent`（CHG-018）。

```rust
impl MentalModel {
    /// 检查当前NPC是否可以与目标NPC分享此模型
    pub fn can_share_with(&self, target: &NpcData, relationship: &Relationship, context: &ShareContext) -> bool {
        match &self.shareability {
            Shareability::Public => true,
            Shareability::InGroup { group } => {
                match group {
                    GroupFilter::Family => relationship.is_kin(),
                    GroupFilter::Faction => relationship.shares_faction(),
                    GroupFilter::CloseFriends => relationship.trust > 0.8,
                    GroupFilter::Guild => relationship.shares_guild(),
                }
            },
            Shareability::SpecificTarget { target: t } => *t == target.id,
            Shareability::Private => {
                // 私人信念仅在极端情况下分享
                context.is_deathbed  // 临终
                || context.is_torture  // 酷刑
                || relationship.trust > 0.95  // 极度信任
            },
        }
    }
}
```

### 7.2 Shareability的动态变化

```rust
/// 在对话后和消化中调用
pub fn update_shareability(model: &mut MentalModel, feedback: &SharingFeedback) {
    match feedback {
        // 分享后被惩罚→该模型及相似模型降级
        SharingFeedback::Punished { severity } => {
            model.shareability = Shareability::Private;
        },
        // 分享后得到正面反馈→可能升级
        SharingFeedback::Validated => {
            if let Shareability::InGroup { .. } = &model.shareability {
                model.shareability = Shareability::Public;
            }
        },
        // 酒后→临时+0.3，但分享后立即恢复
        SharingFeedback::IntoxicatedShare => { /* 不持久 */ },
    }
}
```

---

## 八、跨代传递：信念如何在NPC之间流动

### 8.1 六条传递路径

**全部通过已有通道。零新trait。**

```
路径1：对话传递（WisdomSharing）
  NPC A 持有 MentalModel → 对话中分享 → NPC B assess_and_integrate
  通道：语言表达系统的 DialogueIntent::WisdomSharing
  对接已有：语言表达只管传递，不需要知道 MentalModel 内部结构

路径2：书写传递（WritingImpulse）
  NPC 的 SurfacingType::WritingImpulse → LifeTrace → TextSegment(MentalModelRecord) → PhysicalBook
  后来的NPC阅读 → ReadingSession → assess_and_integrate
  通道：历史系统的已有 LifeTrace API
  对接已有：历史系统只管存储文本，不需要知道 MentalModel

路径3：绘画/音乐传递（DrawingImpulse/PlayingImpulse）
  SurfacingType根据NPC技能选择媒介：
    painting_level > writing_level → DrawingImpulse → 画作物品(ItemDef)
    music_level > writing_level → PlayingImpulse → AestheticEvent
  通道：物品系统已有 ItemCreation / 审美系统已有 AestheticEvent::Create

路径4：长期师徒传递（DeepMentorship）
  ApprenticeshipSession > 365天 → DeepMentorship升级
  → 师父的MentalModel传给徒弟（confidence×0.7）
  → 徒弟的CognitiveStyle被师父的风格拉近（EMA, 0.02/年）
  → 徒弟的core_values与师父对齐
  通道：技能系统的 ApprenticeshipSession 超时升级

路径5：观察归纳
  NPC B 反复观察到 NPC A 的行为模式
  → 对标已有感官PerceptEntry序列 → 记忆编码
  → try_induce_pattern → 从观察中自行归纳MentalModel
  通道：感官→记忆（已有管线）

路径6：死亡遗留（ThoughtImprint）
  NPC死亡时
  → confidence > 0.9 + emotional_charge > 0.7 + source=SelfRefined的模型
  → 有概率（0.05-0.2，取决于emotional_charge）遗留为空间印记
  → 对标已有历史系统 AetherImprint（+1子类型 ThoughtImprint）
  → 后人通过树读机制感知（对标已有感官系统的特殊感知通道）
```

### 8.2 接收端的统一认知评估

```rust
/// 六条传递路径的接收端共享同一个评估函数
/// 纯函数——在NPC crate内部
pub fn assess_and_integrate_mental_model(
    incoming: &MentalModel,
    source: ModelSource,
    source_credibility: CredibilityFactors,
    existing_models: &[MentalModel],
    cognitive_style: &CognitiveStyle,
    personality: &BigFive,
    emotion: &EmotionState,
    self_narrative: &SelfNarrative,
) -> MentalModelIntegration {
    // 1. 理解门槛
    let comprehension = match source {
        ModelSource::ReadFromBook(_) => {
            // 对标语言表达系统的 literacy→理解力映射
            cognitive_style.abstract_concrete * 0.5 + cognitive_style.analytic_intuitive * 0.5
        },
        ModelSource::TaughtByNpc(_) => {
            source_credibility.speaker_charisma * 0.4 + cognitive_style.analytic_intuitive * 0.3 + 0.3
        },
        _ => 0.8,
    };
    
    // 2. 冲突检测
    let conflict = find_most_conflicting_model(incoming, existing_models);
    let conflict_severity = if let Some(ref existing) = conflict {
        if existing.confidence > 0.7 && incoming.confidence > 0.6 {
            Some(ConflictSeverity::Severe)
        } else {
            Some(ConflictSeverity::Moderate)
        }
    } else {
        None
    };
    
    // 3. 信任评估
    let trust_factor = source_credibility.overall_trust() 
                     * personality.agreeableness 
                     * (1.0 - cognitive_style.rigid_flexible * 0.5);
    
    // 4. 情绪染色
    let emotional_mod = 1.0 + emotion.pleasure * 0.2;
    
    // 5. 认知失调容忍检查
    let dissonance_tolerance = compute_biases(cognitive_style, emotion, /*tide*/, personality)
        .cognitive_dissonance_tolerance;
    
    // 6. 整合决策
    if trust_factor * emotional_mod < 0.15 {
        MentalModelIntegration::reject("untrustworthy source")
    } else if conflict_severity == Some(ConflictSeverity::Severe) 
              && dissonance_tolerance < 0.3 {
        MentalModelIntegration::reject("irreconcilable conflict")
    } else if conflict_severity == Some(ConflictSeverity::Moderate) {
        MentalModelIntegration::partial_accept(incoming, conflict.unwrap(), dissonance_tolerance)
    } else {
        let effective_confidence = incoming.confidence 
                                 * trust_factor 
                                 * emotional_mod 
                                 * comprehension;
        MentalModelIntegration::accept(effective_confidence)
    }
}
```

### 8.3 自我重遇：阅读自己过去写的东西

```rust
/// NPC偶然发现30年前自己写的日记
pub fn self_reencounter_reading(
    self_writing: &TextSegment,
    current_npc: &NpcData,
    old_self_snapshot: &HistoricalSelfSnapshot,
) -> SelfReencounterEffect {
    let temporal_distance = current_npc.age - self_writing.created_at;
    let cognitive_gap = compute_cognitive_gap(old_self_snapshot, current_npc);
    
    if cognitive_gap > 0.4 {
        SelfReencounterEffect::CognitiveShock {
            gap: cognitive_gap,
            // "我当年怎么这么想？！"
        }
    } else if temporal_distance > 10.0 {
        SelfReencounterEffect::Nostalgia {
            warmth: cognitive_gap * 0.5 + 0.3,
        }
    } else {
        SelfReencounterEffect::MildRevisitation
    }
}
```

---

## 九、MindAttribution：Theory of Mind

### 9.1 数据结构

```rust
/// NPC关于"另一个NPC在想什么"的归因
/// 对标：MentalModel
/// 上限：16条，约960B
pub struct MindAttribution {
    pub target: NpcId,                        // 我对谁的看法
    pub what_target_thinks: BeliefContent,     // 我认为ta在想/相信什么
    pub about_whom_or_what: Option<EntityId>,  // 关于谁或关于什么
    pub confidence: f32,                       // 我对这个归因的确信度
    pub source: AttributionSource,             // 形成来源
    pub formed_at: GameInstant,
}

pub enum AttributionSource {
    TargetStatedDirectly,      // ta自己说的——最高置信
    ObservedBehavior,          // 我观察到的
    ToldByThirdParty(NpcId),   // 第三方告诉我的
    Inferred,                  // 我自己推理的——最低置信
}
```

### 9.2 MindAttribution的决策影响

- 如果A认为B不喜欢C→A在与B和C同时互动时调整行为
- 如果A认为B不知道X→A可能利用信息不对称（对标DeceptionIntent）
- 对话中→A可能避免在B面前夸C
- 信息传播→A可能决定"这个信息不能告诉B"

**这是已有系统的自然扩展——不新增系统。**

---

## 十、BeliefHistory：信念演变里程碑

### 10.1 数据结构

```rust
/// NPC信念演化史——对标 SelfNarrative.life_chapters
/// 上限16条，约768B
pub struct BeliefHistory {
    pub entries: ArrayVec<BeliefMilestone, 16>,
}

pub struct BeliefMilestone {
    pub timestamp: GameInstant,
    pub belief_snapshot: MentalModelSnapshot,
    pub trigger_event: Option<MemoryId>,
    pub change_type: MilestoneType,
    pub narrative_hook: String,  // "那时候我才明白..."
}

pub enum MilestoneType {
    SourceShifted,       // 外来→内化
    ConfidenceSurge,     // 突然确信
    Abandoned,           // 放弃旧信念
    CreativeLeap,        // 自己想到新想法
    Merged,              // 两个信念融合
    SelfReencountered,   // 重读自己过去的文字
    WorldviewShift,      // 整体世界观变化
}
```

### 10.2 回顾性叙事

老年NPC被问及"你的人生信条是什么？"→扫描BeliefHistory→生成叙事回应：

```
"我年轻时相信X，后来经历了Y（BeliefMilestone），现在我相信Z。"
```

对标已有SelfNarrative.life_chapters的回顾模式。

---

## 十一、泛化质量与过度拟合

### 11.1 泛化质量——纯函数，不存储

```rust
/// 对标Physiology::from_vitals()——从已有数据派生，不存储
pub fn generalization_quality(model: &MentalModel) -> f32 {
    // 高diversity + 中等confidence = 良好泛化
    // 高confidence + 低diversity = 经典过度拟合
    // 低confidence + 低diversity = 欠拟合（没有足够的证据）
    model.supporting_diversity * (1.0 - (model.confidence - 0.5).abs() * 0.4)
}
```

### 11.2 睡眠正则化对泛化质量的维护

详见 [[005-睡眠认知加工与正则化]]。

### 11.3 Zeigarnik效应

```rust
/// 未消化的不确定模型→占用反刍资源
pub fn zeigarnik_contribution(models: &[MentalModel]) -> f32 {
    models.iter()
        .filter(|m| !m.abandoned)
        .filter(|m| m.confidence > 0.3 && m.confidence < 0.7)  // 不确定区间
        .filter(|m| m.counter_count > 0)                         // 有未消化的反例
        .map(|m| m.emotional_charge.abs() * (1.0 - m.confidence.abs()))
        .sum::<f32>()
        .min(1.0)
}
```

### 11.4 惰性遗忘

```rust
/// MentalModel未被激活的时间越长→激活阈值越高→越难被检索
pub fn update_activation_threshold(model: &mut MentalModel, current: GameInstant) {
    let days_inactive = (current - model.last_activated).0 as f32;
    // 指数松弛：半年不用→0.7，一年→0.85
    model.activation_threshold = 0.3 + 0.7 * (1.0 - (-0.005_f32 * days_inactive).exp());
}
```

---

## 十二、回答关键设计问题

### Q: NPC学习别人的东西能消化成自己的想法吗？

**能。** 通过双阶段消化管线：
- **微消化**：每记忆编码时贝叶斯更新→用新经验持续验证外来信念
- **宏消化**：每7天→消化深度>0.8时source从`TaughtByNpc/ReadFromBook`转移为`SelfRefined`

一年后，高反思型+高灵活的NPC可能已将外来信念完全内化——甚至忘了是从哪学的。低反思型+顽固的NPC可能仍保持原始source。

### Q: NPC的思考能有逻辑关系吗？

**能。** 通过关联链模型：触发记忆→共享参与者/地点/情绪→跳转→推理。不是形式逻辑（那是性能灾难），而是惰性扩展的关联网络遍历。每一步受CognitiveStyle调制——分析型NPC跳得深、直觉型NPC在第一跳就产出结论。

### Q: 思考能掺杂不理性因素吗？

**必须能。** 确认偏误放大支持证据、负面偏误让坏事权重更高、自我服务偏差让成功归因于能力失败归因于环境。这些不是设计缺陷——是活人感的核心。

### Q: 思考能支撑感性为主角、理性为佐料吗？

**能。** "伤春悲秋"的完整链条已经存在：
```
下雨(Weather) → 视觉场景阴沉(感官) → 情绪melancholy(情绪引擎)
→ 同情绪记忆检索(记忆) → LifeChapter丧失章节激活(SelfNarrative)
→ ThoughtFragment浮现(本模块) → NPC叹气/停步/望窗外(Godot)
```
本模块只添加了**情绪→记忆→自我叙事→思考**的自动桥接（`aesthetic_contemplation_chain()`），链条的每一环都已存在。

---

> **下一文档**: [[004-思考涌现与浮现机制]]
