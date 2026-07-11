# DEVLOG: 2026-07-11 — Sprint-066 手感系统运行时（I1-4）

> **冲刺**: Sprint-066 — 手感系统运行时（角色控制器 008）
> **阶段**: Phase 2 — 垂直切片

## 今日目标
- [x] I1 缓冲满容量优先级淘汰（`push_bounded`）
- [x] I2 过期清理 + pop_if 物理重检 drain（`input_buffer_system` 重写）
- [x] I3 落地预输入（由 I2 涌现）+ 端到端集成测试
- [x] I4 边缘吸附（仅玩家）
- [x] 机械门 + 交接

## 做了什么
- **编码前审计**：逐条把提案对照 008/003/004/002 + 代码，修 6 项（详见提案 §审计修正）。最关键——证伪了"I5 已隐式满足"的初判（`current.is_none()` 只门控**接受**不门控 drain），并纠正 I2 的 loco 来源（position+terrain+coyote，非 CMovementState）。Q3/Q4 呈用户裁决 → I5 剥离、ledge 仅玩家。
- **单一权威 loco**：抽 `base_locomotion` + `resolve_effective_loco` 进 `woworld_core::kinematics`，把 movement_mode/action_system/movement_system(死代码) 三处 byte-identical 的私有 `compute_locomotion` 收成一处。顺带删了 movement_mode 里被丢弃的 `_prev_loco` 冗余 raycast。
- **I1/I2**：`CInputBuffer::push_bounded`（满容量淘汰最低优先级）；`input_buffer_system` 重写为「过期清理 → 解算 effective_loco → 优先级排序 → 只 drain 物理可行条目（复用 `physics_req.is_satisfied_by`）」。
- **I3**：不写独立着地特判——空中 Jump 因 physics_req=Grounded 不满足留缓冲，落地 loco 转 Grounded 即 drain。写了 `sprint066_feel.rs` 驱动真 `input_buffer→action_system`（ActionController）验证空中缓冲→落地起跳 + 过期丢弃。
- **I4**：`apply_ledge_snap`（raycast + 坡度门，008 §七），`With<CInputFeelConfig>` 玩家门控。纯函数 3 场景 + 一个 wiring/gating 测试（玩家吸附 vs NPC 不吸附）。

## 遇到的问题
- **签名变更连锁**：`input_buffer_system` 加 `terrain/registry/now` 三参 → 两个集成测试（step5e_pipeline / sprint062_actionresolver）call 点编译失败 → 各补参数（step5e 加了 `elapsed` 累计游戏秒作 `now`）。
- **新 Component 字段炸结构体字面量**：`CInputFeelConfig` 加 ledge 字段 → coyote_time_system 测试的字面量构造缺字段 → 补 `..Default::default()`。
- **unused import 清理**：删私有 `compute_locomotion` 后 `LocomotionMode`/`WorldPos`/`Vec3` 在 3 个文件变悬空 → 有的删 import、有的移进 test 模块。cargo check 逐个揪出。

## 学到的东西
- **"隐式满足"要证伪不能证真**：I5 表面看 `current.is_none()` 门控就够了，深挖才发现 drain 与 accept 是两层、`request_buf.clear()` 才是丢数据的元凶。审计价值全在这种反直觉处。
- **消解重复顺手做，但先验等价**：`compute_locomotion` 逐字节相同才敢合并；movement 要的是 base（土狼窗内物理上仍腾空），action 要的是 effective（放宽物理门）——拆成两个函数各取所需，不硬合成一个带标志位的。
- **实机激活 ≠ 代码完成**：I1-3 因玩家有 CInputBuffer 而真上线（空中跳跃现在落地触发）；I4/coyote 因玩家缺 CInputFeelConfig/CCoyoteTime 而 inert。验收要查"真实体有没有挂对组件 + 真 TOML flag"，不能只看单测绿。

## 会话后半段：候选A 接线 + 诚实降级
- **候选A**：给 Godot 玩家 spawn 补挂 `CInputFeelConfig` + `CCoyoteTime`，代码路径全通，写集成测试 `coyote_grace_jump_after_walking_off_edge`（用 SkyVoid mock 强制 airborne）跑通 coyote→input_buffer→action 全链。顺带发现并消解 coyote_time_system 里**第 4 处** compute_locomotion（Sprint-066 主体只收了三处，handoff 已订正）。tests 1046→1047。
- **用户一句话点醒**："整个世界都是曲线，没有棱角。"——查 `is_walkable`(terrain.rs:428)：`on_surface(|y-h|<1m)` + 坡度<45°。平滑 Perlin 高度场下 `movement_system` 每帧把 y 贴回地表、无 >1m 断崖、陡坡是"留原地"，`is_walkable` **永不因走路 flip false**。所以 coyote-jump 与 I4 边缘吸附**实机不可触发、不可验**——我之前"已激活"的说法过头了。
- **诚实降级**（不改代码，只改措辞）：handoff/CLAUDE.md/代码注释统一改标"**已接线·休眠**（无棱角地形不可触发，待体素碰撞移动解锁）"，并在 §🚀 新增候选 E（体素碰撞移动里程碑）作为真开关。保留接线（零风险、就绪待解锁），不 overclaim。

## 今日学到（补）
- **代码通 ≠ 世界里能玩**：土狼跳/边缘吸附是"平台跳跃"手感，需要断崖/悬垂/台阶几何；平滑高度场根本没有触发条件。机制建好但世界喂不进输入 = 休眠。验收"实机可玩"必须连**世界几何能否产生触发态**一起查，不能只查组件挂对。

## 明日计划（下一会话）
- [x] Sprint-066 I1-4 + 候选A 全部提交推送（末端 `1db25a9`）
- [ ] 🥇 候选 B：I5 空闲门控（忙碌时缓冲保留 + combo cancel_window 交互设计）
- [ ] 🥈 候选 C：玩家接 Vitals + Block 键位（持续/充能动作实机可玩）
- [ ] 🔭 候选 E（大件）：体素碰撞移动——一次点亮土狼跳/边缘吸附/多地形手感
