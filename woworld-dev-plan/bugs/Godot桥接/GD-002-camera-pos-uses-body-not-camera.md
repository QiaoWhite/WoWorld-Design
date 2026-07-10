---
id: GD-002
title: 浮点原点 camera_pos 用 body 位置而非真实相机 → 低眼高看穿地形
type: 🟡反直觉陷阱
module: Godot桥接
status: ✅已修复
confidence: ✅确信
discovered: 2026-07-10
resolved: 2026-07-10
last_verified: 2026-07-10
grep_keys: [camera_pos, 浮点原点, floating origin, 透明, transparent, 看穿, see-through, 背面, backface, cull_back, LOD0, VoxelChunk, 眼高, eye height, body vs camera, 相机高度, 地形偏高]
env:
  godot: "4.7-stable"
  renderer: "Forward+"
  os: [Windows]
  gpu: "所有"
relations:
  - {target: GD-001, type: 同源}  # 同为浮点原点 camera-relative 顶点变换的正确性陷阱
---

## 症状识别
- LOD0 VoxelChunk 地形在**低眼高第一人称视角**下"透明/看穿"（看到背面）——站立时最明显。
- 高空/俯视相机看不出问题；只有相机贴近地面（眼高 ~1.7m）才暴露。
- `[CamDiag]` 诊断显示脚点 `Δfeet-terrain=0.00`（角色定位正确），排除"卡地"和近平面。

## 根因
浮点原点着色器（`voxel_terrain.gdshader` / `terrain.gdshader`）用 camera-relative 顶点变换：
```glsl
vec3 rel = world_pos - camera_pos;               // camera_pos 是 uniform
POSITION = PROJECTION_MATRIX * vec4(mat3(VIEW_MATRIX) * rel, 1.0);
```
`camera_pos` uniform **必须等于真实渲染相机（Camera3D）的世界位置**。但代码用
`get_player_position()`（= 玩家 **body/节点** 位置）喂它，而 Camera3D 是 body 的**子节点，
偏移 +1.7m（eye height）**。二者差 1.7m：

- `camera_pos` 偏低 1.7m → `rel = world_pos - camera_pos` 偏大 1.7m
- → 地形整体在视空间**渲染偏高 1.7m**
- → 相机（真实位置）落在被抬高的地表**之下** → `cull_back` 剔除正面、露出背面 = **透明**

**为什么曾"正常"**：旧 player.gd 让 body 悬浮，眼高 ≈ 地表 +3.4m。被抬高 1.7m 的错误地表仍在
相机下方 1.7m，恰好掩盖了 bug。一旦把眼高降到真实的 +1.7m（角色控制器 ECS 驱动，脚点贴地），
相机恰好落在错误地表上 → 暴露。Clipmap（LOD1+，距离远）同样有此误差但视觉上看不出。

## 误诊路径（禁止重复尝试）
| 假设 | 为什么错 |
|------|---------|
| 角色卡在地面里（脚点低于地表） | ❌ `[CamDiag]` 证 `Δfeet-terrain=0.00`，脚点精确贴地 |
| 相机近平面 `near=1.0` 太大切入地形 | ❌ 改 `near=0.2` 无效（near 仍是合理改进，保留） |
| VoxelChunk 三角 winding 反了 / 等值面≠heightfield | ❌ `terrain.rs` 等值面 = `pos.y - sample_height` = height_at，表面一致 |
| RENDER-002 阴影深度不连续 | ❌ 那是"全黑/亮斑"光照异常，非"透明看穿" |

## 验证方法
- 修复前：ECS 驱动玩家、眼高 1.7m，站立时 LOD0 地面透明看穿。
- 修复后：`camera_pos` 改用真实 Camera3D 世界位置，地面实心。
- 快速判别：若"透明"随相机升高而消失/降低而出现，且脚点定位正确 → 大概率是 camera_pos 高度错。

## 修复
`terrain_chunk.rs` `process()` 中 camera_pos uniform 的来源改为真实相机：
```rust
let cam_world = self.base().get_viewport()
    .and_then(|vp| vp.get_camera_3d())
    .map(|c| c.get_global_position())
    .unwrap_or_else(|| /* fallback: player_pos */);
// px/py/pz(仅供 voxel + clipmap 的 camera_pos uniform) = cam_world.{x,y,z}
```
`get_player_position()`（body 位置）仍用于大气/LOD——那些对 1.7m 差异不敏感，不要改。

## 预防
- **浮点原点 shader 的 `camera_pos` 永远取真实渲染相机世界位置**，绝不用 body/角色根节点。
- 相机与角色根节点有 eye-height 偏移时，此 bug 必然出现；只是高相机会掩盖。
