---
id: RENDER-002
type: 🔴回归
status: ✅已修复
confidence: ✅确信
date_found: 2026-07-06
date_fixed: 2026-07-07
commit_introduced: aad15c0 (Sprint 033 — PBR法线修复 + voxel shader 切换为 dFdx/dFdy)
commit_fix: (current)
grep_keys: [LOD0, VoxelChunk, 光照异常, 正方形区域, 全黑, 锯齿亮斑, 阴影, shadow, shadow acne, z-fighting, 阴影贴图, shadows_disabled, 自阴影, depth discontinuity]
env: {godot: "4.7", renderer: "Forward+", os: "Windows", gpu: "GTX 1660 SUPER"}
relations: [{target: "RENDER-001", type: "同Sprint", note: "同为 aad15c0 Sprint 033 引入——PBR 法线修复触发了一系列渲染变更"}]
---

# RENDER-002: VoxelChunk LOD0 阴影贴图深度不连续 → 视角依赖光照异常

## 症状识别

- LOD0 **正方形区域**光照与其他区域明显不同
- 摄像机**垂直向下**：该区域**全黑**
- 摄像机**抬起倾斜**：正方形内部出现**方正的、多条边的光亮区域**，边缘**锯齿状**
- 海拔**足够高**：异常**仅局限于 LOD0 正方形**内
- 海拔**较低**：异常会**蔓延**到相邻 LOD 区域，尤其屏幕边缘
- `unshaded` 模式下完全正常 → 问题100%在光照/阴影
- LOD 1-7 Clipmap 渲染不受影响（但禁用 clipmap 阴影后次要异常也消失）

## 误诊路径

| ❌ 误诊方向 | 为什么看似合理 | 实际排除原因 |
|------------|--------------|------------|
| **dFdx/dFdy 法线计算错误**（初版结论） | camera-angle 依赖 + 屏幕空间导数对小三角形不稳定 | 还原为 mesh 顶点法线后问题依然存在 |
| **三角形绕序反转** | face_normal 方向影响 cull_back 剔除 | 修改绕序 → 完全透明（全剔除），排除 |
| **vertex COLOR 未传入** | 黑色=COLOR=(0,0,0) | 诊断日志 + `* 10` 测试验证 COLOR 正确传入 |
| **PBR 参数 (ROUGHNESS/SPECULAR)** | 显式参数导致亮度差异 | terrain shader 完全相同参数正常渲染 |
| **绕序 + 法线 3c3018c 交互** | 法线修复与绕序修复时间差导致的极性反转 | 修改后全透明，排除 |
| **L1 clipmap 与 VoxelChunk z-fighting** | 30-40m 重叠区深度竞争 | 扩大 L1 孔洞产生新缝隙，且核心区域不受影响 |
| **VoxelChunk 自阴影 acne（仅 cast_shadows=OFF）** | 改善了"全黑"但未根除 | 低空屏幕边缘异常依旧 |

## 根因

**DirectionalLight3D 阴影贴图中，VoxelChunk（MC 等值面提取）与 clipmap（高度场 displacement）产生的深度值存在微小差异。** 两者代表同一地形表面，但深度计算路径不同：
- VoxelChunk: `MODEL_MATRIX * VERTEX`（实际 3D 几何体）
- Clipmap: `VERTEX + node_pos` → vertex shader 从 heightmap 设 Y 值

这些微小深度差在阴影贴图中形成 LOD 边界的**深度不连续**。DirectionalLight3D 的 PCF 滤波跨边界采样时，邻近纹素的深度值跳变 → 阴影判定错误 → 视角依赖的暗区/亮块。

**视角依赖性来源**：PCF 采样模式由屏幕空间投影决定。摄像机垂直时采样模式对称，倾斜时不对称 → 异常随视角变化。

**为什么低空屏幕边缘更严重**：低空时地面覆盖更多屏幕像素，PCF 滤波在边界处影响更多片段。屏幕边缘的极端透视角度放大深度差异。

## 解决方案

**Terrain 全部"只投射阴影，不接收阴影"：**

| 改动 | 文件 | 作用 |
|------|------|------|
| `render_mode shadows_disabled` | `voxel_terrain.gdshader` | VoxelChunk 不接收阴影 |
| `render_mode shadows_disabled` | `terrain.gdshader` | Clipmap 不接收阴影 |
| `cast_shadows = ON` (默认) | VoxelChunk + Clipmap | 两者都投射阴影（填入阴影贴图） |

**设计原则**：地形是连续表面，不需要自阴影（LOD 间本就不应有阴影边界）。`cast_shadows=ON` 确保阴影贴图有完整地形深度——其他物体（NPC、建筑）投射到地形上的阴影通过它们自己的 shader 正常运作。

## 验证方法

1. Godot 启动 → LOD0 正方形光照与 L1+ 区域一致
2. 摄像机垂直向下 → 无全黑方块
3. 摄像机倾斜 → 无锯齿几何亮斑
4. 低空飞行 → 屏幕边缘无异常阴影蔓延
5. `unshaded` vs `shadows_disabled` 模式视觉效果一致

## 代码位置

- `woworld/godot/shaders/voxel_terrain.gdshader:2` — `shadows_disabled`
- `woworld/godot/shaders/voxel_terrain.gdshader:13` — `COLOR = COLOR;`（显式顶点色传递，确保 GPU 绑定 COLOR 属性）
- `woworld/godot/shaders/terrain.gdshader:2` — `shadows_disabled`
- `woworld/crates/woworld_godot/src/voxel_chunk.rs:44` — `cast_shadows=ON`（显式设置 + 注释）

## 关联 Bug

- **RENDER-001** (PBR 法线透视插值漂移): 同为 `aad15c0` Sprint 033 引入——该 Sprint 修改了 voxel shader 法线计算并恢复了 cull_back
- **WG-001** (Clipmap fp32 精度裂缝): 同为 LOD 边界问题，但根因不同

## 复发记录

_无_
