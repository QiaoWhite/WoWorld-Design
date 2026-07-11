# Handoff: 2026-07-11 — Sprint-066 手感系统运行时（角色控制器 008 · I1-4）

> **冲刺**: Sprint-066 — 手感系统运行时（缓冲淘汰 / 物理重检 / 落地预输入 / 边缘吸附）
> **日期**: 2026-07-11
> **阶段**: Phase 2 — 垂直切片
> **冲刺状态**: ✅ 完成（I1-4 达成 + 计划↔文档审计修正，1046 tests，clippy 零警告，**未提交**）

## 📊 冲刺回顾

### 目标达成
| 目标 | 状态 | 备注 |
|------|------|------|
| 1 (I1+I2): 缓冲淘汰 + pop_if 物理重检 | ✅ | `push_bounded` 容量4优先级淘汰 + `input_buffer_system` 过期清理 + 物理重检 drain（复用 `physics_req.is_satisfied_by`）；抽 `resolve_effective_loco` 消解三处重复 |
| 2 (I3): 落地预输入着地帧消费 | ✅ | I3 完全由 I2 涌现——空中 Jump 留缓冲，落地 loco→Grounded 即 drain 起跳；端到端集成测试（真 ActionController）+ 过期路径 |
| 3 (I4): 边缘吸附（仅玩家） | ✅ | `apply_ledge_snap` raycast + 坡度门（008 §七）；`With<CInputFeelConfig>` 玩家门控（Q4=A）；纯函数 3 场景 + wiring/gating 测试 |
| — (I5): 空闲门控 | ⏭️ | 审计确认真缺口但有取舍（Q3=A）——**单独立项**，不在本冲刺 |

### 关键决策
- **单一权威 loco 解算**：抽 `woworld_core::kinematics::{base_locomotion, resolve_effective_loco}`，消解 movement_mode_system / action_system / movement_system(死代码) **三处** byte-identical 私有 `compute_locomotion` 重复。base=位置→loco；effective=base+土狼 grace 上调。
- **物理重检单一权威**：I2 的 `is_physically_possible` **复用** `physics_req.is_satisfied_by(effective_loco)`（与 action_controller 接受路径同源），不自造第二套判定——规避双权威。
- **缓冲窗口权威在 action registry**：`CInputFeelConfig` **不加** `jump_buffer_secs`（审计 F1）——窗口是 per-action `def.buffer_window_ms`，已写入 `BufferedInput.expires_at`。
- **I3 无独立特判**：落地消费不挂 `CJustLanded`——`effective_loco` 转 Grounded 令缓冲 Jump 转合法即 drain，是 I2 的自然涌现（审计 F5）。
- **边缘吸附仅玩家**：`With<CInputFeelConfig>` 门控（Q4=A）；纯高度场下与 `height_at` 落点重合（近乎无操作），待体素碰撞移动接管后 raycast 命中体素边缘几何才有可见效果。

## 💾 恢复点（下一会话 AI 必读）

> ⚠️ 下一会话 AI 启动首先读这里。

- **当前冲刺**: Sprint-066 — 已完成（I1-4 + 审计修正），**未提交**（工作区有改动）。
- **机械门状态**: build ✅ / test ✅ **1046 passed** / clippy ✅ 零警告 / fmt ✅ clean
- **上次提交**: `796afb9`（handoff-sprint065 审计追踪待办）。本冲刺**尚未提交**。
- **本冲刺改动清单**（未提交）:
  ```
  woworld_core/src/kinematics.rs                   (+base_locomotion/resolve_effective_loco +5 测试)
  woworld_ecs/components/input_state.rs            (CInputBuffer::push_bounded + CInputFeelConfig 补 ledge 字段 + 测试)
  woworld_ecs/systems/input/input_buffer_system.rs (重写：新签名 terrain/registry/now + 过期 + 物理重检 drain + 测试)
  woworld_ecs/systems/input/action_resolver_system.rs (push→push_bounded)
  woworld_ecs/systems/input/coyote_time_system.rs  (测试 ..Default::default() 适配新字段)
  woworld_ecs/systems/action/action_system.rs      (用 resolve_effective_loco，删私有 compute_locomotion)
  woworld_ecs/systems/movement/movement_mode_system.rs (用 base_locomotion，删私有拷贝 + 冗余 _prev_loco)
  woworld_ecs/systems/movement/movement_system.rs  (删死代码 compute_locomotion_mode + apply_ledge_snap + query 加 CInputFeelConfig + 测试)
  woworld_ecs/tests/sprint066_feel.rs              (新增·I3 端到端集成)
  woworld_ecs/tests/{sprint062_actionresolver,step5e_pipeline}.rs (适配新签名)
  woworld_godot/src/terrain_chunk.rs               (input_buffer_system 调用点补参数)
  文档: sprint-066 提案 / 本 handoff / DEVLOG
  ```
