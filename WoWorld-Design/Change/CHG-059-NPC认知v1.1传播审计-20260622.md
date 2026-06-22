> **变更编号**: CHG-059
> **日期**: 2026-06-22
> **状态**: 完成 (审计+编辑)
> **来源**: CHG-057 (NPC认知系统v1.1深度设计) + CHG-058 (NPC认知系统自审修正)
> **影响范围**: 16个模块·80+文件编辑·14个审计文件

## 概述

CHG-057 和 CHG-058 在 06-认知与智慧系统中引入了根本性架构变更（PatternExpression 数学地基、DomainSignature 替代 MentalModelDomain 枚举、四层记忆压缩、deliberation_depth、counterfactual_regret、感官-认知桥梁、基于注册的 formalize_innovation() 等），但 15+ 消费模块的文档未反映这些变更。

CHG-059 系统地将这些 v1.1 认知变更传播到所有受影响模块——审计每个模块的文档过时引用、缺失概念、接口不一致和级联需求，并编辑所有受影响文件。

## 审计协议 (4维度)

1. **过时引用** — 仍引用已移除/重命名概念的文档
2. **缺失概念** — CHG-057/058 新增职责但模块文档未声明
3. **接口不一致** — 002-接口入口 vs 06-认知 001-接口出口 的不匹配
4. **新上游需求** — 模块适配后对其他模块提出的级联需求

## 执行统计

| Phase | Modules | Files Edited | Key Changes |
|-------|---------|-------------|-------------|
| Phase 1 (震中) | 05-感官与知觉 (8文件), 08-NPC行动涌现 (6文件), 24-概念与语言地基+13-语言表达 (7文件) | ~21 | 最严重断裂——感知产出新增 EmotionalCharge/GazeEstimate/crowd_emotional_field；AgentSnapshot v1.1 认知段同步；DomainSignature 替换 MentalModelDomain；EnrichedPerceptBatch 消费声明 |
| Phase 2 (内部) | 07-生命周期 (8文件), 02-NPC活人感主模块 (4编辑), 13-语言表达 (8文件) | ~20 | 中度断裂——CognitiveAgingPath 所有者更正；MemoryStore 四层升级；deliberation_depth 消费声明；counterfactual_regret 集成；source_confidence 源混淆 |
| Phase 3 (外部) | 06-战斗 (5), 07-魔法 (5), 08-技能 (4), 09-文化 (4), 14-经济 (4), 11-权力 (4), 12-历史 (5), 23-建筑 (3) | ~34 | 轻度断裂——EnrichedPerceptBatch 消费声明；formalize_innovation() 注册制；DomainSignature 消费路径；Mathematics 子类认知原子 |
| Phase 4 (基础设施) | 26-存档系统 (3), 00-全局基础设施 (2) | ~5 | 轻微断裂——MemoryStore 四层持久化；PatternExpression 序列化；woworld_core 新增类型注册 |
| **总计** | **16模块** | **~80文件编辑** | — |

## 关键修复

