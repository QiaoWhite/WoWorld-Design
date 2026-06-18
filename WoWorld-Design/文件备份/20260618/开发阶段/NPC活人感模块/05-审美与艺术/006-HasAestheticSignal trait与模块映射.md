> **开发代号**: WoWorld (Wonder World)
> **文档类型**: 参考讨论草稿 — 可以矛盾、模糊、未决
> **创建日期**: 2026-06-16
> **关联**: [[文件备份/20260618/开发阶段/NPC活人感模块/05-审美与艺术/002-信号维度详细设计|002]] · [[文件备份/20260618/开发阶段/NPC活人感模块/05-审美与艺术/005-事件原子详细设计|005]]

# 006 — HasAestheticSignal trait 与各模块实现映射

## 一、Trait 定义

```rust
/// 审美模块定义的 trait——各模块在自己的实体类型上实现
/// 
/// 对标 ConsumableEffect 模式（Life 定义 schema → 物品存储）
/// 对标 EnchantmentSchema 模式（物品定义 → Magic 存储）
pub trait HasAestheticSignal {
    /// 返回该实体的客观美学特征向量
    /// 
    /// 调用时机：仅在 NPC Attend 到该实体时（lazy evaluation）
    /// 性能：必须在 <1µs 内完成（纯计算，零 I/O）
    fn aesthetic_signal(&self, ctx: &SignalContext) -> AestheticSignal;
}

/// 传递给 HasAestheticSignal 实现的上下文
pub struct SignalContext {
    /// 当前时间（用于光照/天气/季节对信号的影响）
    pub time: GameTime,
    /// 观察者位置（用于视角/距离对信号的影响）
    pub observer_position: Option<Vec3>,
    /// 天气状态（用于室外场景信号计算）
    pub weather: Option<WeatherSample>,
}
```

## 二、12 个实现者详细映射

### 2.1 ItemEntId — 实体物品

**所属模块**：物品系统  
**已有资产**：`AestheticProps`（13 StyleTag + ornamentation_level + color_palette + silhouette + fabric_quality + cleanliness_factor）

**映射逻辑**：

| Signal 维度 | 来源 | 映射方式 |
|------------|------|---------|
| fluency | StyleTag（Formal/Casual）+ silhouette（Fitted/Loose）| Formal+Fitted → 0.8, Casual+Loose → 0.3 |
| novelty | StyleTag（Exotic）+ era_hint | Exotic → 0.6, Contemporary → 0.1 |
| complexity | ornamentation_level + style_tags 数量 | 直接 1:1 |
| harmony | color_palette 的色彩协调度 | 调色板内部色相差分→0-1 |
| expressiveness | StyleTag（Religious/Royal/Martial 情感承载）| Religious → 0.7, Plain → 0.1 |
| virtuosity | Quality × fabric_quality | Quality=Perfect + fabric_quality=1.0 → 0.95 |

**注意**：物品系统只存储 `AestheticProps`。`HasAestheticSignal` 的映射函数由物品系统实现——审美模块不知道 `StyleTag` 的存在。

### 2.2 BuildingId — 建筑

**所属模块**：世界生成  
**已有资产**：`BuildingStylePreferences`（RoofStyle + WallMaterial + DecorationLevel + ColorPalette + symmetry + scale + AdjacencyModifiers）

| Signal 维度 | 来源 |
|------------|------|
| fluency | symmetry + WallMaterial（规整石材→高，有机材料→中）|
| novelty | RoofStyle（Dome/Spire→高，Flat→低）+ 文化风格匹配度 |
| complexity | DecorationLevel 直接映射 + scale（monumental→高） |
| harmony | ColorPalette 的 hue_family 协调 + proportion 的整数比 |
| expressiveness | RoofStyle（Spire→高情感，Flat→低）+ CulturalBeautyStandard 渗透 |
| virtuosity | DecorationLevel + WallMaterial（ImportedStone→高）+ 建筑质量 |

### 2.3 CreatureId — 生物（动物/怪物/NPC身体）

**所属模块**：生命系统  
**已有资产**：四层质量防线中的美学评分（5 因子：比例和谐度/色彩一致性/功能一致性/轮廓可识别性/生态位契合）

