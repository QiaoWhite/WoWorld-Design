//! 相机工具函数 —— SmoothDamp 家族 + 地形碰撞夹紧
//!
//! SmoothDamp: Unity 风格平滑阻尼（临界阻尼 + 速度状态）。
//! resolve_camera_arm: 地形射线夹紧臂长（纯函数，引擎无关）。
//!
//! 参见: 玩家系统 007 §四/§八/§十四

use glam::{Quat, Vec3};

use crate::spatial::TerrainQuery;
use crate::types::WorldPos;

// ── 常量 ────────────────────────────────────────

/// 相机臂长地板（TP 防入头 / FP 允许 0）
pub const MIN_ARM: f32 = 0.3;

/// 化身隐藏臂长阈值（near_clip + capsule_radius + margin ≈ 0.9 → 1.0）
pub const HIDE_ARM_THRESHOLD: f32 = 1.0;

// ── SmoothDamp ──────────────────────────────────

/// Unity `Mathf.SmoothDamp` 移植——临界阻尼平滑，无过冲。
///
/// - `current`: 当前值
/// - `target`: 目标值
/// - `velocity`: 速度状态（跨帧保持）
/// - `smooth_time`: 约达到目标的时间 (s)
/// - `dt`: 帧间隔 (s)
/// - `max_speed`: 可选最大变化速率（None = 不限）
pub fn smooth_damp(
    current: f32,
    target: f32,
    velocity: &mut f32,
    smooth_time: f32,
    dt: f32,
    max_speed: Option<f32>,
) -> f32 {
    let st = smooth_time.max(0.0001);
    let omega = 2.0 / st;
    let x = omega * dt;
    let exp = 1.0 / (1.0 + x + 0.48 * x * x + 0.235 * x * x * x);
    let change = current - target;
    let temp = (*velocity + omega * change) * dt;
    let mut v = (*velocity - omega * temp) * exp;
    let mut result = target + (change + temp) * exp;

    // 防止过冲（current 越过 target）
    if (change > 0.0) == (result > target) {
        result = target;
        v = (result - target) / dt;
    }

    if let Some(ms) = max_speed {
        if v.abs() > ms {
            v = ms * v.signum();
        }
    }

    *velocity = v;
    result
}

/// Vec3 版的 `smooth_damp`（逐轴独立平滑）。
pub fn smooth_damp_vec3(
    current: Vec3,
    target: Vec3,
    velocity: &mut Vec3,
    smooth_time: f32,
    dt: f32,
    max_speed: Option<f32>,
) -> Vec3 {
    let mut vx = velocity.x;
    let mut vy = velocity.y;
    let mut vz = velocity.z;
    let rx = smooth_damp(current.x, target.x, &mut vx, smooth_time, dt, max_speed);
    let ry = smooth_damp(current.y, target.y, &mut vy, smooth_time, dt, max_speed);
    let rz = smooth_damp(current.z, target.z, &mut vz, smooth_time, dt, max_speed);
    *velocity = Vec3::new(vx, vy, vz);
    Vec3::new(rx, ry, rz)
}

/// Quat 版平滑——无状态 slerp + 临界阻尼系数。
///
/// 用于 `character_facing_system`，避免将角速度作为组件存储。
///
/// - `current`: 当前朝向
/// - `target`: 目标朝向
/// - `smooth_time`: 约达到目标的时间 (s)
/// - `dt`: 帧间隔
/// - `turn_rate_rad_s`: 可选最大转向速率（弧度/秒）
pub fn smooth_damp_quat(
    current: Quat,
    target: Quat,
    smooth_time: f32,
    dt: f32,
    turn_rate_rad_s: Option<f32>,
) -> Quat {
    let st = smooth_time.max(0.0001);
    let omega = 2.0 / st;
    let t = 1.0 - (-omega * dt).exp();

    if let Some(rate) = turn_rate_rad_s {
        let diff_angle = current.angle_between(target);
        let max_angle = rate * dt;
        let clamp_t = if diff_angle > 1e-6 {
            (max_angle / diff_angle).min(t)
        } else {
            t
        };
        current.slerp(target, clamp_t)
    } else {
        current.slerp(target, t)
    }
}

// ── 相机碰撞 ────────────────────────────────────

