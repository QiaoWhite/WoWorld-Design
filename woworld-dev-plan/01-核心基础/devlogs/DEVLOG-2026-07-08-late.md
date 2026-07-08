# DEVLOG — 2026-07-08（深夜）

## Sprint-060: 玩家系统 Phase 1 — ControlMode + 夺舍NPC

### 基线: 757 tests → Sprint-060: 783 tests (+26)

---

## 架构决策

综合 10 维工程维度推敲后，三个关键裁决：

| 裁决 | 方案 | 理由 |
|------|------|------|
| 玩家物理 | Godot CharacterBody3D + ECS Position 双向同步 | 宪法§2 规定仅玩家保留 PhysicsServer3D；利用 movement_system 的 `&Goal` 强制 query 实现零侵入 |
| GOAP 后台 | 移除 Goal（Phase 1） | movement_system 的 query 强制要求 `&Goal`，移除后自动跳过。Emotion/Needs/Social 全速运转。Phase 2 恢复 Goal + ControlMode filter |
| Tab 切换 | 摄像机前方可见 NPC（dot > 0） | 复用 entity_visual_system 输出，零额外查询 |

**核心洞察**: 夺舍 ≠ 创建新实体。夺舍 = 切换 `player_ecs_entity` 引用 + 移除 Goal/Wander。利用已有 ECS query 约束，**零原有 System 修改**。

---

## Part A: 核心类型（woworld_core + woworld_ecs）

| 文件 | 内容 | 测试 |
|------|------|------|
| `woworld_core/src/player.rs` | `ControlMode` (Auto/Manual) + `ActionDomain` (6域, bitmask 预备) + `#[derive(Default)]` | 7 |
| `woworld_ecs/src/components/player.rs` | `PlayerComponent` + `ControlModeComponent` (纯数据, 铁律合规) | 5 |

**设计对齐**: CHG-063 §1.1 规定 `Manual { action_override: Action }`。Phase 1 简化为 `Manual`（无字段）—— `Action` 类型在 woworld_ecs，引入会循环依赖 woworld_core。Phase 2 解决。

## Part B: 夺舍 System（woworld_ecs）

| 文件 | 内容 | 测试 |
|------|------|------|
| `woworld_ecs/src/systems/player/mod.rs` | 模块入口 | — |
| `woworld_ecs/src/systems/player/possess.rs` | `find_possessable_entities` + `possess_entity` + `unpossess_entity` + `sync_player_position` | 15 |

**关键设计**:
- `find_possessable_entities`: 过滤 Creature + 有 NPC 核心Component + 排除当前玩家。dot_product × 距离排序。
- `possess_entity`: 移除 Goal + Wander → 添加 PlayerComponent + ControlModeComponent::Manual。保留所有其他 Component。
- `unpossess_entity`: 移除 PlayerComponent + ControlModeComponent。Goal 由 need_evaluation → goal_resolution 自然恢复。
- `sync_player_position`: 每帧 CharacterBody3D.position → ECS Position。

## Part C: Godot 集成（woworld_godot）

| 文件 | 内容 |
|------|------|
| `terrain_chunk.rs` | +4 fields (bare_player_entity, possession_candidate_index, tab_was_pressed, f_key_was_pressed) + 5 methods (handle_possess_tab, handle_unpossess, handle_unpossess_internal, sync_possessed_position, is_possessing, get_camera_forward, handle_possess_by_entity) |
| `debug_console.rs` | +pending_possess_request 字段 + `possess <entity_id>` 命令 + cmd_possess 函数 |

**操作**:
- **Tab**: 夺舍摄像机前方最近 NPC → 瞬移 CharacterBody3D → 切换 player_ecs_entity
- **F**: 退出夺舍 → 恢复自由相机模式 → NPC 从当前位置恢复 GOAP
- **F3 → `possess <id>`**: 夺舍指定实体（控制台命令）

## 顺手修复

| 问题 | 文件 | 修复 |
|------|------|------|
| Clippy `&*e` | entity_visual.rs | `&*e` → `&e` |
| Clippy collapsible if | terrain_chunk.rs | 合并嵌套 if |
| Clippy derivable_impls | player.rs (ControlMode), player.rs (PlayerComponent) | `#[derive(Default)]` |
| cargo fmt | 全 workspace | 格式化 |

## 测试分布

| Crate | 测试数 | 变化 |
|-------|--------|------|
| `woworld_atmosphere` | 26 | — |
| `woworld_core` | 286 | +6 |
| `woworld_ecs` | 413 | +20 |
| `woworld_worldgen` | 58 | — |
| **合计** | **783** | **+26 (757→783)** |

```
cargo test       ✅ 783 全绿
cargo clippy     ✅ 零警告
cargo build      ✅ DLL 已更新
cargo fmt        ✅ 已格式化
```

## 架构亮点

1. **零侵入**: movement_system、entity_visual_system 未修改——利用已有 query 约束（`&Goal` 强制、player_ecs_entity 排除）
2. **不创建新实体**: 夺舍 = 引用切换 + Component 增删。被夺舍NPC仍然是完整NPC
3. **涌现自然恢复**: 退出夺舍 → need_evaluation (Desire) → goal_resolution (Goal) → movement_system (移动)——单帧内恢复自主生活
4. **10 维工程推敲**: 每个裁决都综合了架构解耦、模块协作、性能、可维护性、数学、模块特色、涌现、社区实践、Godot特性、玩家体验
