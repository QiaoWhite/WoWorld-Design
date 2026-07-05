//! 世界时间类型 — 昼夜循环的权威定义
//!
//! `WorldTime`（不可变快照）+ `WorldClock`（运行时推进）均在 core 中定义。
//! 所有消费 crate 平等引用。不依赖 Godot——引擎无关。
//!
//! 参见: `WoWorld-Design/开发路线图/002-轨A-正式开发.md` A.5

/// 昼夜四阶段
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TimeOfDay {
    /// 黎明 (day_progress ∈ [0.20, 0.30))
    Dawn,
    /// 白天 (day_progress ∈ [0.30, 0.70))
    Day,
    /// 黄昏 (day_progress ∈ [0.70, 0.80))
    Dusk,
    /// 夜晚 (day_progress ∈ [0.80, 1.0) ∪ [0.0, 0.20))
    Night,
}

impl TimeOfDay {
    /// 从 day_progress (0.0-1.0) 推导阶段
    pub fn from_progress(p: f64) -> Self {
        let p = p.rem_euclid(1.0);
        if p < 0.20 {
            TimeOfDay::Night
        } else if p < 0.30 {
            TimeOfDay::Dawn
        } else if p < 0.70 {
            TimeOfDay::Day
        } else if p < 0.80 {
            TimeOfDay::Dusk
        } else {
            TimeOfDay::Night
        }
    }
}

/// 世界时间不可变快照
///
/// 由 `WorldClock::advance()` 每帧重新生成。
/// 消费方按值复制（Copy），零分配查询。
#[derive(Copy, Clone, Debug)]
pub struct WorldTime {
    /// 日内进度 0.0-1.0（0.0 = 午夜, 0.25 = 日出, 0.5 = 正午, 0.75 = 日落）
    pub day_progress: f64,
    /// 自世界起始以来的天数
    pub day_number: u64,
    /// 年内进度 0.0-1.0（0.0 = 春分, 0.25 = 夏至, 0.5 = 秋分, 0.75 = 冬至）
    pub season_progress: f64,
    /// 当前昼夜阶段
    pub phase: TimeOfDay,
    /// 全局光照等级 0.0-1.0（0.03 = 夜间环境光, 1.0 = 正午全亮）
    pub light_level: f32,
    /// 太阳高度角（弧度, 0.0 = 地平线, π/2 = 天顶）
    pub sun_elevation: f64,
    /// 太阳方位角（弧度, 0.0 = 北, π/2 = 东, π = 南, 3π/2 = 西）
    pub sun_azimuth: f64,
}

impl WorldTime {
    /// 从日内进度推导完整 WorldTime
    ///
    /// `day_progress`: 0.0-1.0 归一化日内时间
    /// `day_number`: 累计天数
    /// `days_per_year`: 一年多少天（默认 120，可调）
    pub fn from_progress(day_progress: f64, day_number: u64, days_per_year: u64) -> Self {
        let dp = day_progress.rem_euclid(1.0);
        let phase = TimeOfDay::from_progress(dp);

        // 太阳轨道: dp=0.25 日出(0°), dp=0.5 正午(90°), dp=0.75 日落(0°), dp=1.0 午夜(-90°)
        // elevation = sin(angle), 映射到 [-π/2, π/2]
        let angle = (dp - 0.25) * std::f64::consts::TAU;
        let sun_elevation = angle.sin().asin();

        // 方位角 日出东(π/2) → 正午南(π) → 日落西(3π/2)
        // 线性：π/2 + angle，归一化到 [0, 2π)
        let sun_azimuth = (std::f64::consts::FRAC_PI_2 + angle).rem_euclid(std::f64::consts::TAU);

        // light_level: smoothstep(0°, 15°, sun_elevation) 缩放 + 夜景基底 0.03
        let light_level = {
            let raw = smoothstep(0.0, 0.2618, sun_elevation); // 0°→15° 过渡带
            (raw * 0.97 + 0.03) as f32
        };

        let season_progress = (day_number % days_per_year) as f64 / days_per_year as f64;

        Self {
            day_progress: dp,
            day_number,
            season_progress,
            phase,
            light_level,
            sun_elevation,
            sun_azimuth,
        }
    }
}

