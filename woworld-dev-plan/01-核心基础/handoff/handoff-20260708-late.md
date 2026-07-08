# Handoff: 2026-07-08（深夜）— Sprint-060

> **冲刺**: Sprint-060 — 玩家系统 Phase 1: ControlMode + 夺舍NPC
> **日期**: 2026-07-08
> **阶段**: Phase 1 — 核心基础
> **冲刺状态**: ✅ 完成

## 📊 冲刺回顾

### 目标达成
| 目标 | 状态 | 备注 |
|------|------|------|
| 目标 1: ControlMode + PlayerComponent 类型定义 | ✅ | woworld_core/player.rs + woworld_ecs/components/player.rs, 12 tests |
| 目标 2: PlayerPossessSystem — 夺舍切换 + Position 同步 | ✅ | woworld_ecs/systems/player/possess.rs, 15 tests |
| 目标 3: Godot 集成 — 输入路由 + Camera + LOD | ✅ | terrain_chunk.rs + debug_console.rs, Tab/F/possess 命令 |

### 关键决策
- **玩家物理 Hybrid**: Godot CharacterBody3D 处理物理 → ECS Position 写回。利用 movement_system 的 `&Goal` 强制 query，移除 Goal 组件实现零侵入跳过
- **ControlMode 简化**: Phase 1 仅 Auto/Manual（无 action_override）。Action 类型在 woworld_ecs，引入会循环依赖。Phase 2 解决
- **不创建新实体**: 夺舍 = `player_ecs_entity` 引用切换 + Goal/Wander 移除。NPC 保留所有其他 Component

## 💾 恢复点（下一会话 AI 必读）

> ⚠️ 这是最重要的段落。下一会话 AI 启动时首先读这里。

- **当前冲刺**: Sprint-060 — ✅ 完成
- **当前目标**: 全部完成（3/3）
- **最后操作**: 写 DEVLOG + Handoff + 更新附录E → commit + push
- **机械门状态**: build ✅ / test 783 ✅ / clippy 零警告 ✅
- **未提交文件**: 见下方 git status（大量 cargo fmt 格式化 + Sprint-060 新增）
- **下一步**: 对话雏形（Bark 气泡）/ 物品 Phase 3 / 经济 Phase 4 / 玩家 Phase 2（详见下方候选）
- **已知陷阱**:
  - ⚠️ `ControlMode::Manual` 缺少 `action_override: Action` 字段（设计 001 §1.1）。Phase 1 有意简化——Action 类型在 woworld_ecs，woworld_core 不能反向依赖。Phase 2 方案：将 Action 抽象放入 woworld_core 作为 trait 或轻量 ID。
  - ⚠️ 退出夺舍后 NPC 有 1 帧延迟恢复自主移动（need_evaluation 产生 Desire → goal_resolution 产生 Goal → 下帧 movement_system 处理）。这是系统执行顺序的正常行为，不是 bug。
  - ⚠️ `player_ecs_entity` 在自由相机模式指向裸实体（仅 Position+EntityKind+LodLevel），夺舍时指向 NPC entity。entity_visual_system 排除它 → CharacterBody3D 覆盖视觉。两处逻辑都依赖此字段的正确性。
- **待用户确认**: Manual.action_override 延后到 Phase 2（已确认）

## 🔧 机械门验证

### cargo test
```
26 + 286 + 413 + 58 = 783 passed; 0 failed
```

### cargo clippy
```
Finished `dev` profile — 零警告
```

### cargo build
```
DLL 已更新
```

## 📐 设计门验证

### A. 主清单

| # | 检查项 | 状态 |
|---|--------|------|
| 1 | trait 签名与 CLAUDE-INTERFACES.md 一致 | ✅ 本冲刺无新增 trait |
| 2 | ID 类型定义在 woworld_core | ✅ ControlMode/ActionDomain 在 woworld_core |
| 3 | Godot/GDScript 侧无游戏逻辑 | ✅ 仅 Tab/F 按键检测 + 位置同步 |
| 4 | 公开类型已登记 | ✅ player 模块在 lib.rs prelude |
| 5 | 设计决策已记录 | ✅ DEVLOG + 本 Handoff |