/// 地形射线夹紧臂长——纯函数，可单测。
///
/// 从 pivot 沿 `dir`（须归一化）发射射线，最长 `desired + margin`。
/// 命中则返回 `(distance - margin).clamp(MIN_ARM.min(desired), desired)`。
///
/// - `MIN_ARM.min(desired)` 地板：FP（desired=0）时 arm 可为 0；TP 时地板 0.3 防入头。
/// - 零 desired / 零方向 → 直接返回 desired（安全的 no-op）。
pub fn resolve_camera_arm(
    terrain: &dyn TerrainQuery,
    pivot: WorldPos,
    dir: Vec3,
    desired: f32,
    margin: f32,
) -> f32 {
    if desired < 0.001 {
        return 0.0;
    }
    let dir_n = dir.normalize_or_zero();
    if dir_n.length_squared() < 1e-6 {
        return desired;
    }
    let max_dist = desired + margin;
    match terrain.terrain_raycast(pivot, dir_n, max_dist) {
        Some(hit) => {
            let floor = MIN_ARM.min(desired);
            (hit.distance - margin).clamp(floor, desired)
        }
        None => desired,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::SurfaceMaterial;
    use crate::types::TerrainHit;

    // ── Mock TerrainQuery ─────────────────────

    struct MockTerrain {
        hit_distance: Option<f32>,
    }

    impl TerrainQuery for MockTerrain {
        fn height_at(&self, _pos: WorldPos) -> f32 {
            0.0
        }
        fn normal_at(&self, _pos: WorldPos) -> Vec3 {
            Vec3::Y
        }
        fn terrain_raycast(
            &self,
            _origin: WorldPos,
            _direction: Vec3,
            _max_dist: f32,
        ) -> Option<TerrainHit> {
            self.hit_distance.map(|d| TerrainHit {
                point: WorldPos {
                    x: 0.0,
                    y: 0.0,
                    z: d as f64,
                },
                normal: Vec3::Y,
                material: SurfaceMaterial::Mud,
                distance: d,
            })
        }
        fn density_at(&self, _pos: WorldPos) -> f32 {
            0.0
        }
        fn is_walkable(&self, _pos: WorldPos) -> bool {
            true
        }
        fn surface_material_at(&self, _pos: WorldPos) -> SurfaceMaterial {
            SurfaceMaterial::Mud
        }
        fn medium_at(&self, _pos: WorldPos) -> crate::material::Medium {
            crate::material::Medium::Air
        }
        fn light_level_at(&self, _pos: WorldPos) -> f32 {
            1.0
        }
        fn sample_horizon(&self, _pos: WorldPos, directions: &[Vec3]) -> Vec<f32> {
            vec![0.0; directions.len()]
        }
    }

    fn pivot() -> WorldPos {
        WorldPos {
            x: 10.0,
            y: 1.5,
            z: 10.0,
        }
    }

    fn dir_back() -> Vec3 {
        Vec3::Z // Godot +Z = 身后
    }

    // ── smooth_damp ────────────────────────────

    #[test]
    fn test_smooth_damp_converges() {
        let mut vel = 0.0_f32;
        let mut cur = 0.0_f32;
        let target = 10.0_f32;
        let dt = 1.0 / 60.0;
        for _ in 0..120 {
            cur = smooth_damp(cur, target, &mut vel, 0.3, dt, None);
        }
        // 120 帧 (2s) 内应收敛到 1% 以内
        assert!((cur - target).abs() < 0.1, "cur={cur} target={target}");
    }

    #[test]
    fn test_smooth_damp_no_overshoot() {
        let mut vel = 0.0_f32;
        let mut cur = 0.0_f32;
        let target = 10.0_f32;
        let dt = 1.0 / 60.0;
        for _ in 0..200 {
            cur = smooth_damp(cur, target, &mut vel, 0.1, dt, None);
            // 应单调不减
            assert!(cur <= target + 1e-5);
        }
    }

    #[test]
    fn test_smooth_damp_instant_with_zero_time() {
        let mut vel = 0.0_f32;
        let cur = 5.0_f32;
        // 极小的 smooth_time → 近乎瞬间
        let r = smooth_damp(cur, cur, &mut vel, 0.0001, 0.016, None);
        // 当前 == 目标 → 不变
        assert!((r - cur).abs() < 0.001);
    }

    #[test]
    fn test_smooth_damp_vec3_converges() {
        let mut vel = Vec3::ZERO;
        let mut cur = Vec3::ZERO;
        let tgt = Vec3::new(5.0, 10.0, -3.0);
        let dt = 1.0 / 60.0;
        for _ in 0..120 {
            cur = smooth_damp_vec3(cur, tgt, &mut vel, 0.2, dt, None);
        }
        assert!((cur.x - tgt.x).abs() < 0.1);
        assert!((cur.y - tgt.y).abs() < 0.1);
        assert!((cur.z - tgt.z).abs() < 0.1);
    }

    // ── smooth_damp_quat ───────────────────────

    #[test]
    fn test_smooth_damp_quat_no_change_when_aligned() {
        let q = Quat::IDENTITY;
        let r = smooth_damp_quat(q, q, 0.1, 0.016, None);
        let angle = r.angle_between(q);
        assert!(angle < 0.001, "angle={angle}");
    }

    #[test]
    fn test_smooth_damp_quat_rotates_toward_target() {
        use std::f32::consts::FRAC_PI_2;
        let current = Quat::IDENTITY;
        let target = Quat::from_rotation_y(FRAC_PI_2); // 90° yaw
        let dt = 1.0 / 60.0;
        // 第一帧应确实旋转
        let r1 = smooth_damp_quat(current, target, 0.1, dt, None);
        let angle1 = r1.angle_between(current);
        assert!(angle1 > 0.0, "should rotate on first frame");

        // 多帧后应收敛到 target
        let mut cur = current;
        for _ in 0..120 {
            cur = smooth_damp_quat(cur, target, 0.1, dt, None);
        }
        let final_angle = cur.angle_between(target);
        assert!(final_angle < 0.05, "final_angle={final_angle}");
    }

    #[test]
    fn test_smooth_damp_quat_clamps_turn_rate() {
        use std::f32::consts::PI;
        let current = Quat::IDENTITY;
        let target = Quat::from_rotation_y(PI); // 180°
        let dt = 1.0 / 60.0;
        let max_rate = 1.0_f32; // 1 rad/s
        let max_per_frame = max_rate * dt;
        let r = smooth_damp_quat(current, target, 0.2, dt, Some(max_rate));
        let angle = r.angle_between(current);
        assert!(
            angle <= max_per_frame + 0.001,
            "angle={angle} max_per_frame={max_per_frame}"
        );
    }

    // ── resolve_camera_arm ─────────────────────

    #[test]
    fn test_camera_arm_no_hit_returns_desired() {
        let terrain = MockTerrain { hit_distance: None };
        let arm = resolve_camera_arm(&terrain, pivot(), dir_back(), 4.0, 0.3);
        assert!((arm - 4.0).abs() < 0.001);
    }

    #[test]
    fn test_camera_arm_hit_reduces_by_margin() {
        let terrain = MockTerrain {
            hit_distance: Some(3.0),
        };
        let arm = resolve_camera_arm(&terrain, pivot(), dir_back(), 4.0, 0.3);
        assert!((arm - 2.7).abs() < 0.001, "arm={arm}");
    }

    #[test]
    fn test_camera_arm_clamped_to_desired_on_far_hit() {
        let terrain = MockTerrain {
            hit_distance: Some(10.0),
        };
        let arm = resolve_camera_arm(&terrain, pivot(), dir_back(), 4.0, 0.3);
        assert!((arm - 4.0).abs() < 0.001, "hit beyond desired should not extend arm");
    }

    #[test]
    fn test_camera_arm_blocked_floors_at_min_arm() {
        let terrain = MockTerrain {
            hit_distance: Some(0.2),
        };
        // desired=4.0, margin=0.3, hit=0.2 → floor = MIN_ARM(0.3).min(4.0) = 0.3
        let arm = resolve_camera_arm(&terrain, pivot(), dir_back(), 4.0, 0.3);
        assert!((arm - 0.3).abs() < 0.001, "arm={arm} should floor at MIN_ARM");
    }

    #[test]
    fn test_camera_arm_fp_zero_desired_allows_zero() {
        let terrain = MockTerrain {
            hit_distance: Some(1.0),
        };
        // FP: desired=0, floor = MIN_ARM(0.3).min(0.0) = 0.0
        let arm = resolve_camera_arm(&terrain, pivot(), dir_back(), 0.0, 0.3);
        assert!((arm - 0.0).abs() < 0.001, "FP arm must stay 0, got {arm}");
    }

    #[test]
    fn test_camera_arm_zero_direction_safe() {
        let terrain = MockTerrain {
            hit_distance: Some(1.0),
        };
        let arm = resolve_camera_arm(&terrain, pivot(), Vec3::ZERO, 4.0, 0.3);
        assert!((arm - 4.0).abs() < 0.001, "zero dir should return desired unchanged");
    }
}
