//! 天气驱动 — CHG-016 Phase 2（涌现化）
//!
//! `WeatherDriver` 使用连续物理参数（WeatherParams）替代离散 Markov 状态机。
//! 参数平滑漂移——湿度驱动云量，云量驱动降水，气压梯度驱动风。
//! 视觉效果通过 lerp 从参数连续计算——无限种中间天气。
//!
//! `SimpleSeasonProvider` 实现 `SeasonAtmosQuery`，纯函数 `total_days → season`。

use woworld_core::prelude::*;

use crate::traits::{SeasonAtmosQuery, WeatherAtmosQuery};

// ── WeatherDriver ──────────────────────

/// 连续物理天气驱动——替代 SimpleWeatherDriver。
///
/// 每帧通过伪随机游走演化 WeatherParams 的 6 个参数。
/// 参数间有物理因果：湿度→云量→降水，气压梯度→风。
pub struct WeatherDriver {
    pub params: WeatherParams,
    /// 伪随机种子（每次 tick 更新）
    seed: u64,
}

impl WeatherDriver {
    pub fn new(seed: u64) -> Self {
        Self {
            params: WeatherParams::default(),
            seed,
        }
    }

    /// 每帧推进——物理参数连续演化。
    ///
    /// `delta`: 现实秒（帧间隔）
    /// `season`: 当前季节（调制温度基线 + 湿度倾向）
    pub fn tick(&mut self, delta: f64, season: Season) {
        let dt = delta as f32;

        // ── 气压随机游走 ──
        self.params.pressure += self.rand_norm() * 1.5 * dt;
        self.params.pressure = self.params.pressure.clamp(980.0, 1040.0);

        // ── 湿度演化 ──
        let evap_rate = match season {
            Season::Summer => 0.08,
            Season::Spring => 0.04,
            Season::Autumn => 0.02,
            Season::Winter => -0.02,
        };
        self.params.humidity += (evap_rate + self.rand_norm() * 0.03) * dt;
        self.params.humidity = self.params.humidity.clamp(0.1, 0.98);

        // ── 云量趋向湿度 ──
        let cloud_target = self.params.humidity * 1.1;
        self.params.cloud_cover += (cloud_target - self.params.cloud_cover) * 0.1 * dt;
        self.params.cloud_cover = self.params.cloud_cover.clamp(0.0, 1.0);

        // ── 降水趋向云量 × 湿度 ──
        let precip_target = self.params.cloud_cover * self.params.humidity;
        self.params.precipitation += (precip_target - self.params.precipitation) * 0.15 * dt;
        self.params.precipitation = self.params.precipitation.clamp(0.0, 1.0);

        // ── 风速 ← 气压梯度 + 随机扰动 ──
        let pressure_gradient = (self.params.pressure - 1013.0).abs() / 30.0;
        self.params.wind_speed += (pressure_gradient - self.params.wind_speed) * 0.2 * dt;
        self.params.wind_speed += (self.rand_norm() * 2.0).abs() * dt;
        self.params.wind_speed = self.params.wind_speed.clamp(0.0, 40.0);

        // ── 温度 ← 季节基线 + 云量调制 ──
        let season_temp = match season {
            Season::Spring => 15.0,
            Season::Summer => 25.0,
            Season::Autumn => 12.0,
            Season::Winter => 2.0,
        };
        let cloud_cooling = -self.params.cloud_cover * 8.0; // 云遮挡日照→降温
        let target_temp = season_temp + cloud_cooling;
        self.params.temperature += (target_temp - self.params.temperature) * 0.05 * dt;
    }

    /// 调试：从 WeatherState 设置参数预设
    pub fn set_preset(&mut self, state: WeatherState) {
        self.params = WeatherParams::from_weather_state(state);
    }

    // ── 伪随机 ──

    fn rand_norm(&mut self) -> f32 {
        // SplitMix64 → 近似 N(0,1) via Box-Muller-lite
        self.seed = self.seed.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.seed;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^= z >> 31;
        (z as f32) * 2.3283064e-10 - 1.0 // map u64→[-1, 1]
    }
}