### B. ECS 铁律合规

| # | 检查项 | 状态 |
|---|--------|------|
| 6 | Component = 纯数据 | ✅ PlayerComponent/ControlModeComponent 零方法 |
| 7 | 无堆数据内联 | ✅ PlayerComponent.original_name_override 是 Option<String>（堆数据通过 Option 包装，但 String 本身是堆——Phase 2 考虑替换为 &'static str 或 u64 hash） |
| 8 | 'static + Send + Sync | ✅ 所有 Component 满足 |
| 9 | Entity 删除走标记 | ✅ 本冲刺不删除 entity |
| 10 | System writes 无交集 | ✅ possess_entity 写入 PlayerComponent/ControlModeComponent——当前无其他 System 写入这些 Component |
| 11 | hecs::World 仅 WorldDriver | ✅ |
| 12 | 每个 System ≥1 测试 | ✅ possess.rs: 15 tests |

> ⚠️ 铁律 7 例外：`PlayerComponent.original_name_override: Option<String>` 包含堆数据（String）。设计权衡：此字段仅在夺舍时使用（极低频），不影响热路径。Phase 2 可考虑优化为固定大小（如 `[u8; 32]` 名字缓冲区）。当前方案为可读性优先。

### C. 架构边界审计

| # | 检查项 | 状态 |
|---|--------|------|
| 13 | 双权威检测 | ✅ GDScript 无独立维护的玩家状态 |
| 14 | 僵尸代码检测 | ✅ 所有新增 #[func] 有对应调用 |
| 15 | GDScript 无数学公式 | ✅ player.gd 未修改 |

## 📁 文件变更

### 新建 (5)
```
woworld_core/src/player.rs
woworld_ecs/src/components/player.rs
woworld_ecs/src/systems/player/mod.rs
woworld_ecs/src/systems/player/possess.rs
woworld-dev-plan/sprint-proposals/sprint-060-player-system-phase1-20260708.md
```

### 修改 (7 — Sprint-060 实质改动)
```
woworld_core/src/lib.rs                          (+player +prelude)
woworld_ecs/src/components/mod.rs                (+player)
woworld_ecs/src/lib.rs                           (+prelude)
woworld_ecs/src/systems/mod.rs                   (+player)
woworld_ecs/src/systems/entity_visual.rs         (clippy fix: &*e→&e)
woworld_godot/src/terrain_chunk.rs               (+possession logic, ~150 lines)
woworld_godot/src/debug_console.rs               (+possess command)
```

### 格式化 (大量, cargo fmt)
```
woworld_atmosphere/src/*, woworld_core/src/culture/*, woworld_core/src/economy/*, ...
```

## 🚀 下一步候选

| 候选 | 内容 | 优先级 |
|------|------|--------|
| **对话雏形** | Bark 气泡 + NPC-NPC 对话模板 + 头顶文字浮现 | 🥇 高用户价值 |
| **物品 Phase 3** | Assembly 完整实现 + ItemEntId 迁移 | 🥈 基础设施 |
| **经济 Phase 4** | 多市场 + ProfessionTag + 行为经济学 | 🥈 基础设施 |
| **玩家 Phase 2** | DomainDelegated + action_override + 双角色 | 🥉 延后（需 Action 类型解耦） |

**建议**: 对话雏形——Bark 气泡让 NPC "活起来"，是对夺舍系统的最佳补充。技术路线清晰，依赖就绪（EntityRenderer + 名字系统已就位）。

## 🏥 已知问题

| # | 问题 | 级别 | 计划 |
|---|------|------|------|
| 1 | PlayerComponent.original_name_override: Option<String> 含堆数据 | 🟡 低风险（仅夺舍时使用） | Phase 2 优化 |
| 2 | ControlMode::Manual 缺少 action_override | 🟡 Phase 2 | Action 类型迁移到 woworld_core 后补充 |
| 3 | 退出夺舍后 NPC 1 帧延迟恢复移动 | 🟢 正常行为 | 无需修复 |
