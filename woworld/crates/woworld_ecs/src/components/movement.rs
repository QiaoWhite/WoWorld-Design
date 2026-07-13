//! NPC 移动 Component — ECS 铁律合规
//!
//! Movement 定义移动参数，Wander 定义无目标时的漫游方向。
//! 消费方: `systems/npc/movement.rs`
//!
//! 后续: speed 可从 BigFive 派生（外向性→快，尽责性→稳），Phase 2。

use glam::Vec3;
use serde::{Deserialize, Serialize};

/// 移动参数——movement_system 每帧驱动
///
/// `speed` 后续可从 BigFive + urgency 派生。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Movement {
    /// 基础移动速度 (m/s)
    pub speed: f32,
    /// 到达判定半径 (m)，进入此范围视为到达目标
    pub arrival_radius: f32,
}

impl Default for Movement {
    fn default() -> Self {
        Self {
            speed: 3.0,          // 步行 ~3 m/s
            arrival_radius: 0.5, // 半米内算到达
        }
    }
}

/// 漫游状态——当 Goal.target_pos 为 None 时，沿此方向移动
///
/// 定期重选方向以避免原地打转。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Wander {
    /// 归一化移动方向（XZ 平面）
    pub direction: Vec3,
    /// 剩余漫游时间 (s)，到期后重选方向
    pub remaining: f32,
}

impl Default for Wander {
    fn default() -> Self {
        Self {
            direction: Vec3::X, // 初始朝 +X
            remaining: 0.0,     // 0 → 立即触发重选
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_movement_default_walk_speed() {
        let m = Movement::default();
        assert!(m.speed > 0.0);
        assert!(m.speed < 10.0);
    }

    #[test]
    fn test_movement_arrival_radius_positive() {
        let m = Movement::default();
        assert!(m.arrival_radius > 0.0);
    }

    #[test]
    fn test_wander_direction_is_unit() {
        let w = Wander {
            direction: Vec3::new(1.0, 0.0, 0.0),
            remaining: 2.0,
        };
        assert!((w.direction.length() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_wander_default_triggers_immediate_reselect() {
        let w = Wander::default();
        assert_eq!(w.remaining, 0.0);
    }

    #[test]
    fn test_movement_custom_speed() {
        let m = Movement {
            speed: 5.0,
            arrival_radius: 1.0,
        };
        assert!((m.speed - 5.0).abs() < f32::EPSILON);
        assert!((m.arrival_radius - 1.0).abs() < f32::EPSILON);
    }
}
