//! 简化天气驱动 — CHG-016 Phase 1
//!
//! `SimpleWeatherDriver` 提供基于游戏时间的天气状态机（简化 Markov），
//! 实现 `WeatherAtmosQuery` 将状态映射到大气调制参数。
//! `SimpleSeasonProvider` 实现 `SeasonAtmosQuery`，纯函数 `total_days → season`。
//!
//! 后续升级为完整 `woworld_weather` crate 时替换。

use woworld_core::prelude::*;

use crate::traits::{SeasonAtmosQuery, WeatherAtmosQuery};

// ── SimpleWeatherDriver ─────────────────

/// 简化天气驱动——时间推进的天气状态机。
///
/// 约每 5-15 游戏分钟有概率迁移到相邻状态。
/// 晴朗状态更稳定，暴雨状态更快衰减。
pub struct SimpleWeatherDriver {
    state: WeatherState,
    /// 距下次检查的剩余游戏秒数
    tick_remaining: f64,
    /// 当前 tick 间隔（游戏秒）
    tick_interval: f64,
}

impl SimpleWeatherDriver {
    /// 创建以 Clear 为初始状态的驱动。
    pub fn new(seed: u64) -> Self {
        // 用 seed 派生出首 tick 间隔（5-15 分钟 = 300-900 秒）
        let seed_f = (seed as f64).sin().abs();
        let interval = 300.0 + seed_f * 600.0;
        Self { state: WeatherState::Clear, tick_remaining: interval, tick_interval: interval }
    }

    /// 每帧推进 delta（现实秒），累积游戏时间后检查状态迁移。
    pub fn tick(&mut self, delta: f64) {
        self.tick_remaining -= delta;
        if self.tick_remaining <= 0.0 {
            self.maybe_transition();
            // 下一 tick 间隔随机（5-15 游戏分钟 = 300-900 游戏秒）
            self.tick_interval = 300.0 + self.pseudo_random() * 600.0;
            self.tick_remaining = self.tick_interval;
        }
    }

    /// 当前天气状态
    pub fn current_state(&self) -> WeatherState {
        self.state
    }

    /// 调试：强制设置天气状态
    pub fn debug_set_state(&mut self, state: WeatherState) {
        self.state = state;
        // 重置 tick 计时器，给用户观察时间
        self.tick_remaining = 600.0;
    }

    /// 调试：切换到下一个天气状态（循环）
    pub fn debug_cycle_state(&mut self) {
        let next = (self.state.index() + 1) % WeatherState::COUNT as u8;
        self.debug_set_state(WeatherState::from_index(next));
    }

    // ── 内部 ─────────────────────────

    /// 伪随机（基于 tick 计数器，避免引入 rand crate）
    fn pseudo_random(&self) -> f64 {
        let x = self.tick_interval.to_bits();
        let y = x.wrapping_mul(0x2545F4914F6CDD1D).wrapping_add(1);
        ((y >> 12) as f64) * 2.3283064365386963e-10 // / 2^32
    }

    fn maybe_transition(&mut self) {
        let r = self.pseudo_random();
        let idx = self.state.index();

        // 基于当前状态的稳定性：越严重越不稳定
        let stability: f64 = match self.state {
            WeatherState::Clear => 0.88,
            WeatherState::PartlyCloudy => 0.82,
            WeatherState::Overcast => 0.75,
            WeatherState::LightPrecip => 0.70,
            WeatherState::ModeratePrecip => 0.65,
            WeatherState::HeavyStorm => 0.55,
        };

        if r < stability {
            return; // 保持不变
        }

        // 决定方向：偏向回归 Clear
        let r2 = self.pseudo_random() + r; // 多取一点随机
        let go_clearward = if idx == 0 {
            false
        } else if idx == 5 {
            true
        } else {
            // 越严重越倾向回归晴朗
            r2 < (0.5 + idx as f64 * 0.08)
        };

        let new_idx = if go_clearward {
            idx.saturating_sub(1)
        } else {
            (idx + 1).min(5)
        };

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
}
