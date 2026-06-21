# 004-AgentSnapshot与连续参数涌现

> **关联**: [[001-NPC行动涌现总纲|001-总纲]] · [[../07-生命周期系统/001-生命周期系统总纲|生命周期系统 CHG-041]]
> **日期**: 2026-06-18 | **修订**: 2026-06-21 — 认知段扩展 CognitiveTide 3字段（v1.0→v1.1）

## §一、设计动机

传统的"年龄门控"写死了 NPC 能做什么。WoWorld 的替代方案: AgentSnapshot——一个紧凑的连续参数快照，贯穿全部三层原子。**没有任何二进制判断。所有差异从连续参数的物理计算中涌现。**

## §二、完整结构定义

```rust
struct AgentSnapshot {  // ~108 bytes, SoA 友好布局
    // === 身体 (7 fields, f32 each) — 从 Life.Vitals 派生 ===
    strength: f32,            // 0-1 → GRASP/LIFT/STRIKE力度
    dexterity: f32,           // 0-1 → DODGE/THROW精度/GRASP稳定性
    stamina: f32,             // 0-1 → MOVE持续/STRIKE频率上限
    health_penalty: f32,      // 0-1 中毒/受伤/疾病 → 全原子效率×系数
    mobility_factor: f32,     // 0-1 年龄+体型+负重 → MOVE速度/类型可达性
    fatigue: f32,             // 0-1 → 全原子效率衰减(指数), WAIT恢复
    hunger: f32,              // 0-1 → Eat目标GOAP权重(线性)
    
    // === 认知 (8 fields) — 从 技能+认知系统+NPC人格 派生 ===
    skill_vector_compressed: u64,   // 压缩技能向量(98技能→64bit bitmask)
    mental_model_count: u8,         // 概念数量 → 幼儿<10, 成人≤20
    cognitive_damping: f32,         // 0-1 认知僵化 → 学习新概念速率×系数
    planning_horizon: u8,           // GOAP搜索深度 (幼儿1→成人5-7→老年3-4)
    literacy: f32,                  // 0-1 → READ/WRITE可行性
    cognitive_load: f32,            // ★ 0-1 → 全原子execution_noise放大·决策延迟·GOAP效用折扣
    rumination_pressure: f32,       // ★ 0-1 → 分心·战术精度衰减·社交对话质量下降
    mind_quietude: f32,             // ★ 0-1 → 战略层深度·施法稳定性·专注力
    // 以上3字段从 CognitiveTide 每决策周期映射——影响全部三层原子(战斗/魔法/社交/经济)
    
    // === 社交 (4 fields) — 从 权力+名声+文化 派生 ===
    legal_capacity: f32,            // 0-1 法律行为能力 → SIGN/INVEST/PROCLAIM
    reputation_flags: u32,          // 压缩声望标记(32 bit = 32种声望维度)
    social_standing: f32,           // 0-1 → 说服力/交涉成功率
    religious_participation: f32,   // 0-1 → PRAY/SACRIFICE效能
    
    // === 生命阶段 (4 fields) — 从 AgeClock+Gompertz+InfantDependency 派生 ===
    developmental_phase: u8,        // 描述性标签: 0=Infant..7=Elder
    age_ratio_in_phase: f32,        // 0-1 当前阶段内位置(连续进度)
    fertility_potential: f32,       // 0-1 sigmoid曲线
    gompertz_mortality_risk: f32,   // 当前帧死亡风险(基础×指数加速)
    
    // === 环境 (3 fields) — 从 WeatherQuery 派生 ===
    wetness: f32,                   // 0-1 → 摩擦力×系数/导电性×系数
    temperature_exposure: f32,      // K → HEAT/COOL耐受偏移
    intoxication: f32,              // 0-1 → 全原子精度×噪声(乘法叠加)
}
```

## §三、完整派生公式

> **约定**: AgentSnapshot 所有字段为 0-1 归一化值。派生公式确保输入→输出的归一化映射。

