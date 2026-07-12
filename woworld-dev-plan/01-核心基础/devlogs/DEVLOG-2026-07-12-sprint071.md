# DEVLOG 2026-07-12 — Sprint-071 V3a 代谢闭环

> **冲刺**: Sprint-071 — V3a 代谢闭环·采集→吃·真实库存交互
> **状态**: ✅ 完成·1111 tests 全绿·clippy/fmt 零警告
> **提案**: [[../../sprint-proposals/sprint-071-V3a-代谢闭环-20260712]]（计划文档: `~/.claude/plans/fancy-moseying-stallman.md`）

## 做了什么

将 `movement.rs` 中 `satisfy_goal` 的魔法需求满足（到达=hunger-=0.5）替换为真实的代谢闭环：到达→采集入库→进食消费→hunger 真实下降+HP 恢复。

### 架构

```
Block A1:  needs_decay（需求累积）
Block A2:  movement_system（FindFood 到达→插 ArrivedAtTarget 标记，不降 hunger）
Block A2.5: harvest_on_arrival_system（读标记→查植被→yield resolver→入库→删标记）
Block A3.5: consume_system（饥饿+有食物→吃→降 hunger+回 HP）★ 独立 cmd，立即 flush
Block A4:  need_evaluation（看到更新后 hunger）→ goal_resolution → action_weight
```

### 产物

| 文件 | 内容 |
|------|------|
| `woworld_core/src/item/mod.rs` | `ConsumableEffect` +`hunger_restore` +`hp_restore` |
| `woworld_core/src/vegetation.rs` | +`PlantYield` struct + `ProductCategory::resolve_plant_yield()`（yield resolver） |
| `woworld_ecs/src/components/goal.rs` | +`ArrivedAtTarget` 标记组件（movement→harvest 解耦） |
| `woworld_ecs/src/systems/npc/movement.rs` | FindFood 到达→插 ArrivedAtTarget（不再魔法降 hunger） |
| `woworld_ecs/src/systems/npc/harvest.rs` | **新** — `harvest_on_arrival_system`（+5 tests） |
| `woworld_ecs/src/systems/npc/metabolism.rs` | **新** — `consume_system`（+6 tests） |
| `woworld_godot/src/terrain_chunk.rs` | +Block A2.5(harvest) +Block A3.5(consume) |
| `woworld/assets/items/test_items.toml` | +4 可采集食物（野莓/野蘑菇/野生坚果/野草药） |

### 审计修复

- **软 bug**：consume_system 和 need_evaluation 共享 cmd → need_evaluation 读到旧 hunger。修复：consume 独立 Block A3.5 立即 flush。

## 测试

**+11 tests**（ecs: harvest 5 + metabolism 6）。1111 tests 全绿。

## 诚实边界

- HARVEST 复合原子（GRASP+CUT+SCOOP+STACK）未实现——直接库存操作
- `ingest_food()` 仅 2/7 字段（hunger+hp）
- 元素瓶颈模型延迟
- TOML 交互配方表延迟（硬编码 yield resolver）
- FindWater/Rest/SafePlace 等仍走原 satisfy_goal

## 设计文档对齐

三轮审计（编码前双 agent + 编码后单 agent）。所有偏差已在代码注释 + 计划 §〇 记录。零意外偏差。

---

> **下游**: V3b 市场接真（order_creation 读真实 Needs/库存盈余）
> **关联**: [[../../02-垂直切片/README]] §4 · 计划文档 `~/.claude/plans/fancy-moseying-stallman.md`
