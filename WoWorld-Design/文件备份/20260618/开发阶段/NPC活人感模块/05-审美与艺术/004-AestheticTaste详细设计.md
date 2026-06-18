> **开发代号**: WoWorld (Wonder World)
> **文档类型**: 参考讨论草稿 — 可以矛盾、模糊、未决
> **创建日期**: 2026-06-16
> **关联**: [[参考文档/026-审美系统设计探讨-20260616/设计草稿/001-理论框架与维度论证|001]] · [[003-判断维度详细设计|003]]

# 004 — AestheticTaste 详细设计

## 一、AestheticTaste 完整定义

```rust
/// 审美品味——每个 NPC 持有的主观审美滤镜
/// Copy 类型，32 bytes（6×f32 + 3×f32 + u32）
/// 存储在 NPC.NpcData 中
/// 审美模块定义 struct 和派生公式，NPC 模块存储
#[derive(Copy, Clone)]
pub struct AestheticTaste {
    /// 6 个信号维度的个人权重 [0, 1]
    /// 索引: [fluency, novelty, complexity, harmony, expressiveness, virtuosity]
    pub dimension_weights: [f32; 6],
    
    /// 熟悉度偏好 [-1, 1]
    /// -1=追求陌生和新奇，0=中性，1=偏爱熟悉和安全
    pub familiarity_bias: f32,
    
    /// 审美开放度 [0, 1]
    /// 0=封闭（仅接受本文化审美标准），1=极度开放（接受任何审美风格）
    /// 影响：novelty 倒 U 峰值位置、Adopt 阻力、跨文化审美宽容度
    pub aesthetic_openness: f32,
    
    /// 复杂度容忍度 [0, 1]
    /// 0=偏好极度简单，1=享受极度复杂
    /// 影响：complexity 倒 U 峰值位置
    pub complexity_tolerance: f32,
    
    /// 生成时的个人种子——不可变
    /// 用于确定性个人偏移和 jitter 补充
    pub personal_seed: u32,
}

// 维度索引常量
pub const DIM_FLUENCY: usize = 0;
pub const DIM_NOVELTY: usize = 1;
pub const DIM_COMPLEXITY: usize = 2;
pub const DIM_HARMONY: usize = 3;
pub const DIM_EXPRESSIVENESS: usize = 4;
pub const DIM_VIRTUOSITY: usize = 5;
```

## 二、Taste 的初始派生：derive_taste()

### 2.1 青春期调用（对标 CulturalBeautyStandard 吸收）

