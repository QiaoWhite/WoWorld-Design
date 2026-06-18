# 004-AgentSnapshot与连续参数涌现

> **关联**: [[001-NPC行动涌现总纲|001-总纲]] · [[../07-生命周期系统/001-生命周期系统总纲|生命周期系统 CHG-041]]
> **日期**: 2026-06-18

## §一、设计动机

传统的"年龄门控"写死了 NPC 能做什么。WoWorld 的替代方案: AgentSnapshot——一个紧凑的连续参数快照，贯穿全部三层原子。**没有任何二进制判断。所有差异从连续参数的物理计算中涌现。**

## §二、完整结构定义

```rust
struct AgentSnapshot {  // ~96 bytes, SoA 友好布局
    // === 身体 (7 fields, f32 each) — 从 Life.Vitals 派生 ===
    strength: f32,            // 0-1 → GRASP/LIFT/STRIKE力度
    dexterity: f32,           // 0-1 → DODGE/THROW精度/GRASP稳定性
    stamina: f32,             // 0-1 → MOVE持续/STRIKE频率上限
    health_penalty: f32,      // 0-1 中毒/受伤/疾病 → 全原子效率×系数
    mobility_factor: f32,     // 0-1 年龄+体型+负重 → MOVE速度/类型可达性
    fatigue: f32,             // 0-1 → 全原子效率衰减(指数), WAIT恢复
    hunger: f32,              // 0-1 → Eat目标GOAP权重(线性)
    
    // === 认知 (5 fields) — 从 技能+认知系统+NPC人格 派生 ===
    skill_vector_compressed: u64,   // 压缩技能向量(98技能→64bit bitmask)
    mental_model_count: u8,         // 概念数量 → 幼儿<10, 成人≤20
    cognitive_damping: f32,         // 0-1 认知僵化 → 学习新概念速率×系数
    planning_horizon: u8,           // GOAP搜索深度 (幼儿1→成人5-7→老年3-4)
    literacy: f32,                  // 0-1 → READ/WRITE可行性
    
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

## §三、派生规则

```rust
impl AgentSnapshot {
    fn from_sources(
        vitals: &Vitals,           // Life 004
        lifecycle: &LifecycleState, // 生命周期 CHG-041
        skills: &[SkillEntry],      // 技能系统
        cognition: &CognitiveStyle, // 认知系统 CHG-032
        power: &PowerProfile,       // 权力系统 CHG-023
        weather: &WeatherSample,    // 天气系统 CHG-016
    ) -> Self { /* 各字段的派生公式 */ }
}
```

**关键**: AgentSnapshot 不存储在任何持久化系统中——是每次决策周期（L1: 0.3s）从各模块数据**瞬时派生**的临时视图。

## §四、年龄涌现矩阵

所有阶段差异从连续参数涌现——不设二进制门控。完整的阶段行为特征表详见 [[001-NPC行动涌现总纲#§五|001 §五]]。

## §五、涌现验证实例

### 中毒不劳动
`health_penalty=0.6` → GRASP force_check ×0.4 → 拿不动大锤 + GOAP评估: 买药(0.95) > 硬撑(0.096) → 去药铺

### 儿童不能锻造
`strength=0.3` → GRASP(8kg锤) force_check: 0.3×1.0=0.3 < 8×0.3=2.4 → 拿不动。用小锤可能打得动小件→涌现"学徒做小件"

### 老人从战士转指挥官
`strength 0.95→0.5` + `stamina 恢复×0.5` → STRIKE效用持续下降 → GOAP自动转向 strategy/command 等高认知低体力的行动

### 幼儿不能结婚
`fertility_potential=0` + `legal_capacity=0` → Marry的precondition_check失败。不是"年龄不到不能结婚"——是生理和法律能力连续为零。
