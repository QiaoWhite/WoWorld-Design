# BACKLOG: Vf 食物源落地（可采集植被 · P2.25 最小子集）

> **类型**: 前置研究 Spike 产出的 backlog 草案（**非本冲刺执行项**）
> **来源**: Sprint-067 目标 3（只调研，不写实现代码）
> **日期**: 2026-07-12
> **里程碑**: 村庄切片第 4 步 Vf（`0/10` 序列见 [[../02-垂直切片/README]] §3）
> **状态**: 📋 草案——待 V0+V1 后作为独立中等 worldgen 冲刺提案

## 定位

让 `VegetationProvider::query_harvestable`（[[../../WoWorld-Design/Happy Game/开发阶段/世界生成/012-植被覆盖生成]]）返回**真实**可采集实例（Berry/Mushroom/Nut），供 V2 牵引移动 + V3a 代谢闭环消费。当前 `worldgen/vegetation.rs:168` 恒返回 `vec![]`。这是 A1 下"食物来自采集"的物理前提——**必须在 V2/V3a 之前落地**。

## crate 决策建议（本 spike 核心产出）

| 路线 | 内容 | 建议 |
|------|------|------|
| **B（推荐·Vf MVP）** | 最小扩展 worldgen 的 `BiomeVegetation`，填 `query_harvestable` stub：群系→硬编码可采集物种表 + 单层 Poisson 抖动 + 确定性 `instance_id` | ✅ Vf 走此路 |
| **A（拆独立大 backlog）** | 新建 `woworld_vegetation` crate + 完整 P2.25（VMC/TC 双层 + 香农熵物种表 + rayon + LMDB） | ⏳ 需先立 `woworld_life`/`PlantSpeciesRegistry` |

**理由**：设计文档反复引用的 `woworld_vegetation` / `woworld_life` 两个 crate **代码库中不存在**（workspace 仅 5 crate）。路线 A 前提缺失，会立刻卡在无物种数据源。路线 B 零新依赖、零新 crate，用现有 `BiomeClassifier` 即可产出"非空 harvestable"踏脚石。

## Research-first（🟡 Poisson 确定性放置）

库内确认**无任何 Poisson/blue-noise/scatter 实现**——需新写。行业标准 = **Bridson 2007**（O(n)、背景网格 `r/√n`、活跃列表、annulus [r,2r]、k=30 尝试上限）。per-chunk 确定性：`chunk_seed = worldSeed ^ (cx·prime1) ^ (cy·prime2)` + 种子化 PRNG（Mulberry32/splitmix）——**与设计 012 §八 `plant_instance_id = hash_4(seed,"plant_instance",cx,cy,idx)` 契约一致**。
- 参考 URL（保留可追溯）：Bridson SIGGRAPH 2007 <https://www.cs.ubc.ca/~rbridson/docs/bridson-siggraph07-poissondisk.pdf>

## 复用点（禁重复造轮子）

- 确定性 RNG：`woworld_ecs/src/prng.rs:10-33`（splitmix64 变体，无 `rand` 依赖）
- splitmix64 hash：`woworld_worldgen/src/noise_gen.rs:16`（`mix64`，可作 `hash_3/hash_4` 构建块）
- 群系分类：`woworld_worldgen/src/biome.rs:79`（`classify(WorldPos)`）
- trait/类型落点：`woworld_core/src/vegetation.rs:42`（`query_harvestable`）、`:126`（`HarvestableInfo`）、`:143`（`ProductCategory` 含 Berry/Mushroom/Nut）
- Poisson 参数/种子规范：[[../../WoWorld-Design/Happy Game/开发阶段/世界生成/012-植被覆盖生成]] §Poisson（:205-235）+ §八确定性契约（:504-517）
- 香农熵物种筛选：[[../../WoWorld-Design/Happy Game/开发阶段/生命/010-植被群落与覆盖]] §3.2（:313-347，`H=-Σ(p·ln p)`→richness clamp[1,5]）

## 接口纪律（A1）

资源查询一律**复用** `VegetationProvider::query_harvestable`（食物）+ `OceanProvider`（水），**禁新建"需求满足点"等平行 trait**（违概念所有权铁律）。

## 开放问题（待 Vf 提案裁决）

1. **crate**：Vf 做路线 B（worldgen 内扩展）还是等 A？→ 建议 B，A 拆独立 backlog。
2. **`instance_id` 类型**：保留代码现状 `u64` vs 引入设计文档的 `PlantInstanceId` newtype？
3. **`regen_state`**：是否给 `HarvestableInfo` 补 `RegenState`（枚举已定义 `core/vegetation.rs:155` 但未接线）？
4. **物种数据源**：MVP 硬编码（Forest→Berry/Mushroom/Nut，Grassland→Herb/Flower）vs 等 `PlantSpeciesRegistry`？
5. **splitmix 收敛**：库内 splitmix64 已重复 ≥4 处——是否借 Vf 收敛到 `woworld_core`？
6. **VMC/TC 双层**：MVP 是否需要 1km VMC 抽象，还是直接在 32m TC / 局部半径查询上跑 Poisson？

## ⚠️ 代码 vs 设计分歧（如实记录）

代码 `HarvestableInfo{ instance_id: u64, 无 regen_state }`（`core/vegetation.rs:126`）与 [[../../WoWorld-Design/Happy Game/开发阶段/生命/010-植被群落与覆盖]] §1.2 设计（`instance_id: PlantInstanceId` + `regen_state: RegenState`）不一致。Vf 落地时须裁决：对齐文档补齐，或正式记录简化决策。

## 验收草案（未来实现时，非现在）

- `query_harvestable(pos, radius)` 返回确定性非空 `Vec<HarvestableInfo>`（同种子→同实例→同 `instance_id`）。
- 至少 Berry/Mushroom/Nut 三类；群系差异（Forest 多、Grassland 少）。
- 测试：确定性复现 + 半径查询正确 + `instance_id` 不碰撞。

---

> **上游**: [[sprint-067-V0V1-地基与昼夜涌现-20260712]] 目标 3 · [[../02-垂直切片/README]] §4 Vf 行
> **设计依据**: [[../../WoWorld-Design/Happy Game/开发阶段/世界生成/012-植被覆盖生成]] · [[../../WoWorld-Design/Happy Game/开发阶段/生命/010-植被群落与覆盖]]
