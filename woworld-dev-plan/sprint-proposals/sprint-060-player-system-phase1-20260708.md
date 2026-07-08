# Sprint-060: 玩家系统 Phase 1 — ControlMode + 夺舍NPC

> **提案日期**: 2026-07-08
> **提案状态**: ⏳ 待审批
> **所属阶段**: Phase 1 — 核心基础（里程碑 1J·ECS 基础设施扩展）
> **关联 CHG**: [[CHG-063-玩家系统新建-20260624]]

## 📋 依赖前提检查

| 前置项 | 状态 | 备注 |
|--------|------|------|
| CHG-063 玩家系统设计 | ✅ 完成 | 6 篇~1,448 行，设计完备无歧义 |
| ECS NPC 系统就位 | ✅ | 22 Components + 28 Systems 就位（BigFive/Emotion/Movement/Goal/Needs/...） |
| EntityRenderer 就位 | ✅ | Sprint-059 交付，NPC 胶囊体+标签渲染正常 |
| 调试控制台就位 | ✅ | F3 控制台，entity selection/info/listnpc 正常 |
| player_ecs_entity 骨架 | ✅ | WorldDriver 已有 `player_ecs_entity: Option<hecs::Entity>` |
| LOD 系统就位 | ✅ | 所有实体入 LOD，处方写回 ECS |
| 物品/经济系统 | N/A | 本冲刺不涉及 |

**结论**: ✅ 所有依赖满足，无阻塞。

## 🎯 目标（3 个）

### 目标 1: ControlMode + PlayerComponent — ECS 类型定义

- **验收标准**: 
  - `woworld_core` 新增 `ControlMode` enum（Auto/Manual/DomainDelegated）+ `ActionDomain` enum（6 域）
  - `woworld_ecs` 新增 `PlayerComponent` + `ControlModeComponent`
  - `PlayerComponent` 标记玩家实体（is_player, active_character_id）
  - `ControlModeComponent` 存储当前控制模式 + manual_domains
  - 所有新类型有测试（≥10 tests）
  - `cargo test --workspace` 全绿
- **涉及模块**: 玩家系统（CHG-063·001/003/006）
- **涉及代码**: `woworld_core/src/player.rs`（新）+ `woworld_ecs/src/components/player.rs`（新）

### 目标 2: PlayerPossessSystem — 夺舍切换 + Position 同步

- **验收标准**:
  - Tab 键在摄像机前方可见 NPC 之间循环夺舍（dot product + 距离排序）
  - 夺舍时：CharacterBody3D 瞬移到目标 NPC 的 ECS Position → player_ecs_entity 切换到该 NPC → 移除 NPC 的 Goal + Wander 组件
  - 退出夺舍（F 键）：恢复 NPC 的 Goal 生成 → player_ecs_entity 清空 → NPC 从当前位置继续 GOAP 自主生活
  - 每帧：CharacterBody3D.global_position 写回被夺舍 NPC 的 ECS Position
  - Debug 控制台 `info` 可查看被夺舍 NPC 的完整 Component（Vitals/Emotion/BigFive/Needs/...）
  - 被夺舍 NPC 在 EntityRenderer 中排除渲染（CharacterBody3D 覆盖视觉），其他 NPC 正常
  - `possess <entity_id>` 控制台命令（自由选择任意实体）
  - 新 System 有测试（≥10 tests）
  - `cargo test --workspace` 全绿
- **涉及模块**: 玩家系统（CHG-063·003 双角色与托管模式）
- **涉及代码**: `woworld_ecs/src/systems/player/possess.rs`（新）+ `woworld_godot/src/terrain_chunk.rs`（集成）

### 目标 3: 输入路由 + 摄像机跟随 — Godot 集成

- **验收标准**:
  - 夺舍后：WASD 输入 → CharacterBody3D.move_and_slide()（和现在一样），结果写回 ECS Position
  - Camera 保持在 CharacterBody3D 子节点（架构不变，无需 reparent）
  - 玩家实体在 LOD 中永远 LOD0（is_player=true 传给 LODCoordinator）
  - `entity_visual_system` 正确排除被夺舍的 NPC（避免重复渲染）
  - 已有 player.gd 保留自由相机模式，F 键退出夺舍回到自由相机
  - Godot 功能门验证：Tab 夺舍 → WASD 移动 → 位置同步 → F 退出 → NPC 恢复 GOAP 移动
  - `cargo test --workspace` 全绿
- **涉及模块**: 玩家系统（CHG-063·001/006）
- **涉及代码**: `woworld_ecs/src/systems/player/` + `woworld_godot/src/terrain_chunk.rs` + `godot/scripts/player.gd`（少量修改）

## 🧪 研究事项

| 问题 | 级别 | 研究计划 | 结果 |
|------|------|---------|------|
| Godot CharacterBody3D.global_position 写回 ECS 的线程安全 | 🟢 | 已知——都在主线程，直接写入 | — |
| hecs Entity 移除 Goal + Wander 后 goal_system 的行为 | 🟡 | goal_system 当前为 stub（NPC spawn 时固定 Goal），后续 Goal 再生由 goal_system 负责 | 冲刺执行中验证 |
| Tab 切换时 CharacterBody3D 瞬移 vs 平滑过渡 | 🟢 | 瞬移（set_global_position）——Phase 1，后续加 fade 效果 | — |

