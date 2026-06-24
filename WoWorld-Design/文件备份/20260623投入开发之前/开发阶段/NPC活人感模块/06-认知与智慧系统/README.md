> **开发代号**: WoWorld (Wonder World)
> **引擎**: Godot 4.6.2
> **模块编号**: 06
> **模块名称**: NPC认知与智慧系统
> **父模块**: [[../NPC活人感开发文档ver2.0|NPC活人感模块]]
> **状态**: 🚧 开发中
> **创建日期**: 2026-06-17
> **最后更新**: 2026-06-22 (CHG-057 v1.1 深度设计)

---

# 06-NPC认知与智慧系统 · 工作目录

## 正式规格文档

| 编号 | 文档 | 状态 |
|------|------|------|
| [[001-认知与智慧系统总纲]] | 001-认知与智慧系统总纲 | ✅ v1.0 · ★ v1.1 (CHG-057) |
| [[002-CognitiveStyle与认知偏误详细设计]] | 002-CognitiveStyle与认知偏误 | ✅ v1.0 |
| [[003-MentalModel与智慧积累]] | 003-MentalModel生命周期 | ✅ v1.0 · ★ v1.1 (CHG-057) |
| [[004-思考涌现与浮现机制]] | 004-ThoughtTrigger与ThoughtFragment | ✅ v1.0 |
| [[005-睡眠认知加工与正则化]] | 005-睡眠与梦境 | ✅ v1.0 |
| [[006-创新管线与跨领域对接]] | 006-创新与领域系统 | ✅ v1.0 · ★ v1.1 (CHG-057) |
| [[007-跨模块依赖与接口契约]] | 007-接口与契约 | ✅ v1.0 |

## 核心概念

| 概念 | 说明 |
|------|------|
| **CognitiveStyle** | 4维认知风格(直觉-分析/冲动-反思/具象-抽象/顽固-灵活)——从BigFive+wisdom+经历派生 |
| **CognitiveTide** | 3维认知潮汐(cognitive_load/rumination_pressure/mind_quietude)——每决策周期更新，映射至 AgentSnapshot 认知段 |
| **MentalModel** | ≤20个心智模型——NPC对世界运行方式的认知图式。★ v1.1: domain 从枚举改为 DomainSignature 涌现 |
| **PatternExpression** ★ v1.1 | 认知系统的数学地基——因果步骤+结构LSH+领域签名。定义在 woworld_core |
| **DomainSignature** ★ v1.1 | u64 领域签名——替代 MentalModelDomain 枚举。领域从原子类型组合涌现 |
| **ThoughtFragment** | 自发思考片段——4种触发(情绪/记忆/环境/社交) + 4种处理模式 |
| **deliberation_depth** ★ v1.1 | 连续深思熟虑深度——trait_capacity × state_capacity × stakes。不硬编码阈值 |
| **predict_outcome()** ★ v1.1 | 将 MentalModel 应用于候选路径的预测——纯函数，不是世界仿真 |
| **counterfactual_regret()** ★ v1.1 | 比较实际结果与模式预测——不需要反事实模拟 |
| **SleepCognitiveProcessing** | 睡眠时认知正则化——梦境=Dropout+突触修剪。自适应荒诞度 |
| **InnovationPipeline** ★ v1.1 | 6阶段统一创新管线——formalize改注册制，默认产物=AcademicWork |
| **CognitiveBiases** | 从CognitiveStyle+EmotionState纯函数派生——不存储，每次查询计算 |
| **学科涌现** ★ v1.1 | 学科不来自枚举。DomainSignature空间聚类 → 学科涌现 |
| **四层记忆压缩** ★ v1.1 | L0 Hot→L1 Cold→L2 Era→L3 Life。信息密度驱动连续压缩率 |
| **修辞化渲染** ★ v1.1 | Creative Leap（跨域类比）× TextGenerator（修辞化渲染）= 比喻 |

## 统一设计原则 ★ v1.2

