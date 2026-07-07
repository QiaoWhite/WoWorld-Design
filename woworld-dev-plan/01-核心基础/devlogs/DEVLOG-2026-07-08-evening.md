# DEVLOG — 2026-07-08（晚间）

## Sprint-059: NPC 调试可视化 + 通用实体渲染管线 + 可视化 Phase 2

### 基线：737 tests → Sprint-059: 757 → Phase 2: 757 tests（+20 total）

---

## Sprint-059: 数据管线 + EntityRenderer 重构 + Bethesda 控制台

### Part A：数据管线（woworld_core + woworld_ecs）

| 文件 | 内容 | 测试 |
|------|------|------|
| `woworld_core/src/entity_visual.rs` | `EntityVisual` + `EntityDebugSnapshot` + `DebugSection` + `DebugField` | 5 |
| `woworld_core/src/naming.rs` | `NpcName` + 种子→名字生成器（50音节+30姓=1500组合，中文风格） | 5 |
| `woworld_ecs/src/systems/entity_visual.rs` | `entity_visual_system` — 每帧收集(Entity, EntityVisual)对；`entity_debug_system` — 按需收集选中实体全量 Component；`NameCache` 归属 system 内部 | 10 |

### Part B：渲染层（woworld_godot）

| 文件 | 内容 |
|------|------|
| `entity_renderer.rs` | **重构**：消费 `&[(hecs::Entity, &EntityVisual)]` → CapsuleMesh(radius=0.4, h=1.8) + Label3D billboard(头顶2.2m) + Rotation同步 + LOD裁剪(render_lod≥4跳过/≥2无标签/距离>50m无标签) + entity_aabbs() + raycast_select() + highlight_entity() |
| `debug_console.rs` | `DebugConsole` struct + `ConsoleState` + 命令注册表(8命令) + CanvasLayer UI(RichTextLabel+LineEdit+ColorRect) + 命令历史(↑↓) + Viewport 动态尺寸布局 |

### Part C：集成与修复

| 文件 | 内容 |
|------|------|
| `movement.rs` | **修复**：计算 direction 后写入 Rotation Component (Option<&mut Rotation>) |
| `terrain_chunk.rs` | spawn_npc 新增 Rotation::default()；每帧 entity_visual_system → EntityRenderer.sync；DebugConsole 初始化+F3+mouse click raycast+高亮同步+命令队列；LOD 处方写回 ECS；所有实体纳入 LOD 计算器；is_console_open() 暴露 |
| `player.gd` | `_input()` 和 `_physics_process()` 检查 `is_console_open()` → 提前返回 |
| `woworld_core/lib.rs` | +entity_visual, +naming 模块+prelude |
| `woworld_ecs/systems/mod.rs` | +entity_visual 模块 |
| `woworld_godot/lib.rs` | +debug_console 模块 |

---

## 可视化 Phase 2: 鼠标 raycast 选中 + 高亮 + 修复

| 修复项 | 内容 |
|--------|------|
| LOD 不可见 bug | **根因 1**: `LodCoordinatorInput.entities` 只包含 Player——NPC render_lod 永远为 4(不可见)。修复：收集所有 ECS 实体送入 LOD 计算器 |
| LOD 处方不回写 | **根因 2**: `compute_lod()` 结果只存 HashMap 不写入 ECS LodLevel。修复：结果写回 `self.ecs.get::<&mut LodLevel>()` |
| 红色 get_global_transform 报错 | **根因 3**: 实体节点先 `set_global_position` 后 `add_child`。修复：先入树再设位置 |
| 控制台 UI 不渲染 | **根因 4**: CanvasLayer 挂在 Node3D 下 + anchor 系统不生效。修复：绝对定位，`toggle()` 时动态读取 Viewport 尺寸计算比例布局 |
| 控制台遮挡点击 | **根因 5**: 背景/输出 Control 拦截鼠标。修复：`MouseFilter::IGNORE`，仅输入框保持 STOP |
| 鼠标 raycast 选中 | Camera3D → project_ray → AABB slab 交测 → 选中最近命中 → 金色高亮 |
| listnpc 距离修复 | ConsoleState.player_pos 每帧同步，cmd_listnpc 消费 |

---

## 架构亮点

1. **数据中间层**：`EntityVisual` (woworld_core, 引擎无关) — 一条管线服务 EntityRenderer + DebugConsole + 未来 Bark/感官/骨骼
2. **Bethesda 式控制台**：F3 开关 + 8 命令 + 鼠标点击选中实体 + 金色高亮 + 命令历史
3. **PAD→颜色映射**：Emotion.pleasure/arousal/control → RGB 连续色域，非离散分类
4. **LOD 全链路修复**：实体纳入 LOD 计算 → 处方写回 ECS → EntityRenderer 消费 render_lod
5. **Rotation 修复**：movement_system 写朝向，NPC 面向移动方向

## 测试分布

| Crate | 测试数 | 变化 |
|-------|--------|------|
| `woworld_atmosphere` | 26 | — |
| `woworld_core` | 280 | +10 |
| `woworld_ecs` | 393 | +10 |
| `woworld_worldgen` | 58 | — |
| **合计** | **757** | **+20 (737→757)** |

```
cargo check      ✅ 零错误
cargo test       ✅ 757 全绿
cargo build      ✅ DLL 已更新
```

## 控制台实测效果

- F3 → 底部黑框控制台 + 鼠标释放
- `nameshow` → NPC 头顶程序化中文名（持久）
- `debugcolor` → 情绪颜色映射（持久）
- 鼠标点击 NPC → 金色高亮 + ID 输出
- `info` → 选中实体全量 Component（Vitals/Emotion/BigFive/Goal/Needs/Movement/Lifecycle）
- `listnpc` → 按距离排序列出 NPC + 当前 Goal
- `entitycount` → EntityKind 统计
- ↑↓ 命令历史

## 下一步建议

| 方向 | 内容 |
|------|------|
| **玩家系统** | 夺舍 NPC + 控制切换（CHG-063 设计就位） |
| **对话雏形** | NPC-NPC 基础对话模板 + Bark 气泡基础 |
| **物品 Phase 3** | ItemEntId 迁移 + Assembly 完整实现 |
| **经济 Phase 4** | 多市场 + ProfessionTag + 行为经济学全量 |