| Signal 维度 | 来源 |
|------------|------|
| fluency | 轮廓可识别性 + 比例和谐 |
| novelty | 生态位契合的逆（怪异生物 novelty 高）|
| complexity | 体节数量 + 色彩种类 + 附肢复杂性 |
| harmony | 色彩一致性 + 比例和谐 |
| expressiveness | 面部肌肉/姿态语言的能力（智能种族高，低级动物低）|
| virtuosity | 不适用（自然生物非"制作"），种子生成质量映射 0.1-0.3 |

### 2.4 NpcId — NPC 整体形象（身体+着装）

**所属模块**：NPC  
**已有资产**：`judge_outfit()`（装备系统）+ `conventional_attractiveness()`（NPC 02）

**合成逻辑**：
```rust
fn aesthetic_signal(npc: &NpcState) -> AestheticSignal {
    let body_signal = npc.body.aesthetic_signal();    // CreatureId
    let outfit_signal = npc.equipment.outfit_signal(); // ItemEntId 聚合
    
    AestheticSignal {
        fluency:        body_signal.fluency * 0.5 + outfit_signal.fluency * 0.5,
        novelty:        outfit_signal.novelty * 0.7 + body_signal.novelty * 0.3,
        complexity:     outfit_signal.complexity * 0.6 + body_signal.complexity * 0.4,
        harmony:        outfit_signal.harmony * 0.6 + body_signal.harmony * 0.4,
        expressiveness: npc.emotion.visible_expression() * 0.4 
                       + outfit_signal.expressiveness * 0.4 
                       + body_signal.expressiveness * 0.2,
        virtuosity:     outfit_signal.virtuosity * 0.8 + body_signal.virtuosity * 0.2,
    }
}
```

**关键**：NPC 的情绪状态影响 expressiveness——开心时仪态更流畅，愤怒时面部更不对称（harmony↓）。

### 2.5 VehicleId — 载具

**所属模块**：载具系统  
**已有资产**：`VehicleAesthetics`（bow_shape + stern_shape + decoration_density + figurehead_tradition）

**映射**：decoration_density → complexity/virtuosity，bow_shape → fluency/harmony，figurehead_tradition → expressiveness/novelty。

### 2.6 ScenePosition — 自然/城市景观

**所属模块**：世界生成  
**组成**：地形参数 + 天气 + 光照 + 时间 + 季节

| Signal 维度 | 来源 |
|------------|------|
| fluency | 天际线平滑度 + 地形起伏的规律性 |
| novelty | 稀有地形（峡谷/火山/冰川）→高，普通平原→低 |
| complexity | 植被密度 + 地形褶皱度 + 水文复杂度 |
| harmony | 色彩季节协调 + 水体反光 + 云层形态 |
| expressiveness | 天气戏剧性（暴风雨→高，晴天→低）+ 光线情感质量 |
| virtuosity | 人造景观（城市/花园）→建筑的 virtuosity；纯自然→低 |

**性能注意**：ScenePosition 的 Signal 计算最昂贵（需要地形查询+天气查询+光照计算）。但调用频率低（仅 NPC 在特定位置停驻时），可接受。

### 2.7 PerformanceRef — 瞬时表演事件

**所属模块**：文化/NPC  
**子类型**：音乐演奏、舞蹈、戏剧、诗歌朗诵、说书

| Signal 维度 | 来源 |
|------------|------|
| fluency | 演奏/舞蹈的流畅度 ← performer 的技能水平 |
| novelty | 曲目/编舞的新颖度 |
| complexity | 声部层次/编舞复杂度/叙事层次 |
| harmony | 节奏精确度 + 和声协调 + 编舞同步 |
| expressiveness | 表演者的情感投入度 + 作品本身的情感承载 |
| virtuosity | 表演者的技能水平直接映射 |

### 2.8 SkillActionRef — 技艺过程

**所属模块**：技能系统  
**覆盖**：打铁、绣花、烹饪、木工、炼金…任何可被旁观的技能执行过程

| Signal 维度 | 来源 |
|------------|------|
| fluency | motion_fluency（动作的流畅度 ← skill_level） |
| novelty | visible_progress（材料变化的可见性。铁→剑是新颖的） |
| complexity | procedural_complexity（工序步骤数 + 工具种类） |
| harmony | rhythmic_harmony（动作节奏的规律性——"打铁三下一停"） |
| expressiveness | visible_devotion（执行者的专注/投入程度 + 情绪外显） |
| virtuosity | (skill_level / 100.0).min(1.0) |