```rust
/// NPC 进入青春期时（~12-14岁）调用一次
/// 对标 NPC 02-性别模块的 absorb_culture()
pub fn derive_taste(
    personality: &BigFive,
    culture_params: &CultureCoreParams,
    beauty_standard: &CulturalBeautyStandard,
    parents_taste: Option<&AestheticTaste>,
    personal_seed: u32,
) -> AestheticTaste {
    let mut rng = DeterministicRng::from_seed(personal_seed);
    
    // ── 1. 文化基准权重 ──
    let mut weights = [0.5_f32; 6];  // 中性起点
    
    // 文化 artistry → expressiveness 权重
    weights[DIM_EXPRESSIVENESS] += culture_params.artistry * 0.30;
    // 文化 uncertainty_avoidance → fluency 权重（偏好清晰可预测）
    weights[DIM_FLUENCY] += culture_params.uncertainty_avoidance * 0.20;
    // 文化 individualism → novelty 权重（重视独特表达）
    weights[DIM_NOVELTY] += culture_params.individualism * 0.15;
    // 文化 power_distance → virtuosity 权重（重视技艺等级）
    weights[DIM_VIRTUOSITY] += culture_params.power_distance * 0.25;
    // 文化 indulgence → harmony 权重（追求感官愉悦和谐）
    weights[DIM_HARMONY] += culture_params.indulgence * 0.20;
    
    // ── 2. 个性偏差 ──
    // openness → novelty↑, expressiveness↑, complexity_tolerance↑
    weights[DIM_NOVELTY]        += personality.openness * 0.25;
    weights[DIM_EXPRESSIVENESS] += personality.openness * 0.15;
    weights[DIM_COMPLEXITY]     += personality.openness * 0.10;
    
    // conscientiousness → virtuosity↑, fluency↑（重视技艺和整洁）
    weights[DIM_VIRTUOSITY] += personality.conscientiousness * 0.20;
    weights[DIM_FLUENCY]    += personality.conscientiousness * 0.10;
    
    // agreeableness → harmony↑（追求协调）
    weights[DIM_HARMONY] += personality.agreeableness * 0.20;
    
    // extraversion → expressiveness↑（重视情感外显）
    weights[DIM_EXPRESSIVENESS] += personality.extraversion * 0.15;
    
    // neuroticism → fluency 负向（高神经质对不流畅更敏感/排斥）
    weights[DIM_FLUENCY] -= personality.neuroticism * 0.15;
    
    // ── 3. 个人随机偏移（同文化同个性的 NPC 仍有个体差异）──
    for dim in 0..6 {
        weights[dim] += rng.f32_range(-0.12, 0.12);
        weights[dim] = weights[dim].clamp(0.05, 0.95);  // 不归零
    }
    
    // ── 4. 反叛因子（高 openness + 低 agreeableness 的青少年主动偏离父母）──
    if let Some(parents) = parents_taste {
        let rebellion = personality.openness * 0.5 + (1.0 - personality.agreeableness) * 0.3;
        if rebellion > 0.5 {
            for dim in 0..6 {
                let parent_deviation = 1.0 - parents.dimension_weights[dim];
                weights[dim] += parent_deviation * rebellion * 0.2;
                weights[dim] = weights[dim].clamp(0.05, 0.95);
            }
        }
    }
    
    // ── 5. 派生字段 ──
    let familiarity_bias = culture_params.uncertainty_avoidance * 0.6 
                         - personality.openness * 0.4
                         + rng.f32_range(-0.10, 0.10);
    
    let aesthetic_openness = culture_params.openness_to_outsiders * 0.40
                           + personality.openness * 0.40
                           + (1.0 - beauty_standard.aesthetic_confidence) * 0.20
                           + rng.f32_range(-0.08, 0.08);
    
    let complexity_tolerance = personality.openness * 0.60
                             + culture_params.artistry * 0.40
                             + rng.f32_range(-0.10, 0.10);
    
    AestheticTaste {
        dimension_weights: weights,
        familiarity_bias: familiarity_bias.clamp(-1.0, 1.0),
        aesthetic_openness: aesthetic_openness.clamp(0.0, 1.0),
        complexity_tolerance: complexity_tolerance.clamp(0.0, 1.0),
        personal_seed,
    }
}
```

## 三、Taste 的演化

### 3.1 mature_taste() — 年更新

每游戏年调用一次（对标文化 drift σ=0.003/年）：