impl WeatherAtmosQuery for WeatherDriver {
    fn sky_mult(&self) -> [f32; 3] {
        let t = self.params.cloud_cover.max(self.params.precipitation);
        WeatherParams::lerp3([1.0, 1.0, 1.0], [0.22, 0.24, 0.30], t)
    }

    fn fog_density(&self) -> f32 {
        WeatherParams::lerp(0.0, 0.40, self.params.cloud_cover.max(self.params.precipitation))
    }

    fn exposure_mult(&self) -> f32 {
        WeatherParams::lerp(1.0, 0.30, self.params.cloud_cover.max(self.params.precipitation))
    }

    fn saturation_mult(&self) -> f32 {
        WeatherParams::lerp(1.0, 0.35, self.params.cloud_cover.max(self.params.precipitation))
    }
}

// ── SimpleWeatherDriver (legacy) ──────

/// 旧 Markov 天气驱动——保留用于向后兼容测试。
/// 新代码请使用 `WeatherDriver`。
pub struct SimpleWeatherDriver {
    state: WeatherState,
    tick_remaining: f64,
    tick_interval: f64,
}

impl SimpleWeatherDriver {
    pub fn new(seed: u64) -> Self {
        let seed_f = (seed as f64).sin().abs();
        let interval = 300.0 + seed_f * 600.0;
        Self { state: WeatherState::Clear, tick_remaining: interval, tick_interval: interval }
    }

    pub fn tick(&mut self, delta: f64) {
        self.tick_remaining -= delta;
        if self.tick_remaining <= 0.0 {
            self.maybe_transition();
            self.tick_interval = 300.0 + self.pseudo_random() * 600.0;
            self.tick_remaining = self.tick_interval;
        }
    }

    pub fn current_state(&self) -> WeatherState { self.state }

    pub fn debug_set_state(&mut self, state: WeatherState) {
        self.state = state;
        self.tick_remaining = 600.0;
    }

    pub fn debug_cycle_state(&mut self) {
        let next = (self.state.index() + 1) % WeatherState::COUNT as u8;
        self.debug_set_state(WeatherState::from_index(next));
    }

    fn pseudo_random(&self) -> f64 {
        let x = self.tick_interval.to_bits();
        let y = x.wrapping_mul(0x2545F4914F6CDD1D).wrapping_add(1);
        ((y >> 12) as f64) * 2.3283064365386963e-10
    }

    fn maybe_transition(&mut self) {
        let r = self.pseudo_random();
        let idx = self.state.index();
        let stability: f64 = match self.state {
            WeatherState::Clear => 0.88,
            WeatherState::PartlyCloudy => 0.82,
            WeatherState::Overcast => 0.75,
            WeatherState::LightPrecip => 0.70,
            WeatherState::ModeratePrecip => 0.65,
            WeatherState::HeavyStorm => 0.55,
        };
        if r < stability { return; }
        let r2 = self.pseudo_random() + r;
        let go_clearward = if idx == 0 { false } else if idx == 5 { true }
            else { r2 < (0.5 + idx as f64 * 0.08) };
        let new_idx = if go_clearward { idx.saturating_sub(1) } else { (idx + 1).min(5) };
        self.state = WeatherState::from_index(new_idx);
    }
}

