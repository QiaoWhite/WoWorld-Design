---
id: GD-004
title: hecs World 重建后 entity bits 重叠 → EntityRenderer 复用旧节点 → 颜色/状态错误继承
type: 🟡反直觉陷阱
module: Godot 桥接
status: ✅已修复
confidence: ✅确信
discovered: 2026-07-15
resolved: 2026-07-15
last_verified: 2026-07-15
grep_keys: [entity bits, overlap, 重叠, 复用, EntityRenderer, clear_nodes, needs_full_rebuild, queue_free, hecs rebuild, load, color inherit, 颜色继承, 节点复用, 存档, save, load颜色变, load color change, 胶囊颜色, to_remove为空, alive.contains_key, UPDATE路径]
env:
  godot: "4.7-stable"
  os: [Windows]
relations:
  - {target: GD-003, type: 并发}
---

## 症状识别
- load 存档后，玩家胶囊颜色与 load 前不同（例如从粉色变为绿色）——实际继承自其他旧实体
- 所有 NPC 的名字标签、位置均正确更新——只有胶囊颜色和材质未变（`sync()` 的 UPDATE 路径不处理材质）
- 重复 load 同一存档 → 颜色一致、稳定（新旧 entity bits 对应关系确定，每次复用同一组旧节点）
- 技术层面：`sync()` 的 `to_remove` 列表始终为空——新旧 entity bits 相同，旧节点全部被误判为"仍存活"
- 技术层面：`create_node` 对全部实体不被调用——均走 UPDATE 路径

## 误诊路径
| 尝试过的方案 | 结果 | 为什么无效 |
|-------------|------|-----------|
| 修复颜色哈希算法（stable_id化） | 必要但不够 | 哈希正确但 `create_node` 没被调用，颜色没机会写入 |
| 修复 name_cache（跨 load 稳定名字） | 必要但不够 | 名字正确但 `create_node` 没被调用 |
| 补 LodLevel::default() | 必要但不够 | 只解决了可见性（`is_visible()` 需要 LodLevel），没解决颜色 |
| 在 `handle_load` 中调用 `clear_nodes` 直接清空 nodes | 无效 | `handle_load` 在 `sync()` 之后运行——清空发生在帧末，下一帧 `sync()` 又被旧逻辑填充 |
| **clear_nodes 改为 flag + sync 内部清空** | ✅有效 | `sync()` 开头检测标记 → 清空 → 同一调用内 `create_node` 重建所有节点 |

## 根因
`handle_load` 中 `self.ecs = EcsWorld::new()` 销毁旧 hecs World 并创建新 World。
`restore_entities` 在新 World 中 spawn 实体，entity bits 从 `(1<<32)|0` 重新开始分配——
和旧 World 的起始位置**完全相同**。

`EntityRenderer::sync()` 以 `hecs::Entity.to_bits()` 为 key 管理节点 HashMap：
```rust
let alive: HashMap<hecs::Entity, &EntityVisual> = ...; // key = 新 entity bits
let to_remove = self.nodes.keys().filter(|e| !alive.contains_key(e));
// → 新 bits 和旧 bits 完全一致 → alive.contains_key() 全部返回 true → to_remove 为空
```

旧节点零移除。新实体在 `self.nodes.contains_key(entity)` 检测中全部命中 →
走 UPDATE 路径（只更新 position/rotation/label/bubble，不更新材质和颜色）。

**具体案例**（示意，实际 ID 因存档不同而异）：初始 spawn 时玩家 = id=0，NPCₓ = id=5。load 后 snapshot 顺序使玩家 = id=5，NPCᵧ = id=0。
新玩家(id=5) 的 `contains_key` 命中旧 NPCₓ(id=5) 的节点 → 复用节点 + UPDATE 路径 → 继承了 NPCₓ 的颜色。
旧玩家(id=0) 则被新 NPCᵧ(id=0) 复用 → NPCᵧ 继承了旧玩家的粉色胶囊。

⚠️ **与 GD-003 的交互**：此 bug 导致 `create_node` 不运行 → 颜色未更新。但即使解决了此 bug
让 `create_node` 运行，若不同时修复 GD-003（`set_surface_override_material` 不生效），
`create_node` 中设置的正确颜色也不会渲染到屏幕。

## 解决方案
1. `EntityRenderer` 新增 `needs_full_rebuild: bool` 字段（默认 `false`）
2. `handle_load` 末尾调用 `renderer.clear_nodes()` → 将标记设为 `true`
3. `sync()` 开头检测标记 → `true` 时先 `drain()` 清空 `self.nodes`（每个旧 node 调 `queue_free()`），再继续正常流程
4. 关键：清空在 `sync()` **内部**执行而非 `handle_load` 末尾直接操作。
   因为 `process()` 每帧执行顺序为 `sync()` → …（ECS/渲染）… → `handle_load()`：
   - 若在 `handle_load` 直接 `self.nodes.drain()` → `sync()` 早已在本帧开头用旧 entity bits 填充好了 `self.nodes`
   - 清空仅影响本帧剩余时间，下一帧 `sync()` 又照旧逻辑填充
   - 改为标记延迟 → 下一帧 `sync()` **开头第一件事**就是检测标记并清空 → 随后同一调用内 `create_node` 全量重建

## 验证方法
1. 启动游戏 → `save 1` → `load 1` → 观察玩家胶囊
2. 颜色应与 load 前一致（同一 stable_id → 同一 hash 色）
3. 若不一致且观察到 NPC 也交换了颜色 → 本 bug 复发
4. 代码层面：`sync()` 第一行插入临时 `godot_print` 打印 `self.nodes.len()`——load 后此值应为 0（刚清空）而非 21

## 代码位置
- `crates/woworld_godot/src/entity_renderer.rs` — `EntityRenderer` 结构体（`needs_full_rebuild` 字段）、`sync()`（检测+清空）、`clear_nodes()`（设置标记）
- `crates/woworld_godot/src/terrain_chunk.rs::handle_load` — 调用 `clear_nodes`

## 关联 Bug
- [[GD-003]] — 并发：此 bug 阻止 `create_node` 运行，GD-003 则使即使运行了颜色也不渲染
