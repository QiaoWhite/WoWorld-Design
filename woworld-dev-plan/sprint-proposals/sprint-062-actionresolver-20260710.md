# Sprint-062: ActionResolver 与输入解析（角色控制器 004）

> **提案日期**: 2026-07-10
> **提案状态**: ✅ 已完成（P1-P6 达成，965 tests，Block A0 激活）——见 [[../01-核心基础/handoff/handoff-20260710-sprint062]]
> **所属阶段**: Phase 2 — 垂直切片
> **所属里程碑**: 角色控制器（模型动作与物理系统）

## 📋 依赖前提检查

| 前置项 | 状态 | 备注 |
|--------|------|------|
| 角色控制器核心三层完成（Block A0 管线就位） | ✅ | 2026-07-10 Step 5e，927 tests |
| `ControlMode` / `ActionDomain` 已定义 | ✅ | `woworld_core::player`（Auto/Manual 二态 + 6 域枚举） |
| `CActionRequestBuf` 组件就位 | ✅ | `woworld_ecs::components::action_state`（SmallVec，**是 Component 非 Resource**） |
| `CInputBuffer` + `input_buffer_system` 就位 | ✅ | 已 drain CInputBuffer→CActionRequestBuf，但**无系统填充 entries**——本冲刺补 |
| `CMoveIntent` / `ControlModeComponent` / `possess.rs` 就位 | ✅ | `woworld_ecs` |
| 感官系统（`NearbyInteractables` 数据源） | ❌ | 未建——本冲刺用**可测 Resource 壳**优雅降级，感官系统建好后零返工接线 |
| 装备系统数据（第二层装备映射） | 🟡 | 装备组件存在但未接入 resolver——用 stub 数据源优雅降级（无装备→回退空手） |
| 设计文档无歧义 | 🟡 | 001 总纲 Phase 0/1 顺序与 002 §六 D1 订正不一致（见 §需用户裁决 Q1） |

## 🎯 目标（≤3 个）

### 目标 1: 输入契约 + 意图生成（P1 + P3-a）
- **验收标准**: `cargo test -p woworld_core` 通过；`InputAction` 枚举（~40 动作 5 组）+ `InputState`（帧输入快照，pressed/released/held + move_direction + 相机增量 + `was_pressed`/`was_released`/`is_held` 查询）+ 热键栏映射表定义并测试；最小 `player_input_system`（`InputState.move_direction` + pace 修饰 → `CMoveIntent`）通过单测。
- **涉及模块**: 角色控制器 004 `WoWorld-Design/.../角色控制器/004-ActionResolver与输入解析.md` §一/§五
- **涉及代码**: `woworld_core/src/input.rs`（扩展）；`woworld_ecs/src/systems/player/player_input.rs`（新）

### 目标 2: ActionResolverSystem 六层映射 + 域过滤 + 释放检测（P2 + P3-b）
- **验收标准**: `cargo test -p woworld_ecs` 通过；`action_resolver_system` 实现第一层（直接键 Jump/Dodge/Parry）、第三层（热键栏）、第五层（特殊技能）、第六层（ControlModeToggle）**全实现**，第二层（装备）用 stub 数据源优雅降级；`ControlMode::manual_domains()` 域过滤（Auto→∅ / Manual→全 6 域）；持续动作 `was_released` → RELEASE 请求；即时动作直写 `CActionRequestBuf`，时敏动作（跳跃/闪避/连招）入 `CInputBuffer`。每个分支有单测。
- **涉及模块**: 角色控制器 004 §二/§五/§六；003 ActionController（消费方，不改）
- **涉及代码**: `woworld_core/src/player.rs`（补 `manual_domains()`）；`woworld_ecs/src/systems/input/action_resolver_system.rs`（新）

