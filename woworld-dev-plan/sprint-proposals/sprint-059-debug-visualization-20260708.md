# Sprint-059 — NPC 调试可视化 + 通用实体渲染管线

> **提案**: 2026-07-08 | **前置**: 物品 Phase 2 ✅ + 经济 Phase 3 ✅（737 tests）
> **设计文档**: [[007-调试可视化与EntityRenderer架构]]

## 一、动机

21 个 NPC 在 ECS 中拥有完整心智（BigFive/Emotion/Needs/Goal/Inventory/经济），但在 Godot 中表现为无差别的彩色方块——名字、情绪、行为全部不可见。调试涌现系统需要一个 Bethesda 式控制台：选中实体 → 查看完整内部状态。

更重要的是：当前 `EntityRenderer` 直接 query ECS，缺乏数据中间层。这阻塞了未来所有消费实体位置/身份的管线（Bark 气泡、感官 EntityIndex、骨骼动画）。

## 二、交付清单

### Part A：数据管线（woworld_core + woworld_ecs）

| 文件 | 内容 | 测试 |
|------|------|------|
| `woworld_core/src/entity_visual.rs` | `EntityVisual` + `EntityDebugSnapshot` + `DebugSection` + `DebugField` | 5 |
| `woworld_core/src/naming.rs` | 种子→名字生成器（音节拼接，确定性） | 5 |
| `woworld_ecs/src/systems/entity_visual.rs` | `entity_visual_system` + `entity_debug_system` | 12 |

### Part B：渲染层（woworld_godot）

| 文件 | 内容 | 说明 |
|------|------|------|
| `entity_renderer.rs` | **重构**：消费 EntityVisual → CapsuleMesh + Label3D + Rotation + LOD 裁剪 | BoxMesh→CapsuleMesh；消费 render_lod 做可见性裁剪 |
| `debug_console.rs` | `DebugConsole` struct + `ConsoleState` + 命令注册表 + CanvasLayer UI + raycast 选中 + 命令历史 | 8 个首批命令；DebugConsole 是 plain struct（非 GodotClass），与 EntityRenderer 模式一致 |

### Part C：集成与修复

| 文件 | 内容 |
|------|------|
| `woworld_ecs/.../movement.rs` | **修复**：计算 direction 后写入 Rotation Component |
| `woworld_godot/.../terrain_chunk.rs` | spawn_npc 新增 `Rotation::default()`；调用 entity_visual_system；init DebugConsole；暴露 `is_console_open()`；记录 player_entity；边缘检测变量 |
| `player.gd` | 新增 `is_console_open()` 检查——控制台开启时跳过所有输入处理 |
| `woworld_core/src/lib.rs` | 新增 `pub mod entity_visual;` + `pub mod naming;` |
| `woworld_ecs/src/systems/mod.rs` | 新增 `pub mod entity_visual;` |
| `woworld_godot/src/lib.rs` | 新增 `pub mod debug_console;`（struct 非 GodotClass，无需 GDExtension 注册） |

## 三、架构变化

### 新数据流

```
之前: EntityRenderer ──query──> hecs::World ──> Godot Node3D

之后: hecs::World ──[entity_visual_system]──> Vec<EntityVisual>
          │                                       │
          │                                ┌──────┴──────┐
          │                                ▼              ▼
          │                         EntityRenderer   DebugConsole
          │                         (CapsuleMesh     (CanvasLayer
          │                          +Label3D)       控制台)
          │
          └─────[entity_debug_system]──> EntityDebugSnapshot
                                          (info 命令触发)
```

### movement_system 修复

`movement_system` 计算 `direction` 后将朝向写入 `Rotation` Component。这是独立 bug——即使没有可视化，`Rotation` 也是实体空间状态的一部分。

## 四、首批命令（8 个）

| 命令 | 功能 | 关闭控制台后保持？ |
|------|------|------------------|
| `help` | 列出所有命令及帮助 | — |
| `nameshow` | 切换头顶名字显示 | ✅ |
| `debugcolor` | 切换情绪→颜色映射 | ✅ |
| `info` | 打印选中实体全量 Component 数据 | — |
| `listnpc [n]` | 列出最近的 n 个 Creature | — |
| `select <id>` | 按 hecs entity bits 选中 | ✅ |
| `clear` | 清空输出缓冲 | — |
| `entitycount` | 按 EntityKind 统计实体数 | — |

## 五、信息边界

| 显示层 | 名字 | 情绪数值 | HP | BigFive | Goal |
|--------|------|---------|-----|---------|------|
| 头顶（nameshow on） | ✅ | ❌ | ❌ | ❌ | ❌ |
| 控制台 info | ✅ | ✅ | ✅ | ✅ | ✅ |
| 游戏模式（未来） | ⚠️ 仅认识的人 | ❌ | ❌ | ❌ | ❌ |

头顶只放身份标识，数值全部在控制台面板里。

## 六、预估

| 指标 | 值 |
|------|-----|
| 新增代码 | ~1,200 行 |
| 测试 | +22（累计 737→759） |
| 修改文件 | 11 个（5 新建 + 6 修改） |
| 风险 | 🟢 低——纯扩增，不改变现有 API |
| 预计 sprint 数 | 1 |