- **下一步**: Sprint-066 已闭环但**未提交**——先跑一遍机械门确认，`git add` + commit + push。下一冲刺候选见 §🚀。
- **已知陷阱 / 待接线**:
  - ⚠️ **I4 边缘吸附 + M4 土狼时间在实机不激活**：Godot 玩家实体（`terrain_chunk.rs:652+`）**未挂 `CInputFeelConfig` / `CCoyoteTime`**——I4 ledge snap 不触发（且纯高度场本就无操作），M4 coyote 用 fallback 0.15。激活需在玩家配方补这两组件（**会一并打开 coyote-jump 行为**，是 input/玩家配方定稿冲刺的决定，本冲刺不擅动）。
  - ✅ **I1-I3 在实机已激活**：玩家有 `CInputBuffer`，真 `[action.jump]` bufferable=true/physics_req=Grounded → 空中按跳跃现在**留缓冲、落地起跳**（此前被 drain+clear 丢弃）。这是本冲刺可见的手感修复。
  - ⚠️ `input_buffer_system` 签名变更（+`terrain`/`registry`/`now`）——新调用点必须传这三个；`now` 是累计游戏秒（过期基线），非 dt。
  - ⚠️ I5 空闲门控**未做**：控制器忙碌时 input_buffer 仍 drain + `request_buf.clear()` 丢弃非取消类缓冲输入（连招走 cancel_window 中断路径不受影响）。单独立项。

## 🔧 机械门验证

### cargo test（真实输出）
```
TOTAL PASSED: 1046   (core 402 + worldgen 58 + atmosphere 26 + ecs 558 + 集成 2 + godot 0)
（净 +20：kinematics +5 / input_state +5 / input_buffer +2 / movement ledge +4 / sprint066_feel +2 / 其余适配）
```

### cargo clippy
```
cargo clippy --workspace -- -D warnings → Finished（零警告）
```

### cargo fmt --check
```
FMT CLEAN
```

### cargo build --workspace
```
Finished `dev` profile（.dll 已更新）
```

## 📐 设计门验证（15 项）

### A. 主清单
| # | 检查项 | 状态 |
|---|--------|------|
| 1 | trait 签名与 CLAUDE-INTERFACES.md 一致 | ✅ 未改跨模块 trait；`input_buffer_system` 是函数签名变更（非 trait） |
| 2 | ID 类型定义在 woworld_core | ✅ 未新增 ID；`base_locomotion`/`resolve_effective_loco` 加在 core kinematics |
| 3 | 无 Godot/GDScript 游戏逻辑 | ✅ terrain_chunk.rs 仅调用点补参，无逻辑 |
| 4 | 公开类型登记术语表 | ✅ 附录A 追加 `base_locomotion`/`resolve_effective_loco`/`push_bounded`/ledge 字段 |
| 5 | 架构决策记录 | 🟡 单一权威 loco 解算记于本 handoff §关键决策（实现级，未单独立 ADR） |

