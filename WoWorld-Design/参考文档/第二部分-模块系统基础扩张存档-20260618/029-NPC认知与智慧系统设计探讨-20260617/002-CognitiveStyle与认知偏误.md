> **开发代号**: WoWorld (Wonder World)
> **文档类型**: 参考文档 · 讨论大纲
> **创建日期**: 2026-06-17
> **父文档**: [[000-模块大纲与导航]]

---

# 002-CognitiveStyle与认知偏误

## 待讨论的核心问题

### 1. CognitiveStyle的4维定义

- analytic_intuitive（分析-直觉）：0=纯直觉，1=纯分析——如何推理
- reflective_impulsive（反思-冲动）：0=纯冲动，1=纯反思——思考深度
- abstract_concrete（抽象-具象）：0=纯具象，1=纯抽象——思维粒度
- rigid_flexible（顽固-灵活）：0=顽固，1=灵活——信念可塑性

### 2. 派生公式——从已有数据到CognitiveStyle

- 从BigFive：各维度对应哪个OCEAN因子？
- 从MentalAttributes.wisdom：智慧→抽象+反思
- 从mental_age：年轻人→具象+冲动，老年人→抽象+反思
- 从life_event_count：经历越多→越灵活
- 阻尼机制：防止反馈回路爆炸

### 3. CognitiveStyle随人生的变化

- 对标AestheticTaste模式：青春期初始→年度成熟→事件调制
- 初始派生（NPC出生时）
- 每7天/SelfNarrative::reflect()时重派生
- 重大事件（impact>0.8）可能触发即时微调
- CognitiveStyle反过来影响BigFive长期漂移

### 4. CognitiveBiases——7种偏误的惰性派生

- 确认偏误、负面偏误、近因效应、自我服务偏差、可得性启发、认知失调容忍、反刍倾向
- 每一项如何从CognitiveStyle + 当前EmotionState + CognitiveTide派生
- 具体的数学公式（sigmoid/EMA/加权组合）

### 5. EmbodiedCognition——身体状态→认知调制

- 对标Physiology从Vitals派生的模式
- 疼痛→cognitive_load↑, quietude↓
- 饥饿→短期偏误↑
- 醉酒/高烧→关联松弛→跨界关联↑但质量↓
- 温度→来自WeatherQuery

### 6. CognitiveNorms——文化→认知风格

- 对标CommunicationNorms从CultureCoreParams派生的模式
- 辩论风格（Confrontational/Eristic/Dialectical/Consensus）
- 不确定性容忍
- 认识论立场（经验主义↔理性主义）
- MentalModel分享规范
