# Sprint-041: NPC 目标系统 — Desire → Goal 转换

> **提案**: 2026-07-05 | **前置**: Sprint 040 ✅

## 目标

### Goal Component + GoalResolutionSystem

| 交付 | 说明 |
|------|------|
| `Goal` Component | goal_type: GoalType, urgency: f32, target_pos: Option<WorldPos> |
| `GoalType` | FindFood, FindWater, FindRest, Idle |
| `GoalResolutionSystem` | 读 Desire → 选 Goal → cmd: remove Desire, insert Goal |
| 测试 | Desire→Goal 转换, 无 Desire 时 Idle, 优先级验证 |

## 预估

- 1 sprint, ~150 行, 🟢 低风险
