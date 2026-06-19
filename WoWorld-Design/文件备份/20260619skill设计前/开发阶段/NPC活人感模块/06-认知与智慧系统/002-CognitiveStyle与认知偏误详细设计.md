> **开发代号**: WoWorld (Wonder World)
> **引擎**: Godot 4.6.2
> **模块**: 06-NPC认知与智慧系统
> **文档类型**: 正式开发规格 · CognitiveStyle与认知偏误
> **版本**: v1.0
> **创建日期**: 2026-06-17
> **父文档**: [[001-认知与智慧系统总纲]]

---

# 002-CognitiveStyle与认知偏误详细设计

## 目录

- [一、CognitiveStyle：4维认知风格](#一cognitivestyle4维认知风格)
- [二、CognitiveTide：3维认知潮汐](#二cognitivetide3维认知潮汐)
- [三、CognitiveBiases：7种认知偏误](#三cognitivebiases7种认知偏误)
- [四、EmbodiedCognitionModifiers：身体→认知](#四embodiedcognitionmodifiers身体认知)
- [五、CognitiveNorms：文化→认知风格](#五cognitivenorms文化认知风格)
- [六、CognitiveBreak：认知功能的失效模式](#六cognitivebreak认知功能的失效模式)
- [七、Meta-Cognition：反思性自指](#七meta-cognition反思性自指)
- [八、cognitive_distress：认知健康综合指标](#八cognitive_distress认知健康综合指标)

---

## 一、CognitiveStyle：4维认知风格

### 1.1 数据结构

```rust
#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct CognitiveStyle {
    /// 分析-直觉：0=纯直觉（走感觉/直觉判断），1=纯分析（走逻辑/推理）
    pub analytic_intuitive: f32,
    
    /// 反思-冲动：0=纯冲动（立即反应），1=纯反思（深思熟虑）
    pub reflective_impulsive: f32,
    
    /// 抽象-具象：0=纯具象（关注具体事物），1=纯抽象（关注概念/模式）
    pub abstract_concrete: f32,
    
    /// 顽固-灵活：0=顽固（信念难以改变），1=灵活（愿意调整信念）
    pub rigid_flexible: f32,
}
```

### 1.2 初始派生（NPC出生/创建时）

```rust
impl CognitiveStyle {
    /// 从已有数据派生认知风格
    /// 对标：AestheticTaste从BigFive派生（CHG-029）
    pub fn derive(
        big_five: &BigFive,
        wisdom: f32,           // 来自 MentalAttributes.wisdom (0-1)
        mental_age: f32,       // 派生自记忆条目数/2000
        life_event_count: u32, // 来自 SelfNarrative.life_chapters 长度 + impact>0.5记忆数
    ) -> Self {
        Self {
            // 分析-直觉：
            //   尽责性高 → 倾向于有条理地分析问题
            //   开放性低 → 倾向于依赖已知经验而非探索新思路
            analytic_intuitive: (big_five.conscientiousness * 0.7 
                               + (1.0 - big_five.openness) * 0.3)
                               .clamp(0.0, 1.0),
            
            // 反思-冲动：
            //   神经质低 → 情绪稳定，能从容思考
            //   开放性高 → 愿意花时间多角度考虑
            //   外向性低 → 不急于对外回应
            reflective_impulsive: ((1.0 - big_five.neuroticism) * 0.5 
                                + big_five.openness * 0.3 
                                + (1.0 - big_five.extraversion) * 0.2)
                                .clamp(0.0, 1.0),
            
            // 抽象-具象：
            //   开放性高 → 喜欢概念和可能性
            //   wisdom高 → 能将具体经验提升到原理层面
            //   年龄增长 → 积累更多经验框架，思维自然更抽象
            abstract_concrete: (big_five.openness * 0.5 
                              + wisdom * 0.3 
                              + sigmoid(mental_age - 0.3_f32) * 0.2)
                              .clamp(0.0, 1.0),
            
            // 顽固-灵活：
            //   开放性高 → 愿意接受新观点
            //   尽责性低 → 不执着于"应该怎么做"
            //   经历越多 → 越能理解世事无常
            rigid_flexible: (big_five.openness * 0.6 
                           + (1.0 - big_five.conscientiousness) * 0.2 
                           + sigmoid(life_event_count as f32 * 0.01) * 0.2)
                           .clamp(0.0, 1.0),
        }
    }
}
```

### 1.3 阻尼年度重派生

**为什么需要阻尼**：CognitiveStyle→MentalModel消化→SelfNarrative→BigFive漂移→CognitiveStyle重派生。正反馈回路需要阻尼防止爆炸。

```rust
/// 带阻尼的重派生——在对标 SelfNarrative::reflect() 的7天周期中调用
pub fn derive_with_damping(
    previous: &CognitiveStyle,
    raw_derived: &CognitiveStyle,
    life_event_count: u32,
) -> CognitiveStyle {
    // 阻尼系数：
    // 年轻时（经历少）→ 阻尼强（认知风格更稳定）
    // 年长后（经历多）→ 阻尼弱（更容易改变）
    let damping = 0.85 - (life_event_count as f32 / 500.0).min(0.35);
    // damping范围：[0.50, 0.85]
    
    CognitiveStyle {
        analytic_intuitive: previous.analytic_intuitive * damping 
                          + raw_derived.analytic_intuitive * (1.0 - damping),
        reflective_impulsive: previous.reflective_impulsive * damping 
                           + raw_derived.reflective_impulsive * (1.0 - damping),
        abstract_concrete: previous.abstract_concrete * damping 
                         + raw_derived.abstract_concrete * (1.0 - damping),
        rigid_flexible: previous.rigid_flexible * damping 
                      + raw_derived.rigid_flexible * (1.0 - damping),
    }
}
```

### 1.4 重大事件即时调制

对标 emotion shock override（impact>0.85时情绪跳跃惯性）。

```rust
/// 重大事件后可能的微调
/// 仅在事件 impact > 0.8 时调用
pub fn event_modulation(style: &mut CognitiveStyle, event: &EventMemory) {
    let magnitude = (event.impact_score - 0.8).max(0.0) * 0.1; // 最大偏移0.02
    
    // 创伤性事件 → 可能降低开放性（行为上表现为更谨慎）
    if event.emotional_encoding.valence() < -0.7 {
        style.rigid_flexible = (style.rigid_flexible - magnitude * 0.5).max(0.0);
        style.reflective_impulsive = (style.reflective_impulsive + magnitude * 0.3).min(1.0);
        // 创伤后→更谨慎（更rigid），但也更反思
    }
    
    // 正面冲击性事件 → 可能增加开放性
    if event.emotional_encoding.valence() > 0.8 && event.event_type == EventType::Revelation {
        style.rigid_flexible = (style.rigid_flexible + magnitude).min(1.0);
        style.abstract_concrete = (style.abstract_concrete + magnitude * 0.5).min(1.0);
    }
}
```

---

## 二、CognitiveTide：3维认知潮汐

### 2.1 数据结构

```rust
#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct CognitiveTide {
    /// 认知负载：0=完全空闲心智，1=全力以赴
    /// 对标：注意力系统
    pub cognitive_load: f32,
    
    /// 反刍压力：有多少未消化的高impact记忆在"排队等候处理"
    /// 对标：SelfNarrative.stagnation_sense
    pub rumination_pressure: f32,
    
    /// 心智安静度：0=思绪纷飞/焦躁，1=深度安静（心流/冥想/出神）
    /// 对标：情绪引擎的 arousal 维度（但独立变化——高arousal情绪≠高认知负载）
    pub mental_quietude: f32,
    
    /// 漫游驱动——从以上三维派生，不独立存储
    /// mind_wander_drive = (1 - cognitive_load) × rumination_pressure × (1 - mental_quietude)
}
```

### 2.2 每决策周期更新

```rust
impl CognitiveTide {
    pub fn update(
        &mut self,
        current_action: &ActionRecord,
        physiology: &Physiology,      // 已有：从Vitals同步
        emotion: &EmotionState,       // 已有
        recent_high_impact_memories: usize,  // 最近7天impact>0.6的记忆数
        neuroticism: f32,
        time_since_last_sleep: Duration,
        recent_experience_narrowness: f32,
    ) {
        // 1. 认知负载：从当前行动的注意力需求查表
        self.cognitive_load = match current_action.category() {
            ActionCategory::Combat => 0.95,
            ActionCategory::Crafting | ActionCategory::SkillPractice => {
                let skill_level = current_action.primary_skill_level();
                if skill_level > 70.0 { 0.2 }  // 专家→心流→低负载
                else if skill_level > 40.0 { 0.4 }
                else { 0.6 }  // 新手→高负载
            },
            ActionCategory::SocialInteraction => 0.5,
            ActionCategory::Navigation => 0.15,
            ActionCategory::Idle => 0.02,
            _ => 0.3,
        };
        
        // 2. 反刍压力：未消化的高impact记忆
        self.rumination_pressure = (recent_high_impact_memories as f32 / 10.0).min(1.0)
                                 * (1.0 - big_five.agreeableness * 0.3)  // 低宜人性→更沉溺
                                 * (1.0 + neuroticism * 0.5);              // 高神经质→更反刍
        
        // 3. 心智安静度
        let flow_bonus = if self.cognitive_load < 0.3 && current_action.is_skilled(60.0) {
            0.4  // 熟练技能×低负载→心流
        } else { 0.0 };
        
        let nature_bonus = current_action.environment_naturalness();  // 从VisualScene派生
        
        self.mental_quietude = (flow_bonus 
                              + nature_bonus * 0.3 
                              + (1.0 - self.rumination_pressure) * 0.3
                              + (1.0 - neuroticism) * 0.2)
                              .clamp(0.0, 1.0);
        
        // 4. 清醒微正则化：经验窄度↑ + 清醒时间长 → 漫游驱动↑
        let wakeful_hours = time_since_last_sleep.as_hours();
        let wakeful_wander = recent_experience_narrowness 
                           * sigmoid(wakeful_hours - 4.0)  // 清醒4h后加速
                           * (1.0 - self.cognitive_load);    // 仅空闲时
    }
    
    /// 漫游驱动——派生，不存储
    pub fn mind_wander_drive(&self) -> f32 {
        (1.0 - self.cognitive_load) 
            * self.rumination_pressure 
            * (1.0 - self.mental_quietude)
    }
}
```

---

## 三、CognitiveBiases：7种认知偏误

### 3.1 设计原则

**完全派生——不存储。每次调用时从当前状态惰性计算。**

对标：Physiology从Vitals派生——Physiology不存储，每次调用from_vitals()生成。

### 3.2 数据结构与派生

```rust
/// 认知偏误集合——纯函数产物，零存储
pub struct CognitiveBiases {
    pub confirmation_bias: f32,          // 确认偏误
    pub negativity_bias: f32,            // 负面偏差
    pub recency_weight: f32,             // 近因效应
    pub self_serving_bias: f32,          // 自我服务偏差
    pub availability_heuristic: f32,     // 可得性启发
    pub cognitive_dissonance_tolerance: f32, // 认知失调容忍
    pub rumination_tendency: f32,        // 反刍倾向
}

/// 纯函数——每次调用时从当前状态派生
/// 对标：cognitive_style + emotion + tide → biases
pub fn compute_biases(
    style: &CognitiveStyle,
    emotion: &EmotionState,
    tide: &CognitiveTide,
    personality: &BigFive,
) -> CognitiveBiases {
    CognitiveBiases {
        // 1. 确认偏误：
        //    顽固型高（不愿面对反例），直觉型高（相信直觉>客观证据）
        //    强烈情绪放大偏误
        confirmation_bias: (((1.0 - style.rigid_flexible) * 0.6 
                          + (1.0 - style.analytic_intuitive) * 0.4)
                          * (1.0 + emotion.pleasure.abs() * 0.3))
                          .clamp(0.0, 1.0),
        
        // 2. 负面偏差：
        //    直觉型高，神经质高放大
        //    悲伤情绪进一步放大
        negativity_bias: (((1.0 - style.analytic_intuitive) * 0.4 
                        + personality.neuroticism * 0.6)
                        * (1.0 + emotion.pleasure.min(0.0).abs() * 0.5))
                        .clamp(0.0, 1.0),
        
        // 3. 近因权重：
        //    冲动型高（最新的最重），反思型低（考虑全局）
        recency_weight: ((1.0 - style.reflective_impulsive) * 0.7 + 0.1)
                        .clamp(0.0, 1.0),
        
        // 4. 自我服务偏差：
        //    顽固型高，低宜人性放大
        //    成功后→归因能力；失败后→归因环境
        self_serving_bias: (((1.0 - style.rigid_flexible) * 0.5 
                          + (1.0 - personality.agreeableness) * 0.5))
                          .clamp(0.0, 1.0),
        
        // 5. 可得性启发：
        //    冲动型高（想到啥是啥，不费力检索），高负载时更高
        availability_heuristic: (((1.0 - style.reflective_impulsive) * 0.5
                               + tide.cognitive_load * 0.3
                               + personality.extraversion * 0.2))
                               .clamp(0.0, 1.0),
        
        // 6. 认知失调容忍：
        //    灵活型高（可以同时持有矛盾信念），神经质低
        cognitive_dissonance_tolerance: (style.rigid_flexible * 0.5
                                       + (1.0 - personality.neuroticism) * 0.5)
                                       .clamp(0.0, 1.0),
        
        // 7. 反刍倾向：
        //    冲动型+神经质高→反复咀嚼同一个想法
        rumination_tendency: (((1.0 - style.reflective_impulsive) * 0.4
                            + personality.neuroticism * 0.6))
                            .clamp(0.0, 1.0),
    }
}
```

### 3.3 各偏误在消化管线中的应用

详细的数学公式见 [[003-MentalModel与智慧积累]]。以下为应用概览：

| 偏误 | 影响位置 | 调制方式 |
|------|---------|---------|
| confirmation_bias | micro_digest | 支持性证据权重×1.5, 反面证据权重×0.7 |
| negativity_bias | micro_digest | 负面证据的事实权重×2-3倍 |
| recency_weight | micro_digest | 新记忆在归纳中按指数衰减加权 |
| self_serving_bias | macro_digest | 自我归因成功权重↑，失败归因环境 |
| availability_heuristic | ThoughtTrigger | 生动记忆（高arousal编码）优先被检索 |
| dissonance_tolerance | assess_and_integrate | 容忍低→冲突时更易拒绝外来模型 |
| rumination_tendency | ThoughtTrigger::zeigarnik | 高反刍→不确定模型被频繁拉入思绪 |

---

## 四、EmbodiedCognitionModifiers：身体→认知

### 4.1 设计原则

对标已有模式：`Physiology` 从 `Vitals` 派生——Physiology是主观感知层（0-1归一化），`EmbodiedCognitionModifiers`从Physiology+温度+中毒状态派生。

**纯函数，零存储，每决策周期惰性计算。**

### 4.2 数据结构与派生

```rust
pub struct EmbodiedCognitionModifiers {
    pub cognitive_load_shift: f32,       // 身体状态→额外认知负载
    pub mental_quietude_shift: f32,      // 身体状态→安静度调制
    pub short_term_bias: f32,            // 饥饿→短期偏好
    pub association_looseness: f32,      // 醉酒/高烧→跨界关联松弛
}

pub fn compute_embodied_modulation(
    physiology: &Physiology,              // 已有：pain/hunger/thirst/fatigue/health (0-1)
    temperature_comfort: f32,             // 已有：WeatherQuery::sample() 中的温度舒适度
    intoxication: f32,                    // 已有：ConsumableEffect 中的中毒/醉酒值
    fever: f32,                           // 已有：从生命体征异常检测派生
) -> EmbodiedCognitionModifiers {
    EmbodiedCognitionModifiers {
        // 疼痛→负载显著上升，极端温度→负载上升，饥饿→轻微负载
        cognitive_load_shift: (physiology.pain * 0.3 
                             + (1.0 - temperature_comfort) * 0.2 
                             + physiology.hunger * 0.15)
                             .clamp(0.0, 0.5),  // 最大+0.5（即身体因素最多使负载从0升到0.5）
        
        // 疼痛和高温→安静度下降
        mental_quietude_shift: -(physiology.pain * 0.4 
                               + (1.0 - temperature_comfort) * 0.3)
                               .clamp(-0.6, 0.0),
        
        // 饥饿→短期收益偏误
        short_term_bias: physiology.hunger * 0.3,
        
        // 醉酒+高烧→心智关联松弛
        association_looseness: (intoxication * 0.6 + fever * 0.4).clamp(0.0, 1.0),
    }
}
```

### 4.3 身体状态→认知涌现行为

| 身体状态 | 认知影响 | 可观察行为 |
|---------|---------|-----------|
| 疼痛(>0.5) | 认知负载+0.15，安静度-0.2，可用性启发↑ | 频繁暂停动作、皱眉、自语负面 |
| 饥饿(>0.7) | 短期偏误↑，反刍中食物相关记忆优先 | 走神想食物、conversation topic偏食物 |
| 极热/极冷 | 负载+0.1-0.2，安静度-0.15-0.2 | 心神不宁、更快结束当前活动 |
| 醉酒(>0.5) | 关联松弛+0.3，shareability+0.3(酒后吐真言) | 话语变多、跨界联想增多但质量低 |
| 高烧(>0.5) | 关联松弛+0.2，可能触发梦境入侵清醒 | 恍惚、自言自语、认知现实检验下降 |

---

## 五、CognitiveNorms：文化→认知风格

### 5.1 设计原则

**对标已有模式：`CommunicationNorms`从`CultureCoreParams`派生（CHG-024定义）。**

`CognitiveNorms`是CognitiveStyle的**文化基线偏移**——同一个BigFive的人在不同文化中，其认知风格的文化基线不同。

### 5.2 数据结构与派生

```rust
pub struct CognitiveNorms {
    pub debate_style: DebateStyle,
    pub uncertainty_tolerance: f32,
    pub epistemological_stance: f32,      // 0=经验主义，1=理性主义/逻辑优先
    pub mental_model_sharing_norm: f32,   // 对标 individualism
    pub authority_of_elders_in_thinking: f32, // 对标 power_distance
}

pub enum DebateStyle {
    Confrontational,   // "你说的不对，我的理由是..."
    Dialectical,       // "你说的有道理，但也许还可以..."
    Consensus,         // "我们看看能不能找到一个大家都同意的..."
}

pub fn derive_cognitive_norms(culture: &CultureCoreParams) -> CognitiveNorms {
    CognitiveNorms {
        debate_style: if culture.competition > 0.6 && culture.individualism > 0.5 {
            DebateStyle::Confrontational
        } else if culture.power_distance > 0.7 {
            DebateStyle::Consensus
        } else {
            DebateStyle::Dialectical
        },
        
        uncertainty_tolerance: 1.0 - culture.uncertainty_avoidance,
        
        epistemological_stance: (culture.artistry * 0.3 
                               + culture.individualism * 0.3 
                               + 0.4)
                               .clamp(0.0, 1.0),
        
        mental_model_sharing_norm: 1.0 - culture.individualism * 0.5,
        
        authority_of_elders_in_thinking: culture.power_distance * 0.7,
    }
}
```

### 5.3 文化→认知→涌现

| 文化特征 | CognitiveNorms特征 | 涌现行为 |
|---------|-------------------|---------|
| 高competition + 高individualism | Confrontational辩论风格 | 群体讨论中NPC互相辩论，思想碰撞频率高 |
| 高power_distance + 高uncertainty_avoidance | Consensus风格 + 低容忍 | 年轻人不易挑战年长者的MentalModel，创新传播慢 |
| 高artistry + 高individualism | 理性主义立场 | 逻辑优先于经验，"先想清楚再做" |
| 低power_distance + 高openness | 经验主义 + 高分享规范 | MentalModel自由分享，创新采纳快 |

---

## 六、CognitiveBreak：认知功能的失效模式

### 6.1 设计原则

**不是"精神疾病"标签系统。** 是已有认知偏差的**正常人在极端条件下的极端表现**。

对标已有模式：战斗系统的`PostCombatTrauma`。创伤是已有情绪记忆回路的极端表现。

### 6.2 四种失效模式

```rust
pub enum CognitiveBreak {
    /// 偏执——系统性错误归因："所有人都在害我"
    /// 机制：MindAttribution的威胁归因阈值异常降低
    Paranoia {
        target_groups: Vec<NpcId>,
        intensity: f32,  // 0=轻微多疑, 1=全面妄想
    },
    
    /// 郁结——反刍→无法打破→任何新信息被负面曲解
    /// 机制：negativity_bias被锁定在>0.8，任何正面事件被重新解释
    MelancholicLoop {
        core_loss_memory_id: MemoryId,
        duration_days: u16,
    },
    
    /// 虚妄——自我服务偏差极端放大："只有我是对的"
    /// 机制：self_serving_bias锁定在1.0
    GrandioseDistortion {
        inflated_self_labels: Vec<String>,
        reality_anchors_failed: u8,
    },
    
    /// 解离——自我叙事断裂："这不是我的人生"
    /// 机制：SelfNarrative的life_chapters被标记为"不属于我"
    Dissociation {
        fragmented_chapters: Vec<u8>,  // chapter indices
        depersonalization: f32,
    },
}
```

### 6.3 触发条件

```rust
/// 纯函数——检查是否触发认知失效
/// 对标：PostCombatTrauma 的触发条件
pub fn check_cognitive_break(
    distress: f32,
    recent_trauma: Option<&EventMemory>,  // 最近的高impact事件
    social_isolation_years: f32,
    models: &[MentalModel],
    attributions: &[MindAttribution],
    self_narrative: &SelfNarrative,
    rng: &mut Pcg64,
) -> Option<CognitiveBreak> {
    // 条件1：认知压力长期极高
    if distress > 0.85 && recent_trauma.map_or(false, |e| e.impact_score > 0.95) {
        // 症状取决于人格x事件交互
        // ... (详细数学见完整实现)
    }
    // 条件2：极端社会隔离
    if social_isolation_years > 5.0 {
        // ... 
    }
    None
}
```

### 6.4 恢复

对标已有`PostCombatTrauma`恢复机制：时间×社会支持×正面经历×可能的专业治疗。没有"一键修复"。

---

## 七、Meta-Cognition：反思性自指

### 7.1 定义

Meta-Cognition是 `CognitiveStyle.reflective_impulsive` 的**自指应用**——反思型NPC自然而然地"反思自己的反思"。冲动型NPC几乎从不meta-cognize。

**不新增struct。** 嵌入已有的`SelfNarrative::reflect()`中。

### 7.2 机制

```rust
/// 嵌入 SelfNarrative::reflect() 的步骤之一
/// 对标：reflect() 已有的 6 个步骤（stagnation/labels/chapters/values/aspirations/theme）
pub fn meta_cognitive_review(
    recent_thoughts: &[ThoughtFragmentSeed],   // 最近7天的思考片段
    recent_decision_outcomes: &[DecisionOutcome], // 最近7天的决策结果
    cognitive_style: &CognitiveStyle,
    biases: &CognitiveBiases,
    models: &mut [MentalModel],
) -> Option<MetaCognitiveInsight> {
    // 门槛：反思性>0.6
    if cognitive_style.reflective_impulsive < 0.6 { return None; }
    
    // 1. 后悔检测："我因为信念X做了决定Y，结果Z→负面的"
    //    如果Z是负面结果→触发meta-cognition
    let regret_chains: Vec<_> = recent_decision_outcomes.iter()
        .filter(|d| d.outcome_valence < -0.3)
        .filter_map(|d| trace_decision_to_belief(d, models))
        .collect();
    
    // 2. 偏误自我识别："我是不是太看重最近发生的事了？"
    //    反思型NPC可能识别到自己的近因偏误
    if biases.recency_weight > 0.7 && cognitive_style.reflective_impulsive > 0.8 {
        // 产出meta-cognitive insight:
        // → CognitiveStyle微调（往更反思方向偏移0.005）
        // → 受影响的MentalModel被标记上 "⚠️ 我知道我可能偏颇"
        // → 一条 Revelation 类型记忆被编码
    }
    
    // ...
    None  // 大多数时候不产出——meta-cognition是稀有的
}
```

### 7.3 可察觉表现

- 高meta-cognition NPC在对话中："不过也可能我想错了"、"按我过去的经验是这样，但..."
- 玩家能感觉到这个NPC是深思熟虑的——和冲动型NPC的"一定是这样！"形成对比

---

## 八、cognitive_distress：认知健康综合指标

### 8.1 定义

```rust
/// 认知压力指数——0=认知健康，1=认知崩溃边缘
/// 对标 SelfNarrative.stagnation_sense
/// 完全派生，每游戏日更新一次
pub fn compute_cognitive_distress(
    models: &[MentalModel],
    tide: &CognitiveTide,
    self_narrative: &SelfNarrative,
    personality: &BigFive,
) -> f32 {
    // 1. 信念矛盾指数：高confidence+直接矛盾的模型对数量
    let contradiction_index = count_deep_contradictions(models);
    
    // 2. 反刍压力
    let rumination = tide.rumination_pressure;
    
    // 3. 停滞感
    let stagnation = self_narrative.stagnation_sense;
    
    // 加权组合
    (contradiction_index * 0.35 
   + rumination * 0.25 
   + stagnation * 0.20 
   + personality.neuroticism * 0.20)
   .clamp(0.0, 1.0)
}
```

### 8.2 认知压力的影响

| 程度 | cognitive_distress | 影响 |
|------|-------------------|------|
| 健康 | <0.2 | 正常认知功能 |
| 轻度压力 | 0.2-0.4 | rumination_pressure微增 |
| 中度压力 | 0.4-0.6 | 决策随机性↑, ThoughtFragment更负面 |
| 高度压力 | 0.6-0.8 | negativity_bias放大(×2.0), meta-cognition几乎不触发 |
| 危机 | >0.8持续>30天 | CognitiveBreak可能触发 |

### 8.3 认知危机的缓解路径

1. 睡眠（尤其是深睡+REM）——对标睡眠认知加工的正则化功能
2. 正面社会互动（社会支持缓冲）——对标已有RelationshipSystem
3. 成功的创新或信念验证（mental model confidence上升）——自我效能恢复
4. 时间（慢性衰减，每月-0.05）

---

> **下一文档**: [[003-MentalModel与智慧积累]]