## 📊 决策矩阵（5 维加权）

单一候选，无竞争。玩家系统 Phase 1 是当前最优方向——设计完备、依赖就绪、用户价值最高。

## 📖 必读文档清单

| 文档 | 路径 | 为什么读 |
|------|------|---------|
| 玩家系统总纲 | `WoWorld-Design/.../玩家系统/001-玩家系统总纲.md` | 核心哲学：玩家=NPC+I/O |
| 双角色与托管模式 | `WoWorld-Design/.../玩家系统/003-双角色与托管模式.md` | ControlMode 模型 + 切换规则 |
| 玩家专属 I/O 适配层 | `WoWorld-Design/.../玩家系统/006-玩家专属I-O适配层.md` | 11 系统差异清单 + ~40 输入动作 |
| 接口契约 | `CLAUDE-INTERFACES.md` | 玩家相关 trait/接口 |
| NPC 认知 v1.1 | `WoWorld-Design/.../NPC活人感模块/06-认知与智慧系统/` | GOAP 后台模式参考 |

## 🔌 外部 API 预验证清单

| API | 来源 | 文档验证 | 状态 |
|-----|------|---------|------|
| `hecs::Entity` / `hecs::World` | hecs 0.10 (docs.rs) | Entity 互转已有 `entity_id_from_hecs`/`entity_id_to_hecs` | ✅ |
| `godot::classes::Input` | godot 0.5 crate | 已有使用（天气快捷键/F3 边缘检测） | ✅ |
| `move_and_slide()` | Godot 4.7 CharacterBody3D | 已有使用（player.gd） | ✅ |
| `CharacterBody3D.set_global_position()` | Godot 4.7 Node3D | 夺舍时瞬移 | ✅ 标准 API |

## 🏗️ 架构决策（已推敲裁定）

> 以下三项经综合 10 维工程维度推敲，已确认最优方案。详见推敲过程见本文末尾「架构推敲附录」。

### 决策 1: 玩家物理 — Godot CharacterBody3D + ECS Position 双向同步

**Hybrid 模式**。CharacterBody3D 处理物理（move_and_slide），每帧写回 ECS Position。
利用 movement_system 的 `&Goal` 强制 query 实现零侵入：夺舍时移除 NPC 的 Goal → movement_system 自动跳过。

### 决策 2: 被夺舍 NPC 的 GOAP — 移除 Goal 组件（Phase 1）

**设计对齐**。移除 Goal → movement_system 自动跳过（零查询开销）。Emotion/Needs/Social 等系统全速运转。Phase 2 恢复 Goal + ControlMode filter → GOAP 生成假设动作流（自我叙事合理化）。

### 决策 3: Tab 切换范围 — 摄像机前方可见 NPC

**复用已有数据**。消费 entity_visual_system 的输出（每帧已收集），过滤 render_lod < 2 + camera_forward dot > 0，按面向角→距离排序。零额外查询。

## 📏 预估影响

| 维度 | 预估 |
|------|------|
| 修改文件数 | ~8（woworld_core 1 + woworld_ecs 3 + woworld_godot 2 + player.gd + CLAUDE-INTERFACES.md） |
| 新增代码行 | ~600-800 |
| 新增测试 | ~20-30 |
| 冲刺数 | 1 |
| 风险等级 | 🟡 中（Goal 移除/恢复 + Position 同步初次实现有调试成本） |
| 阻塞其他冲刺 | 否（玩家系统独立模块，不阻塞物品/经济/对话） |

---

## 附录 A：当前架构与交付后对比

### 当前（Sprint-059）—— 核心断裂
```
Godot CharacterBody3D (player.gd) ──独立──> WASD + 自由相机
                                           │
ECS bare player entity (仅3 Component) ──孤立──> 非NPC, 无BigFive/Emotion/...
ECS 20 NPC entities (22+ Component)    ──完整──> Movement/Goal/Emotion/...
                                           │
                                      EntityRenderer (胶囊体+标签)
                                          
问题: player_ecs_entity 指向裸实体, 不是真正的NPC
```

### Sprint-060 交付后 —— "玩家 = NPC"
```
            ┌─── 自由相机模式 (F键) ──> player_ecs_entity = None
            │    和现在一模一样
CharacterBody3D
(player.gd) │                          write-back (每帧)
            │         ┌─────────── ECS Position ◄────────────┐
            └─── 夺舍 ┤                                      │
              (Tab)   │    PlayerComponent (标记)            │
                      │    ControlMode::Manual               │
                      │                                      │
         player_ecs_entity ──→ 某个 NPC entity               │
                               (BigFive/Emotion/Needs/        │
                                Social/Movement/...全保留)     │
                                      │                      │
                                      ├─ movement_system ────跳过 (缺Goal)
                                      ├─ emotion_system ────正常运转
                                      ├─ needs_system ──────正常运转
                                      ├─ social_system ─────正常运转
                                      └─ entity_visual ─────排除 (CharacterBody3D替代)
```

**关键**: 不创建新实体。夺舍 = 切换 `player_ecs_entity` 引用 + 移除 Goal/Wander。
退出 = 恢复 Goal 生成 + 清空 `player_ecs_entity`。零实体创建/销毁。
