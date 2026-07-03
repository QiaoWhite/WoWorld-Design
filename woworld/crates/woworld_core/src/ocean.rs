//! 海洋查询 trait 定义
//!
//! 海洋覆盖世界 70% 表面积。OceanProvider 是所有海洋相关查询的权威入口。
//! 消费方: NPC 移动（避水/亲水）、物理系统（浮力）、水下渲染、战斗、船只航行。
//!
//! 参见: [[CLAUDE-INTERFACES.md]] OceanProvider
//! 参见: `DEVLOG-2026-07-03.md` Sprint-029

use crate::types::WorldPos;

/// 海洋查询 trait（4 方法）
///
/// 当前实现 `HeightfieldOcean` 位于 `woworld_worldgen`。
/// 海平面恒为 `y = 0.0`，未来支持潮汐/风暴/群系降水调制。
pub trait OceanProvider: Send + Sync {
    /// 海平面高度（米，世界坐标 Y 轴）
    ///
    /// `pos.y` 被忽略——海平面仅随水平位置变化。
    /// 当前恒返回 `0.0`，预留未来潮汐/风暴 surge 的空间。
    fn sea_level_at(&self, pos: WorldPos) -> f64;

    /// 波面高度（米）——Gerstner 波叠加后的实际水面偏移
    ///
    /// 消费方: 船只摇晃幅度、浮力计算、水面精确查询。
    /// `time` 是世界时间（秒）——由 `WorldClock.seconds_since_epoch()` 提供。
    fn wave_height_at(&self, pos: WorldPos, time: f64) -> f64;

    /// 水深（米）——`max(0, sea_level - terrain_height)`
    ///
    /// 消费方: 水下渲染（颜色深度渐变）、海洋色板调制。
    /// `pos.y` 被忽略——仅使用 `(x, z)` 查地形高度。
    fn water_depth_at(&self, pos: WorldPos) -> f64;

    /// 给定位置是否在水面以下（`sea_level + wave_height`）
    ///
    /// 消费方: NPC 溺水判定、玩家游泳切换、摄像机水下检测、物理浮力。
    /// `pos.y` 与波面比较——不是海平面，考虑实时波形。
    fn is_underwater(&self, pos: WorldPos, time: f64) -> bool;
}
