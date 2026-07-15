# Handoff: 2026-07-15 — Sprint-075 V6 存档系统深度修复

> **会话类型**: 调试修复 · **日期**: 2026-07-15
> **阶段**: Phase 2 探针切片「活着的村庄」·进度 `▓▓▓▓▓▓▓▓▓▓ 10/10`
> **状态**: ✅ 全部修复·1147 tests·已提交·待 push·待用户验证

## 📊 本会话做了什么

1. **实机验证 V6 存档系统**——进入游戏用 `save 1`/`load 1` 测试，发现多个 bug
2. **诊断+修复 6 bugs**——存档反馈、实体不可见、操控丢失、随机胶囊、颜色变化、info 名字
3. **发现 godot-rust 渲染 bug**——`set_surface_override_material` 材质设值但不渲染（绕过后颜色正确显示）
4. **跨 load 身份稳定性**——Stable ID、名字稳定、颜色稳定、EntityRenderer 全量重建
5. **代码审查**——Agent 审查 `74bb334..a813f5e`，发现 2 关键+3 重要+5 次要问题，修复 3 项
6. **Bug 报告**——GD-003（材质不渲染）、GD-004（entity bits 重叠），写入 `bugs/` 目录

## 💾 恢复点（下一会话 AI 必读）

> ⚠️ 下一会话按 `00-流程总览` **场景 B（冲刺执行）** 启动——**验证修复有效性**。

- **当前阶段**: Phase 2 切片·`10/10`（全部修复完成）
- **机械门状态**: `1147 tests 全绿`（core 412 + worldgen 75 + atmosphere 26 + ecs 614 + save 11 + integration 9），4 ignored，clippy/fmt 零警告，build 通过。
- **提交状态**: ✅ 已提交，**待 push**。基线 = `74bb334`。本次 2 commits：
  - `a813f5e` — Sprint-075 V6 存档系统修复·2 bug 报告（10 files, +554/-97）
  - `b6002a0` — code review fixes（2 files, +57/-16）
- **A1 铁律**: 纯涌现，禁脚手架/假坐标/占位驱动/平行 trait/语音决策 silo。
- **💡 新增重要知识**:
  - `godot-rust 0.5` 的 `set_surface_override_material` 有 bug——材质 CPU 属性正确但 GPU 不渲染。全局改用 `set_material_override`。
  - hecs World 销毁重建后 entity bits 完全重叠（都从 `(1<<32)|0` 开始）→ EntityRenderer 的 HashMap key 命中旧 key → `to_remove` 为空。必须主动清空节点缓存。
  - 颜色设计：有 Emotion → PAD 情绪色；无 Emotion → `stable_hash_color_from_seed(stable_id)` 确定性哈希色。`debugcolor` 命令的 `color_enhanced` 切换目前由 `is_gray` 启发式替代。

### 关键代码改动位置

| 文件 | 改动 |
|------|------|
| `woworld_save/src/snapshot.rs:32` | `player_entity_bits: Option<u64>` — 跨 load 玩家定位 |
| `woworld_godot/src/terrain_chunk.rs:2096-2430` | `handle_load` — CC 修复 + LodLevel + name_cache + stable_id_map + clear_nodes |
| `woworld_godot/src/terrain_chunk.rs:1991-2100` | `handle_save` — 写入 player_entity_bits + 诊断 |
| `woworld_godot/src/entity_renderer.rs:76-84` | `sync` — `needs_full_rebuild` 全量重建机制 |
| `woworld_godot/src/entity_renderer.rs:228-270` | `highlight_entity` — set_material_override + 缓存材质 |
| `woworld_godot/src/entity_renderer.rs:269` | `create_node` — `set_material_override` 替代表面材质 |
| `woworld_godot/src/entity_renderer.rs:37-41` | `EntityNode._material` — 持有材质引用 |
| `woworld_ecs/src/systems/entity_visual.rs:92-105` | `entity_debug_system` — 接受 name_cache + stable_ids |
| `woworld_core/src/entity_visual.rs:16-64` | `EntityVisual.stable_id` — 跨 load 颜色种子 |
| `woworld_godot/src/debug_console.rs:65-67,83,479-483` | `ConsoleState` — name_cache_snapshot + stable_id_snapshot |

### 🐛 Bug 索引新增（2 条）

| ID | 症状 | 根因 |
|----|------|------|
| GD-003 | 材质设值但不渲染 | `set_surface_override_material` GPU 输出 ≠ CPU 属性 |
| GD-004 | load 后颜色继承错误 | hecs 重建 entity bits 重叠 → EntityRenderer 复用旧节点 |

## 📦 产物

- **DEVLOG**: `devlogs/DEVLOG-2026-07-15-sprint075-fixes.md`
- **Handoff**: 本文件
- **Bug 报告**: `bugs/Godot桥接/GD-003-*.md` · `bugs/Godot桥接/GD-004-*.md`
- **Bug 索引**: `bugs/INDEX.md`（更新）

## ⏭️ 下一步

- 用户实机验证 load 后颜色/名字/操控稳定性
- `push` 到 GitHub
- 根据验证结果决定下一步（代码清理 / color_enhanced 死代码 / `EntityNode._material` 是否是真正必需 / 冲刺收尾）