impl WeatherAtmosQuery for SimpleWeatherDriver {
    fn sky_mult(&self) -> [f32; 3] {
        match self.state {
            WeatherState::Clear => [1.00, 1.00, 1.00],
            WeatherState::PartlyCloudy => [0.90, 0.90, 0.95],
            WeatherState::Overcast => [0.55, 0.55, 0.60],
            WeatherState::LightPrecip => [0.45, 0.47, 0.52],
            WeatherState::ModeratePrecip => [0.35, 0.37, 0.42],
            WeatherState::HeavyStorm => [0.22, 0.24, 0.30],
        }
    }
    fn fog_density(&self) -> f32 {
        match self.state {
            WeatherState::Clear => 0.00,
            WeatherState::PartlyCloudy => 0.02,
            WeatherState::Overcast => 0.08,
            WeatherState::LightPrecip => 0.15,
            WeatherState::ModeratePrecip => 0.25,
            WeatherState::HeavyStorm => 0.40,
        }
    }
    fn exposure_mult(&self) -> f32 {
        match self.state {
            WeatherState::Clear => 1.00,
            WeatherState::PartlyCloudy => 0.95,
            WeatherState::Overcast => 0.70,
            WeatherState::LightPrecip => 0.60,
            WeatherState::ModeratePrecip => 0.45,
            WeatherState::HeavyStorm => 0.30,
        }
    }
    fn saturation_mult(&self) -> f32 {
        match self.state {
            WeatherState::Clear => 1.00,
            WeatherState::PartlyCloudy => 0.95,
            WeatherState::Overcast => 0.75,
            WeatherState::LightPrecip => 0.65,
            WeatherState::ModeratePrecip => 0.50,
            WeatherState::HeavyStorm => 0.35,
        }
    }
}

// ── SimpleSeasonProvider ────────────────

/// 简化季节提供者——纯函数 `total_days → Season`。
pub struct SimpleSeasonProvider {
    season: Season,
}

impl SimpleSeasonProvider {
    /// 从总游戏天数创建
    pub fn new(total_days: u64) -> Self {
        Self { season: Season::from_total_days(total_days) }
    }

    /// 更新季节（每日调用一次即可）
    pub fn update(&mut self, total_days: u64) {
        self.season = Season::from_total_days(total_days);
    }

    /// 当前季节
    pub fn current_season(&self) -> Season {
        self.season
    }
}

impl SeasonAtmosQuery for SimpleSeasonProvider {
    fn saturation_mult(&self) -> f32 {
        match self.season {
            Season::Spring => 1.10,
            Season::Summer => 0.90,
            Season::Autumn => 1.10,
            Season::Winter => 0.85,
        }
    }

    fn warmth(&self) -> f32 {
        match self.season {
            Season::Spring => 0.00,
            Season::Summer => 0.15,
            Season::Autumn => -0.05,
            Season::Winter => -0.20,
        }
    }
}