### 目标 3: 上下文解析 + 管线激活（P4 + P5）
- **验收标准**: `NearbyInteractables` + `Interactable` + `interact_priority`（TOML 12 级）+ `resolve_interact_target`（范围 2m / 前方 120° 锥 / 优先级排序 / 歧义检测→轮盘）+ `ActionWheelData` 壳，用内存候选做单测；管线按 `coyote→stamina→movement_mode→input_buffer→**action_resolver**→action(+flush)→movement` 接入 `WorldDriver::process` Block A0；spawn 玩家测试实体挂全套新组件 + `InputState`/`ControlModeComponent`；集成测试端到端验证 `InputAction→ActionRequest→ActionController` 真实执行（Block A0 从休眠转激活）。
- **涉及模块**: 角色控制器 004 §三/§四/§七；001 总纲 §四 帧内完整管线
- **涉及代码**: `woworld_core`（Interactable 值类型）；`woworld_ecs`（NearbyInteractables/ActionWheelData Resource 壳 + resolve 算法）；`woworld_godot/src/terrain_chunk.rs`（管线接入 + spawn）

## 🧪 研究事项

> 输入解析层为业界成熟模式（六层映射 + 上下文动作 + 动作轮盘），设计文档已明确参照 Elin 中键轮盘方案。无 🟠🔴 级新算法。

| 问题 | 级别 | 研究计划 | 结果 |
|------|------|---------|------|
| 输入缓冲 vs 即时解析的顺序语义 | 🟢 | 对照 001 Phase 0 表 + 现有 input_buffer_system | 已定：input_buffer 先 drain 旧缓冲，resolver 后填新缓冲（1 帧 buffer 延迟，标准跳跃缓冲行为） |
| 歧义检测阈值（第二候选 < 第一 ×1.5 且同优先级→轮盘） | 🟢 | 004 §三已给算法 | 直接实现，参数 Provisional |

## 📊 决策矩阵（5 维加权）

单一候选，无竞争。ActionResolver 是 handoff 定论的 🥇 下一步——Block A0 管线已就位但休眠，唯有输入解析层能把真实请求灌入管线激活它。

## 📖 必读文档清单

| 文档 | 路径 | 为什么读 |
|------|------|---------|
| ActionResolver 规格 | `WoWorld-Design/.../角色控制器/004-ActionResolver与输入解析.md` | 编码依据（六层/上下文/轮盘/域过滤） |
| 角色控制器总纲 §四 | `.../角色控制器/001-角色控制器总纲.md` | 帧内完整管线 System 顺序 + 读写集 |
| 手感系统 §二/§三 | `.../角色控制器/008-手感系统.md` | CInputBuffer 缓冲/淘汰/pop_if 语义 |
| ActionController 规格 | `.../角色控制器/003-ActionController与离散动作.md` | 消费方契约（不改，只确认接口） |
| 玩家系统 003 双角色 | `.../玩家系统/003-双角色与托管模式.md` | ControlMode 域委派语义 |

## 🔌 外部 API 预验证清单

> 全部 std / 已用 crate，无新依赖。

| API | 来源 | 文档验证 | 状态 |
|-----|------|---------|------|
| `SmallVec::push/sort_by` | smallvec（已用） | 现有 input_buffer_system 已用 | ✅ |
| `hecs::World::query_mut` `.with::<>` | hecs 0.10（已用） | 现有系统模式 | ✅ |
| `glam::Vec2/Vec3`（move_direction / 锥体点积） | glam 0.28（已用） | — | ✅ |

## ⚠️ 需用户裁决的事项

| # | 事项 | 选项 | AI 推荐 | 用户裁决 |
|---|------|------|--------|---------|
| Q1 | **001 总纲 Phase 0/1 顺序与 002 §六 D1 订正不一致**：001 line 146-156 仍列 MovementModeSystem 在 Phase 0、StaminaGateSystem 在 Phase 1 早段（旧序）；002 §六 + 实机代码已改为 stamina 先于 movement_mode、coyote 最先。 | A: 本冲刺 P6 顺手同步 001 使其对齐 002/代码（机械对齐，D1 已裁决生效）并跑 `/woworldidea-design sync`；B: 不动，记入已知问题留待专门文档保鲜 | **A**——D1 已生效，001 是唯一未同步的滞后文档，代价极小（改 1 表 + sync） | ✅ **A（2026-07-10 裁决）**——P6 顺手同步 001 并跑 sync |
| Q2 | **`NearbyInteractables` / `Interactable` 值类型归属**：最终 owner 是感官与知觉系统（未建）。 | A: 值类型（Interactable/interact_priority）入 `woworld_core`、容器 Resource 壳入 `woworld_ecs`（沿用 ID 类型 woworld_core 归属先例，感官系统建好后共享零返工）；B: 全部临时放 `woworld_ecs`，感官系统建好再迁 | **A**——符合"共享值类型入 woworld_core"架构原则，避免未来迁移 | ✅ **A（AI 按架构惯例决定）** |

