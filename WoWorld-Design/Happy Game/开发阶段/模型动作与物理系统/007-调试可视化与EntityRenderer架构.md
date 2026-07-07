# 007 — 调试可视化与 EntityRenderer 架构

> **开发代号**: WoWorld (Wonder World)
> **状态**: 开发规格 v1.0
> **日期**: 2026-07-08
> **依赖**: [[001-模型动作与物理系统总纲|模型动作总纲]] · [[../UI与UX系统/README|UI/UX 系统]] · [[../玩家系统/001-玩家系统总纲|玩家系统]] · [[../感官与知觉系统/README|感官与知觉系统]]
> **被引用**: [[README|模型动作 README]] · [[../../技术栈方案/001-WoWorld正式技术栈方案v3|技术栈方案 v4.0]]

---

## 〇、模块定位

本文档定义 WoWorld 的 **ECS→Godot 可视化管线**——实体如何从 ECS Component 数据转化为屏幕上的可见表现。它同时服务两条消费路径：

1. **调试可视化**（当前冲刺）：开发者观察模拟状态的工具——Bethesda 式控制台 + 头顶标识
2. **游戏渲染**（未来冲刺）：最终角色的完整表现——骨骼动画 + 面部图集 + Bark 气泡

两条路径共享同一套**数据管线基础设施**，但信息边界和渲染策略完全不同。

### 核心设计原则

1. **ECS 是唯一权威**。Godot 侧不持有任何游戏状态的副本。
2. **纯数据中间层**。ECS Component → 中间 struct（引擎无关）→ Godot 渲染。中间层可单元测试。
3. **调试与游戏分离**。调试层可以"违反"信息边界（显示 NPC 内部状态），但必须能完全关闭。
4. **通用实体**。不区分 NPC/生物/植物/掉落物——所有 `Position` + `EntityKind` 的实体平等进入管线。
5. **Bethesda 式命令驱动**。调试功能通过控制台命令控制，不设"模式切换"按钮。

---

## 一、架构总览

### 1.1 数据管线分层

```
┌─ woworld_ecs (hecs::World) ────────────────────────────────────┐
│                                                                  │
│  entity_visual_system(world, player_entity) → Vec<EntityVisual> │
│  entity_debug_system(world, entity)        → EntityDebugSnapshot│
│                                                                  │
└──────────────────────────┬───────────────────────────────────────┘
                           │ GDExtension 函数调用
                           ▼
┌─ woworld_godot (Godot 4.7) ─────────────────────────────────────┐
│                                                                  │
│  EntityRenderer (struct)                                        │
│    ├── CapsuleMesh × N     — 人形占位体（半径 0.4m, 高 1.8m）    │
│    ├── Label3D × ≤100      — billboard 名字标签（仅近处）        │
│    ├── Rotation 同步        — 朝向运动方向                        │
│    └── LOD 可见性裁剪       — render_lod≥4 跳过, ≥2 无标签       │
│                                                                  │
│  DebugConsole (struct)                                           │
│    ├── 命令注册表            — HashMap<&str, CommandEntry>       │
│    ├── CanvasLayer + RichTextLabel + LineEdit — 控制台 UI        │
│    ├── 实体选中              — Camera3D raycast → AABB 交测      │
│    ├── ConsoleState          — 调试设置（含 hecs::Entity 选中）   │
│    └── 命令队列              — WorldDriver::process() 消费        │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

### 1.2 Crate 归属

```
woworld_core          — EntityVisual, EntityDebugSnapshot, DebugSection (引擎无关)
woworld_ecs           — entity_visual_system, entity_debug_system (仅依赖 core + hecs)
woworld_godot         — EntityRenderer, DebugConsole (GDExtension)
```

`woworld_core` 不依赖 Godot——未来换引擎时数据中间层零改动。

### 1.3 帧内执行顺序

```
WorldDriver::process(delta):
  1. ECS tick（全部 System: emotion/needs/movement/social/age/economy/...）
  2. entity_visual_system()      → Vec<EntityVisual>
  3. EntityRenderer.sync(&visuals) → 更新 Godot 节点树
  4. DebugConsole.tick(delta)    → 处理输入/渲染
  5. 消费控制台命令队列           → execute_command() → 操作 ECS World + 输出
