# DEVLOG 2026-07-12 — Sprint-070 V2 牵引移动

> **冲刺**: Sprint-070 — V2 牵引移动·Goal.target_pos 解析到最近资源点
> **状态**: ✅ 完成·1100 tests 全绿·clippy/fmt 零警告
> **提案**: [[../../sprint-proposals/sprint-070-V2-牵引移动-20260712]]（二轮审查·16 项检查全部修正）

## 做了什么

填补 `Goal.target_pos` 从 `None` 到 `Some(nearest_resource)` 的缺口。饿了的 NPC 现在知道走向最近的浆果丛，渴了的走向最近的水源。需求→目标→移动的涌现闭环首次完整。

### 产物（代码）

| 文件 | 内容 |
|------|------|
| `ecs/goal.rs` | `goal_resolution_system` 签名扩展（+vegetation/ocean/search_radius）+ Pass 1 同步填充 target_pos（`resolve_target_pos` + `find_nearest_water_xz`）+ 8 个新测试 |
| `godot/terrain_chunk.rs` | `BiomeVegetation` 初始化（`ready()` line 373 后·clone classifier·set_vegetation_provider）+ Block A4 调用传参 |

### 架构

```
goal_resolution_system(world, cmd, vegetation, ocean, search_radius)
  │
  ├─ Pass 1: Desire → Goal（同步填充 target_pos）
  │    FindFood: vegetation.query_harvestable(npc_xz, 150m) → nearest
  │    FindWater: find_nearest_water_xz(npc_xz, ocean, 150m) → nearest XZ water
  │    FindRest/FindSafePlace/... → None（Phase 3+）
  │
  └─ Pass 2: 漫游回落（R4·不变）
```

### 最近水源搜索

极坐标网格（8 angles × 8 rings × 150m）——`OceanProvider::water_depth_at()` 逐点检测。返回最近有水 XZ 坐标（Y=0），movement 地形跟随 + 45° 坡度门自然停在岸边。

### VegetationProvider 初始化

WorldDriver `ready()` 中 biome_classifier 创建后立即 `clone()` → `BiomeVegetation::new().with_classifier().with_world_seed()` → `set_vegetation_provider()`。Arc 内部共享 WorldNoise，clone 便宜。

## 测试

**8 个新测试**（ecs·goal）：FindFood 填 target·选最近·无植被→None·空列表→None·FindWater 填 target·无水→None·FindRest 保持 None·搜索半径外→None·确定性验证。

## 诚实边界（本冲刺不做）

- 不解析 FindRest/FindSafePlace/FindSocialContact/BalanceElements/ExpressLibido（Phase 3+）
- 不实现体素 A* 寻路（NPC 直线走向目标·陡坡自然阻挡）
- 不区分淡水/咸水（V3a 的事）
- 不做 Godot 集成测试（woworld_godot 零测试基础设施·手工实机抽查）

## 审计结论

编码前二轮审查（16 项检查·🔴1 + 🟡7 + 🟢8）全部修正落地。编码后 zero regression。

---

> **下游**: V3a 代谢闭环（命门——采集→进食→需求真实下降）
> **关联**: [[../../sprint-proposals/sprint-070-V2-牵引移动-20260712]] · [[../../02-垂直切片/README]] §4
