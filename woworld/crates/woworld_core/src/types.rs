//! 空间基元与实体类型
//!
//! Woworld_core 的基础类型定义。所有坐标、实体、空间事件类型均在此。
//! 参见: `WoWorld-Design/开发路线图/002-轨A-正式开发.md` A.2 Tier 0

use glam::{DVec3 as GlamDVec3, Quat as GlamQuat, Vec3 as GlamVec3};

// ── 坐标与向量 ──────────────────────────────────────

/// 世界坐标（double 精度，覆盖 500km+ 范围）
/// y 轴为高度
#[derive(Copy, Clone, Debug, Default)]
pub struct WorldPos {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// f32 方向向量（SIMD 加速，归一化）
pub type Vec3 = GlamVec3;

/// f64 世界坐标向量（用于大范围运算）
pub type DVec3 = GlamDVec3;

/// 四元数旋转
pub type Quat = GlamQuat;

/// 轴对齐包围盒（f64 世界坐标）
#[derive(Copy, Clone, Debug, Default)]
pub struct Aabb {
    pub min: WorldPos,
    pub max: WorldPos,
}

// ── 实体标识 ──────────────────────────────────────

/// 统一实体标识符 (u64)
///
/// 位布局:
/// - bit63: 0=Item(ItemEntId) / 1=NonItem
/// - bit[62:60]: entity_kind
/// - bit[59:0]: local_id
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntityId(pub u64);

/// 实体类别（bit[62:60] 共 8 种，当前使用 5 种）
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum EntityKind {
    Creature = 0,
    BuildingComponent = 1,
    TerrainFeature = 2, // 矿脉/岩层/黏土/巨石
    DroppedItem = 3,
    Plant = 4,
}

// ── 地形查询 ──────────────────────────────────────

/// 地形射线命中结果
#[derive(Copy, Clone, Debug)]
pub struct TerrainHit {
    pub point: WorldPos,
    pub normal: Vec3,
    pub material: crate::material::SurfaceMaterial,
    pub distance: f32,
}

// ── 空间索引 ──────────────────────────────────────

/// 空间索引中的实体条目
#[derive(Clone, Debug)]
pub struct SpatialEntity {
    pub id: EntityId,
    pub pos: WorldPos,
    pub rot: Quat,
    pub velocity: Vec3,
    pub aabb: Aabb,
    pub entity_kind: EntityKind,
    /// 图层掩码（32 位，每位一个图层）
    pub layer_mask: u32,
}

// ── 空间事件 ──────────────────────────────────────

/// 空间事件——SpatialEventBus 发布/订阅的消息
#[derive(Clone, Debug)]
pub struct SpatialEvent {
    /// 事件类型标签（如 "combat_hit", "spell_cast", "footstep_loud"）
    pub event_type: &'static str,
    /// 事件发生位置
    pub position: WorldPos,
    /// 事件强度 (0.0-1.0)
    pub intensity: f32,
    /// 事件时间戳（模拟秒）
    pub timestamp: f64,
    /// 事件源实体（可选）
    pub source_entity: Option<EntityId>,
}

// ── 气味 ──────────────────────────────────────────

/// 气味源——ScentQuery 懒采样输入
#[derive(Clone, Debug)]
pub struct ScentSource {
    pub position: WorldPos,
    /// 气味类型标签（如 "food_cooking", "blood", "incense"）
    pub scent_type: &'static str,
    /// 初始强度 (0.0-1.0)
    pub intensity: f32,
    /// 衰减速率（每秒衰减的比例）
    pub decay_rate: f32,
    pub source_entity: Option<EntityId>,
}

// ── 声学 ──────────────────────────────────────────

/// 材质声学标签——EntityIndex::acoustic_tag_at() 返回值
/// 值 0-20 对应 21 种 SurfaceMaterial 变体
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AcousticTag(pub u8);