```

### 1.4 EntityRenderer API（重构后）

```rust
impl EntityRenderer {
    /// 创建渲染器，在 `world_root` 下建 "Entities" 容器节点
    pub fn new(world_root: &mut Gd<Node3D>) -> Self;

    /// 每帧调用：消费 EntityVisual 切片，同步 Godot 节点树
    /// - render_lod ≥ 4 → 跳过（无 Node3D）
    /// - render_lod ≥ 2 → CapsuleMesh 无 Label3D
    /// - 距玩家 >50m → 额外裁剪 Label3D
    pub fn sync(&mut self, visuals: &[EntityVisual]);

    /// 设置头顶名字可见性（由 nameshow 命令调用）
    pub fn set_name_visible(&mut self, visible: bool);

    /// 设置情绪颜色增强（由 debugcolor 命令调用）
    pub fn set_color_enhanced(&mut self, enhanced: bool);

    /// 为 raycast 选中提供实体 AABB 列表
    pub fn entity_aabbs(&self) -> &[(hecs::Entity, Vec3, Vec3)];
}
```

---

## 二、数据合同

### 2.1 EntityVisual — 头顶渲染数据

```rust
/// 实体头顶渲染的最小数据集（每实体每帧生成）
#[derive(Debug, Clone)]
pub struct EntityVisual {
    /// 世界空间位置
    pub position: Vec3,
    /// 世界空间朝向
    pub rotation: Quat,
    /// 显示名（调试用，种子生成；游戏模式下为空字符串）
    pub display_name: String,
    /// RGB 颜色暗示（从情绪/状态派生）
    pub color_hint: [f32; 3],
    /// 实体种类
    pub kind: EntityKind,
    /// 渲染 LOD 等级: 0=全细节, 4=不可见
    pub render_lod: u8,
}
```

### 2.2 EntityDebugSnapshot — 控制台详情数据

```rust
/// 单个实体的完整调试快照（按需生成，仅选中实体）
#[derive(Debug, Clone)]
pub struct EntityDebugSnapshot {
    pub entity_bits: u64,
    pub kind: EntityKind,
    pub display_name: String,
    pub position: Vec3,
    pub sections: Vec<DebugSection>,
}

#[derive(Debug, Clone)]
pub struct DebugSection {
    pub title: String,          // "Vitals", "Emotion", "BigFive", "Goal"
    pub fields: Vec<DebugField>,
}