| 修复项 | 影响模块 | 说明 |
|--------|---------|------|
| AgentSnapshot 加入出口 | 08-行动涌现 → 全模块 | AgentSnapshot v1.1（CognitiveTide 3字段：cognitive_load/rumination_pressure/mind_quietude）正式加入 08-NPC行动涌现 001-接口出口。所有消费模块（战斗/魔法/技能/经济等）的 002-接口入口 同步更新 |
| RenderContext 补齐字段 | 05-感官与知觉 | PerceptBatch::enrich() 产出 EnrichedPerceptBatch 需 RenderContext（含 EmotionalCharge/GazeEstimate/crowd_emotional_field）。感官系统 001-接口出口 补齐 |
| CognitiveAgingPath 所有者更正 | 07-生命周期 | 原 06-认知 中定义的 CognitiveAgingPath 实际由生命周期系统 OWN——crystallized_factor()/cognitive_engagement_score()/health_burden() 三函数联合派生。所有权从认知系统更正为生命周期系统 |
| MemoryStore 四层升级 | 02-NPC活人感/26-存档 | L0 Hot(≤2000)→L1 Cold(≤500)→L2 Era(≤20)→L3 Life(≤5)。旧"2000条上限"描述全部更新为四层压缩模型。存档 named_db 键空间扩展 |
| MentalModelDomain → DomainSignature 全量替换 | 24-概念与语言/06-认知/全领域crate | 17个硬编码枚举 → DomainSignature(u64) 涌现。所有引用 MentalModelDomain 的文档全部替换。学科聚类从 DomainSignature 统计涌现 |
| EnrichedPerceptBatch 消费全量声明 | 06-战斗/07-魔法/14-经济/11-权力 | 战斗 combat_intelligence.assess()、魔法 spell_selection、经济 EconomicCognition、权力 legitimacy 推导——全部声明消费 EnrichedPerceptBatch 而非裸 PerceptBatch |
| deliberation_depth 连续涌现 | 02-NPC活人感 GOAP | 不硬编码 reflective>0.4 阈值。deliberation_depth = trait×state×stakes 连续值——GOAP 安全网以 deliberation_depth 而非 CognitiveStyle.reflective 为门控 |
| counterfactual_regret 集成 | 02-NPC活人感 情绪引擎 | 决策后反事实思考——"如果当时选了B会怎样"。影响 regret 情绪强度和后续决策权重调制 |
| source_confidence 源混淆 | 02-NPC活人感 记忆系统 | 记忆源置信度衰减→<0.3 概率性 misattribution。虚假记忆自然涌现——"我以为是我亲眼看到的" |
| formalize_innovation() 注册制 | 全领域crate | 战斗/魔法/艺术/建筑/工艺/社会各领域 crate 注册 ATOM_MASK + consumer_fn。未注册→AcademicWork 归档。不走 MentalModelDomain 枚举分发 |

## 审计文件

所有 14 审计文件位于 `参考文档/039-NPC认知传播审计-20260622/`，覆盖 16 个模块的 4 维度审计结果。详见该目录 [[参考文档/039-NPC认知传播审计-20260622/README|README]]。

审计文件清单：
- 005-感官与知觉审计
- 008-NPC行动涌现审计
- 024-概念与语言地基审计
- 007-生命周期审计
- 002-NPC活人感主模块审计
- 013-语言表达审计
- 006-战斗审计
- 007-魔法审计
- 008-技能审计
- 009-文化审计
- 014-经济审计
- 011-权力审计
- 023-建筑审计
- 012-历史审计
- 026-存档系统审计
- 000-全局基础设施审计

## 新上游需求 (级联)

| 需求 | 提出模块 | 目标模块 | 说明 |
|------|---------|---------|------|
| Mathematics 子类 (0405) | 06-认知 | 08-技能系统 | arithmetic/geometry/algebra/statistics/logic——5个技能。支撑 Mathematician/TaxCollector/Architect 职业 |
| 认知原子 (COUNT/MEASURE/CALCULATE/DERIVE) | 06-认知 | 08-NPC行动涌现 | 4个新物理原子——走标准三层(物理原子→复合→GOAP) |
| PatternExpression 序列化 | 06-认知 | 26-存档系统 | PatternExpression(~120B) 需新增 named_db 键空间——MentalModel 持久化格式升级 |
| DomainSignature 查询 | 24-概念与语言 | 00-全局基础设施 | DomainSignature(u64) 需在 woworld_core 注册为通用类型。domain_similarity() 纯函数 |
| crowd_emotional_field 生产 | 06-认知 | 05-感官与知觉 | EnvironmentPerception 新增 crowd_emotional_field(f32) + crowd_dominant_emotion(Option<BasicEmotion>)——感官系统负责从周边 NPC 聚合 |
| EmotionalCharge 生产 | 06-认知 | 05-感官与知觉 | PerceptEntry 新增 emotional_charge(EmotionalCharge)——感官系统从可观察表情/声音派生 |
| GazeEstimate 生产 | 06-认知 | 05-感官与知觉 | PerceptEntry 新增 gaze_estimate(Option<GazeEstimate>)——纯几何计算，不可靠的 |

## 关联文档

- [[CHG-057]]
- [[CHG-058]]
- [[参考文档/038-NPC认知系统跨模块配合需求-20260622/README|038-跨模块配合需求]]
- [[参考文档/039-NPC认知传播审计-20260622/README|039-传播审计]]
