---
id: GD-003
title: `set_surface_override_material` 材质设值但不渲染——GPU 输出 ≠ CPU 属性值
type: 🟡反直觉陷阱
module: Godot 桥接
status: ✅已修复
confidence: 🤔推测
discovered: 2026-07-15
resolved: 2026-07-15
last_verified: 2026-07-15
grep_keys: [set_surface_override_material, set_material_override, 材质不生效, 颜色不对, 颜色错误, 渲染颜色与albedo不一致, albedo, surface, material, godot-rust, Material, pink, green, wrong color]
env:
  godot: "4.7-stable"
  renderer: "Forward+"
  os: [Windows]
  gpu: "NVIDIA GTX 1660 SUPER"
relations:
  - {target: GD-004, type: 并发}
---

## 症状识别
- 通过 `StandardMaterial3D::set_albedo(Color::from_rgb(1.0, 0.0, 0.0))` 硬编码纯红
- `get_albedo()` 读回确认 `(1.00, 0.00, 0.00)` —— CPU 侧材质属性已正确写入
- `get_shading_mode()` 确认 `UNSHADED`——shading 模式也正确
- 但 GPU 渲染到屏幕的颜色始终不是纯红（偏粉/绿/紫，取决于具体环境）
- 换成 `set_material_override()` 后同一材质、同一属性立即正确渲染为纯红
- ⚠️ 此 bug 与 load 无关——每一帧都错。load 后颜色跳变是 [[GD-004]] 的症状叠加

## 误诊路径
| 尝试过的方案 | 结果 | 为什么无效 |
|-------------|------|-----------|
| 禁用顶点色 FLAG_ALBEDO_FROM_VERTEX_COLOR | 无效 | 顶点色不是原因 |
| 换成 ORMMaterial3D（替代 StandardMaterial3D） | 无效 | 仍通过 `set_surface_override_material` 绑定，同样不被 GPU 使用 |
| 加 emission 发光（同一材质对象） | 无效 | 材质的所有属性（albedo/emission/ORM）均不被渲染——是材质对象未被 GPU 引用，非个别属性问题 |
| 存住材质引用防 GC | 无效 | 不是 GC 问题——`get_albedo()` 能读回，对象存活 |
| 换 BoxMesh 替代 CapsuleMesh | 无效 | 不是 Mesh 类型问题 |
| 关闭 WorldEnvironment adjustment | 无效 | 不是后处理 |
| 怀疑 Sun 光照影响 Unshaded | 排除 | `get_shading_mode()` 确认已是 UNSHADED |
| **换 set_material_override** | ✅有效 | **绕开 surface_override，材质立即生效** |

## 根因
`MeshInstance3D::set_surface_override_material(0, &mat)` 调用后，Godot 对象内部材质属性已写入
（`get_albedo()` 读回正确）且 `get_shading_mode()` 确认 `UNSHADED`——但 GPU 渲染时未使用该材质。

`set_material_override(&mat)` 无此问题。

⚠️ **未确认层级**：不确定是 godot-rust 0.5 FFI bug、Godot 4.7 引擎 bug，还是 Vulkan 驱动的特定问题。
仅确认了症状（材质的 GPU 输出 ≠ CPU 属性值）和绕过方法（`set_material_override`）。

## 解决方案
`entity_renderer.rs` 的 `create_node` 中，
将 `mesh_instance.set_surface_override_material(0, &mat)` 改为 `mesh_instance.set_material_override(&mat.upcast::<Material>())`。

注意：`EntityNode` 增加了 `_material: Gd<Material>` 字段（`mat.clone().upcast::<Material>()`）。
此字段不是修复必须项（`get_albedo()` 能读回证明 Godot 侧对象存活），但作为防御性措施保留——
防止未来 Godot/godot-rust 升级后 refcount 语义变化导致材质被提前回收。

## 验证方法
1. 在 `create_node` 中临时硬编码 `mat.set_albedo(Color::from_rgb(1.0, 0.0, 0.0))`
2. 启动游戏 → 所有胶囊应显示纯红
3. 若为纯红 → `set_material_override` 生效；若非纯红 → bug 重现
4. load 存档 → 同样纯红（结合 GD-004 修复后 `create_node` 在 load 后被调用）

## 代码位置
- `crates/woworld_godot/src/entity_renderer.rs::create_node` — 主要修复点：`set_material_override` 替换 `set_surface_override_material`
- `crates/woworld_godot/src/entity_renderer.rs::update_node` — ⚠️ 死代码（`#[allow(dead_code)]`），但也用了 `set_surface_override_material`。若后续启用需同步修改

## 关联 Bug
- [[GD-004]] — 并发：此 bug 阻止材质颜色渲染到屏幕，GD-004 阻止 `create_node` 被调用
