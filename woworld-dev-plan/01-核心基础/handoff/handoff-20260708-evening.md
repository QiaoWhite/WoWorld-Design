# 会话交接 — 2026-07-08（晚间）

## 💾 恢复点

```bash
cd woworld
cargo check --workspace   # 5 crates, 零错误
cargo test --workspace    # 757/757 ✅
cargo build --workspace   # ✅ DLL 已更新
```

## 本次交付：Sprint-059 + 可视化 Phase 2

### ECS→Godot 数据管线

```
ECS Components ──[entity_visual_system]──> Vec<(Entity, EntityVisual)>
                                               │
                                        ┌──────┴──────┐
                                        ▼              ▼
                                 EntityRenderer   DebugConsole
                                 (CapsuleMesh     (CanvasLayer
                                  +Label3D)       控制台)
```

### 关键接口

```rust
// ── 数据收集 (woworld_ecs) ──
entity_visual_system(&ecs, player_entity, &mut name_cache) -> Vec<(Entity, EntityVisual)>
entity_debug_system(&ecs, entity) -> Option<EntityDebugSnapshot>

// ── EntityRenderer (woworld_godot) ──
renderer.sync(&visuals);
renderer.set_name_visible(bool);
renderer.set_color_enhanced(bool);
renderer.set_player_pos(pos);
renderer.raycast_select(ray_origin, ray_dir) -> Option<hecs::Entity>;
renderer.highlight_entity(entity);
renderer.entity_aabbs() -> Vec<(Entity, Vec3, Vec3)>;

// ── DebugConsole (woworld_godot) ──
console.toggle();          // F3
console.state.name_visible // 持久设置
console.state.selected_entity
console.state.player_pos   // listnpc 距离用

// ── 名字 ──
generate_name(seed) -> NpcName  // 确定性中文名

// ── GDScript ──
WorldDriver.is_console_open()   // player.gd 消费
```

### 控制台命令（8 个）

| 命令 | 功能 | 持久？ |
|------|------|--------|
| `help` | 列出所有命令 | — |
| `nameshow` | 切换头顶名字显示 | ✅ |
| `debugcolor` | 切换情绪→颜色映射 | ✅ |
| `info` | 选中实体全量 Component 数据 | — |
| `listnpc [n]` | 按距离列出 NPC + Goal | — |
| `select <id>` | 按 ID 选中 | ✅ |
| `clear` | 清空输出 | — |
| `entitycount` | 按 EntityKind 统计 | — |

## 修复的 bug

| Bug | 根因 | 修复 |
|-----|------|------|
| 零胶囊体 | LOD 输入只含 Player + 处方不回写 ECS | 全部实体入 LOD + 写回 LodLevel |
| 红色 get_global_transform 报错 | 实体节点先设位置后入树 | 先 add_child 再设位置 |
| 控制台不渲染 | CanvasLayer 布局不生效 | 动态读 Viewport 尺寸 + 绝对定位 |
| 控制台遮挡点击 | Control 拦截鼠标 | MouseFilter::IGNORE |

## 文件变更总览

### 新建 (5)

```
woworld_core/src/entity_visual.rs
woworld_core/src/naming.rs
woworld_ecs/src/systems/entity_visual.rs
woworld_godot/src/debug_console.rs
WoWorld-Design/.../007-调试可视化与EntityRenderer架构.md
```

### 修改 (6)

```
woworld_core/src/lib.rs
woworld_ecs/src/systems/mod.rs
woworld_ecs/src/systems/npc/movement.rs
woworld_godot/src/entity_renderer.rs      (重构)
woworld_godot/src/lib.rs
woworld_godot/src/terrain_chunk.rs        (集成)
godot/scripts/player.gd
```

## 下一步建议

| 方向 | 内容 |
|------|------|
| **玩家系统** | 夺舍 NPC + 控制切换（CHG-063 设计就位） |
| **对话雏形** | NPC-NPC 基础对话模板 |
| **物品 Phase 3** | ItemEntId 迁移 + 制造与配方 |
| **经济 Phase 4** | 多市场 + ProfessionTag + 行为经济学全量 |