```rust
pub fn mature_taste(
    taste: &AestheticTaste,
    npc: &NpcState,
    current_year: u32,
) -> AestheticTaste {
    let age = npc.age_years(current_year);
    let mut new_taste = *taste;
    let mut rng = DeterministicRng::from_seed(taste.personal_seed ^ current_year as u64);
    
    // 1. 年龄效应：complexity_tolerance 倒 U 型
    //    青少年: 高容忍（接受实验性艺术）
    //    中年(25-45): 巅峰
    //    老年(45+): 缓慢下降（偏好熟悉和简洁）
    let complexity_age_factor = if age < 18 {
        0.7 + (age as f32 - 10.0) / 8.0 * 0.3  // 10岁0.7 → 18岁1.0
    } else if age < 45 {
        1.0  // 平台期
    } else {
        (1.0 - (age - 45) as f32 * 0.008).max(0.6)  // 45→95: 1.0→0.6
    };
    new_taste.complexity_tolerance = (new_taste.complexity_tolerance * complexity_age_factor)
        .clamp(0.0, 1.0);
    
    // 2. 技能成长效应：自己的技艺提升 → virtuosity 鉴赏力提升
    let artisan_avg = npc.skills.avg_level(SkillCategory::Artisan);
    let fine_arts_avg = npc.skills.avg_level(SkillCategory::FineArts);
    let max_craft = artisan_avg.max(fine_arts_avg);
    let virtuosity_gain = (max_craft / 100.0).min(0.25);  // 上限 +0.25
    new_taste.dimension_weights[DIM_VIRTUOSITY] += virtuosity_gain * 0.08;  // 渐进
    new_taste.dimension_weights[DIM_VIRTUOSITY] = 
        new_taste.dimension_weights[DIM_VIRTUOSITY].clamp(0.05, 0.95);
    
    // 3. 旅行/接触多样性：去过越多文化区域 → novelty 偏好↑
    let cultures_encountered = npc.memory.distinct_cultures_encountered() as f32;
    let novelty_gain = (cultures_encountered / 15.0).min(0.20);  // 上限 +0.20
    new_taste.dimension_weights[DIM_NOVELTY] += novelty_gain * 0.05;
    new_taste.dimension_weights[DIM_NOVELTY] = 
        new_taste.dimension_weights[DIM_NOVELTY].clamp(0.05, 0.95);
    
    // 4. 微小的自然漂移（对标文化 drift）
    for dim in 0..6 {
        new_taste.dimension_weights[dim] += rng.f32_range(-0.005, 0.005);
        new_taste.dimension_weights[dim] = 
            new_taste.dimension_weights[dim].clamp(0.05, 0.95);
    }
    
    new_taste
}
```

### 3.2 创伤冲击 — 事件驱动（非年更新）

重大负面/正面审美事件可能一次性修改 taste：

```rust
/// 在 React 原子中，如果 |valence| > 0.9 且 somatic_impact > 0.8：
pub fn traumatic_taste_shift(
    taste: &AestheticTaste,
    event: &AestheticEvent,
) -> AestheticTaste {
    let mut new_taste = *taste;
    let intensity = event.judgment.somatic_impact * event.judgment.valence.abs();
    
    if event.judgment.valence < -0.9 {
        // 极度负面的审美体验 → 相关维度敏感度永久偏移
        // 例：看到艺术品被烧毁 → harmony 权重上升（"美是脆弱的"）
        new_taste.dimension_weights[DIM_HARMONY] += intensity * 0.15;
        new_taste.familiarity_bias -= intensity * 0.1;  // 更珍惜熟悉的东西
    }
    
    if event.judgment.valence > 0.9 && event.judgment.respect > 0.9 {
        // 被大师之作彻底征服 → 向该作品的审美方向偏移
        // 这个通过 Adopt 原子处理，不在此处
    }
    
    new_taste
}
```

## 四、Adopt 原子：品味传播的完整力学

### 4.1 AdoptEvent 定义

```rust
pub struct AdoptEvent {
    pub agent: NpcId,
    pub source: TasteSource,
    pub credibility: f32,    // [0,1] 该源在 agent 眼中的可信度
    pub intensity: f32,      // [0,1] 本次接触的强度
}

pub enum TasteSource {
    /// 某个人的品味（最直接的传播）
    Individual { npc_id: NpcId, taste: AestheticTaste },
    /// 一群人的聚合品味 → 社会规范压力
    Group { aggregated_taste: AestheticTaste, group_size: u32 },
    /// 文化的审美标准（通过教化/仪式）
    Culture { culture_id: CultureId },
    /// 某件作品本身的风格（"我想做出那样的东西"）
    Object { item_id: ItemEntId },
}
```

### 4.2 触发条件