**性能关键**：`HasAestheticSignal` 只在有人关注时才被调用（lazy evaluation）。技能系统不需要每帧为所有正在执行的技能计算 Signal。

### 2.9 CombatExchangeRef — 战斗招式交换

**所属模块**：战斗系统

| Signal 维度 | 来源 |
|------------|------|
| fluency | combo_flow（连招流畅度）× interruption_penalty（被打断的扣分） |
| novelty | move_rarity + unexpected_outcome_factor |
| complexity | tactical_depth + move_chain_length |
| harmony | rhythm_of_exchange（攻防节奏） + balance_of_power（力量对比均衡→和谐） |
| expressiveness | combatant_visible_emotion（战士在战斗中的情感——愤怒/绝望/沉着/狂热） |
| virtuosity | combat_skill_visible（战斗技艺的可见度） |

**关键特殊性**：战斗的 stakes 不改变 Signal 本身，但通过 `AestheticContext.stakes` 调制 judgment 的 arousal 和 respect。

### 2.10 MagicConstructRef — 魔法构造体（持久魔法效果）

**所属模块**：魔法系统  
**覆盖**：发光的符文、幻象彩虹、魔法护盾的视觉、召唤物的美学外观

### 2.11 SpellCastRef — 施法过程

**所属模块**：魔法系统  
**覆盖**：咒语吟唱、手势轨迹、法阵绘制、法术的瞬时视觉效果

| Signal 维度 | 来源 |
|------------|------|
| fluency | gesture_fluency ← caster_skill |
| novelty | spell_rarity + element_combination_novelty |
| complexity | spell_tier + multi_element_complexity |
| harmony | mantra_rhythm + gesture_harmony + visual_harmony |
| expressiveness | caster_emotion_visible + paradigm_expression（艺术范式→高） |
| virtuosity | (caster_skill_level / 100.0).min(1.0) |

**魔法范式的文化差异**：通过 `AestheticContext.prior_expectation` 承载——艺术范式文化看同一个火球术的 prior_expectation 更高（"魔法应该像诗一样"）。

### 2.12 RitualRef — 仪式执行

**所属模块**：文化系统（RitualDef）

| Signal 维度 | 来源 |
|------------|------|
| fluency | ritual_execution_precision（仪式执行的精确度） |
| novelty | ritual_rarity（年度一次 vs 每日） |
| complexity | ritual_steps + participant_count |
| harmony | participant_synchrony（参与者同步度——仪式的核心） |
| expressiveness | participant_sincerity（参与者真诚度——"走形式" vs "真心"） |
| virtuosity | officiant_skill（主持者的技艺） |

---

## 三、Trait 的职责边界（关键——防止混淆）

| 责任 | 谁负责 |
|------|--------|
| 定义 `AestheticSignal` struct | **审美模块** |
| 定义 `HasAestheticSignal` trait | **审美模块** |
| 实现 `HasAestheticSignal for ItemEntId` | **物品系统** |
| 实现 `HasAestheticSignal for BuildingId` | **世界生成** |
| 实现 `HasAestheticSignal for CreatureId` | **生命系统** |
| …（其余同理） | **各自所属模块** |
| 存储 `AestheticProps`（物品的原始审美数据） | **物品系统** |
| 将 `AestheticProps` 映射为 `AestheticSignal` | **物品系统**（在自己的 trait impl 中） |
| 知道 `StyleTag` 的存在 | **物品系统**——审美模块不知道 |
| 知道 `RoofStyle` 的存在 | **世界生成**——审美模块不知道 |

**审美模块零反向依赖**——它不知道 `StyleTag`、`RoofStyle`、`VehicleAesthetics`。它只知道 `AestheticSignal`（6 个 f32）。

---

## 四、实现优先级和分类

| 优先级 | 实现者 | 理由 |
|--------|--------|------|
| P0 | ItemEntId, NpcId, ScenePosition | 最常被感知的对象 |
| P1 | BuildingId, CreatureId, PerformanceRef | 常见的审美对象 |
| P2 | SkillActionRef, CombatExchangeRef | 过程审美 |
| P3 | SpellCastRef, MagicConstructRef | 魔法审美 |
| P4 | RitualRef, VehicleId | 低频场景 |

---

> **下一篇**：[[文件备份/20260618/开发阶段/NPC活人感模块/05-审美与艺术/007-跨模块依赖与接口全面清单|007 — 跨模块依赖与接口全面清单]]
