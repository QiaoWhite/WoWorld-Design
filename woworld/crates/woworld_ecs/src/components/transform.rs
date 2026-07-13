//! 空间变换 Component — 纯数据，零方法（铁律 1）
//!
//! 三个基础 Component 表示实体在空间中的位置、朝向和速度。
//! 全部使用 glam SIMD 类型，固定大小，无堆分配（铁律 2）。

use glam::{Quat, Vec3};
use serde::{Deserialize, Serialize};

/// 世界空间位置（相对 floating origin）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position(pub Vec3);

/// 世界空间朝向
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rotation(pub Quat);

/// 线速度（m/s，世界空间）
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Velocity(pub Vec3);

// ── 便利构造 ──

impl Default for Position {
    fn default() -> Self {
        Self(Vec3::ZERO)
    }
}

impl Default for Rotation {
    fn default() -> Self {
        Self(Quat::IDENTITY)
    }
}

impl Default for Velocity {
    fn default() -> Self {
        Self(Vec3::ZERO)
    }
}