**基于模式的向前投射**: NPC 心智的唯一原料是已编码经验。评估未发生路径时——查询已有 MentalModel 产生基于模式的向前投射。投射是单查询的（O(log n)，零 World 引用），输出带不确定性（variance/confidence）。附带三问判定标准——详见 [[003-MentalModel与智慧积累#十六-判定标准|003 §十六]]。

| 操作 | 数学变换 | 输入 |
|------|---------|------|
| 归纳 | 事件 → PatternExpression | EventMemory[] |
| 类比 | Pattern×2 → 合成 Pattern | MentalModel[] |
| 预测 | MentalModel → 应用于候选路径 | MentalModel[] + DecisionOption |
| 后悔 | 预测 − 实际 → regret | OutcomePrediction + EventMemory |
| 压缩 | information_density → 保留/丢弃 | EventMemory |
| 抽象 | 压缩层提升 → Era Digest/Life Abstract | 下层记忆 |

## 跨模块接口 ★ v1.1

| 提供 | 消费方 |
|------|--------|
| CognitiveTide 3字段 | → AgentSnapshot.认知段 (08-行动涌现) |
| CognitiveStyle 4维 | → 技能系统(学习效率) · 战斗(战术决策) · 语言(对话风格) |
| MentalModel | → 决策引擎(mental_model_modulation) · 语言(WisdomSharing) · 历史(ThoughtImprint) |
| PatternExpression + DomainSignature ★ | → 概念与语言地基(修辞/概念翻译) · 学科聚类 |
| predict_outcome() ★ | → GOAP (deliberation_depth 的 stakes 评估) |
| 四层压缩 ★ | → 存档系统 (SaveableModule::snapshot_dirty) |
| SleepProcessing | → Life模块(睡眠挂钩) · 历史(梦境叙事) |
| InnovationPipeline ★ | → 全部领域crate(注册制消费) · 书籍著作(默认产物) |

| 消费 | 来源 |
|------|------|
| crowd_emotional_field ★ | ← 感官系统 (PerceptBatch.environment) |
| EmotionalCharge ★ | ← 感官系统 (PerceptEntry.emotional_charge) |
| source_confidence 衰减 ★ | ← 感官系统 (crowd_emotional_field 在编码时调制) |
| NeedsSystem ★ | ← 进阶需求系统 (predict_outcome 的事实→价值映射) |

## 性能

| 项目 | 数值 |
|------|------|
| CognitiveTide::update() | <1μs/NPC——纯数学 (3次 f32 运算) |
| MentalModel 内存 | ≤20×~120B(PatternExpression) ≈ 2.4KB/NPC ★ v1.1 |
| 四层压缩总计 | ~617KB/NPC | ★ v1.1 |
| deliberation_depth() | <1μs/NPC——3次 MentalModel 查询 | ★ v1.1 |
| pattern_similarity() | <1ns —— 2 XOR + 2 popcount | ★ v1.1 |
| SleepProcessing | 离线批处理——不在帧预算内 |
| 学科聚类 | 事件驱动增量——O(n×k), k<100, 不在帧预算内 | ★ v1.1 |

## 设计哲学

- **基于模式的向前投射** ★ v1.2: 认知输入是过去数据。评估未发生路径时——单查询模式应用（非链式仿真），结果带不确定性。三问判定：已有数据？/ 单查询？/ 带不确定性？
- **认知从人格+经历涌现**: CognitiveStyle 不是随机roll——它从 BigFive × wisdom × 经历深度派生
- **认知潮汐驱动行为质量**: cognitive_load↑ → 战斗/魔法/社交/经济所有原子执行噪声增大。高负载 NPC"变笨"不是脚本——是数学
- **睡眠=认知正则化**: 梦境不是装饰——是 Dropout+突触修剪。自适应荒诞度随 cognitive_load 变化
- **创新从参数涌现**: 创新管线不定义"什么是创新"——它定义"创造性飞跃的数学条件"
- **学科从聚类涌现** ★ v1.1: 设计者不规定有哪些学科——NPC 群体的 MentalModel 在 DomainSignature 空间聚类 → 学科形成
- **领域是涌现的** ★ v1.1: 不来自枚举。DomainSignature(u64) 从 AtomTypeTag 组合派生——不同文化对同一组模型做不同聚类 → 不同学科边界

> **最新变更**: [[../../../../Change/CHG-057-NPC认知系统深度设计-20260622|CHG-057]]
> **最新参考讨论**: [[../../../../参考文档/038-NPC认知系统跨模块配合需求-20260622/README|038 跨模块配合需求]]
> **关联**: [[001-认知与智慧系统总纲]] · [[003-MentalModel与智慧积累]] · [[006-创新管线与跨领域对接]] · [[../08-NPC行动涌现与分类/004-AgentSnapshot与连续参数涌现|AgentSnapshot 认知段]]
