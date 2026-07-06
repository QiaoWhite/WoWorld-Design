---
id: WG-001
title: Clipmap LOD边界 fp32 精度裂缝
type: 🟠架构限制
module: 世界生成
status: ⚠️已知残留
confidence: ✅确信
discovered: 2026-06-29
resolved: 2026-07-01
last_verified: 2026-07-01
grep_keys: [裂缝, crack, seam, gap, fp32, LOD边界, 面片缺失, 透光, 水密, watertight, edge mismatch, 透明缝隙, 天空色缝隙, tile边界]
env:
  godot: "4.7-stable"
  renderer: "Forward+"
  os: [Windows]
  gpu: "所有（fp32 精度是硬件规范）"
relations: []
---

## 症状识别
- 地形表面可见**弯曲、不规则的透明缝隙**（天空色透出），间距 10-50m
- 静止时可见，靠近时有时消失
- 缝隙不是平直的 tile 网格线——是弯曲的、不规则的
- 平坦地形测试（`test_flat_terrain_watertight`）**完全水密**——只在真实地形（有梯度）出现
- MSAA 关闭后"有所缓解但仍在"

## 误诊路径
| 尝试过的方案 | 结果 | 为什么无效 |
|-------------|------|-----------|
| MSAA/TAA/Debanding 关闭 | 无效 | 不是后处理问题 |
| bottom_y 统一 + aligned_vertical() | 无效 | 不是 tile 高度不一致 |
| SH tile 边界重叠 (overlap=1, 35²网格) | 无效 | 重叠不影响光栅化 |
| 边界顶点膨胀 (inflate_edges, 1% 衰减) | 无效 | 膨胀量不够填 fp32 间隙 |
| Skirt geometry (向下裙边三角形) | 无效 + 引入闪透明回归 | 裙边在不同 tile 之间无法连通 |
| 单 mesh 合并 (560 nodes → 8) | 无效（但架构正确） | 证明不是同 LOD 多 MeshInstance3D 问题 |
| 空间哈希顶点焊接 (5cm / 20cm 容差) | 无效 | 焊接不改变 GPU 光栅化精度 |
| half_band 1.0→3.0 (过渡带扩展) | 无效 | 梯度扩展不改变光栅化精度 |
| Camera3D near=1.0 far=20000 | 无效 | 不是深度缓冲精度问题 |
| 背面剔除 CULL_BACK | 引入新 bug（闪透明）→ 回退 | 缝隙处无背面几何 |
| Godot WorldOrigin 节点移动 (v1-v2) | 部分有效但破坏 AABB/Sun | 节点移动和 shader 光学不一致 |
| 顶点数据偏移 (v3) | frustum culling 错误→地形消失 | AABB 在原点但顶点在世界坐标 |
| Rust set_global_position(-snap) (v4) | AABB 错位→双重地形 | 传输路径不同步 |

## 根因
**GPU fp32 光栅化精度不足**——世界坐标达 10,000m 时，fp32 精度仅 ~0.5mm（10,000 / 2^23 ≈ 0.0012），但光栅化插值 + 透视变换的组合误差在相邻三角形边缘产生亚像素级裂缝。这不是几何问题（MC 提取正确），不是材质问题（unshaded 同样出现），是 **Godot 4.7 Forward+ 渲染管线在超大世界坐标下的固有限制**。

## 解决方案
**Vertex Shader Camera-Relative Floating Origin**（v6 最终方案）：
```glsl
// terrain.gdshader vertex()
vec3 rel = VERTEX - camera_pos;           // 相机相对坐标 < 500m → fp32 高精度
vec3 view_pos = mat3(VIEW_MATRIX) * rel;  // 只旋转，跳过 VIEW 平移
POSITION = PROJECTION_MATRIX * vec4(view_pos, 1.0);
```
- 数学等价于标准管线（`mat3(VIEW) * (V-cam) = R*V - R*cam`）
- 场景树完全不变（无 WorldRoot，无 set_global_position）
- 顶点数据永不被修改（世界空间 AABB → frustum culling 正确）
- 每帧仅传一个 `camera_pos` uniform

## 验证方法
1. 启动 Godot → 飞行至不同高度 → 检查地形表面是否还有天空色缝隙
2. `cargo test -p woworld_worldgen test_flat_terrain_watertight` 确认 MC 提取水密
3. `cargo test -p woworld_worldgen test_adjacent_tile_boundary_vertex_deviation` 确认偏差=0

## 代码位置
- `woworld_godot::terrain_chunk::WorldDriver` — ShaderMaterial + camera_pos uniform 注入
- `godot/shaders/terrain.gdshader` — vertex() camera-relative 逻辑
- `godot/shaders/voxel_terrain.gdshader` — 同逻辑，独立 shader

## 关联 Bug
无

## 复发记录
| 日期 | 会话 | 症状是否相同 | 原方案是否有效 | 备注 |
|------|------|-------------|---------------|------|
| — | — | — | — | — |