```rust
impl AgentSnapshot {
    fn from_sources(
        vitals: &Vitals,               // Life 004
        lifecycle: &LifecycleState,     // 生命周期 CHG-041
        skills: &[SkillEntry],          // 技能系统
        cognition: &CognitiveStyle,     // 认知系统 CHG-032
        cognition_tide: &CognitiveTide, // ★ v1.1 认知系统——每决策周期更新(load/rumination/quietude)
        power: &PowerProfile,           // 权力系统 CHG-023
        weather: &WeatherSample,        // 天气系统 CHG-016
    ) -> Self {
        // === 身体 (7 fields) ===
        let strength = vitals.strength_normalized();           // Life.physique→0-1映射
        let dexterity = vitals.dexterity_normalized();
        let stamina = vitals.stamina_pool / vitals.stamina_max; // 当前/最大
        let fatigue = 1.0 - stamina;                            // 疲劳=精力倒数
        let mobility_factor = lifecycle.mobility_curve()        // AgeClock×体型
            * (1.0 - 0.3 * vitals.encumbrance_ratio());         // 负重衰减 ≤30%
        
        // health_penalty: 多个损伤源采用乘法聚合(避免超1.0)
        //   penalty = 1.0 - Π(1.0 - source_i)
        let health_penalty = 1.0 
            - (1.0 - vitals.poison_level())    // 中毒 0-1
            * (1.0 - vitals.injury_aggregate()) // 受伤(各部位max)
            * (1.0 - vitals.disease_level())   // 疾病 0-1
            * (1.0 - fatigue * 0.3);           // 疲劳贡献≤30%
        let hunger = vitals.hunger_normalized();                // 0=饱 1=饿死
        
        // === 认知 (8 fields) ===
        let cognitive_damping = cognition.damping_coefficient(); // CognitiveStyle.damping
        let planning_horizon = (7.0 * (1.0 - cognitive_damping)  
            * lifecycle.maturity_factor()).clamp(1.0, 7.0) as u8;
        let literacy = skills.literacy_level().clamp(0.0, 1.0);
        let mental_model_count = cognition.model_count() as u8;
        let skill_vector_compressed = compress_skills(skills);   // 见§3.1
        
        // === 社交 (4 fields) ===
        let legal_capacity = power.legal_capacity(lifecycle.age_years());
        let reputation_flags = power.reputation_bitmask();
        let social_standing = power.social_standing_normalized();
        let religious_participation = power.religious_participation();
        
        // === 生命阶段 (4 fields) ===
        let developmental_phase = lifecycle.current_phase() as u8; // 0-7
        let age_ratio_in_phase = lifecycle.phase_progress();       // 0-1
        let fertility_potential = lifecycle.fertility_curve();     // sigmoid
        let gompertz_mortality_risk = lifecycle.gompertz_risk();   // 基础×exp加速
        
        // === 环境 (3 fields) ===
        let wetness = weather.surface_wetness.clamp(0.0, 1.0);
        let temperature_exposure = weather.ground_temperature;     // 绝对K值(非归一化——物理公式需要)
        let intoxication = vitals.intoxication_level.clamp(0.0, 1.0);
        
        // === 认知潮汐 (3 fields) — 从 CognitiveTide 直接映射 ★ v1.1 ===
        let cognitive_load = cognition_tide.cognitive_load;         // 0-1
        let rumination_pressure = cognition_tide.rumination_pressure; // 0-1
        let mind_quietude = cognition_tide.mind_quietude;            // 0-1
        // CognitiveTide 每决策周期从 NpcData 更新——反映实时认知状态。
        // 进入 AgentSnapshot 后统一影响: 战斗(决策精度噪声)·魔法(施法稳定性)·
        //   社交(对话质量)·经济(交易判断)·GOAP(效用评估折扣)
        
        Self { /* ... */ }
    }
}
```

### 3.1 skill_vector_compressed 编码方案

98个技能 → 按5大类分组压缩: Combat(取MeleeWeapon组max) / Magic(取Element组max) / Artisan(取Metalwork组max) / Academic(取Natural组max) / Survival(取Gathering组max) → 5×u8层级编码(0-100 → 0-255) = 40 bits + 24 bits保留/标记。精确技能等级由领域模块在调用原子时自行从 `SkillEntry` 获取并填入原子 Input.skill_proficiency。

### 3.2 intoxication 与 execution_noise 的交互

```rust
fn effective_noise(skill_level: f32, intoxication: f32) -> f32 {
    let skill_noise = execution_noise_std(skill_level);  // BASE × (1-proficiency)²
    let intoxication_multiplier = 1.0 + intoxication * INTOX_FACTOR; // INTOX_FACTOR≈2.0 [TUNING]
    (skill_noise * intoxication_multiplier).clamp(0.0, 1.0)
}
// 醉酒 = 技能噪声放大。清醒时无影响，全醉时噪声×3。
// 乘法叠加确保零噪声×任何系数仍为零（大师不受酒精影响——"肌肉记忆"）。
```

## §四、年龄涌现矩阵

所有阶段差异从连续参数涌现——不设二进制门控。完整的阶段行为特征表详见 [[001-NPC行动涌现总纲#§五|001 §五]]。

## §五、涌现验证实例（归一化版本）

### 中毒不劳动
`health_penalty = 1-(1-0.6)×(1-0)×(1-0)×(1-0) = 0.6` → GRASP: `strength×(1-0.6)=0.12 < 锤子阈值0.32` → 拿不动大锤 + GOAP: 买药(0.95) > 硬撑(0.096) → 去药铺

### 儿童不能锻造
`strength=0.3 < 大锤阈值0.32` → GRASP失败。`strength=0.3 > 小锤阈值0.12` → 拿得动小锤 → 涌现"学徒做小件"

### 老人从战士转指挥官
`strength: 0.95→0.5, stamina恢复×0.5` → STRIKE效用持续下降 → GOAP转向 strategy/command（高认知低体力行动）

### 幼儿不能结婚
`fertility_potential=0 + legal_capacity=0` → Marry precondition 失败。连续能力自然为零，非年龄门控。
