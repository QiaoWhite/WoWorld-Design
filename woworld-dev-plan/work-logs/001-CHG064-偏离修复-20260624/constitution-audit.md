# 宪法 §14 架构边界合规审计

> **审计日期**: 2026-06-24
> **审计依据**: CONSTITUTION.md v1.4 §14-§14.5
> **审计范围**: 全部 Rust + GDScript 代码

## 层一：双重权威检测

**检测项**: GDScript 中是否存在与 Rust 重复的计算？

| 文件 | 检测结果 |
|------|----------|
| `woworld/godot/scripts/player.gd` | ✅ 唯一 .gd 文件。`time_manager.gd`（曾是主要违规）已删除 |
| — `terrain_renderer.gd` | ✅ 已删除（死代码，曾被主场景引用） |

**结论**: ✅ 无双重权威。`day_progress`/`season`/`sun_elevation`/天空颜色等全部只在 Rust 侧计算。

## 层二：僵尸代码检测

**检测项**: 所有 `#[func]` 是否被 GDScript 调用？

| `#[func]` | 调用方 | 状态 |
|-----------|--------|------|
| `TerrainChunk::query_height()` | `player.gd:37` | ✅ 活跃 |
| ~~`advance_time()`~~ | — | ✅ 已删除（D01） |
| ~~`get_sun_position()`~~ | — | ✅ 已删除（D01） |
| ~~`get_sky_top_color()`~~ | — | ✅ 已删除（D01） |
| ~~`get_sky_horizon_color()`~~ | — | ✅ 已删除（D01） |
| ~~`get_ambient_light()`~~ | — | ✅ 已删除（D01） |

**结论**: ✅ 零僵尸代码。

## 层三：边界穿越审计

**检测项**: GDScript 每一行是否属于以下三类之一？
1. 读 Input（键盘/鼠标）
2. 读 Rust `#[func]` 返回值
3. 设 Godot 节点属性

### player.gd 逐行审计

| 行 | 内容 | 类别 | 判定 |
|----|------|------|------|
| 5-8 | 客户端常量（SPEED, JUMP_VELOCITY, MOUSE_SENS, GRAVITY） | — | ✅ 客户端调优参数 |
| 10 | UI 状态变量 | — | ✅ 非游戏世界状态 |
| 13 | `Input.set_mouse_mode()` | ③设 Godot 属性 | ✅ |
| 17-21 | 鼠标环顾 → 设 Node3D 旋转 | ①读 Input → ③设节点 | ✅ |
| 23-30 | ESC 键切换鼠标捕获 | ①读 Input | ✅ |
| 34 | `get_node_or_null("../TerrainChunk")` | — | ✅ Godot 场景访问 |
| 37 | `terrain.query_height(...)` | ②读 Rust `#[func]` | ✅ |
| 41-44 | 地面检测 + 重力 | — | ✅ 玩家物理（显式例外） |
| 46-47 | 空格跳跃 | ①读 Input → ③设 velocity | ✅ |
| 50-54 | WASD 输入 | ①读 Input | ✅ |
| 56-63 | 方向/速度 → 玩家物理 | ③设 Godot 属性 | ✅ |
| 65 | `move_and_slide()` | — | ✅ Godot 物理引擎 |
| 68-70 | 贴地 | ③设 Godot 属性 | ✅ |

**结论**: ✅ player.gd 完全合规。无数学公式（sin/cos/lerp/clamp 用于游戏逻辑）、无状态机、无基于游戏世界状态的条件分支。仅有的 `clamp`、`deg_to_rad` 用于摄像机控制（客户端表现），属于宪法明确允许的"渲染后处理"范畴。

## 总结

```
层一（双重权威）  ✅ 通过
层二（僵尸代码）  ✅ 通过
层三（边界穿越）  ✅ 通过
```

**合规等级**: 🟢 全绿。GDScript 铁律无违规。
