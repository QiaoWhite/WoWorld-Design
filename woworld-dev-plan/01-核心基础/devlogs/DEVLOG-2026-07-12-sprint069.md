# DEVLOG 2026-07-12 — Sprint-069 Vf 食物源落地

> **冲刺**: Sprint-069 — Vf 食物源落地（垂直切片「活着的村庄」第 4/10 步）
> **状态**: ✅ 完成·1092 tests 全绿·clippy/fmt 零警告
> **提案**: [[../../sprint-proposals/sprint-069-Vf-食物源落地-20260712]]（6 项裁决 D1-D6）

## 做了什么

让 `VegetationProvider::query_harvestable` 从 `vec![]` stub 升级为**确定性 Poisson disc 采集物生成**。A1 "食物来自采集"的物理前提就位——NPC 现在能查到真实的可采集浆果/蘑菇/坚果点。

### 产物（代码）

| 文件 | 内容 |
|------|------|
| `core/vegetation.rs` | `HarvestableInfo` +`regen_state: RegenState` 字段（对齐设计 010 §1.2） |
| `worldgen/noise_gen.rs` | `mix64` → `pub(crate)`（1 词变更·零重复代码） |
| `worldgen/biome.rs` | `BiomeClassifier` +`pub fn sample_height()`（1 行委托 `WorldNoise`） |
| `worldgen/vegetation.rs` | ① `BiomeVegetation` +`world_seed` + `with_world_seed()` ② `query_harvestable` 实现 ③ Poisson disc Bridson 2007 ④ 群系→产物映射 ⑤ 17 个新测试 |

### 架构

```
query_harvestable(pos, radius)
  │
  ├─ TC 覆盖范围（32m 粒度 + r margin）
  ├─ 群系分类 → 跳过海洋/气候空档/Snowfield
  ├─ Desert 20% 稀疏门
  ├─ Poisson disc 确定性放置（Bridson 2007·r=4m·k=30）
  │   种子 = mix64(world_seed ^ "vegetati" ^ tc_x ^ tc_z)
  ├─ 加权抽取产物类别（Forest→Berry 40%/Mushroom 35%/Nut 25%···）
  ├─ classifier.sample_height(x,z) → 地形 y
  └─ HarvestableInfo { instance_id, species_id, position, category, yield, regen_state: Full }
```

### Poisson disc 参数（Bridson 2007）

- 采样域：TC 32m + 4m margin = 40m×40m
- r=4m（采集物是全集子集·有效间距 ≈ 灌木间距 2m/√0.25）
- 背景网格 r/√2 ≈ 2.828m
- k=30 尝试上限·确定性 RNG（`mix64` 链式种子推进）

### 群系→产物映射（硬编码·待 PlantSpeciesRegistry）

| 群系 | 产物分布 | yield |
|------|---------|-------|
| Forest | Berry 40% / Mushroom 35% / Nut 25% | 1.0-3.0 |
| Grassland | Herb 50% / Berry 30% / Flower 20% | 0.5-2.0 |
| Swamp | Mushroom 60% / Herb 25% / Fiber 15% | 0.5-2.5 |
| Desert | Herb 60% / Fiber 40%（仅 20% TC）| 0.2-1.0 |
| Snowfield | 无 | — |

## 测试

**17 个新测试**（worldgen·vegetation）：确定性复现·半径单调递增·最小间距验证·instance_id 唯一·高度有限·不同世界种子隔离·Poisson 同种子同结果·产物分布覆盖·哨兵值文档。

## 诚实边界（本冲刺不做）

- 不建 `woworld_life`/`woworld_vegetation` crate
- 不实现 VMC 双层架构（直接 TC 粒度）
- 不做 RegenState 消耗/再生（全部 Full）
- 不做季节连线（season_optimal 恒 true）
- 不做 splitmix 跨 crate 收敛（复用同 crate `mix64`·零重复）
- Godot 桥 `set_vegetation_provider` 未调用（预存 gap·V2/V3a 集成时接通）

## 审计结论

编码后审计 7 维（字段/种子/Poisson/群系映射/高度采样/边缘情况/A1）全部通过。发现 2 个自修问题（未使用参数清理·SpeciesId 文档补全）+ 1 个正当细化（yield 按产物类别细分而非群系统一）。

---

> **下游**: V2 牵引移动（读 `query_harvestable` 找最近食物点）→ V3a 代谢闭环
> **关联**: [[../../sprint-proposals/sprint-069-Vf-食物源落地-20260712]] · [[../../02-垂直切片/README]]