/// Hermite smoothstep: 边缘平滑插值
fn smoothstep(edge0: f64, edge1: f64, x: f64) -> f64 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// ── WorldClock — 运行时时钟 ──────────────

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

    /// 当前日内进度 0.0-1.0（0=午夜, 0.25=日出, 0.5=正午, 0.75=日落）
    pub fn day_progress(&self) -> f32 {
        (self.accumulator / self.seconds_per_day).fract() as f32
    }

    /// 快进到特定日内时间（测试用）
    pub fn set_time(&mut self, day_progress: f64) {
        self.accumulator = day_progress * self.seconds_per_day;
        self.current =
            WorldTime::from_progress(day_progress, self.current.day_number, self.days_per_year);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_of_day_from_progress() {
        assert_eq!(TimeOfDay::from_progress(0.0), TimeOfDay::Night);
        assert_eq!(TimeOfDay::from_progress(0.1), TimeOfDay::Night);
        assert_eq!(TimeOfDay::from_progress(0.25), TimeOfDay::Dawn);
        assert_eq!(TimeOfDay::from_progress(0.5), TimeOfDay::Day);
        assert_eq!(TimeOfDay::from_progress(0.75), TimeOfDay::Dusk);
        assert_eq!(TimeOfDay::from_progress(0.9), TimeOfDay::Night);
    }

    #[test]
    fn test_midnight_dark() {
        let wt = WorldTime::from_progress(0.0, 0, 120);
        assert!(
            wt.light_level < 0.1,
            "midnight should be dark, got {}",
            wt.light_level
        );
        assert!(
            wt.sun_elevation < 0.0,
            "sun should be below horizon at midnight"
        );
        assert_eq!(wt.phase, TimeOfDay::Night);
    }

    #[test]
    fn test_noon_bright() {
        let wt = WorldTime::from_progress(0.5, 0, 120);
        assert!(
            wt.light_level > 0.9,
            "noon should be bright, got {}",
            wt.light_level
        );
        assert!(
            wt.sun_elevation > 1.0,
            "sun should be high at noon, got {}",
            wt.sun_elevation
        );
        assert_eq!(wt.phase, TimeOfDay::Day);
    }

    #[test]
    fn test_day_number_preserved() {
        let wt = WorldTime::from_progress(0.3, 42, 120);
        assert_eq!(wt.day_number, 42);
    }

    #[test]
    fn test_progress_wraps() {
        // 1.25 wraps to 0.25 (dawn)
        let wt = WorldTime::from_progress(1.25, 0, 120);
        assert!((wt.day_progress - 0.25).abs() < 0.001);
        assert_eq!(wt.phase, TimeOfDay::Dawn);
    }

    #[test]
    fn test_season_progress() {
        // Day 0 = spring start (0.0), day 30 = summer (0.25), day 60 = autumn (0.5), day 90 = winter (0.75)
        let wt_spring = WorldTime::from_progress(0.5, 0, 120);
        let wt_summer = WorldTime::from_progress(0.5, 30, 120);
        let wt_autumn = WorldTime::from_progress(0.5, 60, 120);
        let wt_winter = WorldTime::from_progress(0.5, 90, 120);

        assert!(wt_spring.season_progress < 0.01);
        assert!((wt_summer.season_progress - 0.25).abs() < 0.01);
        assert!((wt_autumn.season_progress - 0.5).abs() < 0.01);
        assert!((wt_winter.season_progress - 0.75).abs() < 0.01);
    }

    // ── WorldClock 测试 ─────────────────

    #[test]
    fn test_clock_advance_zero_delta() {
        let mut clock = WorldClock::new(60.0);
        let old_dp = clock.current.day_progress;
        clock.advance(0.0);
        assert!((clock.current.day_progress - old_dp).abs() < 0.001);
    }

    #[test]
    fn test_clock_advance_half_day() {
        let mut clock = WorldClock::new(60.0);
        clock.set_time(0.25); // dawn
        let crossed = clock.advance(30.0); // half a game day
        assert!(!crossed);
        assert!(
            (clock.current.day_progress - 0.75).abs() < 0.01,
            "should be dusk, got {}",
            clock.current.day_progress
        );
    }

    #[test]
    fn test_clock_advance_crosses_midnight() {
        let mut clock = WorldClock::new(60.0);
        clock.set_time(0.9); // near midnight
        let crossed = clock.advance(10.0); // 1/6 day, crosses midnight
        assert!(crossed, "should cross day boundary");
        assert_eq!(clock.current.day_number, 1);
        assert!(
            clock.current.day_progress < 0.1,
            "should be just past midnight, got {}",
            clock.current.day_progress
        );
    }

    #[test]
    fn test_clock_advance_multiple_days() {
        let mut clock = WorldClock::new(60.0);
        clock.set_time(0.5);
        let crossed = clock.advance(180.0); // 3 full days
        assert!(crossed);
        assert_eq!(clock.current.day_number, 3);
        assert!((clock.current.day_progress - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_clock_set_time() {
        let mut clock = WorldClock::new(60.0);
        clock.set_time(0.5);
        assert_eq!(clock.current.phase, TimeOfDay::Day);
        assert!(clock.current.light_level > 0.9);

        clock.set_time(0.0);
        assert_eq!(clock.current.phase, TimeOfDay::Night);
        assert!(clock.current.light_level < 0.1);
    }

    #[test]
    fn test_clock_seconds_per_day_parameter() {
        // 120s/day: 30s = 1/4 day
        let mut clock = WorldClock::new(120.0);
        clock.set_time(0.25);
        clock.advance(30.0);
        assert!((clock.current.day_progress - 0.5).abs() < 0.01);
    }
}
