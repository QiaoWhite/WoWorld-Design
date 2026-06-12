# NPC活人感系统——开发者文档

> **版本**：2.0 — Rust 重写版  
> **日期**：2026-06-11  
> **架构**：Rust 模拟核心 + Godot 4.6 LTS 渲染客户端（GDExtension 集成）  
> **定位**：WoWorld NPC 系统的权威实现规格。取代 ver1.01（GDScript 版，已废弃）  
> **关联**：[[018-正式技术栈方案v3-20260610|技术栈 v3.0]] · [[CHG-007]] · [[CHG-008]]
> 
> **文档维护计划**：当前 ~2200 行。超过 10000 行时拆分——Part 2 各章独立为 `01-情绪引擎.md` 至 `04-分层模拟.md`，Part 5 独立为 `05-战斗AI.md` 至 `07-天象感知.md`，Part 6 独立为 `08-工程实现.md`。本文件保留为总纲（数据合同 + trait 索引 + wikilink 导航）。

---

## 绪论：什么是"活人感"

在游戏开发中，"活人感"并非让 NPC 看起来像真人，而是让玩家**感觉**他们像真人。

- "看起来像真人"追求动画质量、面捕精度、对话脚本的丰富度。
- "感觉像真人"追求**行为的不完全可预测性**、**社会关系的动态演变**、**记忆与情绪的持久影响**、**个体在宏大世界中的有限认知**，以及**世界对玩家行为的自洽回应**。

**核心设计原则**：开发者只创造法则，不创造故事。你定义人格参数、行动可能性、情绪规则、文化传播率、概率分布的权重因子；具体故事由 NPC 局部互动以概率和统计的方式自然涌现。

本文档覆盖 NPC 系统的完整架构——从数据合同到心智算法到分层调度到物理表达——全部以 Rust trait/struct 伪代码书写，面向 LLM 辅助开发场景。

---

## 总体架构：六层模型与三大决策环

### 六层模型（由内向外）

```
1. 数据本体 (NPCData struct)
      │
2. 心智核心 (Emotion + Memory + Decision + Social + Culture + Spatial)
      │
3. 感官外壳 (Visual + Auditory + EmotionPerception + SkyPerception ★)
      │
4. 物理躯体 (CharacterBody3D ← Godot 侧 · Rust 侧仅输出骨骼矩阵)
      │
5. 社会投影 (其他 NPC 记忆 + 共识场中的"他")
      │
6. 统计层 (L4 ★ — 超远距纯统计 NPC，无个体实例)
```

### 三大决策环

1. **本地概率决策** (~90%) — 文化习俗 × 个人习惯 × 人格偏移 × 情绪修正 → 加权随机采样
2. **GOAP 安全网** (~9%) — 饥饿 <0.3 / 口渴 <0.25 / 重伤 / 致命威胁 / 重大交易 / 仪式 → 确定性规划
3. **LLM 增强（可选）** (~1%) — 复杂社交场景。仅从预定义行动库选取。安全网关 + 速率限制

---

# Part 1: 数据合同

> **设计意图**：定义 NPC 系统的所有数据结构和接口边界。这些 struct 和 trait 是模块之间的"合同"——不同模块并行开发时，合同保证了集成的一致性。

---

## 1.1 NPCData — 根部 struct

```rust
/// NPC 的唯一真相来源。纯数据，无逻辑。
/// 序列化格式: bincode → LMDB
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcData {
    // ── 身份标识 ──
    pub identity: Identity,

    // ── 人格 ──
    pub personality: BigFive,

    // ── 生理状态 ──
    pub physiology: Physiology,

    // ── 情绪 ──
    pub emotion: EmotionState,

    // ── 记忆（双层: 事件 + 空间）──
    pub memory: MemoryStore,

    // ── 关系 ──
    pub relationships: BTreeMap<NpcId, Relationship>,

    // ── 技能 ──
    pub skills: BTreeMap<SkillId, SkillEntry>,

    // ── 战斗属性 ──
    pub combat: CombatAttributes,

    // ── 知识 ──
    pub knowledge: Knowledge,

    // ── 自我叙事（替代原 LongTermPlanning 占位符）──
    pub self_narrative: SelfNarrative,

    // ── 社会身份 ──
    pub social_identity: SocialIdentity,

    // ── 派系归属 ──
    pub factions: Vec<FactionId>,

    // ── 共识缓存 ──
    pub consensus_cache: BTreeMap<FactionId, ConsensusCache>,

    // ── 行为统计 ──
    pub behavior_stats: BehaviorStats,

    // ── 当前状态（瞬态）──
    pub current_state: CurrentState,

    // ── 元数据 ──
    pub metadata: NpcMetadata,

    // ── v3 新增字段 ──
    pub vehicle_state: Option<VehicleState>,    // 载具上时的状态
    pub sky_perception: SkyPerception,          // 天象感知缓存
    pub construction_task: Option<ConstructionTask>, // 施工任务

    // ── 性别与吸引力系统（02-性别与吸引力系统.md）──
    pub mental: MentalAttributes,               // 心智属性（过渡方案——融合后迁移至 LifeEntity）
    pub physical: PhysicalAttributes,           // 身体属性（已融合至 LifeEntity.physical，此处为计算视图引用）
    pub appearance: PhysicalAppearance,         // 外貌视觉特征（过渡方案）
    pub attraction_template: AttractionTemplate, // 吸引力偏好模板
    pub norm_internalizations: BTreeMap<NormId, NormInternalization>, // 规范内化（稀疏存储）
}
```

### 1.1.1 Identity

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub id: NpcId,                     // "npc_7a3f_001"
    pub name: String,
    pub race: RaceId,
    pub biological_sex: BiologicalSex,   // 生物性别（详见 02-性别与吸引力系统.md §8.1）
    pub age: f32,                      // 当前年龄 (岁)
    pub birth_day: GameDay,            // 出生游戏天数
    pub is_dead: bool,
    pub death_day: Option<GameDay>,
    pub profession: ProfessionId,      // 职业
    pub title: Option<String>,         // 称号/头衔
}
```

### 1.1.2 BigFive（大五人格）

```rust
/// 人格在 NPC 生命周期极少变动。
/// 仅极高冲击力创伤 (>0.9) 或长期环境冲突 (>100 游戏日累积) 时微调 (Δt ≤ 0.05)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BigFive {
    /// 开放性: 高→好奇/尝试新事物; 低→保守/偏好熟悉
    pub openness: f32,           // 0-1

    /// 尽责性: 高→自律/守时/计划性强; 低→随性/易分心
    pub conscientiousness: f32, // 0-1

    /// 外向性: 高→主动社交/群体愉悦; 低→回避大型社交
    pub extraversion: f32,      // 0-1

    /// 宜人性: 高→信任/合作/易感染; 低→竞争/怀疑/交易激进
    pub agreeableness: f32,     // 0-1

    /// 神经质: 高→负面事件敏感/恢复慢; 低→情绪稳定/恢复快
    pub neuroticism: f32,       // 0-1
}

impl BigFive {
    /// 每个 NPC 至少有一个"突出维度"(偏离中位 >0.25)。
    /// 这确保了可感知的人格差异。
    /// (017 测试 Phase 3 #1 验证: 中段人格在 1 分钟观察窗口内不可感知。)
    pub fn has_prominent_dimension(&self) -> bool {
        self.openness.abs_diff(0.5) > 0.25
            || self.conscientiousness.abs_diff(0.5) > 0.25
            || self.extraversion.abs_diff(0.5) > 0.25
            || self.agreeableness.abs_diff(0.5) > 0.25
            || self.neuroticism.abs_diff(0.5) > 0.25
    }
}
```

### 1.1.3 Physiology

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Physiology {
    // ⚠️ 尺度方向已统一为 Life.Vitals 约定:
    //    hunger/thirst: 0.0=极度缺乏 ~ 1.0=完全满足 (0=bad, 1=good)
    //    fatigue:       0.0=精力充沛 ~ 1.0=极其疲劳 (0=good, 1=bad)
    //    与生命系统 [[004-身体状态与生命过程]] 完全一致
    pub hunger: f32,           // 0.0=极度饥饿 ~ 1.0=饱腹, GOAP强制 <0.3
    pub thirst: f32,           // 0.0=脱水 ~ 1.0=不渴, GOAP强制 <0.25
    pub fatigue: f32,          // 0.0=精力充沛 ~ 1.0=极其疲劳, GOAP强制 >0.9
    pub health: f32,           // 0-1 (1=满血, 0=濒死), GOAP强制 <0.3
    pub stamina: f32,          // 瞬时可用体力 (区别于 fatigue)
    // ★ mana 已废弃——改为从 Life.SpiritState + Life.MagicAttributes 读取
    //    见 [[004-身体状态与生命过程|生命/004-身体状态与生命过程]] §四 (Spirit & Magic)
    //    灵元素含量/十元素亲和/魔力强度/控制/抗性/恢复速度——由 Life 基类统一管理
    pub temperature: f32,      // 体感温度 (受季节/天气/服装修正)

    // ★ 部位伤害——持久化存储，非战斗临时数据
    //    损伤值来自 Combat [[012-战后过渡与伤势]] 的部位伤害模型
    //    仅影响属性数值，不影响战斗动作模组（不会因手臂受伤而切换单手动画）
    pub body_part_damage: BodyPartDamage,
}

/// 部位损伤数据——持久化到 NPC 数据中
/// 每个部位独立存储 damage: f32 (0.0-1.0, 0=完好, 1=完全损坏)
/// 具体部位集合由 Life 的形态模板决定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyPartDamage {
    pub head: f32,
    pub torso: f32,
    pub left_arm: f32,
    pub right_arm: f32,
    pub left_leg: f32,
    pub right_leg: f32,
    pub tail: f32,      // 0.0 if no tail
    pub wings: f32,     // 0.0 if no wings
    // 额外肢体由形态模板动态扩展
    pub extra_limbs: Vec<f32>,
}
```

### 1.1.3a Physiology 派生自 Life.Vitals（主观感知层）

`Physiology` 不是独立存储的数据——它是 **NPC 主观感知身体状态的"镜头"**，每次决策前从 `Life.Vitals`（客观生物数据）派生。两者关系：

```
Life.Vitals (客观生物真相, LMDB 存储, 绝对值)
    │
    │  Physiology::from_vitals(vitals, perception_context)
    │  - health = vitals.hp / vitals.max_hp           // 绝对→归一化
    │  - temperature = 体感温度(body_temp, 衣服, 天气)  // 客观→体感
    │  - stamina = vitals.stamina / vitals.max_stamina  // 归一化
    │  - body_part_damage → 直接从 Life 同步（持久化数据）
    │
    ▼
NPC.Physiology (主观感知, NPC 决策用, 0-1 归一化)
    → GOAP 阈值判断 (hunger < 0.3 → 找食物)
    → 情绪引擎 (health < 0.5 → 恐惧上升)
    → 行为选择 (fatigue > 0.9 → 必须睡觉)
```

**为什么保留 Physiology 而非直接用 Vitals**：
- NPC 的 GOAP/情绪/行为代码全部基于 0-1 归一化值做阈值判断——这些值不是"物理事实"而是"身体感受"
- `temperature` 是体感温度（受衣服/天气修正）——Life 的 `body_temperature` 是核心体温（生理事实）
- `health` 是"我觉得自己还剩多少"——Life 的 `hp` 是"还剩多少血"（绝对值）
- 两个抽象层服务于不同目的：Life 管理生物事实，NPC 管理行为决策

```rust
impl Physiology {
    /// 从 Life.Vitals + 感知上下文 派生 NPC 的主观身体感知
    pub fn from_vitals(v: &Vitals, ctx: &PerceptionContext) -> Self {
        Self {
            hunger:     v.hunger,                                           // 直通（已是 0-1）
            thirst:     v.thirst,                                           // 直通（已是 0-1）
            fatigue:    v.fatigue,                                          // 直通（已是 0-1）
            health:     v.hp / v.max_hp.max(1.0),                           // 绝对值→归一化
            stamina:    v.stamina / v.max_stamina.max(1.0),                 // 绝对值→归一化
            temperature: Self::perceived_temperature(v.body_temperature, ctx),
            body_part_damage: v.body_part_damage.clone(),                   // 持久化部位损伤
        }
    }

    /// 体感温度 = 核心体温 + 环境/衣着修正
    fn perceived_temperature(core: f32, ctx: &PerceptionContext) -> f32 {
        let clothing_mod = ctx.clothing_warmth.clamp(0.0, 1.0);
        let weather_mod = ctx.ambient_temperature_offset;
        core + weather_mod * (1.0 - clothing_mod * 0.7)
    }
}

/// 感知上下文——影响 NPC 如何"感受"自己的身体
struct PerceptionContext {
    clothing_warmth: f32,           // 衣着保暖度 0-1
    ambient_temperature_offset: f32, // 环境温度偏移（相对常温）
    // 未来可扩展：疾病/药物/醉酒等感知扭曲因子
}
```

**重要**：NPC 模块的 GOAP/情绪/行为代码 **零修改**——继续使用 0-1 归一化的 `Physiology`。此设计确保：
1. Life 是唯一的生物数据权威源（`Vitals`）
2. NPC 的心智模型完全保留（`Physiology` 的计算视图）
3. 两者通过 `from_vitals()` 明确桥接——不合并、不删除

### 1.1.4 EmotionState

```rust
/// 情绪状态: 三维底层轴 + 8 种基本情绪 + 复合情绪标签
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionState {
    /// 愉悦轴 (-1 ~ 1)
    pub pleasure: f32,

    /// 觉醒轴 (0 ~ 1)
    pub arousal: f32,

    /// 掌控轴 (-1 ~ 1)
    pub control: f32,

    /// 8 种基本情绪强度 (0-1)
    pub basic_emotions: BasicEmotions,

    /// 当前复合情绪
    pub active_composite_label: Option<CompositeEmotion>,
    pub composite_intensity: f32,

    /// ★ 心境层——缓慢变化（天→周），与快速情绪（秒→小时）分离
    /// 心境是情绪的"天气"，情绪是心境的"阵雨"
    pub mood: MoodState,
}

/// ★ 心境状态——独立于瞬时情绪
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoodState {
    pub mood_pleasure: f32,          // -1~1 这段日子开心吗
    pub mood_arousal: f32,           // 0~1  这段日子精力充沛吗
    pub mood_control: f32,           // -1~1 这段日子掌控感如何
    pub mood_label: Option<MoodLabel>,
    pub days_since_last_shift: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MoodLabel {
    Cheerful,     // 开朗
    Melancholic,  // 忧郁
    Irritable,    // 易怒
    Serene,       // 宁静
    Anxious,      // 焦虑
    Apathetic,    // 淡漠
    Euphoric,     // 狂喜
    Despondent,   // 沮丧
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BasicEmotions {
    pub joy: f32,
    pub trust: f32,
    pub fear: f32,
    pub surprise: f32,
    pub sadness: f32,
    pub disgust: f32,
    pub anger: f32,
    pub anticipation: f32,
}
```

