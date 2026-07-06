# NPC 总纲 — ECS 架构

> **关联原文档**: [[开发阶段/NPC活人感模块/NPC活人感开发文档ver2.0]]
> **关联**: [[02-基本需求]] · [[05-认知与智慧]] · [[06-生命周期]] · [[07-行动涌现]]

---

## 这个模块关心什么"细节"

NPC 核心模块定义了"一个 NPC 是什么"的全部细节。每个细节是一个独立的 Component——NPC 是这些 Component 在 Entity 上的**集合**。

NPC 不是一个 struct。NPC 不是一个 class。**NPC 是一组 Component 的组合。**

---

## Component 总览

```
一个"完整的活人NPC"的 Component 集合：

  身份层:  NpcCore, SpeciesId, ProfessionTagId
  身体层:  Vitals, BodyPlan, Physiology
  需求层:  Needs, NeedSensitivity
  认知层:  CognitiveStyle, MentalModelHandle, MemoryHandle
  情感层:  Emotion, BigFive
  社会层:  SocialIdentity, RelationHandle
  行动层:  Goal, Desire, ActionQueue
  空间层:  Position, LodLevel
```

**不是所有 NPC 都有所有 Component。** 
- 一个远处的 NPC（ai_lod >= 3）可能只有 Position + LodLevel + NpcCore
- 一个在战斗中的 NPC 有 CombatState + EquipmentSlots
- 一个在做交易的 NPC 有 Wallet + EconomicCognition

---

## 核心 Component 定义

### NpcCore

```rust
struct NpcCore {
    name_hash: u64,       // 名字 hash → NameStorage Resource 查原文
    gender: GenderTag,    // 标签 Component 的替代——高频不变的数据可以用 enum 字段
    birth_tick: u64,      // 出生时间戳
    home_position: DVec3, // 家的位置（固定）
}
impl Component for NpcCore {}
```

### Emotion

> 代码采用 PAD (Pleasure-Arousal-Dominance) 命名——与心理学 PAD 情绪模型对齐。

```rust
struct Emotion {
    pleasure: f32,       // -1(不愉快) ~ +1(愉快)
    arousal: f32,        // 0(平静) ~ 1(激动)
    dominance: f32,      // 0(被支配) ~ 1(支配)
    source_entity: Option<EntityId>,  // 情绪来源
}
impl Component for Emotion {}
```

### BigFive

```rust
struct BigFive {
    openness: f32,       // 0-1, 经验开放性
    conscientiousness: f32,
    extraversion: f32,
    agreeableness: f32,
    neuroticism: f32,
}
impl Component for BigFive {}
```

---

## AgentSnapshot：派生视图（延后实现）

> ⚠️ **延后实现** — 保留设计意图。实现时为帧局部 `HashMap<Entity, AgentSnapshot>`，**不作为 ECS Component**。避免 archetype 写后立即读的 cache 浪费。具体 26 字段待 ActionSelection System 实现时按需敲定。

`AgentSnapshot` 不是存储的——它是每决策周期从其他 Component 派生出来的 108B 临时视图。

```rust
struct AgentSnapshot {
    // Body (7 fields)
    strength: f32,
    dexterity: f32,
    stamina: f32,
    health_penalty: f32,
    mobility_factor: f32,
    fatigue: f32,
    hunger: f32,
    // Cognitive (8 fields)
    skill_vector_compressed: u64,
    mental_model_count: u8,
    cognitive_damping: f32,
    planning_horizon: u8,
    literacy: f32,
    cognitive_load: f32,
    rumination_pressure: f32,
    mind_quietude: f32,
    // Social (4 fields)
    legal_capacity: f32,
    reputation_flags: u32,
    social_standing: f32,
    religious_participation: f32,
    // Lifecycle (4 fields)
    developmental_phase: u8,
    age_ratio_in_phase: f32,
    fertility_potential: f32,
    gompertz_mortality_risk: f32,
    // Environment (3 fields)
    wetness: f32,
    temperature_exposure: f32,
    intoxication: f32,
}
impl Component for AgentSnapshot {}
```

**派生逻辑**：
```
AgentSnapshot::from_sources():
  Vitals → body fields
  Skills.SkillEntry → skill_vector_compressed
  Cognition.CognitiveStyle + CognitiveTide → cognitive fields
  Power.PowerTopology → social fields
  Lifecycle.AgeClock → lifecycle fields
  Weather.WeatherSample → environment fields
```

---

## System 定义

### SnapshotAssemblySystem

- **触发**: Entity 有 `NpcCore` + `Vitals`，且 `LodLevel.ai_lod <= 2`
- **读**: `&Vitals`, `&SkillHandle`, `&CognitiveStyle`, `&AgeClock`, `&WeatherSample`(Resource)
- **写**: `&mut AgentSnapshot`（重建）
- **与其他 System 的关系**: 无——其他 System 读取 AgentSnapshot 作为输入

---

## Handle 类型（大堆数据的间接引用）

| Handle Component | 指向的 Resource | 数据量 |
|-----------------|---------------|--------|
| `MemoryHandle{storage_key,count}` | `MemoryStorage` Resource | 2000条/Entity |
| `SkillHandle{storage_key,count}` | `SkillStorage` Resource | HashMap/Entity |
| `RelationHandle{storage_key,count}` | `RelationStorage` Resource | BTreeMap/Entity |
| `MentalModelHandle{storage_key,count}` | `MentalModelStorage` Resource | 20条/Entity |

---

## 约束检查清单

- [x] Component 字段纯值类型（Handle 是 u64+u16）
- [x] 大数据走 Handle + Resource
- [x] AgentSnapshot 108B 固定大小——可直接作为 Component
- [x] 不含方法

## 新想法接入点

如果未来想加入"NPC 的饮食习惯"：
1. 定义 `DietaryPreference` Component
2. 写 `DietarySystem`（查询 Needs + DietaryPreference → 调整食物选择）
3. 注册到 Phase 1

不改已有 Component，不改已有 System。
