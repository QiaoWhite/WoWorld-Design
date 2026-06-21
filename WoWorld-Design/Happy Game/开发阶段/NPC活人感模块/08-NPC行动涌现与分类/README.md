# 08-NPC行动涌现与分类 — 模块入口

> **开发代号**: WoWorld (Wonder World)
> **引擎**: Godot 4.6 LTS
> **开发者**: 独立游戏开发者（Solo）
> **版本**: v1.0
> **状态**: 设计讨论阶段
> **最后更新**: 2026-06-18
> **关联大纲**: [[../../../参考文档/034-NPC行动涌现与物理原子层大纲-20260618/README|034-NPC行动涌现与物理原子层大纲]]

## 模块定位

本模块是 **NPC 活人感模块** 的子模块（08），定义 WoWorld 中 NPC 与世界交互的**通用物理语法**。

**核心使命**: 极少数物理基元 × 材料属性 × 连续参数 = 无限行为涌现。不做"NPC 能做什么"的清单——做一套让所有行为自然涌现的规则引擎。

## 子文档索引

| 编号 | 文档 | 内容 |
|------|------|------|
| [[001-NPC行动涌现总纲]] | 001 | 三层原子架构总览、AgentSnapshot 定义、年龄涌现矩阵、核心原则 |
| [[002-物理原子层定义与签名]] | 002 | 35 个物理基元的完整签名、参数、返回值、音频/感官副作用 |
| [[003-领域复合原子与模块注册]] | 003 | ~40 个领域复合原子的定义、物理原子编排序列、模块注册机制 |
| [[004-AgentSnapshot与连续参数涌现]] | 004 | AgentSnapshot 完整字段定义、与生命周期/身体状态/认知的派生关系 |
| [[005-知识种子与历史引擎集成]] | 005 | KnowledgeSeed TOML、历史引擎传播、第一代知识起源问题的完整解答 |
| [[006-技能精度与原子执行噪声]] | 006 | execution_noise() 函数、技能→精度连续映射、高/低技能 NPC 行为差异 |
| [[007-碰撞箱战斗与IK动画管线]] | 007 | 武器/身体碰撞体、轨迹扫掠、IK 解析解、零预设动画的战斗视觉 |
| [[008-MaterialProperties与物品系统对接]] | 008 | MaterialDef 注册表、物品引用模式、对装配树和锻造配方的影响 |
| [[009-性能预算与LOD联动]] | 009 | 碰撞/IK/非Rigid武器的 CPU 预算、LOD 协调器 7 维约束 |
| [[010-跨模块接口与数据合同]] | 010 | 与其他模块的双向接口清单、trait 签名、数据合同 |
| [[011-关键问题回应与设计论证]] | 011 | 用户核心提问的完整记录与设计回应 |

## 三层原子架构

```
Layer 1: 物理基元 (35个)     — MOVE/GRASP/STRIKE/ATTACH/IGNITE/OBSERVE...
    ↓ 纯物理计算。零领域知识。STRIKE 不区分"战斗攻击"和"锻打铁锭"和"砸矿石"
Layer 2: 领域复合原子 (~40个) — HARVEST/CRAFT/BUTCHER/DISASSEMBLE/TRADE/PRAY...
    ↓ 编排多个物理基元序列。由各领域模块注册 (003-补充-农业执法海洋复合原子注册.md)
Layer 3: 抽象行动 (~25个)     — GOAP 可见的 ActionCandidate
    ↓ GOAP 规划器产出 → composite_atom.execute() → physical_atom.execute()
```

## AgentSnapshot v1.1 速查

| 段 | 字段数 | 内容 | ~bytes |
|----|--------|------|--------|
| 身体 | 7 | strength/dexterity/stamina/health_penalty/mobility/fatigue/hunger | 28 |
| 认知 ★v1.1 | 8 | skill_compressed(u64)/mental_model_count/cognitive_damping/planning_horizon/literacy/cognitive_load/rumination_pressure/mind_quietude | 34 |
| 社交 | 4 | legal_capacity/reputation_flags/social_standing/religious_participation | 16 |
| 生命周期 | 4 | developmental_phase/age_ratio/fertility_potential/gompertz_mortality | 16 |
| 环境 | 3 | wetness/temperature_exposure/intoxication | 12 |
| **总计** | **26** | **单 cache line (128B) 友好** | **~108** |

## 性能

| 项目 | 数值 |
|------|------|
| 物理原子执行 | ≤0.45ms/帧 (含碰撞扫掠+IK+MaterialProperties+PBD) |
| AgentSnapshot 更新 | <0.1ms/帧 (26个 f32 插值——从上游字段批量派生) |
| 复合原子编排 | O(N) where N=物理原子序列长度 (通常 3-8) |
| 内存 (100K NPC) | AgentSnapshot ~10.8MB SoA + 碰撞数据 ~0.5MB |

## 与其他模块的关系

本模块消费、被消费的双向关系详见 [[010-跨模块接口与数据合同]]。主要消费方：NPC活人感、战斗、魔法、经济、文化、信仰、生命、模型动画。主要被消费方：物品系统（MaterialProperties）、技能系统（execution_noise）、生命周期系统（AgentSnapshot参数）、天气系统（环境参数）。