### 1.1.5 MemoryStore

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryStore {
    /// 事件记忆 — 容量上限 2000 条
    pub events: Vec<EventMemory>,

    /// 空间记忆 — 五级道路
    pub spatial: Vec<SpatialMemory>,

    /// 被压缩的冷记忆 (摘要化)
    pub cold_memory_summaries: Vec<ColdMemorySummary>,

    /// v3: 重要记忆标记 (战斗/分享/被救/航海事件/施工完成 — 优先保留)
    pub important_event_ids: HashSet<MemoryId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMemory {
    pub id: MemoryId,
    pub timestamp: GameTime,
    pub game_day: GameDay,
    pub location_id: LocationId,
    pub summary: String,
    pub participants: Vec<NpcId>,
    pub participant_roles: BTreeMap<NpcId, String>,
    pub emotional_encoding: EmotionalEncoding,
    pub impact_score: f32,         // 0-1
    pub is_public: bool,
    pub access_count: u32,
    pub decay_factor: f32,
    pub tags: Vec<String>,

    /// v3: 事件类型 (用于记忆优先级和检索)
    pub event_type: EventType,

    /// v3: 是否为"重构后"版本 (自我辩护/效价扭曲后不可逆)
    pub is_reconstructed: bool,
    pub original_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EventType {
    DailyInteraction,         // 日常社交
    ResourceCompetition,      // 资源争夺
    Combat,                   // 战斗
    Cooperation,              // 合作/分享
    Ceremony,                 // 仪式 (婚礼/葬礼/成人礼)
    Travel,                   // 旅行事件
    WeatherExtreme,           // 极端天气经历
    Construction,             // 施工/建造
    Nautical,                 // v3: 航海事件
    Vehicular,                // v3: 载具事件
    Celestial,                // v3: 天象事件 (彩虹/日食/暴风雨等)
    Revelation,               // 重要发现/顿悟
    EmotionalMilestone,       // ★ 情感里程碑——内心的重要发现（详见 02-性别与吸引力系统.md）
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpatialMemory {
    pub id: MemoryId,
    pub path_type: PathType,       // 五级道路
    pub waypoints: Vec<Vec3>,
    pub landmarks: Vec<LandmarkRef>,
    pub confidence: f32,           // 0-1
    pub source: SpatialMemorySource,
    pub use_count: u32,
    pub is_validated: bool,

    /// v3: 道路危险度评估 (影响旅行决策)
    pub danger_assessment: Option<f32>,  // 0-1

    /// v3: 该路径是否有商队/旅行者频繁使用
    pub traffic_level: Option<TrafficLevel>,
}
```

### 1.1.6 v3 新增结构体

```rust
/// 载具状态 — NPC 在载具上时的附加数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VehicleState {
    pub vehicle_id: VehicleId,
    pub role: VehicleRole,       // Crew / Passenger
    pub position_local: Vec3,    // 载具本地坐标
    pub current_duty: Option<CrewDuty>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VehicleRole {
    Crew { rank: CrewRank },
    Passenger,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrewDuty {
    Navigating,        // 操控航向/轨道
    Lookout,           // 瞭望
    Maintenance,       // 维护
    AssistingPassengers,
}

/// 天象感知 — NPC 对天空状态的感知缓存
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkyPerception {
    pub cloud_cover: f32,           // 0-1
    pub cloud_type: CloudType,
    pub rainbow_visible: bool,
    pub storm_approaching: bool,
    pub visibility_quality: f32,    // 0-1
    pub last_updated: GameTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CloudType {
    Clear,
    Scattered,
    Broken,
    Overcast,
    Cumulonimbus,    // 积雨云 — 暴风雨前兆
    Stratocumulus,   // 层积云
    Cirrus,          // 卷云
}

/// 施工任务 — NPC 参与蓝图施工
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstructionTask {
    pub blueprint_id: BlueprintId,
    pub role: ConstructionRole,
    pub assigned_site: Option<Vec3>,
    pub material_inventory: BTreeMap<MaterialId, u32>,
    pub progress: f32,               // 0-1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConstructionRole {
    MaterialGatherer,
    Builder,
    Foreman,          // 工头 — 分配任务给其他施工 NPC
    Supplier,         // 物资运输
}
```

---

## 1.1.7 性别与吸引力新增类型（02-性别与吸引力系统.md）

> 以下类型为性别与吸引力系统的数据合同。定义于此以便所有 NPC 子系统引用。

### AttractionType

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttractionType {
    Physical,       // 身体外貌吸引力
    Charismatic,    // 人格魅力（通过交互逐渐感知）
    Intellectual,   // 智力/才华的吸引
    StatusBased,    // 社会地位的吸引
    Composite,      // 上述多种的组合
}
```

### BiologicalSex

```rust
/// 生物性别——替换原未定义的 Gender 类型
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum BiologicalSex {
    Male,
    Female,
    Hermaphroditic,                          // 同时具有两性生殖能力
    Sequential { current_phase: SequentialPhase }, // 生命中改变性别
    Neuter,                                  // 无性别（亡灵/构装体）
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SequentialPhase { Male, Female, Transitioning }
```

### InteractionType

```rust
/// 社会互动的类型分类
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InteractionType {
    CasualChat,
    Cooperation,
    Conflict,
    Trade,
    IntellectualExchange,
    Courtship,        // ★ 求爱互动
    Ceremony,
    SharedActivity,
}
```

### SocialInteraction

```rust
/// 每次社会互动的基础结构
#[derive(Debug, Clone)]
pub struct SocialInteraction {
    pub interaction_type: InteractionType,
    pub significance: f32,        // 0-1 这次互动有多"有意义"
    pub effort_level: f32,        // 0-1 发起者付出了多少努力
    pub target_id: NpcId,
    pub duration_minutes: f32,
    pub mood_at_time: f32,        // 互动时的情绪基调 -1~1
}

impl SocialInteraction {
    pub fn is_courtship(&self) -> bool {
        matches!(self.interaction_type, InteractionType::Courtship)
    }
}
```

### SkillCategory

```rust
/// 技能类别——用于 skill_preference 的 key
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum SkillCategory {
    Combat,
    Magic,
    Artisan,
    Academic,
    Social,
    Survival,
    Economic,
}
```

### MentalAttributes

```rust
/// 心智属性——过渡方案，存储于 NpcData.mental（融合后迁移至 LifeEntity）
/// 完整定义见 生命/004-身体状态与生命过程 §三
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentalAttributes {
    pub intelligence: f32,       // 0-1
    pub wisdom: f32,             // 0-1 社交洞察/风险评估/抵抗欺骗
    pub willpower: f32,          // 0-1 抵抗恐惧/痛苦忍耐/控制冲动
    pub charisma: f32,           // 0-1 社交说服/领袖力/NPC初始好感度
    pub memory_capacity: u16,    // 0-2000
}
```

### PhysicalAppearance

```rust
/// 外貌视觉特征——过渡方案，存储于 NpcData.appearance（融合后迁移至 LifeEntity）
/// 完整定义见 02-性别与吸引力系统.md §1.1
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalAppearance {
    pub height: f32,             // 相对于种族均值的偏差 -1~1
    pub build: BuildType,        // Slender/Lean/Average/Muscular/Stocky/Heavy
    pub body_proportions: f32,   // -1~1
    pub face_params: FaceParams, // 8 维连续参数
    pub skin_tone: SkinTone,
    pub hair_color: HairColor,
    pub hair_style: HairStyle,
    pub facial_hair: FacialHairStyle,
    pub voice_pitch: f32,
    pub voice_timbre: f32,
    pub grooming_level: f32,     // 0-1 动态变化
    pub scars: Vec<Scar>,
    pub tattoos: Vec<Tattoo>,
    pub distinguishing_marks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BuildType { Slender, Lean, Average, Muscular, Stocky, Heavy }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaceParams {
    pub face_width: f32, pub jaw_shape: f32, pub cheekbone_height: f32,
    pub eye_size: f32, pub eye_spacing: f32, pub nose_bridge: f32,
    pub nose_width: f32, pub lip_fullness: f32,
}
// SkinTone, HairColor, HairStyle, FacialHairStyle, Scar, Tattoo:
// 实现阶段定义——不影响本文档的心智逻辑
```

### NormInternalization

```rust
/// 个体对一条文化规范的三维度内化程度
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormInternalization {
    pub cognitive_acceptance: f32,     // 认知认同 0-1
    pub emotional_alignment: f32,      // 情感顺应 0-1
    pub behavioral_compliance: f32,    // 行为顺从 0-1（受社会代价影响）
}

impl Default for NormInternalization {
    fn default() -> Self {
        Self { cognitive_acceptance: 1.0, emotional_alignment: 1.0, behavioral_compliance: 1.0 }
    }
}
// 稀疏存储: 仅存储偏离 >0.15 的条目。未存储 = 完全内化（接受文化默认）
```

---

## 1.2 核心 Traits

```rust
/// 行为决策引擎 — NPC 心智的核心接口
pub trait DecisionEngine: Send + Sync {
    /// 给定 NPC 当前状态和主观世界感知，返回所选行动
    fn select_action(
        &self,
        npc: &NpcData,
        world: &SubjectiveWorld,
        rng: &mut impl Rng,
    ) -> Action;

    /// GOAP 规划 — 目标+当前状态 → 行动序列
    fn plan_goap(
        &self,
        goal: &Goal,
        npc: &NpcData,
        world: &SubjectiveWorld,
        timeout: Duration,        // 硬超时 v3: ≤2ms
    ) -> Result<Vec<Action>, GoapError>;
}

/// 事件总线订阅 — 所有子系统通过 trait 订阅
pub trait EventSubscriber: Send + Sync {
    fn on_event(&mut self, event: &WorldEvent);
    fn subscribed_event_types(&self) -> Vec<EventCategory>;
}

/// 感官提供者 — 将客观世界翻译为主观感知
pub trait SensoryProvider: Send + Sync {
    fn perceive_visual(&self, npc: &NpcData, world: &WorldState) -> SubjectiveVisual;
    fn perceive_auditory(&self, npc: &NpcData, world: &WorldState) -> SubjectiveAuditory;
    fn perceive_emotion(&self, observer: &NpcData, target: &NpcData) -> PerceivedEmotion;

    /// v3: 天象感知
    fn perceive_sky(&self, npc: &NpcData, weather: &WeatherState) -> SkyPerception;
}

/// NPC 记忆编码器 — 决定什么值得记住
pub trait MemoryEncoder: Send + Sync {
    fn should_encode(
        &self,
        event_type: &EventType,
        impact_score: f32,
        npc: &NpcData,
    ) -> bool;

    fn encode(
        &self,
        event: &WorldEvent,
        npc: &NpcData,
        emotional_state: &EmotionState,
    ) -> EventMemory;
}

/// 战斗 AI — 玩家和 NPC 共享同一个 trait 实现
pub trait CombatAI: Send + Sync {
    /// 战斗风格参数 (从人格派生)
    fn combat_style(&self, npc: &NpcData) -> CombatStyle;

    /// 自动执行一帧战斗逻辑
    /// 玩家调用此函数时，style 来自玩家选择的战斗风格
    /// NPC 调用此函数时，style 来自 NPC 人格派生
    fn execute_frame(
        &self,
        context: &CombatContext,
        style: &CombatStyle,
        high_level_command: Option<CombatCommand>,  // v3: 玩家高层指令
    ) -> CombatAction;

    /// v3: 信息战 — 评估对方实力是否"虚张声势"
    fn assess_bluff(
        &self,
        observer: &NpcData,
        target: &NpcData,
        target_apparent_behavior: &CombatBehavior,
    ) -> BluffAssessment;
}
```

---

# Part 2: 心智核心

> **设计意图**：定义 NPC 的"内心世界"——情绪如何产生和变化、记忆如何编码和衰减、行为如何决策。所有算法在 Rust 侧执行，Godot 侧仅接收渲染所需的数据。

---

## 2.1 情绪引擎

### 2.1.1 普拉奇克情绪轮

| 基本情绪 | 进化功能 | 对立情绪 | 低强度 | 中强度 | 高强度 |
|----------|---------|---------|--------|--------|--------|
| Joy | 趋近有益刺激 | Sadness | Serenity | Joy | Ecstasy |
| Trust | 建立社会联结 | Disgust | Acceptance | Trust | Admiration |
| Fear | 逃避威胁 | Anger | Apprehension | Fear | Terror |
| Surprise | 重新定向注意 | Anticipation | Distraction | Surprise | Amazement |
| Sadness | 保存能量 | Joy | Pensiveness | Sadness | Grief |
| Disgust | 排斥有害物质 | Trust | Boredom | Disgust | Loathing |
| Anger | 移除障碍 | Fear | Annoyance | Anger | Rage |
| Anticipation | 计划与预期 | Surprise | Interest | Anticipation | Vigilance |

### 2.1.2 三维轴映射

```rust
impl EmotionState {
    /// 从三维轴值计算 8 种基本情绪强度
    pub fn update_basic_emotions(&mut self) {
        // Joy: pleasure > 0 + arousal > 0.3
        self.basic_emotions.joy = sigmoid(self.pleasure, 0.3, 0.7)
            * sigmoid(self.arousal, 0.2, 0.5);

        // Anger: pleasure < -0.2 + arousal > 0.5 + control > 0.2
        self.basic_emotions.anger = sigmoid(-self.pleasure, 0.2, 0.6)
            * sigmoid(self.arousal, 0.5, 0.8)
            * sigmoid(self.control, 0.2, 0.5);

        // Fear: pleasure < -0.2 + arousal > 0.5 + control < -0.2
        self.basic_emotions.fear = sigmoid(-self.pleasure, 0.2, 0.6)
            * sigmoid(self.arousal, 0.5, 0.8)
            * sigmoid(-self.control, 0.2, 0.6);

        // Sadness: pleasure < -0.2 + arousal < 0.4
        self.basic_emotions.sadness = sigmoid(-self.pleasure, 0.2, 0.6)
            * (1.0 - sigmoid(self.arousal, 0.3, 0.5));

        // Trust: pleasure > 0.2 + arousal < 0.5 + control > 0.3
        self.basic_emotions.trust = sigmoid(self.pleasure, 0.2, 0.6)
            * (1.0 - sigmoid(self.arousal, 0.4, 0.6))
            * sigmoid(self.control, 0.3, 0.6);

        // ... Surprise, Disgust, Anticipation 类似
    }
}
```

### 2.1.3 33 种复合情绪标签

**第一层（相邻基本情绪组合）**：喜爱(Joy+Trust)、屈从(Trust+Fear)、敬畏(Fear+Surprise)、失望(Surprise+Sadness)、懊悔(Sadness+Disgust)、蔑视(Disgust+Anger)、攻击性(Anger+Anticipation)、乐观(Anticipation+Joy)

**第二层（隔位组合）**：内疚、好奇、绝望、震惊、怨恨、挖苦、胜利狂喜、希望

**第三层（三重复合）**：敌意、焦虑、羞耻、兴奋、自罪感、嫉妒、自豪、同情、尴尬、怀旧、无聊、困惑、决心、宽慰

**★ 第四层（吸引力相关——33 种）**：在以上 30 种基础上新增 3 种与吸引力/求爱相关的高阶复合情绪：

| 新增情绪 | 组成 | 触发条件 | 行为影响 |
|----------|------|---------|---------|
| **深爱** (DeepLove) | Joy + Trust + 高 arousal | attraction>0.78 + reciprocated | 长期陪伴愉悦↑、分离 sadness↑ |
| **单恋** (Longing) | Joy + Sadness + Anticipation | attraction>0.55 + 未 reciprocated | SeekProximity + 紧张、喜悦与焦虑并存 |
| **心碎** (Heartbreak) | Sadness + Surprise + low control | 被拒绝/关系突然结束 | sadness↑↑、社交回避、需恢复期 |

> 总计 **33 种复合情绪**。占有欲通过已有的"嫉妒"(第三层)+Anger 表达——不新增独立标签。

### 2.1.4 情绪动态

```rust
impl EmotionState {
    /// 情绪惯性 — 情绪非瞬间跳变，变化速率由觉醒度决定
    pub fn apply_inertia(
        &mut self,
        target_pleasure: f32,
        target_arousal: f32,
        target_control: f32,
        delta_time: Duration,
    ) {
        let speed = 0.1 + self.arousal * 0.3;  // 高觉醒 → 变化快
        let factor = 1.0 - (-speed * delta_time.as_secs_f32()).exp();

        self.pleasure += (target_pleasure - self.pleasure) * factor;
        self.arousal += (target_arousal - self.arousal) * factor;
        self.control += (target_control - self.control) * factor;
    }

    /// 激变 — 冲击力 >0.85 时惯性覆盖，情绪瞬间跳变
    pub fn apply_shock(&mut self, intensity: f32, direction: &EmotionDirection) {
        if intensity < 0.85 { return; }  // 低于阈值 → 走惯性通路

        match direction {
            EmotionDirection::Positive { pleasure_delta, arousal_delta, control_delta } => {
                self.pleasure = (self.pleasure + pleasure_delta).clamp(-1.0, 1.0);
                self.arousal = (self.arousal + arousal_delta).clamp(0.0, 1.0);
                self.control = (self.control + control_delta).clamp(-1.0, 1.0);
            }
            // ...
        }
    }

    /// 性格稳态点 — 情绪总是回归由人格定义的基线
    pub fn drift_to_baseline(&mut self, personality: &BigFive, delta_time: Duration) {
        let baseline_pleasure = 0.2 - personality.neuroticism * 0.4;    // 神经质 → 愉悦基线偏低
        let baseline_arousal = 0.3 + personality.extraversion * 0.3;     // 外向 → 觉醒基线偏高
        let baseline_control = personality.conscientiousness * 0.6 - 0.1; // 尽责 → 掌控基线偏高

        let drift_speed = 0.02;  // 每天约 2% 回归
        let factor = drift_speed * delta_time.as_secs_f32() / 86400.0;

        self.pleasure += (baseline_pleasure - self.pleasure) * factor;
        self.arousal += (baseline_arousal - self.arousal) * factor;
        self.control += (baseline_control - self.control) * factor;
    }

    /// 生理拉扯 — 身体状态影响情绪轴
    pub fn apply_physiological_pull(&mut self, phys: &Physiology) {
        if phys.hunger < 0.4 { self.pleasure -= 0.001 * (0.4 - phys.hunger); }
        if phys.fatigue > 0.7 { self.arousal -= 0.001 * phys.fatigue; }
        if phys.health < 0.5 { self.control -= 0.001 * (0.5 - phys.health); }
    }

    /// v3: 天象情绪影响 — 温和偏移，非决定
    pub fn apply_sky_influence(&mut self, sky: &SkyPerception, personality: &BigFive) {
        let neuroticism_mod = 0.5 + personality.neuroticism;  // 高神经质 ×1.5

        // 乌云 → 焦虑微升
        if sky.cloud_cover > 0.7 {
            self.pleasure -= 0.02 * neuroticism_mod;
            self.basic_emotions.fear += 0.01 * neuroticism_mod;
        }

        // 彩虹 → 愉悦微升
        if sky.rainbow_visible {
            self.pleasure += 0.03;
            self.basic_emotions.joy += 0.05;
        }

        // 暴风雨前兆 → 加速当前任务
        // (不在此处处理——在决策器中影响权重)
    }
}
```

### 2.1.5 情绪感染（8 重调节因子）

```rust
/// 情绪感染计算
pub fn calculate_emotional_contagion(
    expressor: &NpcData,
    observer: &NpcData,
    expressed_emotion: &EmotionState,
    distance: f32,
) -> f32 {
    if distance > 15.0 { return 0.0; }  // 仅 L1 视野内计算

    let mut strength = expressed_emotion.arousal * 0.4;  // 基础: 表达强度

    // 1. 关系亲密度
    let intimacy = observer.relationships
        .get(&expressor.identity.id)
        .map(|r| r.affection)
        .unwrap_or(0.0);
    strength *= 0.1 + intimacy * 0.9;  // 配偶 0.9-1.0 / 陌生人 0.1

    // 2. 社会地位梯度
    let status_diff = social_status_gradient(expressor, observer);
    strength *= if status_diff > 0.0 { 1.5 } else { 0.5 };

    // 3. 群体相似性
    let similarity = group_similarity(expressor, observer);
    strength *= 0.7 + similarity * 0.6;

    // 4. 观察者人格
    if observer.personality.agreeableness > 0.7 { strength *= 1.4; }
    if observer.personality.neuroticism > 0.7
        && expressed_emotion.pleasure < 0.0 { strength *= 1.3; }

    // 5. 先前情绪一致性
    if (observer.emotion.pleasure * expressed_emotion.pleasure) > 0.0 {
        strength *= 1.2;  // 同向 → 共振
    }

    // 6. 距离衰减
    strength *= 1.0 - (distance / 15.0).clamp(0.0, 1.0);

    strength.clamp(0.0, 1.0)
}
```

### 2.1.6 集体情绪

```rust
/// 集体情绪模型的四个阶段
#[derive(Debug, Clone)]
pub enum CollectiveEmotionPhase {
    /// 情绪级联: 高地位 → 亲信 → 群众逐层扩散
    Cascade { originator_id: NpcId, spread_radius: f32 },

    /// 情绪共振: 同质群体相互感染正反馈 (指数增长，直到饱和)
    Resonance { intensity: f32, affected_group_id: GroupId },

    /// 情绪极化: 群体情绪强度超个体 + 方向一致化 + 社会身份激活
    /// (极难逆转，只有重大反证事件或数周冷却才能消退)
    Polarization { intensity: f32, group_id: GroupId, direction: EmotionDirection },

    /// 情绪免疫: 低宜人性/对立身份/认知中介导致的冷静少数
    Immunity { immune_npc_ids: Vec<NpcId> },
}
```

---

## 2.2 记忆系统

### 2.2.1 编码时机

```rust
impl MemoryEncoder for DefaultMemoryEncoder {
    fn should_encode(
        &self,
        event_type: &EventType,
        impact_score: f32,
        npc: &NpcData,
    ) -> bool {
        match event_type {
            // 必然编码 — 无论冲击力多大
            EventType::Combat | EventType::Ceremony | EventType::Revelation
                | EventType::Nautical | EventType::Construction => true,

            // 阈值编码
            _ => impact_score > 0.1,
        }
    }
}
```

### 2.2.2 冲击力计算

```rust
pub fn calculate_impact(
    scope: f32,       // 影响范围 0-0.3
    consequence: f32,  // 后果深度 0-0.3
    moral_weight: f32, // 道德权重 0-0.2
    reversal: f32,     // 反转程度 0-0.2
) -> f32 {
    scope.clamp(0.0, 0.3)
        + consequence.clamp(0.0, 0.3)
        + moral_weight.clamp(0.0, 0.2)
        + reversal.clamp(0.0, 0.2)
}
```

### 2.2.3 四种记忆重构过滤器

| 过滤器 | 触发条件 | 重构方式 |
|--------|---------|---------|
| 自我辩护 | 掌控 <0 + 自身角色负面 + 尽责性 >0.4 | 逃跑→"战术撤退"; 攻击→"正当防卫"; 背叛→"别无选择" |
| 效价扭曲 | 始终启用 (强度随当前情绪) | 情绪好→正面细节夸大; 情绪差→负面细节放大 |
| 掌控感扭曲 | 掌控 <-0.3 或 >0.5 | 低掌控+成功→"全靠我"; 低掌控+失败→"谁都没办法" |
| 情绪一致性 | 检索时自动生效 | 当前情绪与记忆情绪越近 → 越优先检索 |

重构不可逆：深度重构（自我辩护）永久改写 `original_summary`。

### 2.2.4 记忆容量与衰减

```rust
impl MemoryStore {
    const CAPACITY: usize = 2000;

    /// 衰减因子: e^(-λ × 距今时间), λ 受冲击力、觉醒度、回忆频率修正
    pub fn decay_factor(
        impact_score: f32,
        arousal_at_encoding: f32,
        access_count: u32,
        days_since: f32,
    ) -> f32 {
        let lambda = 0.05
            / (impact_score.max(0.1))      // 高冲击 → 衰减慢
            * (1.0 - arousal_at_encoding * 0.5)  // 高觉醒时编码 → 衰减慢
            * (1.0 / (access_count as f32 + 1.0).sqrt()); // 常回忆 → 衰减慢

        (-lambda * days_since).exp()
    }

    /// 记忆压缩 — 衰减 <0.05 → 摘要化后移入冷记忆
    pub fn compress_cold_memories(&mut self) {
        let cold: Vec<_> = self.events.iter()
            .filter(|m| self.decay_factor(m.impact_score, 0.3, m.access_count,
                  (self.current_day - m.game_day) as f32) < 0.05
                  && !self.important_event_ids.contains(&m.id))
            .cloned()
            .collect();

        for mem in cold {
            self.events.retain(|m| m.id != mem.id);
            self.cold_memory_summaries.push(ColdMemorySummary {
                original_id: mem.id,
                summary: mem.summary.clone(),
                event_type: mem.event_type.clone(),
                compressed_at: self.current_day,
            });
        }
    }

    /// v3: 重要记忆优先保留
    pub fn mark_important(&mut self, id: MemoryId) {
        self.important_event_ids.insert(id);
    }

    /// v3: 情绪峰值事件自动标记为重要 (情绪变化 >0.3 → 写入记忆 + 标记重要)
    pub fn encode_emotional_event(
        &mut self,
        event_type: EventType,
        emotion_delta: f32,
        npc: &NpcData,
        event: &WorldEvent,
    ) {
        if emotion_delta.abs() > 0.3 {
            let memory = /* 编码记忆 */;
            let id = memory.id.clone();
            self.events.push(memory);
            self.mark_important(id);
        }
    }
}
```

### 2.2.5 社交传播衰减

```rust
/// 每次传播冲击力衰减 30%
/// 信任度修正: 高信任 → 衰减小 (0.85) / 低信任 → 衰减大 (0.5)
/// 传闻路线: 传播衰减系数 0.5, 航点逐次丢失 (每次 20%)
pub fn propagate_memory(
    memory: &EventMemory,
    teller: &NpcData,
    listener: &NpcData,
) -> Option<EventMemory> {
    let trust = listener.relationships
        .get(&teller.identity.id)
        .map(|r| r.trust)
        .unwrap_or(0.1);

    let decay = 0.7 * (0.5 + trust * 0.5);  // 信任修正
    let mut propagated = memory.clone();
    propagated.impact_score *= decay;
    propagated.id = new_memory_id();
    propagated.is_public = false;

    if propagated.impact_score > 0.05 { Some(propagated) } else { None }
}
```

---

## 2.3 行为决策

### 2.3.1 概率决策器

```rust
/// 覆盖 ~90% 的日常行为
pub struct ProbabilisticDecisionEngine {
    /// 文化习俗权重 — 从团体数据缓存，每游戏月更新
    cultural_weights: BTreeMap<ActionId, f32>,

    /// 个人习惯权重 — 行为完成后累积更新
    /// ★ 替换原 personal_weights，完整规格见 §2.3.5
    habits: HabitSystem,

    /// 滞后区间 — 目标确定后最小执行时间
    min_execution_time: Duration,  // ≥5s (017 测试 P2 #7 验证)

    /// 切换阈值 — 需求差值 >0.15 才切换目标
    switch_threshold: f32,
}

impl DecisionEngine for ProbabilisticDecisionEngine {
    fn select_action(
        &self,
        npc: &NpcData,
        world: &SubjectiveWorld,
        rng: &mut impl Rng,
    ) -> Action {
        let mut weighted_actions: Vec<(Action, f32)> = self.available_actions(npc, world)
            .into_iter()
            .map(|action| {
                let weight =
                    self.cultural_weight(&action)      // 文化习俗
                    * self.habit_weight(&action, npc)  // 个人习惯（HabitSystem）
                    * self.personality_modifier(&action, &npc.personality) // 人格偏移
                    * self.emotion_modifier(&action, &npc.emotion)         // 情绪修正
                    * self.mood_modifier(&action, &npc.emotion.mood)       // ★ 心境修正（独立于情绪）
                    * self.physiology_modifier(&action, &npc.physiology)   // 生理需求
                    * self.intrinsic_motivation_weight(&action, npc) // ★ 内在驱动（SelfNarrative）
                    * self.time_modifier(&action, world)   // v3: 昼夜修正
                    * self.weather_modifier(&action, world) // v3: 天气修正
                    * self.sky_modifier(&action, &npc.sky_perception) // v3: 天象修正
                    * self.vehicle_modifier(&action, &npc.vehicle_state); // v3: 载具修正
                (action, weight)
            })
            .collect();

        // 加权随机采样
        weighted_sample(&mut weighted_actions, rng)
    }
}

/// 情绪→行动权重修正速查
const EMOTION_ACTION_MODIFIERS: [(EmotionRef, ActionCategory, f32); 48] = [
    // (情绪, 行动类别, 修正系数)
    (EmotionRef::Joy,        ActionCategory::FriendlySocial, 1.4),
    (EmotionRef::Joy,        ActionCategory::PhysicalAttack, 0.1),
    (EmotionRef::Trust,      ActionCategory::FriendlySocial, 1.6),
    (EmotionRef::Trust,      ActionCategory::PhysicalAttack, 0.1),
    (EmotionRef::Fear,       ActionCategory::Escape,         2.0),
    (EmotionRef::Fear,       ActionCategory::FriendlySocial, 0.3),
    (EmotionRef::Anger,      ActionCategory::PhysicalAttack, 1.9),
    (EmotionRef::Anger,      ActionCategory::FriendlySocial, 0.2),
    (EmotionRef::Anticipation, ActionCategory::Explore,      1.8),
    (EmotionRef::Sadness,    ActionCategory::SeekComfort,    1.7),
    (EmotionRef::Sadness,    ActionCategory::Solitude,       1.8),
    // ... 完整 48 条
];

// v3: 天象→行动权重修正
const SKY_ACTION_MODIFIERS: [(SkyCondition, ActionCategory, f32); 8] = [
    (SkyCondition::Overcast,     ActionCategory::OutdoorActivity, 0.85),
    (SkyCondition::Rainbow,      ActionCategory::OutdoorActivity, 1.1),
    (SkyCondition::StormWarning, ActionCategory::OutdoorActivity, 0.5),
    (SkyCondition::StormWarning, ActionCategory::UrgentHarvest,   2.0),
    (SkyCondition::ClearSky,     ActionCategory::OutdoorActivity, 1.05),
    // ...
];

// v3: 载具→行动权重修正
const VEHICLE_ACTION_MODIFIERS: [(VehicleCondition, ActionCategory, f32); 6] = [
    (VehicleCondition::OnShipDeck,     ActionCategory::OutdoorSocial,  1.3),
    (VehicleCondition::OnShipDeck,     ActionCategory::Sleep,          0.7),
    (VehicleCondition::InTrainCabin,   ActionCategory::IndoorSocial,   1.5),
    (VehicleCondition::InTrainCabin,   ActionCategory::Solitude,       0.3),
    (VehicleCondition::CrewOnDuty,     ActionCategory::NavigationTask, 2.0),
    (VehicleCondition::CrewOnDuty,     ActionCategory::Social,         0.1),
];
```

### 2.3.2 GOAP 安全网

```rust
/// 触发条件 — 仅当生存/重大危机时激活 (~9% 决策)
impl GoapPlanner {
    pub fn should_activate(npc: &NpcData) -> Option<Goal> {
        if npc.physiology.hunger < 0.3  { return Some(Goal::SurvivalEat); }
        if npc.physiology.thirst < 0.25 { return Some(Goal::SurvivalDrink); }
        if npc.physiology.fatigue > 0.9 { return Some(Goal::SurvivalSleep); }
        if npc.physiology.health < 0.3  { return Some(Goal::SurvivalHeal); }
        if npc.current_state.combat_threat > 0.7 {
            return Some(Goal::SurvivalDefend);
        }
        // v3: 载具紧急情况
        if let Some(ref vs) = npc.vehicle_state {
            if vs.role == VehicleRole::Crew { rank: _ } && npc.current_state.ship_damage > 0.7 {
                return Some(Goal::VehicleEmergencyRepair);
            }
        }
        // ★ 求爱目标触发（02-性别与吸引力系统.md §5.3）
        // 无伴侣 + 适婚年龄 + 文化婚姻压力 → FindPartner
        if npc.is_eligible_for_pairing()
            && npc.cultural_marriage_pressure() > 0.3
        {
            return Some(Goal::FindPartner {
                min_attraction: 0.55,
                desired_structure: npc.attraction_template.relationship_preference.preferred_structure.clone(),
                urgency: npc.cultural_marriage_pressure(),
            });
        }
        // 已有 crush + attraction>0.55 + 未 Bonded → PursueRomanticInterest
        if let Some(target_id) = npc.highest_attraction_target() {
            if let Some(rel) = npc.relationships.get(&target_id) {
                if rel.attraction > 0.55
                    && !matches!(rel.courtship_state, Some(CourtshipState::Bonded { .. }))
                {
                    return Some(Goal::PursueRomanticInterest {
                        target_id,
                        urgency: rel.attraction,
                    });
                }
            }
        }
        None
    }

    /// Plan B 生成 — 目标不可达时的备选策略
    /// (017 测试 P2 #6 验证: 无 Plan B → NPC 行为退化为随机游荡)
    pub fn fallback_plan(
        goal: &Goal,
        npc: &NpcData,
        world: &SubjectiveWorld,
    ) -> Vec<Action> {
        match goal {
            Goal::SurvivalEat => vec![
                Action::ExpandSearchRadius(2.0),        // 扩大搜索
                Action::AskForFood(npc.nearest_social_contact(world)), // 求助
                Action::LowerNeedThreshold(0.15),       // 忍一忍
                Action::CraftFood,                      // 自己做/采集
                Action::FightForResource,               // 争夺
            ],
            // ★ 求爱目标 Plan B（02-性别与吸引力系统.md §5.3）
            Goal::FindPartner { .. } => vec![
                Action::ExpandSocialCircle,             // 扩大社交圈
                Action::TravelToNearestSettlement,      // 去更大的聚落
                Action::ImproveAppearance,              // 打理自己
                Action::AskFriendForIntroduction,       // 请朋友介绍
            ],
            Goal::PursueRomanticInterest { target_id, .. } => vec![
                Action::SeekProximity(target_id),       // 制造偶遇
                Action::DisplaySkill { audience: target_id }, // 展示能力
                Action::GiveGift(target_id),            // 送礼
                Action::AcceptRejection,                // 接受现实——放手
            ],
            // ... 其他 Goal 的 Plan B
        }
    }

    /// 规划超时: ≤2ms (018 v3 性能预算)
    const PLANNING_TIMEOUT_MS: u64 = 2;
}
```

### 2.3.3 LLM 增强（可选）

```rust
/// LLM 仅在以下条件全部满足时调用:
/// 1. L1 NPC (距离玩家 ≤50m)
/// 2. 玩家主动交互
/// 3. 复杂社交场景 (超过模板覆盖范围)
/// 4. NPC 冷却已过 (每游戏日 ≤1 次)
/// 5. 玩家 token 预算未耗尽
///
/// LLM 输出约束:
/// - 仅从 ActionLibrary (预定义行动库) 中选择行动 ID
/// - 安全网关验证: 行动 ID ∈ 允许集合 + 目标实体存在 + 不违反 NPC 核心人格
/// - 返回结构: { action_id: String, target_id: Option<String>, dialogue_text: String }
pub struct LlmEnhancer {
    api_config: LlmApiConfig,          // 玩家自备 API Key
    rate_limiter: RateLimiter,         // 异步队列, 每游戏日 ≤1 次/NPC
    action_library: ActionLibrary,     // 预定义行动库 (不允许自由发挥)
    safety_gateway: SafetyGateway,     // 实体过滤器 + 行动边界检查
    cache: LruCache<String, DialogueResponse>,  // 相似上下文复用
}
```

---

## 2.4 关系系统

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relationship {
    pub affection: f32,      // 好感度 -1~1 (短期波动, 天级)
    pub trust: f32,          // 信任度 -1~1 (长期积累, 年级)
    pub familiarity: f32,    // 熟悉度 0~1 (基于互动次数和质量)
    pub attraction: f32,     // ★ 吸引力 0~1（详见 02-性别与吸引力系统.md §4.2）
    pub attraction_type: AttractionType, // ★ 吸引力类型
    pub status: StatusRelation, // 支配/服从/平等

    /// v3: 关系来源 (血缘/婚姻/工作/社交偶遇 → 影响关系变化速率)
    pub source: RelationshipSource,

    pub last_interaction: GameDay,
    pub total_interactions: u32,

    /// ★ 求爱状态（详见 02-性别与吸引力系统.md §6.1）
    pub courtship_state: Option<CourtshipState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatusRelation {
    Dominant { power_differential: f32 },
    Submissive { power_differential: f32 },
    Equal,
}

/// v3: 关系来源
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RelationshipSource {
    Bloodline,           // 血缘 → 信任度高, 变化慢
    Marriage,            // 婚姻 → 信任度高, 易受伤 (背叛冲击力 ×2)
    Partnership,         // ★ 非婚姻伴侣关系（详见 02-性别与吸引力系统.md §6.1.4）
    Coworker,            // 同事 → 熟悉度高, 情感浅
    Friendship,          // 朋友 → 双向
    Acquaintance,        // 泛泛之交
    SharedTrauma,        // v3: 共同经历 (一起航海/战斗/施工) → 信任加速
}

/// 信任度的独特时间动态 (017 测试 P4 #3 验证):
/// - 好感度: 短期波动, 受最近事件影响, 随时间微衰减
/// - 信任度: 长期积累, 如果关系没有被负面事件破坏, 时间本身会加深信任
/// - 背叛: 破坏信任比降低好感度更严重 (信任恢复慢 ~10 倍)
impl Relationship {
    pub fn decay_over_time(&mut self, days_since_last_interaction: f32) {
        // 好感度 — 天级衰减
        self.affection *= (-0.01 * days_since_last_interaction).exp();

        // 信任度 — 如果没有负面事件, 时间反而增加信任 ("认识得久=更可信")
        if self.affection > 0.0 && days_since_last_interaction > 30.0 {
            self.trust = (self.trust + 0.001 * days_since_last_interaction.sqrt()).min(1.0);
        }

        // 熟悉度 — 不衰减 (经历过的不会"变得不熟悉")
    }
}
```

---

## 2.5 社会认知与身份

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialIdentity {
    /// 自我标签: ["铁匠", "父亲", "猎户", "酒鬼"]
    /// ★ 自我标签——从 SelfNarrative.self_concept 派生（缓存视图）
    /// 实际权威来源: npc.self_narrative.self_concept
    pub self_labels: Vec<String>,

    /// 感知到的社会地位 (通过社会比较动态更新)
    pub perceived_status: f32,   // -1~1

    /// 团体角色
    pub group_roles: BTreeMap<GroupId, GroupRole>,

    /// 文化身份标签
    pub cultural_identity: CulturalIdentity,

    /// v3: 职业身份 (影响施工/载具角色分配)
    pub professional_identity: ProfessionalIdentity,
}

/// 偏见模型: 内群体偏好 + 外群体歧视
pub fn social_bias(observer: &NpcData, target: &NpcData) -> f32 {
    let in_group = observer.factions.iter()
        .any(|f| target.factions.contains(f));
    if in_group { 1.2 } else { 0.7 }  // 内群体 ×1.2 / 外群体 ×0.7
}

/// 支配/服从互动: 权力结构从个体冲突中统计涌现
pub fn update_dominance(
    winner: &mut NpcData,
    loser: &mut NpcData,
    contest_type: ContestType,
) {
    // 胜利方 → 支配地位 +Δ
    // 失败方 → 服从地位 -Δ
    // 多次交锋 → 地位差距固化
}
```

---

## 2.6 文化系统

```rust
/// 三层文化结构 — 从行为统计到深层信念
pub struct CulturalSystem {
    /// 表层: 行为习俗 ← 行为频率统计, 每游戏月更新
    pub customs: CulturalCustoms,

    /// 中层: 规范与禁忌 ← 社会反馈积累 (赞许/惩戒计数)
    pub norms: CulturalNorms,

    /// 深层: 价值观与信念 ← 缓慢结晶/代际漂移
    pub values: CulturalValues,
}

/// 文化动态:
/// - 群体隔离 → 分化 (习俗漂移)
/// - 群体接触 → 融合 (中间文化涌现)
/// - 文化传播速率 = f(贸易量, 婚姻率, 征服关系, 传教活动)

/// ★ 中层规范的具体结构（02-性别与吸引力系统.md §8.2）
/// CulturalNorms = Vec<CulturalNorm>
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CulturalNorm {
    pub id: NormId,
    pub description: String,
    pub scope: NormScope,               // 规范约束的对象范围
    pub severity_of_violation: f32,     // 违反的严重程度 0-1
    pub social_feedback_count: SocialFeedbackCount, // 赞许/惩戒计数
}

/// 规范的作用范围——性别只是其中一维
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NormScope {
    Everyone,
    SpecificGender(BiologicalSex),      // ★ 性别相关的规范
    SpecificAge { min: f32, max: f32 },
    SpecificProfession(ProfessionId),
    SpecificSocialClass(SocialClass),
    Intersection(Vec<NormScope>),       // 交叉约束: "女性+贵族"
}
```

---

## 2.7 空间认知与导航

```rust
/// 五级道路模型
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PathType {
    HardenedRoad,    // 1级: 世界生成/大型工程 → 物理道路纹理
    DirtPath,        // 2级: 高频使用踩出 → 路径贴花
    MemoryRoute,     // 3级: 个人走过记住 → 仅存认知地图
    RumorRoute,      // 4级: 他人告知 → 仅存认知地图 (航点精度低)
    MagicPath,       // 5级: 魔法/特殊能力 → 特殊视觉效果
}

/// 导航策略: 认知优先 → 局部烘焙 → 体素 A*
/// (017 测试验证: 认知优先命中率 > 80%)
pub struct NavigationSystem {
    /// 认知地图 — NPC 在熟悉区域走"记忆路线"
    cognitive_map: CognitiveMap,

    /// 局部烘焙 — 不熟悉区域 → 100m 半径 NavMesh A*
    local_navmesh: Option<NavMesh>,

    /// 体素 A* — L3/L4 NPC → 粗粒度可通行性网格
    /// (每日/周运行一次, 不要求实时)
    voxel_astar: VoxelAStar,

    /// v3: 高速移动导航 — >30 m/s 时自动切换粗粒度 (2m 网格替代 0.5m)
    high_speed_grid: Option<CoarseGrid>,
}

/// v3: 长途旅行 — 主干道上的动态旅行者 NPC
/// 旅行者密度 = f(道路等级, 当地治安, 经济繁荣度)
pub fn traveler_density(
    road_level: u8,
    security_index: f32,
    prosperity_index: f32,
) -> f32 {
    let base = match road_level {
        1 => 0.15,  // 硬化路 — 高频使用
        2 => 0.05,  // 土径
        _ => 0.01,  // 野路
    };
    base * security_index * (0.5 + prosperity_index * 0.5)
}

/// v3: 城市功能分区 — 对 NPC 空间行为的影响
#[derive(Debug, Clone)]
pub struct UrbanDistrict {
    pub district_type: DistrictType,
    pub activity_multipliers: BTreeMap<ActionCategory, f32>,
    pub typical_npc_density: f32,
    pub noise_level: f32,  // 影响感官 — 市场嘈杂, 富人区安静
}

#[derive(Debug, Clone)]
pub enum DistrictType {
    MarketDistrict,     // 交易权重 ×1.5
    CraftDistrict,      // 生产权重 ×1.5
    ReligiousDistrict,  // 仪式权重 ×2.0
    ResidentialRich,    // 安静, NPC 密度低
    ResidentialPoor,    // 嘈杂, NPC 密度高
    PortDistrict,       // v3: 港口区 — 装卸/交易/航海行为
    StationDistrict,    // v3: 车站区 — 火车/载具行为
    AdministrativeCenter,
}
```

---

## 2.8 自我叙事引擎（SelfNarrative）★ 新增

> **设计意图**: 取代原 `LongTermPlanning` 占位符字段（从未定义）。NPC 不只是行事——它反思自己做过的事，从记忆归纳出"我是谁"、"我的人生在往哪里走"。这是从"行为模拟"到"存在模拟"的关键跨越。
>
> **核心理念**: 开发者不写任何生命故事。只定义反思算法和归纳规则。具体每个 NPC 的自我认知——全是涌现。

### 2.8.1 数据结构

```rust
/// 自我叙事引擎——NPC 对自我的持续归纳
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfNarrative {
    /// 自我标签——每个标签带置信度和来源
    pub self_concept: Vec<SelfLabel>,

    /// 生命章节——从记忆摘要中归纳
    pub life_chapters: Vec<LifeChapter>,

    /// 核心价值观——从反复出现的正/负面经历中结晶
    pub core_values: Vec<CoreValue>,

    /// 当前人生阶段的主题
    pub current_life_theme: LifeTheme,

    /// 长期期望——"我想成为什么样的人"
    pub aspirations: Vec<Aspiration>,

    /// 上次反思的游戏日
    pub last_reflection: GameDay,

    /// 停滞感——替代原 boredom_accumulated
    /// 0=人生充满意义和新体验, 1=极度停滞/无聊
    pub stagnation_sense: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfLabel {
    pub label: String,           // "铁匠"、"母亲"、"幸存者"
    pub confidence: f32,         // 0-1
    pub source: LabelSource,
    pub valence: f32,            // -1(负面自我认知) ~ 1(正面)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LabelSource {
    SelfAttributed,                           // "我认为我是..."
    OtherAttributed { source_npc: Option<NpcId> }, // "别人说我是..."
    EventDriven { event_id: MemoryId },       // 某次经历后形成
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifeChapter {
    pub title: String,
    pub start_day: GameDay,
    pub end_day: Option<GameDay>,
    pub dominant_emotion: f32,   // 该时期的平均愉悦度
    pub key_events: Vec<MemoryId>,
    pub significance: f32,       // 对"我是谁"的重要程度 0-1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreValue {
    pub value: String,           // "诚实"、"自由"、"忠诚"、"安全"
    pub strength: f32,           // 0-1
    pub source_experiences: Vec<MemoryId>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LifeTheme {
    Exploration,     // 探索——尝试新方向，未定型
    Building,        // 建设——建立事业/家庭
    Mastery,         // 精进——深耕已有方向
    Crisis,          // 危机——重大变故后的重新评估期
    Decline,         // 衰退——老年期，回顾多于计划
    Legacy,          // 遗产——关注身后留下什么
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aspiration {
    pub description: String,     // "成为镇上最好的铁匠"
    pub category: AspirationCategory,
    pub progress: f32,           // 0-1
    pub priority: f32,           // 0-1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AspirationCategory {
    SkillMastery,    // 掌握一项技能
    SocialStanding,  // 获得社会地位
    Creation,        // 创造某物（建筑/艺术品/著作）
    Relationship,    // 建立/修复重要关系
    Knowledge,       // 理解某件事
    Legacy,          // 留下持久的影响
}
```

### 2.8.2 反思周期

每 7 游戏日（或重大事件冲击力 >0.8 后立即触发），自我叙事引擎运行一次：

```rust
impl SelfNarrative {
    /// 周期性自我反思——将近期经历编织进自我认知
    pub fn reflect(&mut self, npc: &NpcData, current_day: GameDay) {
        // 1. 停滞检测——近期有多少 meaningful 新记忆?
        let recent_count = npc.memory.events.iter()
            .filter(|m| m.game_day.days_since(current_day) < 30.0)
            .filter(|m| m.impact_score > 0.2)
            .count();
        self.stagnation_sense = if recent_count < 2 {
            (self.stagnation_sense + 0.1).min(1.0)    // 生活一成不变 → 停滞感上升
        } else {
            (self.stagnation_sense - 0.15).max(0.0)   // 有新鲜事 → 停滞感下降
        };

        // 2. 自我标签更新
        //   - 从近期记忆中提取重复出现的角色/行为 → 强化相关 SelfLabel
        //   - 长时间未出现的标签 → confidence 缓慢衰减
        //   - high impact events → 可能新增或移除标签
        //   - 摄入 CombatNarrativeDetector 产生的 ProposedSelfLabelChange

        // 3. 生命章节检测
        //   - stagnation_sense > 0.7 持续 >30 天 + no new chapters recently
        //     → 可能开启新章节（"我需要改变点什么"）
        //   - 冲击力 >0.8 的事件 → 自动开启新章节
        //   - 每 365 游戏日 → 自然章节回顾

        // 4. 核心价值观结晶
        //   - 检索所有正面高冲击力记忆 → 统计关联的情感/情境
        //   - "每次我做X都感到骄傲" → X 相关的价值观强化
        //   - 长期未验证的价值观 → strength 衰减

        // 5. Aspiration 生成与更新
        //   - stagnation_sense > 0.6 + 高开放性 → ExploreUnknown / MasterSkill
        //   - 高尽责性 + Building 主题 → CreateSomething
        //   - 中年 + 未实现的 aspiration → 可能触发 Crisis 主题（"中年危机"）
        //   - 高宜人性 → DeepenConnection / SocialStanding
        //   - 老年 / 濒死经历 → LeaveLegacy

        // 6. LifeTheme 转换
        match self.current_life_theme {
            LifeTheme::Exploration => {
                if self.aspirations.iter().any(|a| a.progress > 0.5) {
                    self.current_life_theme = LifeTheme::Building;
                }
            }
            LifeTheme::Building | LifeTheme::Mastery => {
                if self.stagnation_sense > 0.7 && npc.identity.age > npc.adult_age() * 1.5 {
                    self.current_life_theme = LifeTheme::Crisis;
                }
            }
            LifeTheme::Crisis => {
                // Crisis 结束后 → 根据人格决定下一个主题
                if npc.personality.openness > 0.6 {
                    self.current_life_theme = LifeTheme::Exploration;
                } else {
                    self.current_life_theme = LifeTheme::Mastery;
                }
            }
            LifeTheme::Decline => {
                if npc.identity.age > npc.max_lifespan() * 0.85 {
                    self.current_life_theme = LifeTheme::Legacy;
                }
            }
            _ => {}
        }

        // LifeTheme 触发条件:
        //   age > 70% 最大寿命 + body decline >20% → Decline
        //   age > 85% 或 closeness_to_death >0.8 → Legacy

        self.last_reflection = current_day;
    }
}
```

### 2.8.3 内在驱动目标

> 扩展 GOAP——从纯粹匮乏驱动到意义驱动。内在目标进入概率决策器（`intrinsic_motivation_weight` 乘数），**不走** GOAP 安全网——它们是"想要"而非"必须"。

```rust
/// 内在驱动目标——从 SelfNarrative 生成
#[derive(Debug, Clone)]
pub enum IntrinsicGoal {
    /// 好奇心——探索未知
    ExploreUnknown {
        target_type: ExplorationTarget,
        urgency: f32,
    },
    /// 能力感——精进一项技能
    MasterSkill {
        skill_id: SkillId,
        target_level: f32,
    },
    /// 自主性——做出与自我叙事一致的决定
    ActOnValue {
        value: String,
    },
    /// 创造——留下自己的痕迹
    CreateSomething {
        creation_type: CreationType,
    },
    /// 连接——深化/修复重要关系
    DeepenConnection {
        target_id: NpcId,
        desired_depth: f32,
    },
    /// 传承——为身后做准备
    LeaveLegacy {
        legacy_type: LegacyType,
    },
}

/// 从 SelfNarrative 生成内在目标——每 7 游戏日调用一次
fn generate_intrinsic_goals(narrative: &SelfNarrative, npc: &NpcData) -> Vec<IntrinsicGoal> {
    let mut goals = Vec::new();

    // 停滞驱动——"生活需要改变"
    if narrative.stagnation_sense > 0.6 {
        match npc.personality.openness {
            o if o > 0.6 => goals.push(IntrinsicGoal::ExploreUnknown {
                target_type: ExplorationTarget::NearestUnexplored,
                urgency: narrative.stagnation_sense,
            }),
            _ => goals.push(IntrinsicGoal::MasterSkill {
                skill_id: npc.highest_skill_id(),
                target_level: (npc.highest_skill_level() + 0.2).min(1.0),
            }),
        }
    }

    // Aspiration 驱动——"我在成为我想成为的人吗?"
    for aspiration in &narrative.aspirations {
        if aspiration.progress < 0.5 && aspiration.priority > 0.5 {
            goals.push(aspiration_to_intrinsic_goal(aspiration));
        }
    }

    // Legacy 驱动——"我死后留下什么?"
    if narrative.current_life_theme == LifeTheme::Legacy {
        goals.push(IntrinsicGoal::LeaveLegacy {
            legacy_type: LegacyType::TeachApprentice,
        });
    }

    goals
}
```

### 2.8.4 习惯系统（HabitSystem）

> 替换原 `personal_weights: BTreeMap<ActionId, f32>`（有声明无算法）。赋予习惯完整的生命周期。

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HabitSystem {
    pub habits: BTreeMap<ActionId, HabitEntry>,
    pub routines: Vec<Routine>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HabitEntry {
    pub weight: f32,                // 当前权重 0-1
    pub repetition_count: u32,      // 累计执行次数
    pub last_performed: GameDay,
    pub satisfaction_history: f32,  // EMA, -1~1
    pub is_automatic: bool,         // >30次 + weight>0.6 → 自动化
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Routine {
    pub actions: Vec<ActionId>,
    pub typical_time_of_day: f32,    // 0-1 (0=日出, 1=日落)
    pub typical_location: LocationId,
    pub consistency: f32,            // 这个 routine 有多固定 0-1
    pub disruption_tolerance: f32,   // 偏离 routine 时的不适感 0-1
}

impl HabitSystem {
    /// 行为完成后调用——加强习惯
    pub fn reinforce(&mut self, action: &Action, satisfaction: f32, day: GameDay) {
        let entry = self.habits.entry(action.id).or_insert(HabitEntry {
            weight: 0.1,
            repetition_count: 0,
            last_performed: day,
            satisfaction_history: 0.0,
            is_automatic: false,
        });
        entry.repetition_count += 1;
        entry.last_performed = day;
        entry.weight = (entry.weight + 0.02 * satisfaction).min(1.0);
        entry.satisfaction_history =
            entry.satisfaction_history * 0.9 + satisfaction * 0.1; // EMA
        if entry.repetition_count > 30 && entry.weight > 0.6 {
            entry.is_automatic = true;
        }
    }

    /// 习惯衰减——未执行时的不适
    pub fn decay(&mut self, action_id: &ActionId, days_since: f32) -> Option<f32> {
        let entry = self.habits.get_mut(action_id)?;
        if entry.is_automatic && days_since > 3.0 {
            entry.weight -= 0.01 * days_since;
            return Some(0.05 * days_since);  // 不适感——影响心境
        }
        None
    }

    /// 用于决策器的权重查询
    pub fn habit_weight(&self, action: &Action, _npc: &NpcData) -> f32 {
        self.habits.get(&action.id)
            .map(|e| 0.5 + e.weight * 0.5)  // 中性基线 0.5，习惯在 0.5-1.0 之间调节
            .unwrap_or(0.5)                   // 未曾做过的行为 = 中性权重
    }
}
```

### 2.8.5 心境与情绪的交互

在 §2.1 情绪动态中新增心境相关方法：

```rust
impl EmotionState {
    /// ★ 心境累积——情绪→心境（天到周尺度）
    /// 每天调用一次
    pub fn accumulate_mood(&mut self, personality: &BigFive) {
        // 情绪向心境的缓慢传递
        self.mood.mood_pleasure += (self.pleasure - self.mood.mood_pleasure) * 0.05;
        self.mood.mood_arousal += (self.arousal - self.mood.mood_arousal) * 0.05;
        self.mood.mood_control += (self.control - self.mood.mood_control) * 0.05;

        // 心境惯性——比情绪慢得多
        // 情绪变化速度: 秒到小时
        // 心境变化速度: 天到周

        // 人格对心境漂移的影响
        let mood_baseline_pleasure = 0.2 - personality.neuroticism * 0.4;
        self.mood.mood_pleasure += (mood_baseline_pleasure - self.mood.mood_pleasure) * 0.01;

        self.mood.days_since_last_shift += 1.0;

        // 心境标签更新
        self.mood.mood_label = Self::classify_mood(&self.mood);
    }

    /// ★ 心境对情绪的偏置——"有色眼镜"效应
    pub fn apply_mood_bias(&mut self, event_pleasure: f32) -> f32 {
        if self.mood.mood_pleasure > 0.5 {
            event_pleasure * 1.2   // 好心境 → 放大正面体验
        } else if self.mood.mood_pleasure < -0.3 {
            event_pleasure * 0.8   // 坏心境 → 削弱正面体验
        } else {
            event_pleasure
        }
    }

    fn classify_mood(mood: &MoodState) -> Option<MoodLabel> {
        match (mood.mood_pleasure, mood.mood_arousal) {
            (p, a) if p > 0.5 && a > 0.5 => Some(MoodLabel::Euphoric),
            (p, a) if p > 0.3 && a > 0.3 => Some(MoodLabel::Cheerful),
            (p, _) if p > 0.3 && mood.mood_control > 0.3 => Some(MoodLabel::Serene),
            (p, a) if p < -0.5 && a > 0.5 => Some(MoodLabel::Despondent),
            (p, _) if p < -0.3 && a < 0.3 => Some(MoodLabel::Melancholic),
            (p, a) if p < -0.2 && a > 0.5 => Some(MoodLabel::Irritable),
            (_, a) if a < 0.2 => Some(MoodLabel::Apathetic),
            (p, a) if p < -0.3 && a > 0.3 => Some(MoodLabel::Anxious),
            _ => None,
        }
    }
}

// ★ 心境→决策权重修正（在 ProbabilisticDecisionEngine 中）
const MOOD_ACTION_MODIFIERS: [(MoodLabel, ActionCategory, f32); 8] = [
    (MoodLabel::Melancholic,  ActionCategory::Solitude,       1.3),
    (MoodLabel::Melancholic,  ActionCategory::FriendlySocial, 0.7),
    (MoodLabel::Irritable,    ActionCategory::PhysicalAttack, 1.2),
    (MoodLabel::Irritable,    ActionCategory::Cooperation,    0.8),
    (MoodLabel::Serene,       ActionCategory::Creative,       1.2),
    (MoodLabel::Anxious,      ActionCategory::Escape,         1.3),
    (MoodLabel::Euphoric,     ActionCategory::FriendlySocial, 1.4),
    (MoodLabel::Apathetic,    ActionCategory::Explore,        0.5),
];
```

### 2.8.6 预期/恐惧——未来事件着色心境

> 无需新增存储——动态计算。每周从 GOAP 计划和记忆中检测即将发生的重要事件。

```rust
/// 预期状态——每周计算一次，O(10)
pub fn compute_anticipation(npc: &NpcData) -> AnticipationEffect {
    let mut near_future_valence = 0.0;
    let mut count = 0;

    // 从记忆中检索未来 7 天内即将发生的事件
    // (如已安排的婚礼、预期的战斗、计划中的旅行)
    for memory in &npc.memory.events {
        if memory.tags.contains(&"upcoming".into()) {
            let days_until = memory.expected_occurrence_day().days_until();
            if days_until < 7.0 {
                near_future_valence += memory.emotional_encoding.valence
                    / (1.0 + days_until);  // 越近 → 影响越大
                count += 1;
            }
        }
    }

    AnticipationEffect {
        near_future_valence: if count > 0 {
            (near_future_valence / count as f32).clamp(-1.0, 1.0)
        } else { 0.0 },
        // 注入心境: mood_pleasure += near_future_valence * 0.1
    }
}

pub struct AnticipationEffect {
    pub near_future_valence: f32, // -1(恐惧) ~ 1(期待)
}
```

---

# Part 3: 分层模拟

> **设计意图**：100K+ NPC 不可能全精度模拟。四层 LOD (L1→L4) + 具身化生命周期 + 升降级机制使这可行。调度器在 Rust 侧运行，Godot 侧仅接收渲染队列。

---

## 3.1 四层模拟

| 维度 | L1 全模拟 | L2 轻量 | L3 统计 | **L4 超远距 ★** |
|------|----------|---------|---------|----------------|
| 数量上限 | ≤1,000 | ≤10,000 | ~89,000 | **~数百万** |
| 距离玩家 | ≤50m | 50-150m | >150m | **>5km** |
| 3D 节点 | 完整 | 简化 | 无 | **无** |
| 情绪引擎 | 完整 | 仅衰减收敛 | 无 | **无** |
| 记忆系统 | 完整读写 | 只读 | 无 | **无** |
| 行为决策 | 概率+GOAP | 简化状态机 | 无(宏观推演) | **无(人口/经济/文化指标批量)** |
| 社会认知 | 完整 | 休眠 | 无 | **无** |
| 更新频率 | 每 0.2s | 每 1-2s | 每日/周 | **每周/月** |
| 渲染 | 完整 | 简化 | 零 | **零 (仅城市灯光/炊烟存在感)** |

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimulationLevel {
    L1,  // 全模拟
    L2,  // 轻量
    L3,  // 统计
    L4,  // v3: 超远距 — 仅宏观指标, 无个体实例
}

/// L4 NPC 不分配个别的 NpcData — 仅存储宏观统计:
/// { region_id: { population: u32, avg_prosperity: f32, culture_seed: u64, dominant_faction: FactionId } }
```

## 3.2 具身化生命周期

```
GHOST (纯数据, 无坐标) → ANCHORED (分配坐标, 等待地形生成)
  → MANIFESTING (节点实例化, 无碰撞) → EMBODIED (完全物理实体)
```

降级逆序: EMBODIED → MANIFESTING → ANCHORED → GHOST

```rust
#[derive(Debug, Clone)]
pub enum EmbodimentState {
    Ghost,                          // 纯数据
    Anchored { position: Vec3 },    // 有坐标, 等地形
    Manifesting { node: RID },     // 3D 节点就位, 等碰撞烘焙
    Embodied { node: RID },        // 完全物理实体
}
```

## 3.3 升降级调度器

```rust
pub struct SimulationScheduler {
    l1_capacity: usize,    // ≤1,000
    l2_capacity: usize,    // ≤10,000
    player_positions: Vec<Vec3>,  // 玩家 + 同伴位置

    // 滞后区间 — 防止边界抖动 (017 测试确认)
    upgrade_distance: f32,  // 50m (L2→L1)
    downgrade_distance: f32, // 55m (L1→L2)
    min_level_duration: Duration, // 3s

    // 紧急升级触发
    emergency_triggers: EmergencyTriggers,
}

pub struct EmergencyTriggers {
    pub combat_detected: bool,     // 战斗触发 → 区域内 NPC → L1
    pub high_impact_event: bool,   // 冲击力 >0.85 → 在场 NPC → L1
    pub player_interaction: bool,  // 玩家主动交互 → 该 NPC → L1
}

/// 关键角色 override — 特定 NPC 永久锁定某个 Level
pub enum CriticalOverride {
    Critical,    // 永久 L1 (如国王/主角家族)
    High,        // L2+ (如重要商人/将领)
    Normal,      // 正常升降级
}
```

## 3.4 L4 视觉锚点 — 远方的文明存在感

> **设计意图**：L4 NPC（超远距数百万，>5km）没有个体实例。但玩家站在山顶望向地平线时——需要感知到"那些城市里有人在生活"。L4 视觉锚点 = L3 宏观统计的"远距离投影"，**与 L2/L1 共享同一数据源**，确保远看和近看一致。

### 核心原则：单一数据源 + 逐级展开

```
同一份 L4 宏观统计 ──┬─ L4 视觉锚点 (>5km)
(区域人口/繁荣度/     │   炊烟/火光/农田/道路/天际线
 文化/经济/季节/      │
 L3行为统计)          ├─ L3 统计 NPC (150m-5km)
                     │   宏观推演——日常行为批量模拟
                     │   群体统计行为 = L4 锚点的微观基础
                     │
                     └─ L2/L1 个体 NPC (<150m)
                         具体的人、具体的事
```

### 白天可见锚点

```
炊烟密度 (柱/100m²)
  = 人口密度 × cooking_frequency × meal_time_factor × season_factor
  └─ cooking_frequency 来自 L3 统计: 该区域 NPC 日均 cooking 行为次数
  └─ 荒废城市: 炊烟密度 < 正常值的 20%

农田纹理
  = 耕作面积 × farming_frequency × season_stage
  └─ farming_frequency 来自 L3 统计
  └─ 春季(翻土深色) → 夏季(绿色) → 秋季(金黄) → 冬季(休耕/雪盖)
  └─ 荒废: 退化为杂草地纹理 (gradual, lerp over ~30 game days)

道路车流 (移动小点)
  = road_traffic_level × prosperity × security_index
  └─ road_traffic_level 来自 L3 统计: 每小时经过该道路的 NPC 数
  └─ 荒废: 道路上几乎没有移动点

建筑轮廓密度
  = building_count (静态, 人口生成时确定)
  └─ 废弃建筑: roof_collapse_probability = f(繁荣度 < 0.3 的持续天数)
  └─ 废墟纹理 lerp over ~90 游戏天
```

### 夜晚可见锚点

```
地平辉光 (非现代均匀辉光——是离散火光的集合)
  = ∑(light_source_density × light_radius) × 0.15(中世纪火光系数)
  └─ 繁荣度↓ → 火把/灯油消耗↓ → 辉光面积收缩 (而非"变暗")
  └─ 算法: 从 L3 统计的 nightly_light_sources 推算辉光半径

城墙哨塔火炬
  = 驻军存在? 火炬可见 : 熄灭
  └─ 哨塔火炬熄灭 = 该城无执法力量 —— 强烈的叙事信号

个别火光点 (铁匠铺/酒馆/教堂)
  = prosperity × specific_profession_density
  └─ 铁匠铺火光 = 工匠区活跃度
  └─ 酒馆灯光 = 社交活跃度
  └─ 教堂/寺庙烛光 = 宗教活跃度
```

### 插值动画 — 城市在"慢慢暗淡"

```
每帧: anchor_value = lerp(current, target, rate × delta_days)

速率自适应——衰退越快，灯光/炊烟变化越快:

  小幅波动 (Δ <0.05, 正常经济):
    rate = 0.03/day → ~100天收敛
    几乎不可见。正常波动不应让远方城市"闪烁"。

  中幅衰退 (Δ 0.05-0.2, 局部饥荒/贸易中断):
    rate = 0.05/day → ~60天收敛
    2-3个月内可见地变暗。

  大幅衰退 (Δ 0.2-0.5, 战争/大饥荒):
    rate = 0.10/day → ~30天收敛
    1个月内显著暗淡——"北方的城在慢慢暗下去"= 无声叙事。

  灾难性衰退 (Δ >0.5, 屠城/瘟疫):
    rate = 0.30/day → ~10天收敛
    10天内几乎熄灭。除非战斗火光/浓烟替代日常辉光。
```

### 跨距离一致性校验

当玩家距离从 L4 过渡到 L3 时 (~5km):

```
L4锚点推断的 population_density ≈ L3实际统计的 population_density?

  差异 <10% → 通过。视觉连续。
  差异 ≥10% → L4 锚点立即跳变到最新 L3 数据。
    └─ 原因: L4 统计上周更新后, L1/L2 本周剧烈变化 (战争/移民)
    └─ 视觉: "远看以为繁华, 走近发现荒废"——这是叙事时刻, 不是 bug
```

### 性能

- 每个 L4 城市每帧: 3-5 次 f32 lerp + 纹理参数写入
- 1000 座城同时可见 (极端, 大部分被地形遮挡): ~0.005ms
- 可完全忽略

```rust
/// 每帧处理预算 (60fps = 16.7ms)
/// 018 v3 性能预算: Rust 模拟核心 ≤7ms, Godot 渲染 ≤8ms
pub struct FrameSliceConfig {
    pub l1_per_frame: usize,     // 200 个 L1 NPC/帧 (0.2s 周期 → 1000 L1 全覆盖)
    pub l2_per_frame: usize,     // 500 个 L2 NPC/帧 (1-2s 周期 → 10000 L2 全覆盖)
    pub l3_batch_interval: Duration,  // 每日/周批量
    pub l4_batch_interval: Duration,  // v3: 每周/月批量

    pub goap_concurrent_limit: usize, // ≤8 并发
    pub goap_timeout_per_plan: Duration, // ≤2ms
}
```

### 性能

- 每个 L4 城市每帧: 3-5 次 f32 lerp + 纹理参数写入
- 1000 座城同时可见 (极端): ~0.005ms, 可忽略

## 3.5 分帧策略

```rust
/// 每帧处理预算 (60fps = 16.7ms)
/// 018 v3 性能预算: Rust 模拟核心 ≤7ms, Godot 渲染 ≤8ms
pub struct FrameSliceConfig {
    pub l1_per_frame: usize,        // 200 L1/帧 (0.2s 周期 → 1000 L1 全覆盖)
    pub l2_per_frame: usize,        // 500 L2/帧 (1-2s 周期 → 10000 L2 全覆盖)
    pub l3_batch_interval: Duration,  // 每日/周批量
    pub l4_batch_interval: Duration,  // 每周/月批量
    pub goap_concurrent_limit: usize,   // ≤8 并发
    pub goap_timeout_per_plan: Duration, // ≤2ms
    pub snapshot_interval_frames: u32,   // 12 帧 (5Hz, 对齐 L1 更新)
}
```

---

# Part 4: 物理表达

> **设计意图**：心智在 Rust 侧运行。物理表达是 Rust → Godot 的数据通道——骨骼矩阵、渲染实例数据、感官反馈。Godot 侧不处理 NPC 逻辑。

---

## 4.1 Rust → Godot 数据通道

```rust
/// 每帧从 Rust 发送到 Godot 的 NPC 渲染数据包
#[derive(Debug)]
pub struct NpcRenderPacket {
    pub npc_id: NpcId,
    pub simulation_level: SimulationLevel,
    pub world_position: Vec3,
    pub rotation: Quat,
    pub bone_matrices: [Mat4; 50],       // 骨骼矩阵 (GPU skinning)
    pub per_instance_data: PerInstanceData,  // MultiMesh TextureBuffer
}

#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct PerInstanceData {
    pub animation_state_id: f32,
    pub animation_time: f32,
    pub emotion_pleasure: f32,
    pub emotion_arousal: f32,
    pub fatigue: f32,
    pub holding_item_type: f32,   // 0=无, 1=工具, 2=武器, 3=食物
    pub social_distance_mod: f32, // v3: 社交距离修正 (关系可见性)
    pub vehicle_role: f32,        // v3: 0=无, 1=crew, 2=passenger
}
```

## 4.2 骨骼矩阵计算

```rust
/// Rust CPU 批量计算骨骼矩阵 (rayon 并行)
/// 输出: PackedByteArray → Godot TextureBuffer → vertex shader 蒙皮
pub fn compute_bone_matrices(
    npcs: &[&NpcData],
    pose_database: &PoseDatabase,
    delta_time: Duration,
) -> Vec<[Mat4; 50]> {
    npcs.par_iter()
        .map(|npc| {
            let pose = pose_database.get_active_pose(
                npc.current_state.current_action,
                npc.current_state.animation_time,
            );
            // 姿态 → 关键帧插值 → 骨骼矩阵链
            pose.interpolate_to_matrices(npc.current_state.animation_time)
        })
        .collect()
}
```

## 4.3 MultiMesh 渲染

```
Godot 侧:
  MultiMeshInstance3D × 3 (L1/L2/L3 各一个 MultiMesh)
  └─ MultiMesh.buffer: 每实例 1 个 vec4 (4 floats)
  └─ TextureBuffer: 扩展 per-instance 数据 (PerInstanceData, 8+ floats)
      └─ vertex shader 中 texelFetch() 读取
```

## 4.4 非语言表达

```rust
/// 从情绪状态派生姿态微调
pub fn emotional_pose_offset(emotion: &EmotionState) -> PoseOffset {
    PoseOffset {
        head_tilt: emotion.sadness * -5.0,        // 悲伤 → 低头
        shoulder_slump: emotion.fear * 3.0,       // 恐惧 → 蜷缩
        chest_expand: emotion.joy * 2.0,           // 喜悦 → 挺胸
        gaze_avert: emotion.disgust * 0.5 + emotion.fear * 0.3,  // 厌恶/恐惧 → 避开视线
        fist_clench: emotion.anger * 4.0,          // 愤怒 → 握拳
        social_distance_bias: emotion.trust * -0.3 + emotion.fear * 0.5, // v3: 社交距离情绪修正
    }
}
```

---

# Part 5: 扩展系统（v3 新增）

> **设计意图**：018 v3 引入的所有 NPC 新能力——半自动战斗、载具行为、天象感知、蓝图施工、长途旅行、海洋环境——在本 Part 统一定义。

---

## 5.1 半自动战斗 AI

```rust
/// 玩家和 NPC 共用同一套战斗 AI 代码。
/// 唯一差异: 玩家可以通过 CombatCommand 发送高层指令。
///
/// 战斗风格从 NPC 人格自动派生。
/// 玩家通过选择战斗风格 = 选择了一组"虚拟人格参数" → 战斗 AI 执行。

#[derive(Debug, Clone)]
pub enum CombatStyle {
    Reckless,    // 高神经质 + 低宜人性    — 高攻低防, 不收手
    Tactical,    // 高尽责性 + 低外向性    — 利用掩体, 等待时机
    Supportive,  // 高宜人性 + 高外向性    — 辅助队友, 控制敌人
    Mage,        // 高开放性 + 高魔法       — 远程魔法, 范围攻击
}

impl CombatStyle {
    pub fn from_personality(p: &BigFive, magic_skill: f32) -> Self {
        if magic_skill > 0.7 && p.openness > 0.6 { return CombatStyle::Mage; }
        if p.neuroticism > 0.6 && p.agreeableness < 0.4 { return CombatStyle::Reckless; }
        if p.conscientiousness > 0.6 && p.extraversion < 0.4 { return CombatStyle::Tactical; }
        if p.agreeableness > 0.6 && p.extraversion > 0.6 { return CombatStyle::Supportive; }
        CombatStyle::Tactical  // 默认
    }

    /// 风格 → 行为参数
    pub fn parameters(&self) -> CombatParameters {
        match self {
            CombatStyle::Reckless => CombatParameters {
                attack_frequency: 1.8,
                block_probability: 0.05,
                dodge_probability: 0.10,
                retreat_threshold: 0.20,  // HP<20% 才逃跑
                environment_use: 0.1,
            },
            CombatStyle::Tactical => CombatParameters {
                attack_frequency: 0.7,
                block_probability: 0.65,
                dodge_probability: 0.55,
                retreat_threshold: 0.35,
                environment_use: 0.8,
            },
            CombatStyle::Supportive => CombatParameters {
                attack_frequency: 0.5,
                block_probability: 0.4,
                dodge_probability: 0.3,
                retreat_threshold: 0.30,
                environment_use: 0.5,
                heal_priority: 1.5,       // 优先治疗队友
                buff_priority: 1.2,
            },
            CombatStyle::Mage => CombatParameters {
                attack_frequency: 0.6,
                block_probability: 0.2,
                dodge_probability: 0.4,
                retreat_threshold: 0.35,
                environment_use: 0.6,
                spell_priority: 2.0,       // 优先魔法
                keep_distance: 15.0,       // 保持 15m 距离
            },
        }
    }
}

/// v3: 玩家高层指令 (战斗中可随时发出)
#[derive(Debug, Clone)]
pub enum CombatCommand {
    SwitchStyle(CombatStyle),               // 切换战斗风格
    UseSpecialSkill(SkillId),               // 手动释放关键技能/魔法大招
    UseItem(ItemId),                        // 使用物品
    FocusTarget(NpcId),                     // 集火
    ProtectTarget(NpcId),                   // 保护队友
    Retreat,                                // 撤退
    Surrender,                              // 投降 (触发社会后果)
    Negotiate,                              // 谈判
    Pursue,                                 // 追击逃跑的敌人
}

/// ─────────────────────────────────────────
/// 半自动战斗系统
/// ─────────────────────────────────────────
///
/// **状态**: 完整设计见 `开发阶段/战斗/` (13篇开发文档)。
/// 以下为 NPC 活人感模块对战斗系统的接口定义。
///
/// 核心原则:
///   - 玩家 AI = NPC AI = 同一套 Rust 代码
///   - 不弹窗、不暂停、不减速、不子弹时间
///   - 玩家不操作 = 角色完全自主战斗
///   - 玩家可随时通过干预条覆盖 AI 决策
///
/// 详见: [[001-战斗系统总览]] · [[003-三层战斗模型]] · [[007-半自动战斗与HUD]]

/// ─────────────────────────────────────────
/// v3: 战斗→记忆/关系/道德的涌现叙事桥梁
/// ─────────────────────────────────────────
///
/// 设计意图: IF…THEN…ELSE 不作为战斗决策引擎 (决策引擎的设计尚未确定),
/// 但作为"检测值得被记住的战斗时刻"的触发开关——
/// 将战斗系统和记忆/关系/道德系统连接起来。
///
/// 这是半自动战斗中"战斗在讲故事"的核心机制。

pub struct CombatNarrativeDetector;

impl CombatNarrativeDetector {
    /// 战斗结束后调用——检测所有值得被记忆的"叙事检测器"
    pub fn detect_memorable_moments(
        combat_record: &CombatRecord,
        participant: &NpcData,
    ) -> Vec<ProposedMemory> {
        let mut memories = Vec::new();

        // 检测器 1: 持久战
        if combat_record.duration_seconds > 120.0
            && combat_record.result == CombatResult::Draw
        {
            memories.push(ProposedMemory {
                event_type: EventType::Combat,
                summary: format!(
                    "和 {} 打了很久，谁也奈何不了谁",
                    combat_record.primary_opponent_name(participant)
                ),
                impact_score: 0.6,
                importance: Importance::High,
                relationship_delta: Some(RelationshipDelta {
                    affection: 0.05,  // 不打不相识 — 微弱的尊重
                    trust: 0.1,       // 了解了他的实力
                }),
            });
        }

        // 检测器 2: 以弱胜强
        if combat_record.result == CombatResult::Victory
            && combat_record.opponent_skill_ratio(participant) > 1.5
        {
            memories.push(ProposedMemory {
                event_type: EventType::Combat,
                summary: format!(
                    "我竟然打败了 {} —— 一个比我强得多的人",
                    combat_record.primary_opponent_name(participant)
                ),
                impact_score: 0.8,
                importance: Importance::Critical,  // 终生难忘
                relationship_delta: None,
                self_label_change: Some(SelfLabelChange {
                    add: vec!["强大的战士".into()],
                    remove: vec![],
                }),
            });
        }

        // 检测器 3: 被放过一马
        if combat_record.was_spared_by_opponent(participant) {
            let opponent = combat_record.sparing_opponent(participant);
            memories.push(ProposedMemory {
                event_type: EventType::Combat,
                summary: format!(
                    "{} 放了我一马。我本该死在那里的。",
                    opponent.name
                ),
                impact_score: 0.75,
                importance: Importance::Critical,
                relationship_delta: Some(RelationshipDelta {
                    affection: 0.3,   // 感激
                    trust: 0.15,      // "他是可以信任的人"
                }),
                moral_question: Some(MoralQuestion {
                    question: "我欠他一条命。这份债该怎么还？",
                    duration_days: 365,  // 这份记忆会持续很久
                }),
            });
        }

        // 检测器 4: 对投降者残忍
        if combat_record.kill_was_merciless(participant)
            && combat_record.opponent_was_surrendering(participant)
        {
            memories.push(ProposedMemory {
                event_type: EventType::Combat,
                summary: format!(
                    "我杀了 {} —— 即使他已经投降了。",
                    combat_record.primary_opponent_name(participant)
                ),
                impact_score: 0.7,
                importance: Importance::High,
                relationship_delta: None,
                self_label_change: Some(SelfLabelChange {
                    add: vec!["残忍".into()],
                    remove: vec!["仁慈".into()],
                }),
                reputation_impact: Some(ReputationImpact {
                    scope: ReputationScope::Local,  // 围观者会传播
                    tag: "杀降者".into(),
                    valence: -0.6,
                }),
            });
        }

        // 检测器 5: 被远远强于自己的人碾压
        if combat_record.result == CombatResult::Defeat
            && combat_record.opponent_skill_ratio(participant) > 2.0
            && combat_record.duration_seconds < 30.0
        {
            memories.push(ProposedMemory {
                event_type: EventType::Combat,
                summary: format!(
                    "{} 几乎瞬间就击败了我。我完全没有机会。",
                    combat_record.primary_opponent_name(participant)
                ),
                impact_score: 0.6,
                importance: Importance::High,
                relationship_delta: Some(RelationshipDelta {
                    affection: -0.1,   // 屈辱
                    trust: 0.0,
                }),
            });
        }

        // 检测器 6: 保护了重要的人
        if combat_record.protected_target_in_danger(participant) {
            let protected = combat_record.protected_target(participant);
            memories.push(ProposedMemory {
                event_type: EventType::Combat,
                summary: format!(
                    "我在战斗中保护了 {}。",
                    protected.name
                ),
                impact_score: 0.5,
                importance: Importance::High,
                relationship_delta: Some(RelationshipDelta {
                    affection: 0.25,
                    trust: 0.3,  // "我可以依靠他"
                }),
            });
        }

        // 检测器 7: 暴风雨/天象介入战斗 (v3)
        if combat_record.weather_intervened {
            memories.push(ProposedMemory {
                event_type: EventType::WeatherExtreme,
                summary: format!(
                    "那场战斗打到了一半，{} 突然变了。我们都在暴风雨中厮杀。",
                    if combat_record.weather_at_combat.is_rain { "天下起了暴雨" }
                    else if combat_record.weather_at_combat.is_snow { "天下起了大雪" }
                    else { "天气突然变了" }
                ),
                impact_score: 0.4,
                importance: Importance::Normal,
                relationship_delta: None,
            });
        }

        // 检测器 8: 围观者反应 (v3)
        if combat_record.bystander_count > 5 {
            memories.push(ProposedMemory {
                event_type: EventType::Combat,
                summary: format!(
                    "至少有 {} 个人看到了那场战斗。这件事会被传开的。",
                    combat_record.bystander_count
                ),
                impact_score: 0.3 + (combat_record.bystander_count as f32 * 0.02).min(0.3),
                importance: Importance::Normal,
                reputation_impact: Some(ReputationImpact {
                    scope: ReputationScope::Local,
                    tag: combat_record.summary_tag(),
                    valence: combat_record.reputation_valence(),
                }),
            });
        }

        memories
    }
}

/// 战斗记录——战斗结束后由系统生成的结构化数据
pub struct CombatRecord {
    pub participants: Vec<NpcId>,
    pub primary_opponent: BTreeMap<NpcId, NpcId>,  // 每个参与者的主要对手
    pub duration_seconds: f32,
    pub result: BTreeMap<NpcId, CombatResult>,      // 每个参与者的结果
    pub opponent_skill_levels: BTreeMap<NpcId, f32>,
    pub bystander_count: u32,
    pub weather_at_combat: WeatherSnapshot,
    pub weather_intervened: bool,
    pub kills: Vec<CombatKill>,
}

pub struct ProposedMemory {
    pub event_type: EventType,
    pub summary: String,
    pub impact_score: f32,
    pub importance: Importance,
    pub relationship_delta: Option<RelationshipDelta>,
    /// ★ SelfLabelChange 的目标已变更为 SelfNarrative.proposed_label_changes
    /// 不再直接写入 SocialIdentity.self_labels。
    /// SelfNarrative.reflect() 在反思周期中摄取并根据一致性接受/拒绝/修改。
    pub self_label_change: Option<SelfLabelChange>,
    pub moral_question: Option<MoralQuestion>,
    pub reputation_impact: Option<ReputationImpact>,
}

/// v3: 法术熟练度 → 自动化降级
///
/// 法术施放次数 → 法术所在层级:
///   1-49 次:  战略层 — 需要完整的有意识咏唱 (慢, 不可移动, 可被打断)
///   50-199 次: 节奏层 — 可以快速施放 (缩短咏唱时间, 可以在移动中施放)
///   200-499 次: 节奏层(本能化中) — 几乎瞬发, 但仍需意识地"选择"法术
///   500+ 次:   本能层 — 火球已经和眨眼睛一样自然了。
///              触发条件满足时身体自动施放 (如"敌人进入10m → 自动冰墙推开")
///
/// 存储成本: 每 NPC 每法术 1 个 u16 计数器。100K NPC × 平均 5 个法术 ≈ 1 MB。
/// 运行时成本: HashMap 查找 + 阈值比较, O(1), 仅战斗时触发。
/// 性能影响: 可忽略。

/// ─────────────────────────────────────────
/// v3: 战后过渡期 — 与 NPC 人格/情感/记忆深度绑定
/// ─────────────────────────────────────────
///
/// 战后反应不是通用模板。是"这次战斗经历了什么 × 他是什么样的人 × 他以前经历过什么"。

pub struct PostCombatTransition;

impl PostCombatTransition {
    /// 战斗结束后调用——生成本次战斗对 NPC 的完整后效
    pub fn process(
        combat_record: &CombatRecord,
        npc: &NpcData,
    ) -> PostCombatEffects {
        let trauma = Self::assess_trauma(combat_record, npc);
        let personality = &npc.personality;
        let memory = &npc.memory;

        PostCombatEffects {
            // ── 情绪残留 ──
            emotion_aftermath: Self::emotion_aftermath(&trauma, personality),
            emotion_decay_duration: Self::decay_duration(&trauma, personality),

            // ── 惊跳反应 ──
            startle_duration_minutes: Self::startle_duration(&trauma, personality),
            // 惊跳期表现: 本能层保持部分激活 → 突然的声响触发微闪避
            // 高神经质+被碾压 → 可能 1-2 小时
            // 低神经质+完胜 → 可能为 0

            // ── 社交行为影响 ──
            social_aftermath: Self::social_aftermath(&trauma, personality),
            // 胜利+高外向性 → 去酒馆庆祝
            // 失败+高神经质+高宜人性 → 回避社交, 在家发呆
            // 失败+高神经质+低宜人性 → 暴躁, 对周围人没好气

            // ── 知识增长 ──
            knowledge_gain: Self::knowledge_from_combat(combat_record, npc),
            // 高开放性 → 从战斗中"学到了一招"
            // 遭遇新敌人类型 → 信息写入记忆(长期有效)

            // ── 长期行为标签 (如果创伤够深) ──
            long_term_behavior_tag: Self::long_term_tag(&trauma, personality, memory),
            // 创伤够深 (impact>0.7 + neuroticism>0.6):
            //   → 生成回避标签: { 情境: "烟雾战斗", 倾向: "回避/谨慎" }
            //   → 未来类似情境中激活 → 影响战略层
            // 如果记忆中有"类似情境的克服经历":
            //   → 创伤被缓冲 (impact ×0.6-0.8)
            // 如果记忆中有"类似情境的创伤经历":
            //   → 创伤被放大 (impact ×1.5-2.0, 旧伤未愈+新伤)
        }
    }

    /// 创伤评估: 本次战斗的"创伤指纹"
    fn assess_trauma(record: &CombatRecord, npc: &NpcData) -> TraumaFingerprint {
        TraumaFingerprint {
            closeness_to_death: 1.0 - record.lowest_hp(npc),   // HP 最低值
            power_ratio: record.opponent_skill_ratio(npc),       // 对手强多少
            result_nature: record.result_nature(npc),            // 险胜/惨胜/完胜/被碾压...
            uncontrollability: record.uncontrollable_moments(npc), // 被连击/被包围/被击倒
            moral_burden: record.moral_burden(npc),  // 杀了不该杀的? 被放过? 放了别人?
            social_humiliation: record.social_humiliation(npc), // 在队友前丢脸? 被抛弃?
        }
    }

    /// 情绪残留: 创伤 × 人格 → 战斗后的情绪偏移
    fn emotion_aftermath(t: &TraumaFingerprint, p: &BigFive) -> EmotionDelta {
        let neuroticism_amplifier = 0.5 + p.neuroticism;  // 0.5-1.5

        let pleasure_delta = match t.result_nature {
            CombatResultNature::CrushingVictory => 0.3,
            CombatResultNature::CloseVictory => 0.1,
            CombatResultNature::PyrrhicVictory => -0.1,  // 惨胜——赢了但不高兴
            CombatResultNature::Draw => 0.0,
            CombatResultNature::CloseDefeat => -0.2,
            CombatResultNature::CrushingDefeat => -0.5 * neuroticism_amplifier,
            CombatResultNature::SparredByOpponent => -0.1, // 被放过——复杂
        };

        EmotionDelta {
            pleasure: pleasure_delta,
            arousal: t.closeness_to_death * 0.5,  // 越接近死亡, 兴奋/恐惧越高
            control: if t.result_nature.is_victory() { 0.2 } else { -0.3 * neuroticism_amplifier },
        }
    }

    /// 惊跳反应持续时间
    fn startle_duration(t: &TraumaFingerprint, p: &BigFive) -> Minutes {
        let base = 5.0;  // 5 分钟基准
        let neuroticism_factor = 0.5 + p.neuroticism * 1.5;  // 0.5-2.0
        let trauma_factor = 1.0 + t.uncontrollability * 2.0;  // 1.0-3.0
        (base * neuroticism_factor * trauma_factor) as Minutes
    }

    /// 长期行为标签: 创伤够深 → 人被那次经历改变
    fn long_term_tag(
        t: &TraumaFingerprint,
        p: &BigFive,
        memory: &MemoryStore,
    ) -> Option<BehaviorTag> {
        let trauma_impact = t.closeness_to_death * t.uncontrollability * t.power_ratio;
        let personality_mod = p.neuroticism * 0.7 + (1.0 - p.conscientiousness) * 0.3;

        if trauma_impact * personality_mod < 0.5 { return None; }

        // 检查记忆中是否有"缓冲经历"
        let buffer = memory.events.iter()
            .filter(|m| m.event_type == EventType::Combat
                && m.tags.contains(&"overcame".into())
                && m.impact_score > 0.5)
            .count() as f32 * 0.15;

        if trauma_impact * personality_mod - buffer < 0.5 { return None; }

        Some(BehaviorTag {
            situation: t.context_tag(),  // "烟雾中的战斗" / "被包围" / "被碾压"
            tendency: "回避/谨慎",
            decay_days: (trauma_impact * 365.0) as u32, // 可能持续数月到数年
        })
    }
}

/// v3: 战斗中的心理维度 — 信息战
pub fn bluff_assessment(
    observer: &NpcData,
    target_apparent_behavior: &CombatBehavior,
    target_actual_state: &NpcData,
) -> BluffAssessment {
    let personality_mod = observer.personality.neuroticism * 0.5
        + (1.0 - observer.personality.openness) * 0.3;  // 更神经质/不开放 → 容易被唬

    let apparent_vs_actual = target_apparent_behavior.confidence
        - target_actual_state.emotion.control;

    if apparent_vs_actual > 0.4 && personality_mod > 0.5 {
        BluffAssessment::FallingForIt   // 被唬住了
    } else if apparent_vs_actual > 0.2 {
        BluffAssessment::Suspicious
    } else {
        BluffAssessment::RealAssessment
    }
}
```

## 5.2 载具上的 NPC 行为

```rust
/// 载具状态是 NPC 行为权重的新输入维度
/// NPC "知道自己在一艘船上/一列火车上" → 行为产生对应的修正

impl DecisionEngine for ProbabilisticDecisionEngine {
    fn vehicle_modifier(&self, action: &Action, vehicle_state: &Option<VehicleState>) -> f32 {
        match vehicle_state {
            None => 1.0,  // 不在载具上 → 无修正
            Some(ref vs) => match vs.role {
                VehicleRole::Crew { rank: _ } => {
                    // 船员: 操控/瞭望/维护权重大幅提升
                    if action.category == ActionCategory::NavigationTask { return 3.0; }
                    if action.category == ActionCategory::Lookout { return 2.5; }
                    if action.category == ActionCategory::Maintenance { return 2.0; }
                    if action.category == ActionCategory::Social { return 0.2; }
                    if action.category == ActionCategory::Sleep { return 0.5; }
                    1.0
                }
                VehicleRole::Passenger => {
                    // 乘客: 社交/观景/休息权重提升, 户外活动受限
                    if action.category == ActionCategory::OutdoorSocial { return 1.5; }
                    if action.category == ActionCategory::Social { return 1.3; }
                    if action.category == ActionCategory::ObserveScenery { return 2.0; }  // v3: 观景
                    if action.category == ActionCategory::Sleep { return 1.2; }
                    if action.category == ActionCategory::OutdoorActivity { return 0.3; }
                    1.0
                }
            }
        }
    }
}

/// v3: 载具专用的 GOAP 目标
pub enum VehicleGoal {
    MaintainCourse,        // 保持航向/轨道
    AvoidObstacle,         // 躲避礁石/冰山/敌方船只
    RepairDamage,          // 暴风雨后维修
    AssistPassengers,      // 安抚/引导乘客
    RestockSupplies,       // 靠港补给 (燃料/食物/魔力)
    LookoutLand,           // 瞭望陆地/港口
    LookoutPirates,        // 瞭望盗贼/敌人
}

/// v3: 载具 NavMesh 策略
///
/// **核心原则**: 载具局部 NavMesh 仅在结构变化时重建 (罕见)。
/// 移动中的动态影响 (摇摆/过弯) 通过姿态补偿和航点过渡处理。
///
/// **船体摇摆**: 在骨骼矩阵中直接施加 PoseOffset (倾斜补偿), 不重建 NavMesh。
///   暴风雨: 乘客移动速度 ×0.6 + 随机方向扰动 ×0.2, 模拟甲板摇晃行走。
///
/// **火车过弯**: 每节车厢独立 NavMesh + 连接点航点。
///   车厢间过道不依赖连续 NavMesh 表面。
///   NPC 跨车厢移动: 当前车厢 NavMesh → 连接航点线 → 目标车厢 NavMesh。
///   航点线 = 两个车厢连接点的简单直线 (过弯时自动弯曲以匹配相对转角)。
pub struct VehicleNavMesh {
    /// 每节车厢/每个甲板区域的独立 NavMesh
    pub compartment_navmeshes: Vec<NavMesh>,

    /// 车厢/区域之间的连接航点
    pub connection_waypoints: Vec<(WaypointId, WaypointId)>,

    /// 船体当前倾斜角度 (用于 PoseOffset)
    pub current_tilt: Vec2,  // (pitch_radians, roll_radians)

    /// 暴风雨强度 (0-1, 影响乘客移动参数)
    pub storm_intensity: f32,
}

/// v3: 载具上的施工 — 建造/修理载具本身
pub enum VehicleConstructionGoal {
    BuildShipAtPortyard,
    LayRailwayTrack,
    RepairVehicleHull,
}
```

## 5.3 NPC 对天象的感知

```rust
impl SensoryProvider for DefaultSensoryProvider {
    fn perceive_sky(&self, npc: &NpcData, weather: &WeatherState) -> SkyPerception {
        let personality = &npc.personality;

        // 基础感知 — 客观天气 → 主观感知 (受人格偏移)
        let cloud_cover = weather.actual_cloud_cover;
        let storm_approaching = weather.storm_front_distance < 5.0; // km

        // 高神经质 NPC 对天气更敏感 (感知的云量 ×1.2)
        let perceived_cloud_cover = cloud_cover * (0.8 + personality.neuroticism * 0.4);

        // 低尽责性 NPC 可能忽视暴风雨前兆
        let perceived_storm = storm_approaching
            && personality.conscientiousness > 0.3;  // 尽责性太低 → "没注意到"

        SkyPerception {
            cloud_cover: perceived_cloud_cover.clamp(0.0, 1.0),
            cloud_type: weather.actual_cloud_type.clone(),
            rainbow_visible: weather.rainbow_active,
            storm_approaching: perceived_storm,
            visibility_quality: weather.visibility,
            last_updated: weather.current_time,
        }
    }
}

/// v3: 天象感知的可观测性 — 确保数据差异在 A 面可见
///
/// **"观天"微姿态的触发条件**:
///   天象变化 (彩虹出现/暴风雨前兆/日食) + 人格过滤 + 概率触发 + 异步执行
///
///   人格过滤: 仅高开放性 (openness>0.6, 对新奇敏感) 或
///            高神经质 (neuroticism>0.6, 对天气敏感) 的 NPC 触发
///
///   概率: 彩虹 → 15% 概率, 暴风雨前兆 → 25% 概率 (更紧迫)
///   持续: 每次抬头 2-4 秒, 不同 NPC 异步 (在 2 分钟内各自触发)
///
///   多人效应: 如果 15% NPC 在 2 分钟内各自抬头 →
///            市场上 3-5 人看天 → 玩家自然注意 →"有人在看什么？"→ 看到彩虹
///
/// **对话模板触发词** (5-8 条, 匹配条件: 天象变化 <30s):
///   "这天气……" | "暴风雨要来了" | "看那边——彩虹！" |
///   "今晚的月亮真亮" | "这雾真大"
///
///   匹配条件 = NPC.sky_perception.storm_approaching / rainbow_visible / etc.
///              + 对话对象在社交距离内 + NPC 人格可触发"观天"

/// v3: 天象在决策器中的使用 — 温和偏移, 非决定
/// 关键原则: "天气是 NPC 活动的考量和影响因素, 但不是决定因素。"
/// 即使暴风雨, 如果 NPC 觉得有必要 (饥饿/救命/埋伏敌人) — 仍然会出门。
///
/// 高神经质 NPC 对天气变化的响应 ×1.5 (更敏感)
/// 低神经质 NPC 几乎不受天气影响 (×0.5)
impl DecisionEngine for ProbabilisticDecisionEngine {
    fn sky_modifier(&self, action: &Action, sky: &SkyPerception) -> f32 {
        let mut modifier = 1.0;

        // 乌云 → 户外活动微降
        if sky.cloud_cover > 0.7 && action.is_outdoor() {
            modifier *= 0.85;
        }

        // 暴风雨前兆 → 加速当前户外任务 (但不取消!)
        if sky.storm_approaching && action.category == ActionCategory::UrgentHarvest {
            modifier *= 2.0;   // 暴风雨来了 — 赶紧收庄稼
        }

        // 彩虹 → 微小的情绪提振已在情绪引擎中处理
        // 这里仅影响"驻足观景"行为的概率
        if sky.rainbow_visible && action.category == ActionCategory::ObserveScenery {
            modifier *= 3.0;   // 彩虹 → 有人停下来看天
            // (只有极少 NPC 会触发 — 这是一种安静的、有人味的涌现)
        }

        modifier
    }
}
```

## 5.4 蓝图施工 — NPC 集体建造

```rust
/// 施工任务 — NPC 从各种来源获得 (自己需要建房 / 团体分配 / 玩家委托)
#[derive(Debug, Clone)]
pub struct ConstructionJob {
    pub blueprint: Blueprint,
    pub site: Vec3,
    pub required_materials: BTreeMap<MaterialId, u32>,
    pub gathered_materials: BTreeMap<MaterialId, u32>,
    pub assigned_workers: Vec<NpcId>,
    pub foreman_id: Option<NpcId>,
    pub progress: f32,            // 0-1
    pub construction_quality: f32, // 受工人技能影响
    pub estimated_game_days: u32, // 预计完工时间 (游戏日)
}

impl ConstructionJob {
    /// 分配施工角色 — 基于 NPC 技能和职业
    pub fn assign_role(&mut self, npc: &NpcData) -> ConstructionRole {
        if npc.identity.profession == ProfessionId::from_str("architect")
            || npc.skills.get(&SkillId::Construction).map(|s| s.level).unwrap_or(0) > 70.0
        {
            ConstructionRole::Foreman  // 工头 — 建筑/设计技能高
        } else if npc.skills.get(&SkillId::Construction).map(|s| s.level).unwrap_or(0) > 40.0 {
            ConstructionRole::Builder  // 工匠
        } else if npc.physiology.stamina > 0.6 {
            ConstructionRole::MaterialGatherer  // 体力好 → 搬运
        } else {
            ConstructionRole::Supplier  // 其他人 → 物资运输
        }
    }

    /// v3: 施工分阶段可视化 — 解决"等待期间无事可看"
    /// Day 1-2: 地基开挖 (地面 SDF + NPC 在场)
    /// Day 3-5: 框架搭建 (建筑骨架可见)
    /// Day 6-8: 封顶+外墙 (轮廓完整)
    /// Day 9-10: 内饰+细节 (窗户/门/瓦片)
    /// 玩家可在任何阶段走到现场——看到 NPC 工作、听到施工音效
    pub fn current_phase(&self) -> ConstructionPhase { /* progress → phase */ }
}

/// v3: 施工 NPC 的行为溢出 — 工地 = 社区的微型经济注入
///
/// 施工 NPC 在工地附近产生"生活溢出":
///   - 去酒馆吃午饭 → 临时增加酒馆密度
///   - 去市场采购材料 → 临时交易活动
///   - 晚上回住所/工棚睡觉
///
/// 玩家感知: 不只是房子在变, 社区节奏也在变。
///
/// 实现: 工头为每个工人注入"施工期生活任务":
///   { EatNearSite, RestAtCamp, PurchaseMaterials, SocializeAtNearbyTavern }
/// 任务权重 = f(施工阶段, 时间, 工人人格)
pub struct ConstructionSpillover {
    pub worker_life_tasks: Vec<Action>,
    pub temporary_economic_boost: f32,   // 周边市场交易 ×1.1-1.3
    pub temporary_population_boost: u32,  // 工人临时增加该区域 NPC 数
}

/// v3: 自然语言委托 → 蓝图
/// "我需要一栋有两个卧室的铁匠铺"
/// → LLM 或模板解析 → Blueprint TOML → 标准化蓝图
pub struct NlBlueprintParser {
    /// 解析自然语言 → 建筑需求参数
    pub fn parse(&self, text: &str, culture_seed: CultureId) -> Result<Blueprint, ParseError> {
        // 提取: 建筑类型、房间数量、特殊要求、风格偏好
        // 映射到参数化建筑模板
        // 输出 .blueprint TOML
        todo!("自然语言→蓝图的解析逻辑")
    }
}
```

## 5.5 长途旅行者 NPC

```rust
/// v3: 在主干道上动态生成旅行者 NPC
/// 这些 NPC 不是城镇的永久居民 — 而是在聚落之间移动的 transient NPC
///
/// 生成逻辑:
///   旅行者密度 = f(道路等级, 当地治安, 经济繁荣度)
///   旅行者类型概率 = f(当地经济, 治理状况, 季节)
#[derive(Debug, Clone)]
pub struct TravelerGenerator {
    pub road_id: RoadId,
    pub road_segment_start: Vec3,
    pub road_segment_end: Vec3,
}

#[derive(Debug, Clone)]
pub enum TravelerType {
    TradeCaravan { merchants: u32, guards: u32, pack_animals: u32 },
    Messenger { urgency: f32, destination: LocationId },
    Wanderer,
    Pilgrim { destination: LocationId },
    AdventurerParty { members: u32 },
    Refugee { origin: LocationId, reason: TravelReason },
    Bandit { gang_size: u32, desperation: f32 },  // 经济崩溃+治理缺失 → 涌现
}

#[derive(Debug, Clone)]
pub enum TravelReason {
    EconomicMigration,   // 当地活不下去 → 流向繁荣城市
    Famine,
    War,
    Religious,
    Adventuring,
    Trade,
}
```

## 5.6 海洋环境中的 NPC 行为

```rust
/// v3: 海洋环境 NPC 行为
///
/// 1. 港口区 NPC — 装卸工/渔船船长/海关官员/商行代理
///    行为权重: 交易 ×1.4, 户外劳动 ×1.3, 社交(酒馆) ×1.2
///
/// 2. 船上 NPC (见 §5.2) — 船员+乘客
///
/// 3. 海上遭遇 NPC — 商船/海盗/渔民/海军巡逻
///    由事件系统生成 — 非永久 NPC, 仅在与玩家船只接近时实例化
///
/// 4. 海岸居民 — 渔村 NPC 的行为权重受潮汐+渔获季节影响

pub fn coastal_npc_behavior_modifier(
    npc: &NpcData,
    location: &LocationType,
) -> BTreeMap<ActionCategory, f32> {
    match location {
        LocationType::CoastalVillage => BTreeMap::from([
            (ActionCategory::Fishing, 2.0),        // 渔村 → 捕鱼权重
            (ActionCategory::BoatRepair, 1.5),
            (ActionCategory::Trade, 1.2),          // 海边贸易
        ]),
        LocationType::PortCity => BTreeMap::from([
            (ActionCategory::Trade, 1.5),
            (ActionCategory::DockWork, 2.0),       // 港口装卸
            (ActionCategory::Social, 1.3),         // 酒馆社交 (水手文化)
        ]),
        LocationType::ShipAtSea => BTreeMap::from([
            (ActionCategory::NavigationTask, 3.0),
            (ActionCategory::Social, 0.4),
            (ActionCategory::Sleep, 0.7),
        ]),
        _ => BTreeMap::new(),
    }
}
```

---

# Part 6: 工程实现

> **设计意图**：性能预算、并发策略、存档格式、调试工具——这些是实现阶段的工程基准。

---

## 6.1 性能预算（v3 重算）

### 帧预算（60fps / 16.7ms）

| 类别 | 预算 | 说明 |
|------|------|------|
| **Rust 模拟核心** | ≤7.0ms | — |
| ├─ NPC 心智 (200 L1 + 500 L2/帧, rayon) | ≤2.5ms | 情绪+心境+概率决策+习惯+内在驱动+社交感知 |
| ├─ 自我叙事反思 (分摊至每日, L1+L2) | ≤0.01ms | 7日周期, ~1570次/游戏日 (帧分摊) |
| ├─ 心境更新 (L1+L2, 每日) | ≤0.01ms | O(1)/NPC, 分摊至帧 |
| ├─ GOAP 规划 (≤8 并发, 2ms 硬超时) | ≤1.5ms | 仅目标变化时触发 |
| ├─ 战斗 AI (半自动, ≤20 活跃战斗) | ≤1.5ms | 统一代码路径 |
| ├─ 骨骼矩阵计算 (rayon, glam SIMD) | ≤0.5ms | 1000 NPC × 50 bones |
| ├─ 导航 (认知+局部烘焙) | ≤0.8ms | 认知优先命中率 >80% |
| ├─ 载具逻辑 | ≤0.3ms | 移动参考系更新 |
| ├─ 蓝图施工调度 | ≤0.2ms | 低频操作 |
| ├─ 时间/天气/天象 | ≤0.2ms | — |
| └─ 世界生成 (后台线程) | ≤0.5ms | — |
| **Godot 渲染** | ≤8.0ms | — |
| ├─ 体素 Mesh (垂直稀疏 Chunk + Clipmap) | ≤2.5ms | — |
| ├─ NPC MultiMesh (1000 实例) | ≤1.0ms | — |
| ├─ GPU skinning shader | ≤0.5ms | — |
| ├─ Gerstner 海洋 | ≤1.0ms | — |
| ├─ 混合体积云 | ≤1.5ms | — |
| ├─ 城市建筑 (含 LOD) | ≤1.5ms | — |
| ├─ 光照+阴影 | ≤1.5ms | — |
| └─ 后处理+UI+天气粒子 | ≤1.0ms | — |
| **Godot 物理** | ≤1.7ms | PhysicsServer3D RID |

### 内存预算

| 数据 | 预算 | 说明 |
|------|------|------|
| NPC 本体 (SoA, 100K+ + L4) | ~45 MB | — |
| NPC 记忆 (LMDB 热缓存, ~6M 条) | ~520 MB | — |
| NPC 关系 (~5M 条) | ~280 MB | — |
| 活跃 Chunk (垂直稀疏) | ~200 MB | — |
| Pose Database + 动画 | ~15 MB | — |
| 导航数据 | ~80 MB | 含载具局部 NavMesh |
| 城市建筑数据 | ~150 MB | 含 LOD |
| 海洋/云/天象缓冲 | ~60 MB | — |
| 载具数据 | ~20 MB | — |
| 其他 | ~80 MB | — |
| **总计** | **~1.4 GB** | 占 32GB RAM 约 4.4% |

### VRAM 预算（目标: GTX 1660 SUPER 6GB）

| 资产 | 预算 | 说明 |
|------|------|------|
| 体素 Mesh (视锥内) | ~1.2 GB | 垂直稀疏 + Clipmap |
| 城市建筑 Mesh+纹理 | ~0.8 GB | 含 LOD |
| NPC MultiMesh 实例数据 | ~0.3 GB | TextureBuffer |
| 环境纹理 (地面/植被/海洋) | ~0.5 GB | — |
| 体积云密度纹理 | ~0.2 GB | 3D 纹理 128³ |
| 阴影贴图 | ~0.2 GB | CSM × 3 |
| 后处理/UI/其他 | ~1.0 GB | — |
| **总计** | **~4.2 GB** | **占 70% — 1.8GB 余量** |

## 6.2 并发策略与帧内执行顺序

> **设计意图**：解决 CHG-009/019 识别的 trait 边界模糊 (1.1) 和情绪感染竞争条件 (1.2)。核心方案：WorldSnapshot（帧级只读快照）+ 严格的帧内执行顺序。

### 6.2.1 WorldSnapshot

```rust
/// 每 12 帧 (5Hz) 从 WorldState 冻结的只读快照。
/// 对齐 L1 决策器更新周期 (每 0.2s)。
/// 所有 NPC 决策器共享同一个 Arc<WorldSnapshot>——消除并行借用冲突。
#[derive(Debug, Clone)]
pub struct WorldSnapshot {
    pub time: GameTime,
    pub weather: WeatherState,
    pub positions: BTreeMap<NpcId, Vec3>,       // 所有 L1+L2 NPC 位置
    pub public_events: Vec<PublicEvent>,         // 本批次的公开事件
    pub spatial_index: Arc<SpatialIndex>,        // 快速空间查询 (<15m 范围)
    pub road_network: Arc<RoadNetwork>,
    pub ocean_state: OceanSnapshot,              // v3: Gerstner 波采样点
    pub sky_state: SkySnapshot,                  // v3: 云/彩虹/暴风雨前兆
}

/// 5Hz 快照节奏:
///   帧  0: 调度器确定 L1/L2/L3/L4 集合 → 冻结 WorldSnapshot
///   帧  1-11: 200 L1 + 500 L2 每帧, 复用同一快照
///   帧 12: 新一轮
/// 最老数据 ≈0.18s — NPC 决策不需要比这更精确。
/// 实时事件 (战斗/紧急) 通过 EventBus 直接推送——不依赖快照。
```

### 6.2.2 帧内执行顺序

```
1. 升降级调度 (确定本帧 L1/L2/L3/L4 集合)
   └─ 滞后区间: 升级 50m / 降级 55m + 3s 最小维持
   └─ 紧急升级: 战斗触发 / 高冲击事件 / 玩家交互

2. (每 12 帧) 冻结 WorldSnapshot → Arc<WorldSnapshot>
   └─ 空间索引重建: BTreeMap<NpcId, Vec3> → SpatialIndex

3. NPC 心智并行更新 (L1: 200/帧 + L2: 500/帧, rayon)
   └─ select_action(npc, Arc<WorldSnapshot>, rng)  // 共享只读快照
   └─ 每个 NPC 独立借用 &mut NpcData — 无竞争

4. 情绪感染串行执行 (仅 L1, <15m 距离, <0.3ms)
   └─ 明确操作范围: 仅本帧确定的 L1 集合 (升降级已完成)
   └─ O(n×k), k<10

5. 战斗 AI 并行更新 (独立隔离, rayon)
   └─ 玩家 + NPC 同一代码路径

6. 渲染数据打包 → GDExtension → Godot
   └─ NpcRenderPacket 批量序列化为 PackedByteArray
```

**安全规则**：
- NPC 之间相互独立：决策/情绪/记忆不共享可变状态
- NPC 之间的关系写入通过 LMDB 事务串行化
- `WorldSnapshot` 在帧批次内不可变：`Arc` 共享，消除所有决策器中的世界状态借用冲突

```rust
/// 主循环
pub fn simulation_frame(world: &mut WorldState, dt: Duration, frame_index: u32) {
    // 1. 调度
    let l1_set = world.scheduler.update(world);
    let l2_set = world.scheduler.l2_npcs();

    // 2. 快照 (每 12 帧)
    let snapshot = if frame_index % 12 == 0 {
        Arc::new(world.freeze_snapshot(&l1_set, &l2_set))
    } else {
        world.current_snapshot().clone()  // Arc::clone — 几乎免费
    };

    // 3. 心智更新
    let l1_batch: Vec<&mut NpcData> = world.npcs.pick_batch(&l1_set, 200);
    l1_batch.par_iter_mut().for_each(|npc| {
        npc.update_mind(&snapshot, dt);
    });

    // 4. 情绪感染 (串行, 仅 L1)
    world.emotion_contagion(&l1_set, &snapshot);

    // 5. 战斗 AI
    world.combat_system.update_frame(&snapshot, dt);

    // 6. 打包 → Godot
    world.pack_render_packets(&l1_set, &l2_set);
}
```

## 6.3 存档

```rust
/// 存档格式: bincode 二进制 → LMDB
/// 流式 Chunk 存储 (无限世界):
///   未修改的 Chunk 不存储 — 读档时从种子重现
///   脏 Chunk + 变更的 NPC 数据 → 增量存档

pub struct SaveSystem {
    db: lmdb::Environment,
    base_seed: u64,
}

impl SaveSystem {
    /// 增量存档: <500ms (仅序列化脏数据)
    pub fn incremental_save(&self, world_state: &WorldState) -> Result<Duration> { /* ... */ }

    /// 全量存档: <5s
    pub fn full_save(&self, world_state: &WorldState) -> Result<Duration> { /* ... */ }

    /// 读档: <10s
    /// 流程: 种子重现基础地形 → 回放修改层 (脏 Chunk) → 加载 NPC 数据
    pub fn load(&self, save_id: &SaveId) -> Result<WorldState> { /* ... */ }

    /// 数据老化: 低冲击力(impact<0.05)且超过 365 游戏日未访问的记忆 → 压缩为摘要
    pub fn age_memories(&self, npcs: &mut [NpcData]) { /* ... */ }
}
```

## 6.4 调试工具

```rust
/// 开发阶段的 NPC 调试面板 (Godot 侧 UI 展示 Rust 侧数据)
pub struct NpcDebugTools {
    /// 世界暂停 + 单 NPC 心智检视
    pub fn inspect_npc(&self, npc_id: NpcId) -> NpcDebugSnapshot {
        NpcDebugSnapshot {
            emotion_axes: (p_a_d),
            active_composite_emotion: String,
            recent_memories: Vec<EventMemory>,  // 最近 20 条
            current_goal: Goal,
            decision_chain: Vec<DecisionStep>,   // 过去 60s 的决策回溯
            relationship_map: BTreeMap<NpcId, Relationship>,
            social_distance: f32,
            sky_perception: SkyPerception,       // v3
            vehicle_state: Option<VehicleState>, // v3
            construction_task: Option<ConstructionTask>, // v3
        }
    }

    /// 头顶 Debug 标签 (Godot 侧 overlay)
    /// 显示: 当前情绪标签 / 目标 / GOAP 状态 / 模拟等级
    pub fn debug_overlay_tags(&self, npc_id: NpcId) -> Vec<String> { /* ... */ }
}
```

## 6.5 Action → Event → Goal 统一映射

> **设计意图**：解决 019 §2.2 — `EventType`、`Goal`、`ActionCategory` 三套枚举各自为政。统一映射表约 30-50 行 match，维护成本极低。

```rust
/// Action 执行后的轻量转换:
///   1. Action 执行 → 查映射 → 生成 EventMemory (如果 impact > 阈值)
///   2. Action 执行 → 查映射 → 满足 Goal 完成条件 → Goal::Complete

pub fn action_to_event(action: &Action, npc: &NpcData) -> Option<EventType> {
    match action.category {
        ActionCategory::FriendlySocial => Some(EventType::DailyInteraction),
        ActionCategory::PhysicalAttack => Some(EventType::Combat),
        ActionCategory::Trade => Some(EventType::DailyInteraction),
        ActionCategory::ShareResource => Some(EventType::Cooperation),
        ActionCategory::Ceremony => Some(EventType::Ceremony),
        ActionCategory::NavigationTask => Some(EventType::Nautical),       // v3
        ActionCategory::Maintenance => Some(EventType::Vehicular),         // v3
        ActionCategory::Construction => Some(EventType::Construction),     // v3
        ActionCategory::ObserveScenery => {
            if npc.sky_perception.rainbow_visible { Some(EventType::Celestial) }
            else if npc.sky_perception.storm_approaching { Some(EventType::WeatherExtreme) }
            else { None }  // 日常观景不值得记忆
        }
        ActionCategory::Escape => {
            if action.impact > 0.3 { Some(EventType::Combat) }
            else { None }
        }
        ActionCategory::Explore => {
            if action.is_novel_location { Some(EventType::Revelation) }
            else { None }
        }
        // ★ 求爱→记忆映射（02-性别与吸引力系统.md）
        ActionCategory::Courtship => {
            if action.is_propose_bond() { Some(EventType::Ceremony) }
            else if action.impact > 0.3 { Some(EventType::EmotionalMilestone) }
            else { None }
        }
        // ... 约 30 条映射
        _ => None,  // 大多数日常行为不值得生成记忆
    }
}

pub fn action_satisfies_goal(action: &Action, goal: &Goal) -> bool {
    match (action.category, goal) {
        (ActionCategory::Eat, Goal::SurvivalEat) => true,
        (ActionCategory::Drink, Goal::SurvivalDrink) => true,
        (ActionCategory::Sleep, Goal::SurvivalSleep) => true,
        (ActionCategory::Heal, Goal::SurvivalHeal) => true,
        (ActionCategory::NavigationTask, Goal::VehicleMaintainCourse) => true,
        (ActionCategory::Maintenance, Goal::VehicleRepairDamage) => true,
        // ★ 求爱→目标映射（02-性别与吸引力系统.md）
        (ActionCategory::Courtship, Goal::FindPartner) => true,
        (ActionCategory::Courtship, Goal::PursueRomanticInterest) => true,
        // ... 约 20 条映射
        _ => false,
    }
}
```

## 6.6 性能优化预留路径

> **设计意图**：019 §2.3 识别。这些不是当前必须实现的——但在性能瓶颈出现时优先从此列表选取。

### L4 批量统计分摊

```
当前: 每周/月集中计算所有 L4 区域统计 → 月度尖峰
优化: 每游戏日计算 1/7 的 L4 区域 (轮转) → 日均摊
      L4 视觉锚点的插值动画已缓冲了变化速率——每日更新足够
```

### 决策器查表缓存

```rust
/// L2 NPC 特别受益: 状态变化慢 (仅衰减+简化状态机)
/// 同一 NPC + 同一情境 (需求值/情绪/天象 未显著变化) → 缓存 1-2 帧
pub struct DecisionCache {
    npc_id: NpcId,
    last_snapshot_hash: u64,       // 情境指纹
    cached_action: Action,
    valid_for_frames: u8,          // 1-2 帧
}

// 缓存命中判断: 需求值变化 <0.05 + 情绪变化 <0.1 + 天象未更新
// 预期命中率: L2 ~60%, L1 ~20% (L1 变化频繁)
```

### LMDB 热记忆分区

```rust
/// 替代全量热缓存搜索:
///   "最近 7 天" 分区 (高访问频率) + "重要标记" 分区 (情绪峰值) + "冷归档" 分区
///
/// NPC 检索记忆时: 热分区 → 重要分区 → 冷归档 (仅在前两者无结果时)
/// 冷归档仅在"回忆往事"的偶然触发时访问——避免 2000 条全扫描
pub struct MemoryPartition {
    pub hot: Vec<EventMemory>,       // 最近 7 天 + access_count > 10
    pub important: Vec<EventMemory>, // important_event_ids
    pub cold_archive: Vec<EventMemory>, // 其余
}
```

---

## 附录 A: 关键参数速查

| 参数 | 值 | 出处 |
|------|-----|------|
| NPC 记忆上限 | 2000 条/人 | §2.2.4 |
| L1 距离 | ≤50m | §3.1 |
| L2 距离 | 50-150m | §3.1 |
| L4 距离 | >5km | §3.1 |
| 情绪感染距离 | ≤15m | §2.1.5 |
| GOAP 规划超时 | ≤2ms | §2.3.2 |
| 决策滞后区间 | ≥5s / 需求差 >0.15 | §2.3.1 |
| L1 每帧更新数 | 200 | §3.4 |
| L2 每帧更新数 | 500 | §3.4 |
| 重要记忆标记阈值 | 情绪 Δ>0.3 | §2.2.4 |
| 暴风雨前兆感知距离 | 风暴前线 <5km | §5.3 |
| 旅行者生成密度 (硬化路) | 15% 概率/天 | §5.5 |
| 施工预计完工 (小型建筑) | 3-7 游戏日 | §5.4 |
| 施工预计完工 (大型载具) | 30-90 游戏日 | §5.4 |

## 附录 B: 人格基调速查

| 人格 | 低分 (0-0.3) | 高分 (0.7-1.0) |
|------|------------|-------------|
| O | 保守/熟悉偏好 | 好奇/尝试新事物 |
| C | 随性/即兴 | 自律/计划性强 |
| E | 独处/少言 | 主动社交/群体愉悦 |
| A | 竞争/怀疑 | 信任/合作 |
| N | 情绪稳定/快恢复 | 敏感/恢复慢 |

## 附录 C: 开发约束

- Godot 4.6 LTS + Rust stable 1.80+ (GDExtension)
- 单机优先, 联机搁置
- 3D 低多边形/像素风格
- 冷兵器+魔法战斗 (半自动)
- AI 生成美术资产
- 世界随机生成 (种子), 初步 25 万 km², 最终 Minecraft 级
- 目标 100K+ NPC (L1/L2/L3/L4), 全部尽数模拟
- LLM 可选 (玩家自备 Key)
- Mod 接口 (TOML 参数调节, 无脚本引擎)
- 硬件目标: GTX 1660 SUPER 6GB 流畅

---

> **本开发文档是 WoWorld NPC 系统的权威实现规格 v2.0。**
> 完整覆盖: 数据合同 (§1) · 心智核心 (§2) · 分层模拟 (§3) · 物理表达 (§4) · 扩展系统 (§5) · 工程实现 (§6)。
> 所有改动基于 CHG-007 的 16 个设计缺陷修正和 CHG-008 的 v3 设计深化。
> 原 ver1.01 (GDScript 版) 已废弃。ver1.01short 已删除。
>
> **关联**: [[018-正式技术栈方案v3-20260610|技术栈 v3.0]] · [[CHG-007]] · [[CHG-008]]