| 触发源 | 条件 | intensity | credibility |
|--------|------|-----------|-------------|
| 反复接触某人穿着 | 过去 30 天见过该人 ≥5 次 | 0.2→0.5（与见面次数正比） | 0.4（"这人品味还行"） |
| 被某件作品强烈打动 | React 中 respect>0.8 或 somatic_impact 触发 | 0.6 | 0.9（被作品说服） |
| 文化教化 | 青春期每年一次 | 0.15 | 0.9（文化权威） |
| 被同伴推荐 | Articulate(mode=Social) 的目标是你 | 0.3 | 与说话者的关系有关 |
| 大师/导师的影响 | 有技能教学关系 | 0.4 | 0.85 |
| 节日集体活动 | 参与节日 Dance/Music 活动 | 0.2 | 0.5（集体压力） |

### 4.3 adopt_effect() 公式

```rust
pub fn adopt_effect(
    agent_taste: &AestheticTaste,
    source_taste: &AestheticTaste,
    source_credibility: f32,
    intensity: f32,
) -> AestheticTaste {
    let mut new_taste = *agent_taste;
    
    // 有效力 = credibility × intensity × (1 - aesthetic_openness的一半)
    // aesthetic_openness 高 → 阻力低 → 更容易被影响
    // 但 aesthetic_openness 也意味着更开放——不完全抗拒被改变
    let resistance = (1.0 - agent_taste.aesthetic_openness) * 0.5;
    let effective_force = source_credibility * intensity * (1.0 - resistance);
    
    // 每维向 source 方向微调
    for dim in 0..6 {
        let delta = (source_taste.dimension_weights[dim] - agent_taste.dimension_weights[dim])
                    * effective_force * 0.25;  // 单次最大偏移 25% 的差距
        new_taste.dimension_weights[dim] = 
            (agent_taste.dimension_weights[dim] + delta).clamp(0.05, 0.95);
    }
    
    // familiarity_bias 也受影响
    let fb_delta = (source_taste.familiarity_bias - agent_taste.familiarity_bias)
                   * effective_force * 0.15;
    new_taste.familiarity_bias = 
        (agent_taste.familiarity_bias + fb_delta).clamp(-1.0, 1.0);
    
    // complexity_tolerance 受影响
    let ct_delta = (source_taste.complexity_tolerance - agent_taste.complexity_tolerance)
                   * effective_force * 0.15;
    new_taste.complexity_tolerance = 
        (agent_taste.complexity_tolerance + ct_delta).clamp(0.0, 1.0);
    
    new_taste
}
```

### 4.4 Adopt 的涌现效应

- **时尚传播**：群体中多人的 Adopt 同方向 → 时尚趋势涌现
- **审美共识**：同文化 NPC 的 taste 在 Adopt 下收敛 → 审美标准从 bottom-up 涌现
- **艺术流派**：一群 NPC taste 收敛到局部最优 → 区域艺术风格 → 可命名和记录
- **文化审美漂移**：大量 Adopt 持续一个方向 → 最终反映到 CulturalBeautyStandard 更新

**注意**：Adopt 是微观机制。宏观现象（时尚/流派/共识）是多个 Adopt 事件的统计涌现——审美模块不编码"时尚"或"流派"。

## 五、与 CulturalBeautyStandard 的关系

### 5.1 区别

| | CulturalBeautyStandard | AestheticTaste |
|---|---|---|
| 所有权 | 文化系统 004 | NPC 模块存储，审美模块定义 |
| 粒度 | 文化级（一个文化的"什么是美"） | 个人级（一个 NPC 的审美偏好） |
| 范围 | 仅人体/外貌 | 所有可审美感知对象 |
| 可变性 | 文化漂移（极慢）或统治者更替 | 年更新 + Adopt 事件（中速） |
| 组成 | ideal_build, grooming_importance, scar_stance… | 6 维权重 + 3 个偏好参数 |

### 5.2 交互

CulturalBeautyStandard 影响 AestheticTaste 的初始派生（通过 `aesthetic_confidence` 和 `artistry` 等参数），但不直接限制个人 taste。个人可以通过 Adopt 偏离文化标准——这正是审美创新的微观基础。

---

> **下一篇**：[[005-事件原子详细设计|005 — 事件原子详细设计]]
