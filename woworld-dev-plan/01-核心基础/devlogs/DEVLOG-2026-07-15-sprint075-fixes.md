# DEVLOG: 2026-07-15 — Sprint-075 V6 快照存档·深度调试修复

> **冲刺**: Sprint-075 — V6 快照存档 bug 修复
> **日期**: 2026-07-15
> **阶段**: Phase 2 探针切片「活着的村庄」·进度 `▓▓▓▓▓▓▓▓▓▓ 10/10`
> **状态**: ✅ 全部修复·1147 tests·clippy/fmt 零警告·已提交·待 push

## 做了什么

V6 存档系统实机验证后，发现并修复 6+ bugs + 对 godot-rust 渲染 bug 做绕过处理：

### 存档系统修复（6 bugs）

1. **save 命令无反馈** — `handle_save` 结果仅 `godot_print!`（到 Godot stdout），游戏内控制台永远看不到完成/失败消息。→ 返回值改为控制台 BBCode，追加到 `console.output_lines`。

2. **load 后实体不可见** — `restore_entities` 从 ComponentBag 重建实体，但 `LodLevel` 是瞬态组件不在 Bag 中。`entity_visual_system` 要求 `(Position, EntityKind, LodLevel)` 三元组 → 恢复后的实体缺 LodLevel → 不可见。→ `handle_load` 中 restore 后补 `LodLevel::default()`。

3. **load 后操控丢失** — 玩家实体缺 Block A0 CC 组件束（`CMoveIntent`/`CMovementControl`/`CMovementState`/`CActiveAction`/`CInputBuffer`/`CInputFeelConfig`/`CCoyoteTime` 等 12 个瞬态组件），`movement_system` 跳过 → 玩家输入落空。`bare_player_entity` 错设为 `player_ecs_entity` → `is_possessing()` 永远 false。→ restore 后修复 CC 组件束 + 正确查找裸玩家实体。

4. **load 后操控随机胶囊** — `player_ecs_entity` 从内存中取旧值查 mapping → load 后 entity bits 已变 → 查到错误实体。→ `WorldSnapshot` 新增 `player_entity_bits: Option<u64>` 字段（save 时写入，load 时从快照读出），精确匹配。

5. **load 后颜色变（双重根因）**：
   - **GD-004**：hecs World 重建后 entity bits 重叠（新旧都从 `(1<<32)|0` 开始）→ `EntityRenderer::sync()` 的 `alive.contains_key()` 全部命中 → 旧节点不被移除 → 新实体走 UPDATE 路径（不更新颜色）→ 颜色错误继承。→ `needs_full_rebuild` flag + `sync()` 内部全量重建机制。
   - **GD-003**：godot-rust 的 `set_surface_override_material` 有 bug——CPU 侧材质属性已正确（`get_albedo()` 读回确认），但 GPU 不渲染。→ 改用 `set_material_override`。

6. **`info` 命令显示错误名字** — `entity_debug_system` 直接 `entity.to_bits().get()` 做种子生成名字 → load 后 bits 变 → 名字变。→ 接受 `name_cache` + `stable_ids` 参数，通过 `ConsoleState` 每帧同步。

### 跨 load 身份稳定性（4 项增强）

- **Stable ID**：`EntityDebugSnapshot` 新增 `stable_id` 字段（= old_id_bits），`info` 和 `selected entity` 消息显示 `(Stable: XXXXXXX)`
- **名字稳定**：`handle_load` 中用 `old_id_bits` 预填充 `name_cache`，覆盖 `entity_visual_system` 的 entity bits 默认种子
- **颜色稳定**：`stable_hash_color_from_seed(stable_id)` 替代 `entity_hash_color(entity_bits)` 用于无 Emotion 实体
- **EntityRenderer `clear_nodes`**：load 后强制全量重建，节点颜色用 `stable_id` 正确计算

### 其他修复

- 移除旁观者左键高亮（仅 F3 控制台模式下左键选中生效）
- EntityRenderer 诊断日志静默
- `highlight_entity` 改用 `set_material_override` + 缓存材质 + 稳定色恢复

### 代码审查修复（3 项）

- `highlight_entity`：`set_surface_override`→`set_material_override`，每帧分配材质→缓存，`entity_hash_color`→恢复 `_material`
- Velocity 覆盖：仅实体无此组件时才插入零速
- `player_entity_bits` 回退：`max_by_key` 优先 NPC（非 `.next()`）

### 发现的 godot-rust 材质渲染 bug（GD-003）

`MeshInstance3D::set_surface_override_material(0, &mat)` 调用后：
- `mat.get_albedo()` → `(1.00, 0.00, 0.00)` ✅
- `mat.get_shading_mode()` → `UNSHADED` ✅
- 屏幕渲染：非纯红（粉/绿/紫）❌

`set_material_override(&mat)` 无此问题。不确定 bug 在 godot-rust FFI、Godot 4.7 引擎、还是 Vulkan 驱动层。

## 机械门

- **1147 tests 全绿**（core 412 + worldgen 75 + atmosphere 26 + ecs 614 + save 11 + integration 9）
- 4 ignored（LMDB 测试——Windows path resolution）
- clippy 零警告（`-- -D warnings`）
- fmt 通过
- build 通过（.dll 已更新）

## 提交

- `a813f5e` — Sprint-075 V6 存档系统修复·2 bug 报告
- `b6002a0` — code review fixes（highlight_entity + Velocity + 回退确定性）
- 基线：`74bb334`

## 产物

- `woworld-dev-plan/bugs/Godot桥接/GD-003-set-surface-override-material-not-rendered.md`
- `woworld-dev-plan/bugs/Godot桥接/GD-004-hecs-entity-bits-overlap-after-rebuild.md`
- `woworld-dev-plan/bugs/INDEX.md`（新增两行）
