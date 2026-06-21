> **开发代号**: WoWorld (Wonder World)
> **引擎**: Godot 4.6.2
> **模块编号**: 06
> **模块名称**: NPC认知与智慧系统
> **父模块**: [[../NPC活人感开发文档ver2.0|NPC活人感模块]]
> **状态**: 🚧 开发中
> **创建日期**: 2026-06-17

---

# 06-NPC认知与智慧系统 · 工作目录

## 正式规格文档

| 编号 | 文档 | 状态 |
|------|------|------|
| [[001-认知与智慧系统总纲]] | 001-认知与智慧系统总纲 | ✅ v1.0 |
| [[002-CognitiveStyle与认知偏误详细设计]] | 002-CognitiveStyle与认知偏误 | ✅ v1.0 |
| [[003-MentalModel与智慧积累]] | 003-MentalModel生命周期 | ✅ v1.0 |
| [[004-思考涌现与浮现机制]] | 004-ThoughtTrigger与ThoughtFragment | ✅ v1.0 |
| [[005-睡眠认知加工与正则化]] | 005-睡眠与梦境 | ✅ v1.0 |
| [[006-创新管线与跨领域对接]] | 006-创新与领域系统 | ✅ v1.0 |
| [[007-跨模块依赖与接口契约]] | 007-接口与契约 | ✅ v1.0 |

## 核心概念

| 概念 | 说明 |
|------|------|
| **CognitiveStyle** | 4维认知风格(直觉-分析/冲动-反思/具象-抽象/顽固-灵活)——从BigFive+wisdom+经历派生 |
| **CognitiveTide** | 3维认知潮汐(cognitive_load/rumination_pressure/mind_quietude)——每决策周期更新，映射至 AgentSnapshot 认知段 |
| **MentalModel** | ≤20个心智模型——NPC对世界运行方式的认知图式 |
| **ThoughtFragment** | 自发思考片段——4种触发(情绪/记忆/环境/社交) + 4种处理模式 |
| **SleepCognitiveProcessing** | 睡眠时认知正则化——梦境=Dropout+突触修剪。自适应荒诞度 |
| **InnovationPipeline** | 6阶段统一创新管线——创造性飞跃从参数组合涌现 |
| **CognitiveBiases** | 从CognitiveStyle+EmotionState纯函数派生——不存储，每次查询计算 |

## 跨模块接口

| 提供 | 消费方 |
|------|--------|
| CognitiveTide 3字段 | → AgentSnapshot.认知段 (08-行动涌现) |
| CognitiveStyle 4维 | → 技能系统(学习效率) · 战斗(战术决策) · 语言(对话风格) |
| SleepProcessing | → Life模块(睡眠挂钩) · 历史(梦境叙事) |
| InnovationPipeline | → 技能系统(技能发现) · 文化(技术传播) |

## 性能

| 项目 | 数值 |
|------|------|
| CognitiveTide::update() | <1μs/NPC——纯数学 (3次 f32 运算) |
| MentalModel 内存 | ≤20×~256B ≈ 5KB/NPC |
| SleepProcessing | 离线批处理——不在帧预算内 |

## 设计哲学

- **认知从人格+经历涌现**: CognitiveStyle 不是随机roll——它从 BigFive × wisdom × 经历深度派生
- **认知潮汐驱动行为质量**: cognitive_load↑ → 战斗/魔法/社交/经济所有原子执行噪声增大。高负载 NPC"变笨"不是脚本——是数学
- **睡眠=认知正则化**: 梦境不是装饰——是 Dropout+突触修剪。自适应荒诞度随 cognitive_load 变化
- **创新从参数涌现**: 创新管线不定义"什么是创新"——它定义"创造性飞跃的数学条件"

> **参考讨论**: [[000-模块大纲与导航|参考文档·设计探讨]]
> **关联**: [[001-认知与智慧系统总纲]] · [[../08-NPC行动涌现与分类/004-AgentSnapshot与连续参数涌现|AgentSnapshot 认知段]]
