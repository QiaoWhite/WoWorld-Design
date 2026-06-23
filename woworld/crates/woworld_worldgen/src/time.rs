//! 世界时钟 — 运行时驱动昼夜循环
//!
//! `WorldClock` 持有可变时间状态，每帧 `advance(delta)` 推进。
//! 消费方通过 `current` 字段读取不可变快照。
//!
//! 参见: `WoWorld-Design/开发路线图/002-轨A-正式开发.md` A.5

use woworld_core::time::WorldTime;

/// 运行时世界时钟
///
/// Godot 侧每帧调用 `advance(delta)`，然后消费 `current` 快照。
#[derive(Clone, Debug)]
pub struct WorldClock {
    /// 当前时间快照（每帧重算）
    pub current: WorldTime,
    /// 现实秒 / 游戏天（默认 3600 = 60 分钟/天，可调 15-120 [TUNING]）
    pub seconds_per_day: f64,
    /// 太阳轨道半径（米，Godot 渲染用）
    pub sun_orbit_radius: f32,
    /// 一年天数（默认 120，可调 [TUNING]）
    pub days_per_year: u64,

    /// 累计现实时间（秒）
    accumulator: f64,
}

impl Default for WorldClock {
    fn default() -> Self {
        Self::new(60.0)
    }
}

impl WorldClock {
    /// 创建新时钟
    ///
    /// `seconds_per_day`: 现实秒/游戏天（默认 60.0 = 60 秒/天用于测试，正式用 3600.0）
    pub fn new(seconds_per_day: f64) -> Self {
        let initial_progress = 0.25; // 日出开局
        let current = WorldTime::from_progress(initial_progress, 0, 120);
        Self {
            current,
            seconds_per_day,
            sun_orbit_radius: 500.0,
            days_per_year: 120,
            accumulator: initial_progress * seconds_per_day,
        }
    }

    /// 每帧推进 delta 现实秒。返回 `true` 表示跨越了日期边界。
    pub fn advance(&mut self, delta_real_seconds: f64) -> bool {
        self.accumulator += delta_real_seconds;
        let day_progress = self.accumulator / self.seconds_per_day;

        let whole_days = day_progress.floor() as u64;
        let fractional = day_progress - whole_days as f64;

        let old_day_number = self.current.day_number;
        let new_day_number = old_day_number + whole_days;
        self.accumulator -= whole_days as f64 * self.seconds_per_day;

        self.current = WorldTime::from_progress(fractional, new_day_number, self.days_per_year);

        new_day_number > old_day_number
    }

    /// 快进到特定日内时间（测试用）
    pub fn set_time(&mut self, day_progress: f64) {
        self.accumulator = day_progress * self.seconds_per_day;
        self.current = WorldTime::from_progress(day_progress, self.current.day_number, self.days_per_year);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_advance_zero_delta() {
        let mut clock = WorldClock::new(60.0);
        let old_dp = clock.current.day_progress;
        clock.advance(0.0);
        assert!((clock.current.day_progress - old_dp).abs() < 0.001);
    }

    #[test]
    fn test_advance_half_day() {
        let mut clock = WorldClock::new(60.0);
        clock.set_time(0.25); // dawn
        let crossed = clock.advance(30.0); // half a game day
        assert!(!crossed);
        assert!((clock.current.day_progress - 0.75).abs() < 0.01, "should be dusk, got {}", clock.current.day_progress);
    }

    #[test]
    fn test_advance_crosses_midnight() {
        let mut clock = WorldClock::new(60.0);
        clock.set_time(0.9); // near midnight
        let crossed = clock.advance(10.0); // 1/6 day, crosses midnight
        assert!(crossed, "should cross day boundary");
        assert_eq!(clock.current.day_number, 1);
        assert!(clock.current.day_progress < 0.1, "should be just past midnight, got {}", clock.current.day_progress);
    }

    #[test]
    fn test_advance_multiple_days() {
        let mut clock = WorldClock::new(60.0);
        clock.set_time(0.5);
        let crossed = clock.advance(180.0); // 3 full days
        assert!(crossed);
        assert_eq!(clock.current.day_number, 3);
        assert!((clock.current.day_progress - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_set_time() {
        let mut clock = WorldClock::new(60.0);
        clock.set_time(0.5);
        assert_eq!(clock.current.phase, woworld_core::time::TimeOfDay::Day);
        assert!(clock.current.light_level > 0.9);

        clock.set_time(0.0);
        assert_eq!(clock.current.phase, woworld_core::time::TimeOfDay::Night);
        assert!(clock.current.light_level < 0.1);
    }

    #[test]
    fn test_seconds_per_day_parameter() {
        // 120s/day: 30s = 1/4 day
        let mut clock = WorldClock::new(120.0);
        clock.set_time(0.25);
        clock.advance(30.0);
        assert!((clock.current.day_progress - 0.5).abs() < 0.01);
    }
}
