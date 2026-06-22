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

### 1.1 PatternExpression —— 数学地基 ★ v1.1 新增

> **设计裁决 (CHG-057)**: PatternExpression 是整个认知系统的数据结构地基。Creative Leap、领域分类、学科涌现、修辞渲染全部构建在它之上。定义在 `woworld_core` —— 零依赖，全模块共享。

```rust
// ═══════════════════════════════════════════════════════════════
// woworld_core/src/pattern.rs — 零依赖
// ═══════════════════════════════════════════════════════════════

/// "当 X 发生时，Y 倾向于跟随" 的数学表达
/// ~120 字节。20 个/NPC = ~2.4KB。1000 L1 = ~2.4MB。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternExpression {
    /// 因果步骤序列（≤8 步，栈分配——零堆分配）
    pub steps: ArrayVec<PatternStep, 8>,

    /// 结构指纹：hash(atom_index, relation, role) —— 不含命名空间和领域上下文
    /// 用于 Creative Leap 跨领域结构匹配。预计算，O(1) Hamming 距离。
    pub structural_lsh: u64,

    /// 领域签名：hash(atom_class 完整 u16, context_hashes)
    /// 用于领域分类、formalize 路由、学科聚类。预计算，O(1)。
    pub domain_signature: u64,

    /// 迁移版本号
    pub version: u8,
}

impl PatternExpression {
    pub fn from_steps(steps: ArrayVec<PatternStep, 8>, context_hashes: &[u64]) -> Self {
        let structural_lsh = Self::compute_structural_lsh(&steps);
        let domain_signature = Self::compute_domain_signature(&steps, context_hashes);
        PatternExpression { steps, structural_lsh, domain_signature, version: 1 }
    }
}
```

#### PatternStep

```rust
/// 模式中的一个因果步骤
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PatternStep {
    /// 原子动作 —— 统一分类（物理/权力/未来扩展）
    pub atom_class: AtomClass,

    /// 此步与下一步的关系
    pub relation: CausalRelation,

    /// 此步在整体模式中的角色
    pub role: StepRole,
}
```

#### AtomClass —— 统一原子分类

```rust
/// 统一原子分类。u16 编码：高 4 位 = 命名空间，低 12 位 = 命名空间内索引。
/// 命名空间常量定义在 woworld_core。各域 crate 实现 IntoAtomClass。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AtomClass(pub u16);

impl AtomClass {
    pub const PHYSICAL: u8 = 0;   // MOVE, GRASP, STRIKE, HEAT, DIP, OBSERVE, ...
    pub const POWER: u8 = 1;      // Constitute, Compel, Extract, Adjudicate, ...
    // 预留: MAGIC=2, ECONOMY=3, FAITH=4, BUILDING=5, ...

    pub fn namespace(self) -> u8 { (self.0 >> 12) as u8 }
    pub fn index(self) -> u16 { self.0 & 0x0FFF }
}

/// 域 crate 实现此 trait 来注册其原子类型
pub trait IntoAtomClass: Copy {
    fn into_atom_class(self) -> AtomClass;
}
```

