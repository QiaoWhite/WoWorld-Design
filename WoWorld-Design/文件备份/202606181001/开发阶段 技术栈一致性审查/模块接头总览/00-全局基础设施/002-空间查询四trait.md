# 空间查询四 trait — 详细签名参考

> **最后验证**: 2026-06-18 | **源文档版本**: CHG-033 权威 | **定位**: woworld_core 定义, world_gen + woworld_spatial 实现
>
> 本文档是 [[001-核心类型注册表|核心类型注册表]] 中空间查询四 trait 的详细签名参考。所有模块通过这四个 trait 进行空间查询 — 替代 Godot PhysicsServer3D (仅玩家保留)。

---

## 一、TerrainQuery (9 方法)

**实现方**: world_gen crate (密度场)  
**类型**: `pub trait TerrainQuery: Send + Sync`  
**消费方**: 感官/导航/动画/战斗/载具/音频

```rust
pub trait TerrainQuery: Send + Sync {
    /// 世界位置的地面高度 (水体返回水面)
    fn height_at(&self, pos: WorldPos) -> f32;

    /// 世界位置的地面法线
    fn normal_at(&self, pos: WorldPos) -> Vec3;

    /// DDA 射线在密度场上步进, 返回第一个命中
    /// ~10µs/射线
    fn terrain_raycast(&self, origin: WorldPos, direction: Vec3, max_dist: f32) -> Option<TerrainHit>;

    /// 某点的密度场值 (0=空气, 1=固体)
    fn density_at(&self, pos: WorldPos) -> f32;

    /// 此位置是否可行走 (地面 + 坡度 ≤ 阈值 + 非液体)
    fn is_walkable(&self, pos: WorldPos) -> bool;

    /// 地表材质 (草/沙/石/木/金属/水/冰/泥/雪)
    fn surface_material_at(&self, pos: WorldPos) -> SurfaceMaterial;

    /// 介质类型 (空气/水/岩浆/虚空)
    fn medium_at(&self, pos: WorldPos) -> Medium;

    /// 光照等级 (0.0=全暗, 1.0=全亮 — 不含动态光源)
    fn light_level_at(&self, pos: WorldPos) -> f32;

    /// 采样水平方向遮挡 (天际线/峡谷/森林遮罩)
    fn sample_horizon(&self, pos: WorldPos, directions: &[Vec3]) -> Vec<f32>;
}

/// 地面命中结果
pub struct TerrainHit {
    pub point: WorldPos,
    pub normal: Vec3,
    pub material: SurfaceMaterial,
    pub distance: f32,
}

/// 地表材质 (21 变体 — 音频模块权威)
pub enum SurfaceMaterial {
    Grass, Sand, Rock, Stone, Wood, Metal, Water, Ice, Mud,
    Snow, Gravel, Clay, Moss, LeafLitter, Cobblestone,
    Marble, Glass, Fabric, Thatch, Bone, Flesh,
}

/// 介质类型
pub enum Medium {
    Air,        // 空气 (默认)
    Water,      // 水 (淡水/咸水由上下文区分)
    Magma,      // 岩浆
    Void,       // 虚空 (世界边界外)
}
```

---

## 二、EntityIndex (6 方法)

**实现方**: woworld_spatial crate  
**类型**: `pub trait EntityIndex: Send + Sync`  
**消费方**: 所有模块 (通过 SpatialQuery trait 访问)

```rust
pub trait EntityIndex: Send + Sync {
    /// 注册实体到空间索引
    fn register(&mut self, entity: SpatialEntity);

    /// 注销实体
    fn unregister(&mut self, entity_id: EntityId);

    /// 更新实体空间变换
    fn update_transform(&mut self, entity_id: EntityId, pos: WorldPos, rot: Quat, velocity: Vec3);

    /// 查询 AABB 内的所有实体
    fn entities_in_aabb(&self, aabb: &Aabb, layer_mask: u32) -> Vec<SpatialEntity>;

    /// 获取实体的 AABB (用于碰撞检测)
    fn entity_aabb(&self, entity_id: EntityId) -> Option<Aabb>;

    /// 查询某位置的声学标签 (用于音频传播)
    fn acoustic_tag_at(&self, pos: WorldPos) -> AcousticTag;
}

/// 空间实体 (不含业务语义 — 实体类型由调用方判断)
pub struct SpatialEntity {
    pub id: EntityId,
    pub pos: WorldPos,
    pub velocity: Vec3,
    pub aabb: Aabb,
    pub layer_mask: u32,        // 粗分类 (NPC/物品/建筑/载具/地形/效果)
    pub stealth: f32,           // 隐蔽度 0-1 (0=显眼, 1=完全隐蔽)
    pub luminosity: f32,        // 发光量 (0=不发光, >1=强烈光源)
    pub opacity: f32,           // 透明度 (1=完全不透明, 0=透明)
}

/// 声学标签
pub enum AcousticTag {
    OpenField, SmallRoom, MediumRoom, LargeHall,
    StoneCathedral, Cave, Tunnel, Forest, Underwater,
    DenseUrban, OpenWater,
}
```

---

## 三、SpatialEventBus (3 方法)

**实现方**: woworld_spatial crate  
**类型**: `pub trait SpatialEventBus: Send + Sync`  
**消费方**: 所有模块 (写入/查询空间事件)