> **审查闭环（2026-07-10）**: 计划 ↔ 004/001/002 规格 ↔ 代码全量交叉核对完成。遗漏 A1/A2、不吻合 B1/B2 已在本提案解决；文档瑕疵 D2（CMoveIntent 注释）P3 顺手订正、D3/D4 确认无 Rust 缺口；Q1/Q2 已裁决。无遗留待解决项。

## 📏 预估影响

| 维度 | 预估 |
|------|------|
| 修改文件数 | ~10（新增 3-4 系统文件 + 扩展 input.rs/player.rs + terrain_chunk 接入 + TOML） |
| 新增代码行 | ~900-1100（含测试） |
| 新增测试 | ~60-75（927 → ~1000） |
| 冲刺数 | 1 |
| 风险等级 | 🟢 低（成熟模式，绞杀者隔离，Block A0 已验证管线骨架） |
| 阻塞其他冲刺 | 否（激活 Block A0 后解锁 NPC/玩家迁移 + Continuous/Charge 006） |

## 🧱 分阶段执行（P1→P6）

| 阶段 | 交付 | 依赖 |
|------|------|------|
| P1 | `InputAction` 枚举 + `InputState`（帧快照 + 查询）+ 热键栏映射（woworld_core） | — |
| P2 | `action_resolver_system` 六层映射（1/3/5/6 全实现，2 优雅降级）→ 即时 CActionRequestBuf + 时敏 CInputBuffer（woworld_ecs） | P1 |
| P3 | `ControlMode::manual_domains()` 域过滤 + 释放检测 + **最小 `player_input_system`**（InputState→CMoveIntent，Movement 域不经 resolver）（core+ecs） | P2 |
| P4 | `NearbyInteractables`/`Interactable`/`interact_priority`(TOML) + `resolve_interact_target` + `ActionWheelData` 壳（core+ecs） | P1 |
| P5 | 管线接入 `WorldDriver::process` Block A0（input_buffer→**action_resolver**→action）+ spawn 玩家测试实体 + 端到端集成测试（ecs+godot） | P3,P4 |
| P6 | 机械门（build/test/clippy）+ 同步 001 顺序（Q1=A 时）+ DEVLOG + Handoff + memory | P5 |

## 🔒 关键架构约束（沿用绞杀者模式）

- ActionResolver **只写** `CActionRequestBuf`/`CInputBuffer`/`ActionWheelData`——不碰体力/魔力/冷却（ActionController 的事）、不改 MovementState（MovementModeSystem 的事）。
- Movement 域走 `player_input_system` 直写 `CMoveIntent`，**不经** resolver（004 §五）。
- NPC **不经** resolver——GOAP 直写同一 `CActionRequestBuf`，ActionController 按优先级仲裁不分来源（004 §七）。
- 新系统 query 带 `With<PlayerComponent>`——绞杀者：旧 20 NPC 无新组件不受影响。
- `CActionRequestBuf` 被 input_buffer/action_resolver/action **三系统顺序写**——Block A0 非并行执行，无数据竞争，与现有 input_buffer+action 模式一致（设计门 #10 顺序写例外，已文档化）。
- 装备/感官数据用 stub 数据源接口——真实系统建好后零返工接线。

## 🚫 明确不做（留待后续冲刺）

- Godot `input_bridge.gd` 真实按键映射（下一冲刺——本冲刺 Rust 侧 + 测试驱动闭环）
- 装备/感官系统真实接线（各自系统冲刺）
- Continuous/Charge 运行时（006）、动作轮盘 Godot 渲染（UI 冲刺）
- 缓冲淘汰完整逻辑 / pop_if 物理重检 / 落地预输入 / 边缘吸附（I1-I5，手感系统接线冲刺）
