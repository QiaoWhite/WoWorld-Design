---
id: GD-001
title: Floating Origin v1-v5 实现全失败
type: 🟡反直觉陷阱
module: Godot桥接
status: ✅已修复
confidence: ✅确信
discovered: 2026-07-01
resolved: 2026-07-01
last_verified: 2026-07-01
grep_keys: [floating origin, 浮点原点, camera relative, vertex shader, AABB, double offset, frustum culling, 双重地形, 地形消失, sun path, world origin]
env:
  godot: "4.7-stable"
  renderer: "Forward+"
  os: [Windows]
  gpu: "所有"
relations:
  - {target: WG-001, type: 引发}  # Floating Origin 是 WG-001 fp32 裂缝的解决方案
---

## 症状识别
- 需要实现 Floating Origin 解决 WG-001 fp32 裂缝
- 每次尝试都有**不同但都严重**的失败模式——不是渐进改进，而是每次换个方向都撞墙

## 误诊路径（5 版全失败）
| 版本 | 方法 | 失败模式 | 为什么无效 |
|------|------|---------|-----------|
| v1 | GDScript WorldOrigin 节点 + `_process` 移动 | Sun 路径断裂 + 性能问题 | WorldOrigin 移动触发所有子节点全局变换重算 |
| v2 | Rust `set_global_position(-snap)` 移动 WorldDriver | 裂缝消失 ✅ 但 AABB 错位 → **双重地形**（原位置+新位置各渲染一份） | Godot 的 AABB 计算在 `set_global_position` 后不同步更新 |
| v3 | 顶点数据偏移（减 camera_pos），节点不动 | AABB 在原点 → frustum culling 基于原点 → **地形全部消失** | 顶点在世界坐标但 AABB 声称在原点——相机看不到世界坐标的几何 |
| v4 | v2 + 本地 snap 追踪（消除 `get_global_position` FFI） | 同 v2——双重地形 | FFI 调用不是根因 |
| v5 | ShaderMaterial + `VERTEX -= camera_pos` | **双倍偏移**：VERTEX 减去 camera_pos 后，VIEW_MATRIX 的平移分量又减一次 | 不知道 VIEW_MATRIX 已包含 -camera 平移 |

## 根因
**每个方案都踩了 Godot 渲染管线的一个不同暗坑**——不是渐进调试，而是每次换方向都遇到新坑：
- v1: WorldOrigin 的 API 语义误解
- v2: AABB 更新时序
- v3: Frustum culling 和顶点坐标的耦合
- v5: VIEW_MATRIX 的双重平移（最隐蔽——shader 里 `VERTEX - camera_pos` 看起来完全正确）

## 解决方案
**v6: Vertex Shader Camera-Relative**——只旋转，跳过 VIEW 平移：
```glsl
vec3 rel = VERTEX - camera_pos;           // 相机相对坐标
vec3 view_pos = mat3(VIEW_MATRIX) * rel;  // mat3 去掉了平移分量！
POSITION = PROJECTION_MATRIX * vec4(view_pos, 1.0);
```
关键洞察：`mat3(VIEW_MATRIX)` 丢弃了第 4 列（平移分量），所以 `mat3(VIEW) * (V-cam)` = 旋转后的相对坐标，与标准管线 `VIEW * V` 完全等价但中间值都是小值。

## 验证方法
1. 启动 Godot → 飞行至 10,000m+ → 地形无裂缝、无双重渲染、无消失
2. 确认 Sun 路径正常（日出日落方向正确）
3. 确认 AABB culling 正确（旋转视角地形不消失）

## 代码位置
- `woworld_godot::terrain_chunk::WorldDriver` — ShaderMaterial 创建 + camera_pos uniform
- `godot/shaders/terrain.gdshader` — vertex() camera-relative
- `godot/shaders/voxel_terrain.gdshader` — 同逻辑

## 关联 Bug
- [[WG-001]] — Floating Origin 是 WG-001 的解决方案

## 复发记录
| 日期 | 会话 | 症状是否相同 | 原方案是否有效 | 备注 |
|------|------|-------------|---------------|------|
| — | — | — | — | — |