```rust
pub trait SpatialEventBus: Send + Sync {
    /// 查询某位置半径内的近期事件
    fn recent_events_in(&self, pos: WorldPos, radius_m: f32, max_age: Duration) -> Vec<SpatialEvent>;

    /// 推送事件到空间事件总线
    fn push_event(&mut self, event: SpatialEvent);

    /// 查询某位置的所有活跃气味源
    fn scent_sources_in(&self, pos: WorldPos, radius_m: f32) -> Vec<ScentSource>;
}

/// 空间事件
pub struct SpatialEvent {
    pub source_pos: WorldPos,
    pub intensity: f32,         // 0-1 (影响传播距离和可见性)
    pub category: EventCategory,
    pub timestamp: GameTimestamp,
    pub source_entity: Option<EntityId>,
}

/// 事件类别
pub enum EventCategory {
    Combat(CombatEventDetail),
    Movement(MovementEventDetail),
    Sound(SoundEventDetail),
    Visual(VisualEventDetail),
    Magic(MagicEventDetail),
    Interaction(InteractionEventDetail),
    Environmental(EnvironmentalEventDetail),
}

/// 气味源
pub struct ScentSource {
    pub pos: WorldPos,
    pub scent_type: ScentType,
    pub intensity: f32,         // 当前浓度 (随时间衰减)
    pub source_entity: Option<EntityId>,
    pub expiry: GameTimestamp,
}
```

**存储架构**: Chunk粒度(16m) ring buffer, 每Chunk 64条, LRU淘汰. 事件自动过期 `max(intensity × 10s, 5s)`.

---

## 四、VisibilityQuery (2 方法)

**实现方**: woworld_spatial (Arc\<TerrainQuery\> + &EntityIndex)  
**类型**: `pub trait VisibilityQuery: Send + Sync`  
**消费方**: 感官/战斗/大日志/音频

```rust
pub trait VisibilityQuery: Send + Sync {
    /// 两点之间是否有视线
    /// DDA 同时检查密度场 + 实体 AABB
    fn line_of_sight(&self, from: WorldPos, to: WorldPos) -> bool;

    /// 两点之间的视线, 如有遮挡返回命中点
    fn line_of_sight_hit(&self, from: WorldPos, to: WorldPos) -> LineOfSightResult;
}

/// 视线结果
pub enum LineOfSightResult {
    Clear,                          // 无遮挡
    TerrainHit(TerrainHit),         // 被地形遮挡
    EntityHit {                     // 被实体遮挡
        entity_id: EntityId,
        point: WorldPos,
        normal: Vec3,
        distance: f32,
    },
    WaterSurface {                  // 被水面遮挡 (水下视线)
        point: WorldPos,
        distance: f32,
    },
}
```

**性能**: DDA射线步进 ~10µs/射线 (密度场 + 实体AABB同时检查).

---

## 五、组合 trait (woworld_core 便捷聚合)

```rust
/// 空间查询的完整聚合 — 大多数消费者解引用这一个 trait 对象即可
pub trait SpatialQuery: TerrainQuery + EntityIndex + SpatialEventBus + VisibilityQuery {}

// 对所有实现了四个子 trait 的类型自动实现
impl<T: TerrainQuery + EntityIndex + SpatialEventBus + VisibilityQuery> SpatialQuery for T {}
```

**不要**直接 import 四个子 trait — 通过 `SpatialQuery` 统一入口访问。

---

## 六、消费者矩阵

| 模块 | TerrainQuery | EntityIndex | SpatialEventBus | VisibilityQuery |
|------|:---:|:---:|:---:|:---:|
| 感官(05) | height/normal/density | entities_in_aabb | recent_events/scent | line_of_sight |
| NPC(02) | is_walkable/normal | entities_in_aabb | recent_events | line_of_sight |
| 战斗(06) | terrain_raycast | entities_in_aabb/entity_aabb | push_event | line_of_sight |
| 动画(17) | height/normal/surface | entity_aabb | push_event | — |
| 载具(15) | height/is_walkable | entities_in_aabb/register | push_event | — |
| 音频(16) | surface/medium | acoustic_tag_at | push_event | line_of_sight |
| 世界生成(03) | (实现方) | — | — | — |
| 大日志 | — | — | recent_events | line_of_sight |

---

## 七、CHG-033 迁移说明

**旧方案** (技术栈 v3.0): Godot PhysicsServer3D — 碰撞检测/射线/空间查询  
**新方案** (CHG-033): Rust 侧空间查询四 trait

**保留 Godot PhysicsServer3D**: 仅玩家 CharacterBody3D  
**删除**: NPC/物品/投射物的 PhysicsServer3D 碰撞体

**收益**:
- 减少 GDExtension 跨边界调用 (碰撞检测不离开 Rust)
- 统一空间查询接口 — 所有模块同一套 trait
- DDA射线 ~10µs/射线 (Rust原生, 无Godot开销)
- 骨架松弛 ~0.03ms/死亡NPC (替代布娃娃物理)

---

> **关联文档**: [[../../模型动作与物理系统/空间查询与物理/001-空间查询与物理系统|空间查询四trait 规格]] · [[001-核心类型注册表|核心类型注册表]] · [[../../../../CLAUDE-INTERFACES.md|CLAUDE-INTERFACES CHG-033]]
