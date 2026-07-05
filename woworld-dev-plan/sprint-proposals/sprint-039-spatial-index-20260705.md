# Sprint-039: SpatialGrid — EntityIndex 实现

> **提案日期**: 2026-07-05
> **提案状态**: ⏳ 待审批
> **所属阶段**: Phase 1 — 核心基础
> **前置**: woworld_core::spatial::EntityIndex trait ✅
> **阻塞解除**: LODCoordinator Phase 3 · NPC 感官 · 战斗射线 · 载具碰撞

## 目标

实现 `EntityIndex` trait 的 concrete 版本——`SpatialGrid`（均匀网格空间索引）。

### Grid-based 方案

- 世界空间划分为 50m×50m×50m 均匀网格
- 每 cell 存储 `Vec<SpatialEntity>`
- Entity→cell 反向查找：`HashMap<EntityId, (i32, i32, i32)>`
- AABB 查询：遍历重叠 cell，收集实体，AABB 精确过滤
- 6 个 trait 方法全部实现

### 复杂度

| 操作 | 复杂度 | 备注 |
|------|--------|------|
| register | O(1) | HashMap + Vec push |
| unregister | O(k) | k = cell 中实体数（通常 <10） |
| update_transform | O(k₁ + k₂) | 跨 cell 时 remove + insert |
| entities_in_aabb | O(c + n) | c = 重叠 cell 数, n = 结果数 |
| entity_aabb | O(1) | HashMap lookup |

### 交付

- `woworld_ecs/src/resources/spatial_grid.rs` — SpatialGrid struct + EntityIndex impl
- WorldDriver 集成 — `spatial_index: SpatialGrid`
- 测试：register/unregister/query/move/empty

## 预估

- **冲刺数**: 1
- **风险**: 🟢 低 — 纯数据结构，无外部依赖
- **代码量**: ~200 行