#[derive(Debug, Clone)]
pub struct DebugField {
    pub label: String,          // "HP"
    pub value: String,          // "85.3 / 100.0"
    pub color_hint: Option<String>,  // BBCode 颜色标签，如 "#ff6666"
}
```

### 2.3 生成规则

- `entity_visual_system`: 每帧运行，遍历所有含 `(Position, EntityKind, LodLevel)` 的实体。排除 Player 的 hecs Entity。从 `LodLevel.render_lod` 读取裁剪等级。从 `Emotion` 的 PAD 三维计算 `color_hint`。维护内部 `NameCache: HashMap<hecs::Entity, String>`——首次遇到实体时调用名字生成器并缓存，后续直接返回缓存值。
- `entity_debug_system`: 仅当控制台命令（如 `info`）请求时运行。对单个 entity 做 match EntityKind → 查询全部关联 Component → 格式化为 DebugSection 列表。
- `NameCache` 归属于 `entity_visual_system`（在 `woworld_ecs` 中），确保名字生成的确定性逻辑可单元测试，且 Godot 侧不持有名字状态。

---

## 三、调试控制台

### 3.1 设计参考

Bethesda（Skyrim/Fallout）控制台模式——命令驱动、可扩展、效果可持久。与 Minecraft F3 的固定覆盖层模式不同——命令系统允许按需查询，扩展性好。

### 3.2 开关与交互

- **F3 键**：WorldDriver 边缘检测。按下→控制台显示，鼠标释放。再按→控制台隐藏，鼠标捕获恢复。
- **控制台开启时**：LineEdit 获取键盘焦点。`player.gd` 通过 `WorldDriver.is_console_open()` 检查并停止所有玩家输入处理（WASD/镜头/跳跃）。
- **命令提交**：Enter 键。WorldDriver 通过边缘检测捕获→读取 LineEdit 文本→压入命令队列→清空 LineEdit。
- **命令历史**：↑↓ 键在历史中导航。WorldDriver 边缘检测→回填 LineEdit 文本。
- **实体选中**：控制台开启时鼠标点击 = Camera3D raycast → 对所有实体的 AABB 做 slab 交测 → 选中最近命中。

### 3.3 UI 布局

```
CanvasLayer (layer=128, 全屏)
└── VBoxContainer (锚定底部 40%）
    ├── RichTextLabel (输出区，可滚动，BBCode)
    │   └── "> nameshow\n[Console] Name display: ON\n> "
    └── HBoxContainer (输入行)
        ├── Label (">" 提示符)
        └── LineEdit (命令输入)
```

输出区最大保留 500 行——超出则裁剪旧行。

### 3.4 首批命令（8 个）

| 命令            | 参数      | 功能                                                                     | 持久？ |
| ------------- | ------- | ---------------------------------------------------------------------- | --- |
| `help`        | —       | 列出所有命令及一行帮助                                                            | —   |
| `nameshow`    | —       | 切换头顶名字显示。控制台关闭后保持                                                      | ✅   |
| `debugcolor`  | —       | 切换情绪→颜色映射（关闭时用 hash 颜色）                                                | ✅   |
| `info`        | —       | 打印选中实体的所有 Component 数据                                                 | —   |
| `listnpc`     | [count] | 列出所有 Creature 实体：名字 + ID + 距离 + 当前 Goal。距离从 ECS Player 实体的 Position 计算 | —   |
| `select`      | `<id>`  | 按 hecs entity bits 选中实体                                                | ✅   |
| `clear`       | —       | 清空控制台输出缓冲                                                              | —   |
| `entitycount` | —       | 按 EntityKind 分组统计实体数量                                                  | —   |

### 3.5 命令注册表

```rust
// woworld_godot/src/debug_console.rs

type CommandFn = fn(args: &[&str], state: &mut ConsoleState, world: &EcsWorld) -> String;

struct CommandEntry {
    func: CommandFn,
    help: &'static str,
}

struct CommandRegistry {
    commands: HashMap<String, CommandEntry>,
}
```

新增命令 = 定义函数 + `registry.insert("cmd", CommandEntry { func, help })`。三行。

### 3.6 控制台状态

`ConsoleState` 定义在 `woworld_godot`（含 `hecs::Entity` 类型，无法放入引擎无关的 `woworld_core`）。

```rust
// woworld_godot/src/debug_console.rs
pub struct ConsoleState {
    /// 头顶名字可见（nameshow 切换）
    pub name_visible: bool,
    /// 情绪颜色增强（debugcolor 切换）
    pub color_enhanced: bool,
    /// 当前选中的实体
    pub selected_entity: Option<hecs::Entity>,
    /// 输出缓冲（最多 500 行）
    pub output_lines: Vec<String>,
    /// 命令历史
    pub command_history: Vec<String>,
    /// 历史导航位置
    pub history_cursor: usize,
}
```

---

## 四、信息边界

### 4.1 两层边界

| 数据 | 头顶标签（调试模式） | 控制台 `info` | 游戏渲染（未来） |
|------|-------------------|-------------|----------------|
| 名字 | ✅（仅近处） | ✅ | ⚠️ 仅认识的 NPC（从声誉涌现） |
| 情绪→颜色 | ✅（debugcolor 开启时） | ✅ 数值 | ❌ 从步态/表情涌现 |
| HP/Vitals | ❌ | ✅ | ❌ |
| Goal | ❌ | ✅ | ❌ |
| BigFive | ❌ | ✅ | ❌ |
| 库存 | ❌ | ✅ | ❌ |
| 钱包余额 | ❌ | ✅ | ❌ |

**原则**：头顶只放身份标识。数值全部在控制台面板里。信息边界的关键差异：调试模式可以看 NPC 内部状态，游戏模式只能看外部可观察的表现。

### 4.2 玩家实体处理

ECS 中有一个 Player 实体（hecs Entity #0，仅 `Position + EntityKind::Creature + LodLevel`）。Godot 侧有独立的 `CharacterBody3D` 玩家角色。`entity_visual_system` 接受 `player_entity: Option<hecs::Entity>` 参数——Player 实体**不**生成 EntityVisual，避免重复渲染。

---

## 五、性能预算

### 5.1 热路径（每帧）

| 操作 | 21 NPC | 1000 NPC | 备注 |
|------|--------|----------|------|
| entity_visual_system | ~3μs | ~50μs | 一次 hecs query |
| CapsuleMesh Node3D 更新 | ~15μs | ~500μs | set_global_position × N |
| Label3D 渲染 | ≤21 draw calls | ≤50 draw calls | 距离 >50m 或 render_lod ≥2 时跳过 |
| AABB raycast 交测 | — | — | 仅控制台开启 + 鼠标点击时 |
| **合计** | **~20μs** | **~550μs** | 占 16.7ms 帧预算的 3.3% |

### 5.2 冷路径（按需）

| 操作 | 耗时 | 触发 |
|------|------|------|
| entity_debug_system | ~20μs | `info` 命令 |
| 名字生成（首次） | ~2μs | 新实体首次渲染 |
| 命令解析 + 执行 | ~5μs | Enter 提交 |

### 5.3 Label3D 裁剪规则

- 距玩家 >50m → 不创建/隐藏 Label3D
- render_lod ≥ 2（远距 imposter 级）→ 不创建/隐藏 Label3D
- render_lod ≥ 4（不可见）→ 不创建任何 Node3D
- 最多同时 100 个 Label3D——超出时优先隐藏最远实体

### 5.4 内存

| 资源 | 21 NPC | 1000 NPC |
|------|--------|----------|
| EntityVisual Vec | ~8KB | ~200KB |
| Node3D 树 | ~42 节点 | ~1000 节点（LOD 裁剪后 ~200） |
| Label3D 节点 | ≤21 | ≤100（阈值裁剪） |
| ConsoleState | ~10KB | ~10KB |

---

## 六、名字生成（临时方案）

### 6.1 当前实现

种子→确定性音节拼接。音节库约 50 个双音节+三音节片段。`generate_name(seed: u64) -> NpcName { given, family }`。同一种子→同一名字。

### 6.2 未来迁移

当 `CultureCoreParams` 的空间分布就位后，名字生成器消费文化参数（音节频率分布、姓氏结构、词序规则）。接口预留：

```rust
// 当前
fn generate_name(seed: u64) -> NpcName;

// 未来
fn generate_name(seed: u64, culture: &CultureCoreParams) -> NpcName;
```

---

## 七、跨模块接口预留

### 7.1 消费 EntityVisual 的管线

| 消费方 | 消费字段 | 触发 | 模块 |
|--------|---------|------|------|
| EntityRenderer | 全部 | 每帧 | 模型动作（本模块） |
| Bark 气泡 tracker | position → Camera3D.unproject_position() | 每帧 | UI/UX 系统 |
| EntityIndex 更新 | position → AABB | 每帧 | 感官与知觉系统 |
| LODCoordinator 输入 | position + kind | 每帧 | LOD 协调器 |
| 骨骼动画管线 | position + rotation → 骨骼矩阵管线入口 | 每帧 | 动画系统（未来） |

### 7.2 变更影响链

- `movement_system`：**新增** Rotation Component 写入（同步于本冲刺修复）
- `spawn_npc`：**新增** `Rotation::default()` 插入（movement_system 写入的前提——实体必须已有 Rotation Component）
- `player.gd`：**新增** `is_console_open()` 检查——控制台开启时停止输入处理
- `woworld_core/src/lib.rs`：**新增** `pub mod entity_visual;` + `pub mod naming;`
- `woworld_ecs/src/systems/mod.rs`：**新增** `pub mod entity_visual;`
- `woworld_godot/src/lib.rs`：**新增** `pub mod debug_console;`（DebugConsole 是 struct 非 GodotClass，无需 GDExtension 注册）

---

## 八、Godot 4.7 技术备忘

### 8.1 使用的内置类型

| 类型 | 用途 |
|------|------|
| `CapsuleMesh` | 人形占位体（PrimitiveMesh 子类，4.0+内置） |
| `Label3D` | billboard 名字标签（`billboard = Enabled`） |
| `CanvasLayer` | 控制台 2D 叠加层（`layer = 128`） |
| `RichTextLabel` | BBCode 控制台输出 |
| `LineEdit` | 命令输入 |
| `Camera3D.project_ray_origin/normal` | 实体选中射线 |

### 8.2 输入路由方案

**不使用信号连接**（godot-rust 0.5.x 中 Rust→Godot signal 样板代码多）。采用 `Input::is_key_pressed()` + 边缘检测，与现有 `debug_weather_cooldown` 模式一致。

关键按键边缘检测变量（WorldDriver 新增）：
- `f3_was_pressed: bool` — F3 控制台开关
- `enter_was_pressed: bool` — 命令提交
- `up_was_pressed: bool` / `down_was_pressed: bool` — 命令历史导航
- `mouse_left_was_pressed: bool` — 实体选中（配合 raycast）

### 8.3 Player 输入阻断

WorldDriver 暴露 `#[func] fn is_console_open(&self) -> bool`。`player.gd` 在 `_input()` 和 `_physics_process()` 开头检查并提前返回。鼠标模式由 WorldDriver 通过 `Input::singleton().set_mouse_mode()` 直接控制。

### 8.4 Camera3D 引用获取

实体选中 raycast 需要 Camera3D 引用。WorldDriver 初始化时通过场景路径获取一次并缓存：

```rust
let camera = self.base()
    .get_node_as::<Camera3D>("../Player/Camera3D")
    .ok();
```

DebugConsole 在 `new()` 时接收 `Option<Gd<Camera3D>>`——不持有 Godot 引用则无法做 raycast（此时仅通过 `select <id>` 命令选中）。

### 8.5 鼠标点击实体选中

控制台开启时，鼠标左键按下（边缘检测）→ 获取鼠标屏幕坐标 → Camera3D 发射射线 → 对所有实体的 AABB 做 slab 交测（§九）→ 选中最近命中。未命中任何实体则保持原有选中。

---

## 九、AABB-Ray 交测

用于控制台开启时鼠标点击选中实体。slab 方法：

```rust
fn ray_aabb_intersect(
    origin: glam::Vec3,
    dir: glam::Vec3,
    aabb_min: glam::Vec3,
    aabb_max: glam::Vec3,
) -> Option<f32> {
    let inv = glam::Vec3::new(1.0 / dir.x, 1.0 / dir.y, 1.0 / dir.z);
    let t1 = (aabb_min - origin) * inv;
    let t2 = (aabb_max - origin) * inv;
    let tmin = t1.min(t2);
    let tmax = t1.max(t2);
    let tnear = tmin.x.max(tmin.y).max(tmin.z);
    let tfar = tmax.x.min(tmax.y).min(tmax.z);
    if tnear <= tfar && tfar > 0.0 {
        Some(tnear.max(0.0))
    } else {
        None
    }
}
```

实体 AABB 从 CapsuleMesh 参数推导：`aabb_min = position - (0.4, 0.0, 0.4)`, `aabb_max = position + (0.4, 1.8, 0.4)`。

---

> **关联**: [[001-模型动作与物理系统总纲|模型动作总纲]] · [[../UI与UX系统/002-HUD与常驻界面|HUD 与常驻界面]] · [[../UI与UX系统/005-跨模块接口与性能预算|UI 性能预算]] · [[../../../../参考文档/034-模型动作与物理系统大纲-20260617/README|参考文档 034]]