#### 关系和角色

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CausalRelation {
    Before,       // 时间先后，不声称因果
    Causes,       // 此步导致下一步
    Enables,      // 此步使下一步可能
    Prevents,     // 此步阻止了本应发生的后续
    Intensifies,  // 此步放大下一步的效果
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StepRole {
    Precondition, // 核心动作的前置条件
    CoreAction,   // 模式的中心动作
    Effect,       // 核心动作的结果
    Modifier,     // 调节核心动作或效果的强度/品质
}
```

#### 签名计算（纯函数，woworld_core）

```rust
impl PatternExpression {
    /// 结构 LSH：只哈希 (atom_index, relation, role)
    /// 不考虑 atom_namespace。两个 pattern 具有相同骨架但不同领域
    /// → structural_lsh 相似 → Creative Leap 匹配。
    fn compute_structural_lsh(steps: &[PatternStep]) -> u64 {
        let mut h = FxHasher::default();
        for w in steps.windows(2) {
            w[0].atom_class.index().hash(&mut h);  // 只有索引——不含命名空间
            w[0].relation.hash(&mut h);
            w[0].role.hash(&mut h);
            w[1].atom_class.index().hash(&mut h);
        }
        if steps.len() == 1 {
            steps[0].atom_class.index().hash(&mut h);
        }
        h.finish()
    }

    /// 领域签名：哈希完整 atom_class（含命名空间）+ 上下文哈希
    /// 上下文哈希由认知 crate 在归纳时从诱导记忆的 EntityKind、MaterialCategory 等计算。
    fn compute_domain_signature(steps: &[PatternStep], context_hashes: &[u64]) -> u64 {
        let mut h = FxHasher::default();
        for step in steps {
            step.atom_class.0.hash(&mut h);  // 完整 u16 —— 含命名空间
        }
        for ctx in context_hashes {
            ctx.hash(&mut h);
        }
        h.finish()
    }
}
```

#### 相似度函数

```rust
/// 模式结构相似度 —— Creative Leap 用
pub fn pattern_similarity(a: &PatternExpression, b: &PatternExpression) -> f32 {
    let hamming = (a.structural_lsh ^ b.structural_lsh).count_ones() as f32;
    1.0 - hamming / 64.0
}

/// 领域相似度 —— Creative Leap 过滤、学科聚类用
pub fn domain_similarity(a: &PatternExpression, b: &PatternExpression) -> f32 {
    let hamming = (a.domain_signature ^ b.domain_signature).count_ones() as f32;
    1.0 - hamming / 64.0
}

/// 是否是 Creative Leap 的有效跨域配对？
pub fn is_cross_domain_pair(a: &PatternExpression, b: &PatternExpression) -> bool {
    let ps = pattern_similarity(a, b);
    let ds = domain_similarity(a, b);
    ps > 0.6          // 结构足够相似 → 可能有可迁移的骨架
    && ds > 0.1       // 领域不完全无关 → 至少有一些共同点
    && ds < 0.6       // 领域不完全相同 → 这才是"跨"领域
}
```

**结构 LSH 的已知局限**：不编码图的完整拓扑——"A→B→C" 和 "A→C→B" 的 structural_lsh 相同。这是用 O(1) 替代图同构（NP-complete）的近似代价。False positive 由 Bayesian 验证循环淘汰。**不完美是涌现的条件**——人类创作中虚假类比的产出同样常见。

---

### 1.2 MentalModel 定义 ★ v1.1 更新

```rust
/// 一个MentalModel代表NPC持有的一个"世界如何运作"的信念
/// 上限：20条，约2.4KB（含 PatternExpression）
#[derive(Clone, Serialize, Deserialize)]
pub struct MentalModel {
    pub id: MentalModelId,
    pub pattern: PatternExpression,           // "当X发生时，Y倾向于跟着发生" ★ v1.1: 类型从占位符升级为完整规范
    pub confidence: f32,                      // 贝叶斯置信度 0-1
    pub supporting_count: u16,               // 正面实例计数
    pub counter_count: u16,                   // 反面实例计数
    pub supporting_diversity: f32,            // 支持证据的情境多样性 (0=全同, 1=高度多样)
    pub emotional_charge: f32,                // 信念的情绪附着强度
    pub emotional_valence: f32,               // 正面或负面信念 (-1=负面, 1=正面)
    pub source: ModelSource,                  // 从哪来的
    pub source_age: f32,                      // 游戏日——持有多久了
    pub formed_at: GameInstant,
    pub last_activated: GameInstant,
    pub last_tested: GameInstant,
    pub activation_threshold: f32,
    pub shareability: Shareability,
    pub hypothesis: bool,
    /// ★ v1.3: 仅模型融合时设为 true（被合并到另一个模型中）
    /// 被证伪的信念不再设置 abandoned——改为 confidence=0.0 保留为"教训"
    pub abandoned: bool,
}

impl MentalModel {
    /// ★ v1.1: domain_signature() 从 pattern 衍生——纯函数，不存储
    /// 替代 v1.0 的 `domain: MentalModelDomain` 枚举
    pub fn domain_signature(&self) -> u64 {
        self.pattern.domain_signature
    }
}

/// ★ v1.1: MentalModelDomain 枚举已移除（CHG-057）
/// 领域完全从 PatternStep.atom_class 的组合中涌现
/// 两个模型的领域关系由 domain_similarity() 的 Hamming 距离决定

pub enum ModelSource {
    SelfDerived,
    SelfRefined,
    TaughtByNpc(NpcId),
    ReadFromBook(WorkId),
    Inherited,
}

pub enum Shareability {
    Public,
    InGroup { group: GroupFilter },
    SpecificTarget { target: NpcId },
    Private,
}
```

---

### 1.3 DomainSignature —— 领域签名 ★ v1.1 新增

```rust
/// 领域签名：u64 hash，从 PatternExpression 的 atom_class + context_hashes 衍生
/// 替代 v1.0 的 MentalModelDomain 枚举（17个硬编码变体）
/// 
/// 使用方式：
///   - Creative Leap 跨域配对：is_cross_domain_pair(a, b) 要求 ds ∈ [0.1, 0.6]
///   - formalize_innovation() 路由：各模块按 ATOM_MASK 注册消费者
///   - 学科涌现：HDBSCAN* 在 domain_signature 空间中聚类 NPC 的 MentalModel
///   - 修辞渲染：相邻 UtteranceConcept 的 domain_similarity < 0.4 → 比喻
pub struct DomainSignature(pub u64);

impl DomainSignature {
    /// 两个领域有多不相似？Hamming 距离归一化
    pub fn similarity(self, other: Self) -> f32 {
        let hamming = (self.0 ^ other.0).count_ones() as f32;
        1.0 - hamming / 64.0
    }
}
```

**与 PatternSignature 的区别**：
- `PatternSignature(u64)`：权力原子类型的哈希，用于文化概念分类（`classify_pattern()`）。定义在概念与语言地基。
- `DomainSignature(u64)`：物理/权力原子类型 + 实体上下文的哈希，用于认知领域分类。定义在 woworld_core。

两者都是 u64 但输入空间不同——PatternSignature 用于"这种关系在我们的语言里叫什么"，DomainSignature 用于"这个信念属于什么领域"。不冲突。

---

### 1.4 容量管理 ★ v1.1 更新：四层压缩

MentalModel 上限 20 条。淘汰优先级不变。但淘汰的模型进入**四层压缩体系**（详见 [[#十三、记忆的四层压缩架构|§十三]]）：

| 淘汰原因 | 去向 |
|---------|------|
| abandoned=true（模型融合产生） | 直接删除 |
| confidence=0.0 超过 3 条时最老的零置信 | 压缩为 BeliefMilestone → BeliefHistory |
| 最久未激活（容量压力） | 压缩为 BeliefMilestone → BeliefHistory |
| 模型合并（macro_digest 融合） | 两个相似模型 → 一个，融合后的停留或也进入压缩 |

```rust
/// ★ v1.3: 零置信模型保留为"教训"——最多 3 条。超出者移入 BeliefHistory
const MAX_ZERO_CONFIDENCE_MODELS: usize = 3;

fn manage_mental_model_capacity(models: &mut ArrayVec<MentalModel, 20>, history: &mut BeliefHistory) {
    if models.len() >= 20 {
        // 1. 清除合并产生的废弃模型
        models.retain(|m| !m.abandoned);
        
        // 2. 零置信模型超过上限 → 淘汰最老的到 BeliefHistory
        let zero_count = models.iter().filter(|m| m.confidence == 0.0).count();
        if zero_count > MAX_ZERO_CONFIDENCE_MODELS {
            let excess = zero_count - MAX_ZERO_CONFIDENCE_MODELS;
            let zero_indices: Vec<usize> = models.iter().enumerate()
                .filter(|(_, m)| m.confidence == 0.0)
                .map(|(i, _)| i)
                .collect();
            for &idx in &zero_indices[..excess] {
                let model = models.remove(idx);
                history.entries.push(BeliefMilestone::from_archived_model(&model));
                // BeliefHistory 已保留 Abandoned 里程碑——分享时可从这里查询
            }
        }
        
        // 3. 标准淘汰
        while models.len() > 20 {
            let oldest_idx = models.iter().enumerate()
                .min_by_key(|(_, m)| m.last_activated)
                .map(|(i, _)| i);
            if let Some(idx) = oldest_idx {
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
        // ★ v1.1: domain 字段已移除。领域从 pattern.domain_signature 涌现。
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
        .filter(|m| {
            // ★ v1.1: 用 domain_similarity 替代 domain 枚举相等
            domain_similarity_raw(m.domain_signature(), new_memory_event_domain_sig(new_memory)) > 0.5
        }) 
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
        
        // 4. ★ v1.3: 证伪——不删除。置信度归零 = "我知道这是错的"
        // 旧行为: abandoned = true → 容量管理器删除 → 教训丢失
        // 新行为: confidence = 0.0 + 情绪翻转 → 保留作为"教训"
        if models[i].confidence < 0.1 
           && models[i].counter_count > models[i].supporting_count * 3
           && models[i].emotional_charge < 0.3 
        {
            models[i].confidence = 0.0;                        // 绝对零——"已被证明是错的"
            models[i].emotional_valence *= -0.5;               // 翻转情绪——"不再喜欢这个做法"
            models[i].source = ModelSource::SelfRefined;       // "我自己悟出来这是错的"
            // ★ 不设 abandoned = true。模型保留在数组中，占用一个槽位。
            // 分享时——confidence=0 → effective_confidence=0 → 接收方自然地接受为"教训"
            // Bayesian 锁零: posterior = (0×L)/(0×L+1×(1−L)) = 0 → 永不恢复
            // 若产生新模式（不同 pattern）→ try_induce_pattern() 创建新模型而非旧模型康复
            events.push(MentalModelEvent::Abandoned { 
                model_id: models[i].id,
                reason: "disproven by evidence".into(),
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
    
    // ★ v1.1 更新：使用 woworld_core 的 is_cross_domain_pair() 和 pattern_similarity()
    // 跨领域配对在 DomainSignature 的 Jaccard 空间中采样——偏好 [0.1, 0.6] 的配对
    // 不再依赖 MentalModelDomain 枚举的硬编码不等
    
    let pairs = sample_cross_domain_pairs(models, 20, rng);  // ★ LSH O(1) 允许采样更多配对
    
    for (model_a, model_b) in &pairs {
        if !is_cross_domain_pair(&model_a.pattern, &model_b.pattern) {
            continue;  // 同域或完全不相关→跳过
        }
        let structural_similarity = pattern_similarity(&model_a.pattern, &model_b.pattern);
        
        if structural_similarity > 0.6 
           && rng.gen_bool((structural_similarity - 0.5) * 0.3) 
        {
            return Some(MentalModel {
                id: MentalModelId::new(),
                pattern: synthesize_pattern(model_a, model_b, structural_similarity),
                confidence: 0.15 + structural_similarity * 0.15,  // 0.15-0.30
                hypothesis: true,  // 标记为假说——消化中特殊对待
                source: ModelSource::SelfDerived,
                shareability: Shareability::InGroup { group: GroupFilter::Family },
                ..MentalModel::default()
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

### Q: NPC 舍弃错误想法后——能把这个教训分享给别人吗？

**能。** 被证伪的信念不再设置 `abandoned = true`（那会导致删除）。改为 `confidence = 0.0` + `emotional_valence 翻转` + `source = SelfRefined`。每个 NPC 最多保留 3 条零置信模型。

分享时——走已有 WisdomSharing。接收方 `assess_and_integrate_mental_model()` 的 `effective_confidence = incoming.confidence × trust_factor × ... = 0 × ... = 0`。接收方得到一个置信度为零的"教训"。如果接收方已持有同模式的高置信度模型——冲突检测触发 MentalModelTest——可能动摇原有信念。

**关键**：绝对零（非 ε）是 Bayesian 吸收态。这个模型不会从 0 被新证据恢复。如果变体模式正确（如"淬火后擦油在低温时有用"）——会通过 `try_induce_pattern()` 产生一个**新的** MentalModel，有不同的 PatternExpression。不是旧模型康复。

零置信模型超出 3 条上限时——最老的移入 BeliefHistory（已有 `Abandoned` 里程碑）。多人对话中查询历史信念时——从 BeliefHistory 检索。

---

## 十三、预测与反事实后悔 ★ v1.1 新增 (CHG-057)

### 13.1 predict_outcome() —— 将已有模式应用于候选路径

> **设计原则**: 预测不是世界仿真。是将 MentalModel（从过去经验归纳的因果模式）应用于候选决策选项——纯函数，O(log n)，零 World 引用。

```rust
/// 预测一个决策选项的情感结果
/// 不为未走过的路径运行模拟——只查询匹配 domain_signature 的已有 MentalModel。
pub struct OutcomePrediction {
    pub expected_pleasure: f32,     // 预测的愉悦度（-1 ~ 1）
    pub confidence: f32,            // 支撑此预测的 MentalModels 平均 confidence
    pub model_count: u8,            // 参与聚合的模型数（<2 = 本质在猜）
    pub variance: f32,              // 不同模型预测的方差（>0.3 = 高度不确定）
}

pub fn predict_outcome(npc: &NpcData, option: &DecisionOption) -> OutcomePrediction {
    let sig = option.domain_signature();
    let matching: Vec<&MentalModel> = npc.mental_models.iter()
        .filter(|m| !m.abandoned && m.confidence > 0.2)
        .filter(|m| domain_similarity_raw(m.domain_signature(), sig) > 0.3)
        .collect();

    if matching.is_empty() {
        return OutcomePrediction {
            expected_pleasure: 0.0,
            confidence: 0.0,
            model_count: 0,
            variance: 0.0,
        };
    }

    // 加权聚合：confidence × emotional_valence × (1 + supporting_diversity) / (1 + source_age)
    let total_weight: f32 = matching.iter().map(|m| aggregation_weight(m)).sum();
    let expected: f32 = matching.iter()
        .map(|m| m.emotional_valence * aggregation_weight(m))
        .sum::<f32>() / total_weight.max(0.001);
    let avg_conf = matching.iter().map(|m| m.confidence).sum::<f32>() / matching.len() as f32;
    let variance = matching.iter()
        .map(|m| (m.emotional_valence - expected).powi(2) * aggregation_weight(m))
        .sum::<f32>() / total_weight.max(0.001);

    OutcomePrediction {
        expected_pleasure: expected.clamp(-1.0, 1.0),
        confidence: avg_conf,
        model_count: matching.len() as u8,
        variance,
    }
}

fn aggregation_weight(model: &MentalModel) -> f32 {
    model.confidence * (1.0 + model.supporting_diversity) / (1.0 + model.source_age * 0.01)
}
```

**关键**：`variance > 0.3` 或 `model_count < 2` → NPC "知道自己不知道"。低确定性预测不会驱动高 stakes 的深思熟虑。

### 13.2 counterfactual_regret() —— 比较实际结果与预测

```rust
/// 评估一个过去决策的反事实后悔
/// 将 MentalModel 应用于未选择的路径 → 比较预测与实际
pub fn counterfactual_regret(npc: &NpcData, decision: &DecisionPoint) -> Option<f32> {
    if decision.unchosen.is_empty() { return None; }
    
    let mut max_predicted = f32::MIN;
    for alt in &decision.unchosen {
        let pred = predict_outcome(npc, alt);
        if pred.model_count >= 1 {
            max_predicted = max_predicted.max(pred.expected_pleasure);
        }
    }
    
    if max_predicted == f32::MIN { return None; }
    
    let regret = (max_predicted - decision.actual_outcome_pleasure).clamp(0.0, 1.0);
    if regret > 0.3 { Some(regret) } else { None }  // 阈下后悔不记录
}
```

**触发时机**：
- **Macro-digestion**（每 7 天）：扫描带 DecisionPoint 的记忆 → counterfactual_regret()
- **ThoughtTrigger::check_wander**（mind_wander_drive > 0.7）：随机回顾一个过去的决策
- **对话触发**：玩家或 NPC 问"你后悔吗？"→ 查询最高 regret 的 DecisionPoint

高后悔 → 编码 Revelation 型 EventMemory → 触发 MentalModel 更新（"我意识到当时的信念可能是错的"）。

### 13.3 DecisionPoint 存储与修剪

```rust
/// 加入 EventMemory（NPC crate 内部）
/// 惰性修剪：仅当 DecisionPoint 无后果时才丢弃
pub struct DecisionPoint {
    pub chosen: DecisionOption,
    pub unchosen: ArrayVec<DecisionOption, 3>,  // ≤3 个可选替代方案
    pub actual_outcome_pleasure: f32,           // 事后填入
    pub formed_at: GameInstant,
}

pub struct DecisionOption {
    pub description: String,
    pub atom_classes: Vec<AtomClass>,  // 涉及的原子类型 → domain_signature
    pub predicted_outcome_pleasure: Option<f32>,
}

impl DecisionOption {
    pub fn domain_signature(&self) -> u64 {
        let mut h = FxHasher::default();
        for ac in &self.atom_classes {
            ac.0.hash(&mut h);
        }
        h.finish()
    }
}
```

修剪策略（macro_digestion 中）：如果 DecisionPoint 产生后 30 天内未产生任何 impact > 0.5 的后续事件 → 删除 DecisionPoint。

### 13.4 source_confidence 分类型衰减 ★ v1.2 新增 (CHG-058)

```rust
/// ★ v1.2: source_confidence 衰减率因 MemorySource 类型而异
/// Inferred（自我推理）初始最低但衰减最慢——自我推理的记忆比道听途说更抗遗忘
pub fn source_confidence_decay_rate(source: MemorySource) -> f32 {
    match source {
        MemorySource::Witnessed => 0.97,     // 最慢："我亲眼看到的"
        MemorySource::Inferred => 0.95,       // 慢：自我推理的信念抵抗遗忘
        MemorySource::ReadFrom(_) => 0.90,    // 中：书面来源稳定但可模糊
        MemorySource::HeardFrom(_) => 0.85,   // 最快：道听途说退化最快
    }
}

/// 年度更新——替换原先的统一 decay_factor 乘法
fn update_source_confidence(mem: &mut EventMemory, years: f32) {
    let rate = source_confidence_decay_rate(mem.original_source);
    mem.source_confidence *= rate.powf(years);
}
```

**衰减验证**（Python）：
```
Year 5: Witnessed=0.816, Inferred=0.310, ReadFrom=0.502, HeardFrom=0.311
Inferred-HeardFrom crossover: 5.0 years
```
5 年后道听途说比自我推理更不可靠——认知上正确。

---

## 十三-A、群体心理免疫 ★ v1.1 新增 (CHG-057 补充)

> **设计裁决**: crowd_suggestibility 不是所有人都同等受影响。NPC 对群体心理的抵抗从七个维度涌现——不是 `if immune { skip }` 的二元门。

### 群体免疫度 —— 连续涌现量

```rust
/// 群体免疫度：0 = 完全从众，1 = 完全免疫
/// 七个抗性因子乘性聚合——任何一个因子为 0 都可能将免疫度压到很低
pub fn crowd_immunity(npc: &NpcData, crowd_group: &GroupId) -> f32 {
    // 因子 1: 认知中介 — 反思型+高安静度 → 有意识地审视群体情绪
    let cognitive_mediation = npc.cognitive_style.reflective_impulsive * 0.6
                            + npc.cognition_tide.mind_quietude * 0.4;

    // 因子 2: 对立身份 — 群体是 out-group → 情绪传染自然减弱
    let identity_distance = if npc.belongs_to(crowd_group) { 0.0 }   // 我是群体一员
                           else if npc.hostile_to(crowd_group) { 1.0 } // 敌视→强免疫
                           else { 0.4 };                               // 中立 outsider

    // 因子 3: 已有信念 — MentalModel 与 crowd 叙事矛盾 → 抵抗
    let belief_conflict = npc.mental_models.iter()
        .filter(|m| m.confidence > 0.6)
        .map(|m| {
            // 如果 crowd 的主导情绪与模型的 emotional_valence 符号相反
            // → 此模型抵制 crowd 叙事
            if (crowd_dominant_valence(crowd_group) * m.emotional_valence) < 0.0 {
                m.confidence * m.emotional_charge.abs()
            } else { 0.0 }
        })
        .max()
        .unwrap_or(0.0);

    // 因子 4: 情绪稳定性 — 低神经质 → 不易被感染
    let emotional_stability = 1.0 - npc.personality.neuroticism;

    // 因子 5: 专业训练 — 相关领域的技能等级 → 专业判断压倒 crowd 恐慌
    // (战士不会被战斗恐慌感染, 水手不会被风暴恐慌感染)
    let professional_training = match crowd_context(crowd_group) {
        CrowdContext::CombatThreat => npc.skills.get(SkillId::Combat).normalized() * 0.7,
        CrowdContext::NaturalDisaster => npc.skills.get(SkillId::Navigation).normalized() * 0.5,
        CrowdContext::EconomicPanic => npc.skills.get(SkillId::Trade).normalized() * 0.5,
        _ => 0.0,
    };

    // 因子 6: 低宜人性 — 不随和的人不随大流
    let low_agreeableness = 1.0 - npc.personality.agreeableness;

    // 因子 7: 社会距离 — 我与群体的社会距离 (陌生人群→我无所谓)
    let social_distance = social_distance_to_group(npc, crowd_group).clamp(0.0, 1.0);

    // 七因子加权聚合 (权重反映各因子的相对重要性)
    let raw_immunity = cognitive_mediation * 0.22
                     + identity_distance * 0.18
                     + belief_conflict * 0.18
                     + emotional_stability * 0.12
                     + professional_training * 0.10
                     + low_agreeableness * 0.12
                     + social_distance * 0.08;

    // 非线性映射：低中段急速升起（"稍微怀疑就不从众"），高段渐进（"完全免疫很难"）
    // sigmoid: 中心在 0.35, 斜率 8
    1.0 / (1.0 + (-(raw_immunity - 0.35) * 8.0).exp())
}

/// 有效可暗示性 = 原始可暗示性 × (1 − 免疫度)
pub fn effective_suggestibility(npc: &NpcData, crowd_group: &GroupId) -> f32 {
    let raw_suggestibility = crowd_suggestibility(npc);  // 从 PerceptBatch 派生
    let immunity = crowd_immunity(npc, crowd_group);
    raw_suggestibility * (1.0 - immunity)
}
```

### 玩家可见的涌现行为

| NPC 类型 | 免疫因子配置 | 行为 |
|----------|------------|------|
| 冷静学者 | reflective=0.9, quietude=0.8, neuroticism=0.2 | 人群中其他人恐慌时站着不动，观察，可能记笔记 |
| 外乡人 | identity_distance=1.0 | "这些本地人在慌什么？"——情感不感染 |
| 老兵 | professional_training=0.9 (Combat) | 新兵四散逃跑时老兵守住阵地——不是勇敢，是技能压制了恐慌 |
| 顽固怀疑者 | low_agreeableness=0.9, rigid_flexible=0.1 | 不管别人怎么说都不信——信念难以改变 |
| 被剥夺者 | social_distance=1.0, belief_conflict=0.8 | "这和我有什么关系？你们权贵的事" |

**关键涌现点**：同一个 NPC 在不同 crowd 中有不同的免疫度。老兵在战斗 crowd 中免疫 → 在社会骚乱 crowd 中可能被感染（pro_training=0 for SocialUnrest）。外乡人在本地人的恐慌中免疫 → 在自己的家乡人群中无免疫。

这与 CollectiveEmotionPhase::Immunity 的关系：
- `CrowdImmunity`：**个体**对**特定群体**的抵抗——连续量
- `CollectiveEmotionPhase::Immunity`：**群体层面**的冷静少数识别——从众多个体的 crowd_immunity 中涌现
- 当 5+ NPC 的 crowd_immunity > 0.7 且属于同一群体 → CollectiveEmotionPhase::Immunity 被触发

---

## 十三-B、NPC 人格概念抽象 ★ v1.1 新增 (CHG-057 补充)

> **设计裁决**: NPC 不接触 BigFive——那是开发者的建模工具。NPC 从行为观察中归纳出自己文化的人格概念。勇气、智慧、开朗、忧郁——这些概念在 NPC 的 MentalModel 中涌现，而不是被设计者预设。

### 人格概念的涌现路径

```
观察 Alice: OBSERVE(tremble) + OBSERVE(flee) + OBSERVE(cry) → [3次高impact记忆]
    → try_induce_pattern() → MentalModel: "某些生物对威胁的反应更强烈"
    → DomainSignature: OBSERVE | POSTURE | MOBILITY | face_expressions | context(Threat)
    
观察 Bob: OBSERVE(smile) + OBSERVE(approach strangers) + OBSERVE(talk) → [3次记忆]
    → try_induce_pattern() → MentalModel: "某些生物主动接近陌生生物"
    → DomainSignature: OBSERVE | POSTURE | MOBILITY | SPEAK | context(Social)

NPC 学者比较 Alice 和 Bob 的模式：
    → creative_leap() 或 try_induce_pattern() 在 DomainSignature 空间中
    → 高 DomainSignature 相似度（都涉及 POSTURE+MOBILITY+face）
    → 抽象出：生物有不同的"行为倾向"——人格维度的最初形式
    → 写作 → TextSegment(人格理论) → PhysicalBook → 传播
```

### 文化的 concept space 命名人格类型

```rust
// NPC 学者持有 MentalModel "人分两种：怯者和勇者"
// 这个模型的 pattern.steps 中 atom_class 涉及 OBSERVE 行为差异
// → DomainSignature 区别于"人分两种：贵族和平民"（那涉及权力 atoms）

// 概念与语言地基的 classify_pattern() 将这个 DomainSignature
// 映射到文化概念空间:
//   文化 A: "怯" (ConceptLocalId = 42), "勇" (43)
//   文化 B: "fearful" (ConceptLocalId = 87), "brave" (88)
//   文化 C: 不分怯/勇——分"冷性"和"热性"，包含更多 behavior atoms
```

**不同文化的人格分类完全不同**：
- 文化 A：两分法——"勇者 / 怯者"
- 文化 B：五分法——类似于 BigFive 但边界和命名不同
- 文化 C：不分人格——把行为差异归因于"命星"（天文 atoms），不归因于人格
- 文化 D：八分法——包含体态特征（height/weight atoms），"高壮型"是一种人格

这些差异从三个源头涌现：
1. **NPC 群体观察到不同的行为原子组合**（不同文化中的人确实行为不同）
2. **ConceptSpace 的聚类不同**（同一组 DomainSignature 在不同文化中被切分不同）
3. **Creative Leap 产生的人格理论不同**（学者用不同的源域来类比人格——"体液说"vs"星座说"vs"心理论"）

### NPC 如何描述另一个 NPC 的人格

对话中：
```
NPC A: "Bob 是个怯者。"
    → A 持有 MentalModel(关于怯者的 pattern)
    → 检查 Bob 的行为记忆中是否有 matching pattern
    → confidence = f(matching_count / total_observations)
    → 生成 UtteranceConcept(concept_id=文化A的"怯"概念, confidence=0.7)
    → TextGenerator 渲染 → "Bob 很胆小。"
```

**不需要新系统。** 这就是 NPC 对另一个 NPC 应用 MentalModel——与"铁匠对刀用淬火技术"的应用是同一个机制。`MindAttribution`（Theory of Mind, §九）已经为此提供了数据结构。

### 与 BigFive 的关系

| | BigFive（开发者模型） | 文化人格概念（NPC 模型） |
|---|---|---|
| 定义者 | 游戏设计者 | NPC 学者群体 |
| 维度数 | 5（固定） | 涌现（2~8，各文化不同） |
| 数学 | O/C/E/A/N 5 个 f32 | DomainSignature 空间聚类 |
| 可变性 | 极少变（仅在极端冲击） | NPC 的 MentalModel confidence 随证据更新 |
| 精度 | 连续 | 离散 + 模糊边界 |
| 消费者 | 认知风格派生、需求敏感度 | NPC 的社交决策、对话内容、书籍著作 |

BigFive 是**物理引擎**——驱动 CognitiveStyle 的派生和情绪/行为的数学。文化人格概念是**NPC 对物理引擎的观测理论**——可能准确、可能偏差、可能完全不同。

> **设计裁决**: 深思熟虑不应该是 `if reflective_impulsive > 0.4` 的硬编码门。它是连续涌现量——特质能力 × 状态能力 × 动机。

```rust
/// 深思熟虑深度 —— 每决策周期，当 GOAP top2 权重差 < 15% 时计算
pub fn deliberation_depth(npc: &NpcData, candidates: &[ActionCandidate]) -> f32 {
    // ── 特质能力：随时间缓慢变化的思考倾向 ──
    // ★ CognitiveStyle 不是一生不变的——它通过三种机制逐渐变化：
    //   1. 阻尼重派生 (每7天): 旧值×damping + 新派生×(1-damping), damping∈[0.50,0.85]
    //   2. 事件调制 (impact>0.8): 创伤 → rigid_flexible↓,reflective↑; 启示 → rigid↑,abstract↑
    //   3. 睡眠剥夺 (临时): >48h清醒 → rigid_flexible 暂时 -0.3
    // 年轻时 damping=0.85, 年长时 damping=0.50——越老越容易改变。
    // 详见 [[002-CognitiveStyle与认知偏误详细设计#1-5-cognitivestyle-的可变性与不可变性|CognitiveStyle 可变性]]。
    let trait_capacity = npc.cognitive_style.reflective_impulsive    // 0-1
        * (1.0 - npc.cognitive_style.rigid_flexible * 0.3);         // 僵化者不思

    // ── 状态能力：此刻脑子清不清醒 ──
    let state_capacity = npc.cognition_tide.mind_quietude
        * (1.0 - npc.cognition_tide.cognitive_load)
        * (1.0 - npc.physiology.intoxication)        // ★ v1.2: AgentSnapshot 是 transient——不存于 NpcData
        * (1.0 - npc.physiology.fatigue)
        * (1.0 - npc.cognitive_distress)
        * (1.0 - npc.emotion.arousal.abs() * 0.7);

    // ── 动机：选项的预测结果差异有多大 ──
    let predictions: Vec<OutcomePrediction> = candidates.iter()
        .take(3)
        .map(|c| predict_outcome(npc, &c.decision_option))
        .collect();
    let stakes = if predictions.len() >= 2 {
        let variance = statistical_variance(&predictions.iter().map(|p| p.expected_pleasure).collect::<Vec<f32>>());
        1.0 / (1.0 + (-(variance - 0.05) * 15.0).exp())  // sigmoid
    } else { 0.0 };

    trait_capacity * state_capacity * stakes
}
```

**玩家可见行为**：
- `depth > 0.3` → ThoughtFragment + SurfacingType::Pause（停顿）
- `depth > 0.5` → GOAP 决策周期从 0.3s 延长到 1-5s
- `depth > 0.7` → NPC 可能在 mutter（喃喃自语）中暴露思考内容
- 低 depth → NPC 不假思索地选效用最高的。可能是冲动的，也可能是清醒的——都合理。

**示例**：
- 清醒的哲学家面对去哪吃饭 → trait=0.9, state=0.9, stakes=0.01 → depth≈0.008 → 不思（没什么好想的）
- 醉酒的哲学家面对搬家 → trait=0.9, state=0.1, stakes=0.7 → depth≈0.06 → 几乎不思（醉了）
- 清醒的农民面对迁徙 → trait=0.2, state=0.9, stakes=0.6 → depth≈0.11 → 不太思（本性不深思）
- 清醒的哲学家面对职业改变 → trait=0.9, state=0.9, stakes=0.7 → depth≈0.57 → 仔细深思

### 14.1 Deliberation Override ★ v1.2 新增 (CHG-058)

> **设计裁决**: deliberation_depth 不仅要减慢决策——它必须能改变选择。当反思型 NPC 的长期情感预测与即时效用冲突时——深思者听从预测。

```rust
/// 两阶段惰性触发——避免为每个决策做昂贵的 MentalModel 查询
pub fn deliberative_modulation(
    candidates: &mut [ActionCandidate],
    npc: &NpcData,
) -> f32 {  // 返回 final_depth
    // ── Stage 1: 廉价初筛 ──
    let trait_capacity = npc.cognitive_style.reflective_impulsive
        * (1.0 - npc.cognitive_style.rigid_flexible * 0.3);
    let state_capacity = npc.cognition_tide.mind_quietude
        * (1.0 - npc.cognition_tide.cognitive_load)
        * (1.0 - npc.physiology.intoxication)
        * (1.0 - npc.physiology.fatigue)
        * (1.0 - npc.cognitive_distress)
        * (1.0 - npc.emotion.arousal.abs() * 0.7);
    let goap_weights: Vec<f32> = candidates.iter().map(|c| c.utility).collect();
    let rough_stakes = sigmoid((variance(&goap_weights) - 0.03) * 20.0);
    let initial_depth = trait_capacity * state_capacity * rough_stakes;

    if initial_depth < 0.2 { return initial_depth; }  // 不思——不触发 Stage 2

    // ── Stage 2: 为所有候选调用 predict_outcome ──
    for candidate in &mut *candidates {
        candidate.prediction = Some(predict_outcome(npc, &candidate.decision_option));
    }

    let predicted_pleasures: Vec<f32> = candidates.iter()
        .filter_map(|c| c.prediction.as_ref())
        .map(|p| p.expected_pleasure)
        .collect();
    let actual_stakes = sigmoid((variance(&predicted_pleasures) - 0.05) * 15.0);
    let final_depth = trait_capacity * state_capacity * actual_stakes;

    // ── 仅在深度足够时调制每候选权重 ──
    if final_depth > 0.3 {
        for candidate in &mut *candidates {
            if let Some(ref pred) = candidate.prediction {
                // 预测太不确定 → 不调制，尊重 GOAP
                if pred.variance > 0.3 || pred.model_count < 2 { continue; }

                let modulation = 1.0 + pred.expected_pleasure * final_depth * 1.2;
                candidate.mental_model_modulation *= modulation.clamp(0.3, 1.7);
                // ★ mental_model_modulation 是 ActionCandidate 上的已有权重环
                // v1.0: 全局常量。v1.2: 每候选的 deliberation 调制
            }
        }
    }

    final_depth
}
```

**调制范围 [0.3, 1.7] 的合理性**：最正预测 × 最深思想 = 1.0×0.6×1.2=0.72 → ×1.72。最负预测 × 最深思想 = −1.0×0.6×1.2=−0.72 → ×0.28。效用差距 3× 可被 1.7/0.3=5.7× 调制覆盖。翻转只在极端条件发生：trait 高 + 状态清醒 + 预测低方差 + 多模型支撑 + 强烈的情感预测分歧。

**幸存压制优先**：`survival_suppression` sigmoid 在 GOAP max_urgency > 0.7 时压制所有权重 → GOAP survival 行动不受 deliberation modulate 影响。饿到濒死的人不会因为"上次节食后感觉很自律"而选择不吃饭。

---

## 十五、记忆的四层压缩架构 ★ v1.1 新增 (CHG-057)

> **设计裁决**: 记忆压缩不是"满了就压"的应急措施——是信息论驱动的速率失真优化。预算固定（~617KB/NPC），信息随时间累积 → 通过降低保真度维持预算。

### 15.1 四层概览

| 层 | 容量 | 大小 | 压缩触发 | 数学 |
|----|------|------|---------|------|
| **L0 Hot Episodic** | ≤2000 | ~512KB | compression_rate > 0.8 或超出上限 | 全字段 |
| **L1 Cold Summary** | ≤500 | ~100KB | 超出上限 → 最老/最低密度条目提升 | text+emotion+participants+location |
| **L2 Era Digest** | ≤20 | ~4KB | 超出上限 → 合并最相邻时代 | theme_tags+情绪曲线+代表事件×3 |
| **L3 Life Abstract** | ≤5 | ~1KB | 终生存留 | 核心叙事弧+pivotal_moment+人生主题 |

**总计 ~617KB/NPC。1000 L1 = ~617MB。零内存泄漏。**

### 15.2 信息密度驱动的连续压缩率

```rust
fn compression_rate(memory: &EventMemory, days_since: f32) -> f32 {
    let information_density = memory.impact_score
        * (0.3 + memory.emotional_encoding.arousal * 0.7)
        * (1.0 + memory.access_count as f32).sqrt()   // sqrt 递减收益——防止反刍支配
        / (days_since + 1.0).ln_1p();                  // 对数时间衰减
    1.0 / (1.0 + information_density * 10.0)
}
```

`access_count.sqrt()` 是关键——它隐式承载叙事偏置：
- 符合 SelfNarrative 的事件被更频繁检索 → access_count ↑ → 密度 ↑ → 更不易被压缩
- 不意味着"客观重要"的事件 → 但 impact_score × arousal 项保证客观显著性仍有发言权
- NPC 的 narrative 驱动检索 → 检索驱动保留 → 保留驱动 narrative。闭环，零硬编码。

### 15.3 Era Digest —— 知识抽象的枢纽

```rust
pub struct EraDigest {
    pub span_days: u32,
    pub dominant_emotion: EmotionTriplet,        // 情绪曲线：初/中/末三采样点
    pub theme_tags: ArrayVec<ThemeTag, 8>,       // "战争""学徒期""婚姻""流浪"
    pub representative_events: [MemoryId; 3],     // 最能代表这个时代的三件事
    pub relationship_count: u8,
    pub location_summary: LocationTrace,
    pub self_label_at_time: Option<SelfLabel>,
}

/// 代表事件选择：纯信息密度排序
/// access_count 隐式承载叙事偏置——不额外加"叙事拟合"步骤。
fn select_representative_events(era_memories: &[&EventMemory]) -> [MemoryId; 3] {
    let mut ranked: Vec<_> = era_memories.iter()
        .map(|m| (m.id, information_density(m, days_since(m.game_day))))
        .collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    [ranked[0].0, ranked[1].0, ranked[2].0]
}
```

### 15.4 Life Abstract —— 人格级别的抽象

```rust
pub struct LifeAbstract {
    pub life_stage: LifeStageLabel,             // "从战乱中幸存" "平凡劳作的一生"
    pub core_narrative: NarrativeArc,           // 上升/下降/平稳/起伏
    pub pivotal_moment: Option<MemoryId>,       // ★ pivot_score 最高的章节边界事件
    pub life_theme_evolved: [LifeTheme; 3],     // 早/中/晚期主题
}

/// Pivotal Moment 选择：事后数据积累驱动的回顾性识别
/// pivot_score = significance × emotional_divergence × duration_weight × theme_change
fn pivotal_moment(chapters: &[LifeChapter]) -> Option<MemoryId> {
    chapters.windows(2)
        .map(|pair| {
            let (prev, curr) = (&pair[0], &pair[1]);
            let emotional_divergence = (curr.dominant_emotion - prev.dominant_emotion).abs();
            let duration_weight = (curr.duration_days() as f32 / 365.0).min(1.0).sqrt();
            let theme_weight = if chapter_theme_changed(prev, curr) { 1.0 } else { 0.4 };
            let score = curr.significance * emotional_divergence * duration_weight * theme_weight;
            (score, curr.key_events.first().copied())
        })
        .max_by(|a, b| a.0.partial_cmp(&b.0).unwrap())
        .and_then(|(_, event)| event)
}
```

**事后回顾性识别**：significance 在 SelfNarrative::reflect() 中随时间更新。章节刚结束时 significance 可能很低（"只是换了份工作"），5 年后上升（"那是我人生的转折"）。pivot_score 随时间收敛。不需要反事实模拟。

### 15.5 压缩调度

| 操作 | 时机 | 预算 |
|------|------|------|
| L0 → L1 压缩 | 睡眠期间（离线批处理） | 不在帧预算内 |
| L1 → L2 提升 | Cold Summary > 500 时 | 同上 |
| L2 → L3 合并 | Era Digest > 20 时（合并最相邻时代） | 同上（macro_digestion 中对） |
| 压缩率更新 | 睡眠期间更新所有记忆 | 同上 |

四层全在 NPC crate 内部。外部模块（存档、对话、GOAP）只通过 `MemoryStore` 的公开 API 查询——不暴露内部层级结构。

### 15.6 L2 Era Digest 合并算法 ★ v1.2 填补 (CHG-058)

```rust
/// 两个时代的相似度——决定是否合并
fn era_similarity(a: &EraDigest, b: &EraDigest) -> f32 {
    let theme_jaccard = jaccard_index(&a.theme_tags, &b.theme_tags);
    let emotion_cosine = cosine_similarity(
        &a.dominant_emotion.as_vec(), &b.dominant_emotion.as_vec()
    );
    let location_overlap = a.location_summary.similarity(&b.location_summary);
    theme_jaccard * 0.4 + emotion_cosine * 0.35 + location_overlap * 0.25
}

/// 扫描所有配对——按相似度降序，贪心合并
fn detect_merge_candidates(eras: &[EraDigest]) -> Vec<(usize, usize, EraDigest)> {
    let mut candidates = Vec::new();
    for i in 0..eras.len() {
        for j in (i+1)..eras.len() {
            let sim = era_similarity(&eras[i], &eras[j]);
            if sim > 0.5 { candidates.push((sim, i, j)); }
        }
    }
    candidates.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    // 时间作为打破平局的第二标准：相似度相等时选时间更近的
    candidates.into_iter()
        .map(|(sim, i, j)| (i, j, merge_two_eras(&eras[i], &eras[j])))
        .collect()
}

fn merge_two_eras(a: &EraDigest, b: &EraDigest) -> EraDigest {
    // representative_events: 6取3 按 information_density
    let all_events: Vec<MemoryId> = a.representative_events.iter()
        .chain(b.representative_events.iter()).copied().collect();
    let top3 = select_top_n_by_density(&all_events, 3);

    // theme_tags: 并集 按出现频率×情绪显著性排序 取前8
    let merged_themes = merge_theme_tags(&a.theme_tags, &b.theme_tags);

    // dominant_emotion: 按 span_days 加权平均
    let emotion = blend_emotion_triplets(
        a.dominant_emotion, b.dominant_emotion,
        a.span_days as f32, b.span_days as f32
    );

    EraDigest {
        span_days: a.span_days + b.span_days,
        representative_events: top3,
        theme_tags: merged_themes,
        dominant_emotion: emotion,
        relationship_count: a.relationship_count + b.relationship_count,
        location_summary: a.location_summary.merge(&b.location_summary),
        self_label_at_time: a.self_label_at_time.or(b.self_label_at_time),
    }
}
```

### 15.7 领域级遗忘 ★ v1.2 新增 (CHG-058)

```rust
/// 运行在睡眠期间（离线批处理）
/// 一个领域长期未激活 → 整个领域的 MentalModel 同时萎缩
fn domain_atrophy(models: &mut [MentalModel], current_day: GameDay) {
    // 按 DomainSignature Hamming 距离简单分组（< 0.3 = 同一领域簇）
    // 零依赖——不调用 006 的 HDBSCAN*
    let clusters = group_by_domain_cluster(models, 0.3);

    for domain_models in clusters.values_mut() {
        let last_touched = domain_models.iter()
            .map(|m| m.last_activated)
            .max()
            .unwrap();
        let inactive_years = (current_day - last_touched) as f32 / 365.0;

        if inactive_years < 1.0 { continue; }

        // 萎缩速率：0 → 0.5 超过 10 年不碰
        let atrophy = ((inactive_years - 1.0) / 10.0).min(0.5);

        for model in domain_models.iter_mut() {
            model.activation_threshold *= 1.0 + atrophy;       // 更难检索
            model.confidence *= 1.0 - atrophy * 0.3;           // 信心逐渐褪色
        }
    }
}

fn group_by_domain_cluster(
    models: &mut [MentalModel],
    threshold: f32,
) -> HashMap<u64, Vec<&mut MentalModel>> {
    let mut clusters: HashMap<u64, Vec<&mut MentalModel>> = HashMap::new();
    for model in models.iter_mut() {
        let sig = model.domain_signature();
        // 找到最近的聚类中心。新中心无匹配→新聚类。
        let cluster_key = clusters.keys()
            .find(|&&key| hamming_similarity(key, sig) < threshold)
            .copied()
            .unwrap_or(sig);
        clusters.entry(cluster_key).or_default().push(model);
    }
    clusters
}
```

**玩家看到**：30 年铁匠→20 年种田。atrophy=0.5。"我以前做过铁匠但现在手生了"——activation_threshold×1.5 → 即使被问到也不易检索。confidence×0.85 → 回忆不准确但不是空白。

---

## 十六、判定标准 ★ v1.2 新增 (CHG-058)

> **用途**: 评估任何新提议的 NPC 心智功能是否符合"基于模式的向前投射"原则。

### 三问

| # | 问题 | YES 示例 | NO 示例 |
|---|------|---------|---------|
| 1 | **所有输入都来自已有数据吗？** | MentalModel[]、MemoryStore、EmotionState、PerceptBatch | World state forward-projected 到 t+1、hypothetical initial conditions |
| 2 | **是单查询还是链式仿真？** | 一次 pattern match、一次 aggregation、一次 comparison。无中间状态 | 步骤 1 的结果作为步骤 2 的输入、模拟循环 |
| 3 | **输出带不确定性吗？** | variance、confidence、model_count、"我不知道"的可编码表示 | 确定的断言、无置信区间的点估计 |

### 判定示例

| 功能 | Q1 | Q2 | Q3 | 结论 |
|------|----|----|----|------|
| `deliberative_modulation()` | ✅ MemoryStore + MentalModel | ✅ 每候选一次查询 | ✅ variance + model_count | 通过 |
| `predict_outcome()` | ✅ MentalModel[] | ✅ 一次聚合 | ✅ OutcomePrediction | 通过 |
| `counterfactual_regret()` | ✅ DecisionPoint + MentalModel | ✅ 一次比较 | ✅ regret 阈下不记录 | 通过 |
| `creative_leap()` | ✅ MentalModel[] | ✅ LSH 一次匹配 | ✅ hypothesis=true | 通过 |
| 链式世界仿真（假设） | ❌ World state + initial conditions | ❌ N 步仿真 | ❌ 确定性输出 | 不通过——不是 NPC 心智 |
| GOAP 物理可行性预演 | ✅ AgentSnapshot + MaterialProperties | ✅ precondition_fn 一次检查 | ✅ Precheck 返回 capability | 不属于 NPC 心智（属于 GOAP） |

三项全 YES → 属于 NPC 心智，可加入。任意 NO → 不是 NPC 心智——属于 GOAP 物理预演或应拒绝。

---

> **下一文档**: [[004-思考涌现与浮现机制]]
