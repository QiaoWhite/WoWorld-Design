# DEVLOG — 2026-07-10（角色控制器 Sprint）

## 角色控制器核心三层实现（编码日·920 tests·零回归）

> 基于 2026-07-09 完成的 13 篇设计规格（2,731 行），实施第一个编码冲刺。
> 关联: [[DEVLOG-2026-07-10]]（同日 CHG-068 视距扩展·设计日）

---

## 一、冲刺规划

三阶段质量保证循环审计划——三轮审计修正 16 个计划缺陷后审批通过。

关键设计决策：
- **绞杀者模式**：旧 NPC movement_system 完全不动，新 System 通过 `Without<Movement>` 互斥
- MovementRecoveryStack 用固定数组（零依赖 woworld_core）
- smallvec 仅加 woworld_ecs
- InputAction 枚举留给 ActionResolver sprint

## 二、实施

### Phase A: Core 类型
- `kinematics.rs`: LocomotionMode stub (CHG-067-SHIM), MovementLock, RotationLock, PhysicsRequirement
- `movement.rs`: Stance/Pace/SpecialMode/MovementState/RecoveryStack/Profile/GaitParams
- `action.rs`: ActionId/Params/Phase/Commitment/ActionDef/ActionLifecycleEvent/InterruptSource 等全部动作类型
- `input.rs`: BufferPriority/InputFeelConfig/BufferedInput
- 358→? core tests

### Phase B: ECS Component/Resource/Event
- 8 Component + 3 Resource + EventChannel\<E\> 双缓冲
- 461 ecs tests

### Phase C: ECS Systems (7 个)
- MovementSystem: 加速度积分+摩擦+地形跟随+坡度检测+MovementLock 全四态
- MovementModeSystem: 介质切换+恢复栈
- StaminaGateSystem: 体力门控+冷却禁止冲刺
- ActionController: 承诺中断仲裁+三规则+CommitmentLevel 门控
- ActionSystem: ECS 包装+阶段计时器+取消窗口计时
- InputBufferSystem: 环形缓冲+优先级淘汰
- CoyoteTimeSystem: 土狼时间（必须在 MovementModeSystem 前运行）

### Phase D: TOML
- movement_profiles.toml (humanoid+wolf)
- action_registry.toml (6 actions)
- input_feel.toml

## 三、实现后审计

三路 agent 并行审计发现 ~30 问题。已修复 8 项（2 CRITICAL + 5 HIGH + 1 LOW），其余为已知设计决策和合理 Sprint 边界限制。

## 四、最终状态

| 指标 | 值 |
|------|-----|
| cargo check --workspace | ✅ |
| cargo test --workspace | ✅ 920 tests |
| cargo clippy --workspace | ✅ 零警告 |
| 旧代码破坏 | 零（绞杀者模式） |
| 新文件 | ~22（19 Rust + 3 TOML） |

## 五、未完成（移交下一会话）

1. **Step 5e: ECS 管线集成** — 新 System 接入 WorldDriver::process()
2. **spawn 测试实体** — 带新 Component 的实体验证端到端
3. **Godot 桥接** — 按键→ActionRequest、Position→Transform3D

## 六、入库

待提交——约 25 文件变更，等待交接后统一 commit。

---

## 七、Step 5e 管线集成 + 全量审计（同日续）

> 接续本冲刺，执行 Step 5e 管线集成，并对 12 篇设计规格 vs 实际代码做三层并行审计。

### Step 5e：角色控制器管线接入 WorldDriver

- `WorldDriver` 新增 4 字段（action_registry / movement_profile_registry / action_instance_counter / action_events）+ 冒烟测试实体句柄。
- `ready()` 加载 action_registry.toml + movement_profiles.toml。
- `process()` 新增 **Block A0**，顺序 = `coyote → stamina → movement_mode → input_buffer → action(+mid_phase_flush) → movement`。
- 新增 `tests/step5e_pipeline.rs` 集成测试：完整 6 系统同序跑 60 帧，实体沿 +X 前进 >1m 且贴地。
- 冒烟测试实体（不可见，向东行走 + 每 120 帧打印位置）验证端到端；验证后可删。

### D1 定论：stamina 在 movement_mode **之前**
恢复栈在踩空帧快照 MovementState；stamina 先降级 pace 可避免恢复栈存过期 Sprinting、落地闪 1 帧。文档 002 §六 写的相反序需回补（待用户裁决）。

### 三层审计（3 子代理并行）+ 修复
- **已修（8 项，全绿）**：
  1. 🔴 `windup_ms=0` 动作卡死（去掉 `&& windup_s>0.0` 守卫）
  2. 🔴 cancel_set 存 key 却比中文 `name` → 规则1永不触发（registry 预解析 `cancel_set_ids`）
  3. 🟡 movement_lock 被打断后泄漏（active=None 复位 Free）
  4. 🟡 rotation_lock 解析后从不写入 CMovementControl（加 `rotation_lock_from_def` 传递）
  5. 🟡 Crouching ×0.5 降速未实现（max_speed 分离 Crouching 分支）
  6. 🟡 Treading 体力语义相反（stamina_rate 改读 `treading_stamina_rate`）
  7. 潜伏 bug `input_feel.toml` 键 `ledge_snap_angle_deg`→`ledge_snap_angle`
  8. 3 个 TOML 解析冒烟测试（此前从未被真正解析）+ 重复注释清理
- **测试**：920 → **927 passed / 0 failed**，clippy 零告警，build 通过。

### 登记：延后项（随后续冲刺修，防遗忘）
| 编号 | 问题 | 归属冲刺 |
|------|------|---------|
| A2 | InterruptSource 硬编码（System→全 Staggered，Instinct→全 DodgeCancel），语义失真 | 战斗/ActionResolver |
| A3 | `interrupt_on_move`（interact 应被移动打断）存而不用 | ActionResolver |
| A7 | Active→Recovery 切换帧 cancel_window 误开 1 帧 | 次要，随动作系统完善 |
| M3 | 滑翔 `glide_h/v_speed`+`glide_accel` 未接线（Gliding 复用 jump/ground×0.3） | 滑翔/垂直移动 |
| M4 | `coyote_time_secs` 字段缺失于 MovementProfile，0.15 硬编码 | 手感系统接线 |
| I1-I5 | 缓冲满淘汰 / 消费 pop_if 物理重检 / 落地预输入 / 边缘吸附 / 空闲门控 | 004 ActionResolver 手感 |
| — | Continuous/Charge 运行时（006 全文）、ActionResolver（004 全文） | 已知 sprint 边界 |

### 用户裁决（已执行）
- **A6：是** → EMERGENCY(100·死亡级) System 中断穿透 Locked 承诺（action_controller.rs + 2 测试），濒死者不再卡死在锁定动作。
- **D1：改** → 002 §六 执行顺序订正为 coyote→stamina→movement_mode，附订正说明。已跑 `/woworldidea-design sync`——零跨模块影响（三系统无外部引用），17-模型动作与物理 变更日志追加内部订正条目。

### 入库
Step 5e 集成 + 8 审计修复共约 12 文件变更（叠加本冲刺 25 文件），待统一 commit。
