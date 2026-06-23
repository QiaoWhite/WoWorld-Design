//! 空间查询 trait 定义
//!
//! 四大核心 trait 定义在 woworld_core，由 woworld_spatial 实现。
//! 仅玩家保留 Godot PhysicsServer3D——其余全部走 Rust 侧空间查询。
//!
//! 参见: `WoWorld-Design/开发路线图/002-轨A-正式开发.md` A.2 Tier 0
//! 参见: [[CLAUDE-INTERFACES.md]] CHG-033

use crate::material::*;
use crate::types::*;
use std::time::Duration;

// ── TerrainQuery ───────────────────────────────────

/// 地形查询（9 方法）
///
/// 消费方: 感官系统/战斗/NPC 移动/世界生成/生命
pub trait TerrainQuery: Send + Sync {
    /// 查询位置的地表高度
    fn height_at(&self, pos: WorldPos) -> f32;

    /// 查询位置的地表法线
    fn normal_at(&self, pos: WorldPos) -> Vec3;

    /// 地形射线检测——从 origin 沿 direction 发射射线
    fn terrain_raycast(
        &self,
        origin: WorldPos,
        direction: Vec3,
        max_dist: f32,
    ) -> Option<TerrainHit>;

    /// 查询位置的密度场值（体素密度）
    fn density_at(&self, pos: WorldPos) -> f32;

    /// 该位置是否可行走
    fn is_walkable(&self, pos: WorldPos) -> bool;

    /// 查询位置的地表材质
    fn surface_material_at(&self, pos: WorldPos) -> SurfaceMaterial;

    /// 查询位置所处的介质
    fn medium_at(&self, pos: WorldPos) -> Medium;

    /// 查询位置的光照等级 (0.0-1.0)
    fn light_level_at(&self, pos: WorldPos) -> f32;

    /// 采样各方向的地平线遮挡（用于天空可见度/环境光遮蔽）
    fn sample_horizon(&self, pos: WorldPos, directions: &[Vec3]) -> Vec<f32>;
}

// ── EntityIndex ────────────────────────────────────

/// 实体空间索引（6 方法）
///
/// 消费方: 感官系统/战斗/NPC/载具
pub trait EntityIndex: Send + Sync {
    /// 注册实体到空间索引
    fn register(&mut self, entity: SpatialEntity);

    /// 从空间索引注销实体
    fn unregister(&mut self, entity_id: EntityId);

    /// 更新实体变换（位置/旋转/速度）
    fn update_transform(&mut self, entity_id: EntityId, pos: WorldPos, rot: Quat, velocity: Vec3);

    /// 查询 AABB 内的所有实体
    fn entities_in_aabb(&self, aabb: &Aabb, layer_mask: u32) -> Vec<SpatialEntity>;

    /// 查询实体的当前 AABB
    fn entity_aabb(&self, entity_id: EntityId) -> Option<Aabb>;

    /// 查询位置的主导声学标签
    fn acoustic_tag_at(&self, pos: WorldPos) -> AcousticTag;
}

// ── SpatialEventBus ────────────────────────────────

/// 空间事件总线（3 方法）
///
/// 消费方: 感官系统/NPC 认知/音频
pub trait SpatialEventBus: Send + Sync {
    /// 查询时间窗口内 AABB 范围内的空间事件
    fn recent_events_in(&self, aabb: &Aabb, time_window: Duration) -> Vec<SpatialEvent>;

    /// 推送空间事件到总线
    fn push_event(&mut self, event: SpatialEvent);

    /// 查询 AABB 范围内的气味源
    fn scent_sources_in(&self, aabb: &Aabb) -> Vec<ScentSource>;
}

// ── VisibilityQuery ────────────────────────────────

/// 可见性查询（2 方法）
///
/// 消费方: 感官系统/战斗/NPC 认知
pub trait VisibilityQuery: Send + Sync {
    /// 两点之间是否可见（DDA march）
    fn line_of_sight(&self, from: WorldPos, to: WorldPos) -> bool;

    /// 射线命中检测——返回第一个命中点
    fn line_of_sight_hit(
        &self,
        from: WorldPos,
        to: WorldPos,
    ) -> Option<(WorldPos, SurfaceMaterial)>;
}
