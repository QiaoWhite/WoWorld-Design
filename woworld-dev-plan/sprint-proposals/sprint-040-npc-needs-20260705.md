# Sprint-040: NPC 基本需求系统 — Needs Component + 衰减 + 评估

> **提案日期**: 2026-07-05
> **提案状态**: ⏳ 待审批
> **前置**: ECS Phase 0-1 ✅

## 目标

### 目标 1: Needs + Desire Component

| Component | 字段 | 说明 |
|-----------|------|------|
| `Needs` | hunger:f32, thirst:f32, fatigue:f32 | 3 基本驱动力, 0=满足 → 1=极度缺乏 |
| `NeedSensitivity` | hunger_sens:f32, thirst_sens:f32, fatigue_sens:f32 | 人格派生系数, Sprint 040 用默认值 |
| `Desire` | kind:DesireKind, urgency:f32 | 临时——由 NeedEvaluation 写入, 由 GoalResolution 消费 |

### 目标 2: HungerDecaySystem + NeedEvaluationSystem

- **HungerDecay**: needs.hunger += 0.01/frame, 同理 thirst/fatigue
- **NeedEvaluation**: `urgency = (current - baseline) / (critical - baseline) * sensitivity`, 任意 urgency > 0.8 → cmd.insert(Desire)

### 目标 3: 集成 + 测试

- WorldDriver process() 注册 2 System
- Needs 衰减测试 + Desire 触发测试
- 152 现有测试零回归

## 预估

- **冲刺数**: 1
- **代码量**: ~200 行
- **风险**: 🟢 低
