//! MovementProfileRegistry — 移动参数的 HashMap 存储
//!
//! TOML 数据驱动: `movement_profiles.toml` → 每个物种一个 MovementProfile。
//! MovementSystem 通过 `default_profile()` 或 `get(name)` 查询。
//!
//! 参见: `WoWorld-Design/.../角色控制器/002-MovementState与连续移动.md` §五

use std::collections::HashMap;
use woworld_core::movement::MovementProfile;

/// 移动参数注册表——TOML 加载的 MovementProfile 集合。
#[derive(Debug, Default)]
pub struct MovementProfileRegistry {
    /// profile 名 → MovementProfile
    profiles: HashMap<String, MovementProfile>,
}

impl MovementProfileRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 按名称查询 profile。
    pub fn get(&self, name: &str) -> Option<&MovementProfile> {
        self.profiles.get(name)
    }

    /// 默认 profile（humanoid）。若未加载则返回 Default。
    pub fn default_profile(&self) -> &MovementProfile {
        self.profiles.get("humanoid").unwrap_or(&DEFAULT_PROFILE)
    }

    /// 从 TOML 字符串加载 profile 集合。
    ///
    /// TOML 格式:
    /// ```toml
    /// [profile.humanoid]
    /// walk_speed = 1.4
    /// ...
    /// ```
    pub fn load_from_toml(&mut self, toml_str: &str) -> Result<(), toml::de::Error> {
        #[derive(serde::Deserialize)]
        struct ProfileRegistryToml {
            profile: HashMap<String, MovementProfile>,
        }

        let parsed: ProfileRegistryToml = toml::from_str(toml_str)?;
        for (key, profile) in parsed.profile {
            self.profiles.insert(key, profile);
        }
        Ok(())
    }
}

/// 兜底——编译期默认 humanoid profile（与 MovementProfile::default() 一致）。
static DEFAULT_PROFILE: MovementProfile = MovementProfile {
    walk_speed: 1.4,
    run_speed: 3.5,
    sprint_speed: 5.5,
    ground_accel: 10.0,
    sprint_accel: 14.0,
    ground_friction: 12.0,
    sprint_friction: 8.0,
    default_turn_rate: 720.0,
    sprint_turn_rate: 360.0,
    air_turn_rate: 180.0,
    knocked_turn_rate: 30.0,
    sprint_stamina_rate: 8.0,
    sprint_min_stamina_to_start: 8.0,
    climb_speed: 0.6,
    climb_accel: 3.0,
    climb_friction: 8.0,
    climb_stamina_rate: 6.0,
    swim_slow_speed: 1.0,
    swim_fast_speed: 2.5,
    dive_speed: 1.5,
    swim_accel_slow: 3.0,
    swim_accel_fast: 5.0,
    swim_friction: 6.0,
    swim_fast_stamina_rate: 10.0,
    swim_slow_stamina_rate: -2.0,
    treading_stamina_rate: 2.0,
    glide_horizontal_speed: 12.0,
    glide_vertical_speed: -1.5,
    glide_accel: 4.0,
    glide_stamina_rate: 3.0,
    jump_horizontal_speed: 3.0,
    gravity: 20.0,
    jump_speed: 7.0,
    mounted_speed: 7.0,
    mounted_accel: 5.0,
    mounted_friction: 4.0,
    knockback_recover_secs: 0.4,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_new_empty_falls_back_to_default() {
        let r = MovementProfileRegistry::new();
        let p = r.default_profile();
        assert!(p.walk_speed > 0.0);
        assert!(p.sprint_speed > p.run_speed);
    }

    #[test]
    fn test_registry_get_missing_returns_none() {
        let r = MovementProfileRegistry::new();
        assert!(r.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_default_profile_is_humanoid() {
        let mut r = MovementProfileRegistry::new();
        r.profiles.insert(
            "humanoid".into(),
            MovementProfile {
                walk_speed: 99.0,
                ..MovementProfile::default()
            },
        );
        let p = r.default_profile();
        assert!((p.walk_speed - 99.0).abs() < 0.01);
    }

    #[test]
    fn test_static_default_has_sensible_values() {
        let p = &DEFAULT_PROFILE;
        assert!(p.walk_speed > 0.0);
        assert!(p.run_speed > p.walk_speed);
        assert!(p.sprint_speed > p.run_speed);
        assert!(p.swim_slow_stamina_rate < 0.0); // 回复体力
    }

    #[test]
    fn test_load_real_movement_profiles_toml_parses() {
        // ★ 集成前置：验证真实 assets/movement_profiles.toml 能被解析。
        let mut r = MovementProfileRegistry::new();
        let toml = include_str!("../../../../assets/movement_profiles.toml");
        r.load_from_toml(toml)
            .expect("movement_profiles.toml 应能解析");
        assert!(r.get("humanoid").is_some(), "humanoid profile 应存在");
        assert!(r.get("wolf").is_some(), "wolf profile 应存在");
    }
}