### B. ECS 铁律
| # | 检查项 | 状态 |
|---|--------|------|
| 6 | Component 纯数据零方法 | 🟡 `CInputBuffer::push_bounded` 是**缓冲入队策略**（栈上 SmallVec 操作，无 ECS 查询）——同 `CEquippedSkills::get`/`MovementState::max_speed` 既有先例，非行为逻辑 |
| 7 | 无堆数据内联 | ✅ 未改 Component 字段的堆性（CInputBuffer 仍 SmallVec[4]） |
| 8 | Component 'static+Send+Sync | ✅ CInputFeelConfig 补 2 个 f32 字段，仍纯值 |
| 9 | Entity 删除标记+统一清理 | ✅ 无 despawn |
| 10 | System writes 无交集 | ✅ ledge snap 复用 movement_system 现有 Position 写路径（无新增写 Position 的 System）；input_buffer 写 CInputBuffer+CActionRequestBuf（与 065 顺序写例外一致） |
| 11 | hecs::World 仅在 WorldDriver | ✅ 未泄漏 |
| 12 | 每 System 至少 1 测试 | ✅ input_buffer 6 / movement ledge 4 / kinematics helper 5 / 集成 2 |

### C. 架构边界
| # | 检查项 | 状态 |
|---|--------|------|
| 13 | GDScript 无独立模拟参数 | ✅ 未触碰 GDScript |
| 14 | #[func] 有调用 | ✅ 未增 #[func] |
| 15 | GDScript 无数学公式 | ✅ 未触碰 |

## 🔍 计划↔文档审计修正（编码前，已落提案 §审计修正）

对照 008 全九节 + 003/004/002 + 代码勘查，编码前修正 6 项：F1 双权威（删 jump_buffer_secs）/ F2 loco 来源（position+terrain+coyote 非 CMovementState）/ F3 谓词复用单一权威 / F5 I3 解耦 CJustLanded / F4 I5 真缺口呈裁（Q3=A 剥离）/ F6 ledge 玩家门控呈裁（Q4=A）。详见提案。

## ⚠️ 已知问题
| # | 问题 | 级别 | 计划 |
|---|------|------|------|
| 1 | I5 空闲门控未做（忙碌时缓冲输入丢失，连招除外） | 🔵 结构性 | 单独立项（Q3=A） |
| 2 | I4 ledge snap + M4 coyote 实机未激活（玩家缺 CInputFeelConfig/CCoyoteTime） | 🔵 结构性 | input/玩家配方定稿冲刺（含 coyote-jump 决定） |
| 3 | ledge snap 纯高度场下与 height_at 重合（近无操作） | 🟢 | 体素碰撞移动接管后自然生效 |
| 4 | 文档↔代码命名漂移（008 `max_snap_*`/`CPlayerTag` vs 代码 `ledge_snap_*`/`PlayerComponent`） | 🟢 | doc-sync（非本冲刺） |

## 🚀 下一步候选
| 候选 | 依赖前提 | 优先级 |
|------|---------|--------|
| A: 玩家实体接入 CInputFeelConfig + CCoyoteTime（激活 I4/M4/coyote-jump） | 决定是否开 coyote-jump | 🥇 |
| B: I5 空闲门控（忙碌时缓冲保留，含 combo cancel_window 交互设计） | 003 ActionController 忙碌语义 | 🥈 |
| C: 玩家接 Vitals + Block 键位（持续/充能动作实机可玩） | input_bridge InputMap | 🥉 |
| D: A2 InterruptSource 语义（Instinct→非全 DodgeCancel） | 战斗中断上下文 | — |

**建议**: 🥇 A（补玩家手感组件，让本冲刺 + Sprint-065 M4 的机制真正上线）或 🥈 B（把 I5 想清楚闭环手感缓冲）。

---

> **上游**: [[handoff-20260711-sprint065]] §🚀 候选 A + §🔎 审计追踪待办
> **提案**: [[../../sprint-proposals/sprint-066-手感系统运行时-20260711]]
> **设计依据**: [[../../../WoWorld-Design/Happy Game/开发阶段/模型动作与物理系统/角色控制器/008-手感系统]]