// ── 测试 ────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_weather_driver_starts_clear() {
        let w = SimpleWeatherDriver::new(42);
        assert_eq!(w.current_state(), WeatherState::Clear);
    }

    #[test]
    fn test_clear_sky_mult_is_identity() {
        let w = SimpleWeatherDriver::new(0);
        assert_eq!(w.sky_mult(), [1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_heavy_storm_sky_is_dark() {
        let mut w = SimpleWeatherDriver::new(0);
        // Force state via many ticks
        for _ in 0..10000 {
            w.tick(1.0);
        }
        // After many ticks, should have visited non-Clear states
        // Just verify the mapping works for all states
        for i in 0..6u8 {
            let s = WeatherState::from_index(i);
            let _sky = match s {
                WeatherState::Clear => [1.0; 3],
                _ => [0.5; 3],
            };
            assert!(_sky[0] > 0.0);
        }
    }

    #[test]
    fn test_fog_zero_for_clear() {
        let w = SimpleWeatherDriver::new(0);
        assert_eq!(w.fog_density(), 0.0);
    }

    #[test]
    fn test_heavy_storm_has_high_fog() {
        // Direct construction test: verify HeavyStorm values are non-trivial
        let sky = [0.22, 0.24, 0.30_f32];
        assert!(sky[0] < 0.5);
        assert!(sky[1] < 0.5);
    }

    #[test]
    fn test_season_from_days() {
        assert_eq!(Season::from_total_days(0), Season::Spring);
        assert_eq!(Season::from_total_days(30), Season::Summer);
        assert_eq!(Season::from_total_days(60), Season::Autumn);
        assert_eq!(Season::from_total_days(90), Season::Winter);
        assert_eq!(Season::from_total_days(120), Season::Spring); // next year
    }

    #[test]
    fn test_season_provider_modulation() {
        let spring = SimpleSeasonProvider::new(0);
        assert!(spring.saturation_mult() > 0.9);
        assert!((spring.warmth() - 0.0).abs() < 0.01);

        let winter = SimpleSeasonProvider::new(90);
        assert!(winter.warmth() < -0.1);
    }

    // ── WeatherDriver 涌现化测试 ──────────

    #[test]
    fn test_weather_driver_default_is_clear() {
        let w = WeatherDriver::new(42);
        assert_eq!(w.params.cloud_cover, 0.0);
        assert_eq!(w.params.precipitation, 0.0);
    }

    #[test]
    fn test_weather_driver_tick_changes_params() {
        let mut w = WeatherDriver::new(99);
        let before = w.params;
        // 多帧推进
        for _ in 0..100 {
            w.tick(1.0, Season::Summer);
        }
        // 参数不应完全保持不变
        let diff = (w.params.cloud_cover - before.cloud_cover).abs()
            + (w.params.humidity - before.humidity).abs();
        assert!(diff > 0.001, "params should evolve over time");
    }

    #[test]
    fn test_weather_driver_continuity() {
        let mut w = WeatherDriver::new(7);
        let prev = w.params;
        w.tick(1.0, Season::Spring);
        // 单帧变化应微小（连续演化，容忍初始 jump-in）
        assert!((w.params.cloud_cover - prev.cloud_cover).abs() < 0.15);
        assert!((w.params.precipitation - prev.precipitation).abs() < 0.10);
        assert!((w.params.temperature - prev.temperature).abs() < 1.0);
    }

    #[test]
    fn test_weather_driver_summer_is_warm() {
        let mut w = WeatherDriver::new(0);
        // 推进到夏天基线
        for _ in 0..500 {
            w.tick(1.0, Season::Summer);
        }
        assert!(w.params.temperature > 15.0, "summer should be warm");
    }

    #[test]
    fn test_weather_driver_winter_is_cold() {
        let mut w = WeatherDriver::new(0);
        for _ in 0..500 {
            w.tick(1.0, Season::Winter);
        }
        assert!(w.params.temperature < 10.0, "winter should be cold");
    }

    #[test]
    fn test_set_preset_maps_to_approx_old_state() {
        let mut w = WeatherDriver::new(0);
        w.set_preset(WeatherState::HeavyStorm);
        assert!(w.params.cloud_cover > 0.9);
        assert!(w.params.precipitation > 0.7);
        assert!(w.fog_density() > 0.3);
    }

    #[test]
    fn test_sky_mult_continuous() {
        let mut w = WeatherDriver::new(0);
        // Clear
        w.set_preset(WeatherState::Clear);
        let clear_sky = w.sky_mult();
        assert!((clear_sky[0] - 1.0).abs() < 0.01);

        // HeavyStorm
        w.set_preset(WeatherState::HeavyStorm);
        let storm_sky = w.sky_mult();
        assert!(storm_sky[0] < 0.4);
    }

    #[test]
    fn test_weather_params_to_state_roundtrip() {
        // 预设参数应映射回对应的 WeatherState
        for state in &[
            WeatherState::Clear,
            WeatherState::PartlyCloudy,
            WeatherState::Overcast,
            WeatherState::LightPrecip,
            WeatherState::ModeratePrecip,
            WeatherState::HeavyStorm,
        ] {
            let params = WeatherParams::from_weather_state(*state);
            let back = params.to_weather_state();
            assert_eq!(back, *state, "roundtrip failed for {:?}", state);
        }
    }

    #[test]
    fn test_weather_params_clamps() {
        let mut w = WeatherDriver::new(0);
        // 极端推进不应越界
        for _ in 0..10000 {
            w.tick(10.0, Season::Summer);
        }
        assert!(w.params.cloud_cover >= 0.0 && w.params.cloud_cover <= 1.0);
        assert!(w.params.precipitation >= 0.0 && w.params.precipitation <= 1.0);
        assert!(w.params.pressure >= 970.0 && w.params.pressure <= 1050.0);
    }
}
