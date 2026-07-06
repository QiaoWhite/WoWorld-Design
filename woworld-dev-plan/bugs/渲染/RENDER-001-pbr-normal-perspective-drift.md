---
id: RENDER-001
title: PBR 法线透视插值漂移（大三角形）
type: 🔴回归
module: 渲染
status: ✅已修复
confidence: ✅确信
discovered: 2026-07-05
resolved: 2026-07-05
last_verified: 2026-07-05
grep_keys: [pbr, normal, 法线, 插值漂移, perspective interpolation, 大三角形, dot(N,L), clipmap, 多边形阴影, 视角依赖, 光照异常, 暗斑, dark patch, 法线重算, dFdx, dFdy, heightmap]
env:
  godot: "4.7-stable"
  renderer: "Forward+"
  os: [Windows]
  gpu: "所有（透视插值是通用行为）"
relations: []
---

## 症状识别
- **太阳当空时，摄像机朝下看地形——全地形出现视角依赖的"多边形阴影"和暗斑**
- 看起来像法线方向错误，但不是均匀的——随视角变化
- 从 unshaded 模式正常 → 确认是光照计算问题
- 切换到 PBR 后首次暴露（之前 unshaded 不可见）
- clipmap 的大三角形（16m+ 边长）尤其明显

## 误诊路径（10 轮诊断）
| 尝试过的方案 | 结果 | 为什么无效 |
|-------------|------|-----------|
| SPECULAR=0 | 略改善但不消失 | 高光不是主因 |
| shadow=false | 略改善但不消失 | 阴影不是主因 |
| ambient_light_disabled | 略改善但不消失 | 环境光不是主因 |
| normalize(NORMAL) | 无效 | NORMAL 在 fragment 已是归一化的——值就是错的 |
| 世界法线可视化 | 确认法线确实在变化 | 但不知道为什么 |
| 自定义 light() 函数 | 隔离到 dot(N,L) 异常 | N·L 在大三角形上的变化模式不对 |
| `ensure_correct_normals` | **无效——Godot 4.7 中此选项是 unimplemented 空操作** | 文档明确标注此选项不工作 |

## 根因
**Clipmap 大三角形（16m+ 边长）的顶点法线在透视投影下发生插值漂移**——透视插值不是线性的，屏幕空间 barycentric 坐标 ≠ 世界空间 barycentric 坐标。大三角形上这种漂移累积到 dot(N,L) 计算中，产生视角依赖的"多边形阴影"。

## 解决方案
**在 fragment shader 中重算法线**——不依赖顶点法线插值：
```glsl
// terrain.gdshader fragment()
// 方案A: heightmap 重算法线（terrain）
// 从 heightmap 纹理采样周围点，有限差分法计算法线

// voxel_terrain.gdshader fragment()
// 方案B: 屏幕空间导数（voxel，顶点法线更密）
vec3 normal = normalize(cross(dFdx(VERTEX), dFdy(VERTEX)));
```
- 保留 Godot 标准 PBR（不切换到自定义光照）
- SPECULAR=0, ROUGHNESS=1.0 作为辅助

## 验证方法
1. 启动 Godot → PBR 材质 → 太阳当空 → 旋转摄像机
2. 确认地形表面无视角依赖的暗斑或"多边形阴影"
3. 对比 unshaded 和 PBR 模式——PBR 光照应均匀、无异常暗区

## 代码位置
- `godot/shaders/terrain.gdshader` — fragment() 中 heightmap 法线重算
- `godot/shaders/voxel_terrain.gdshader` — fragment() 中 dFdx/dFdy 法线重算

## 关联 Bug
无

## 复发记录
| 日期 | 会话 | 症状是否相同 | 原方案是否有效 | 备注 |
|------|------|-------------|---------------|------|
| — | — | — | — | — |
