//! 世界时间纯数据类型
//!
//! 昼夜循环 + 季节时钟的基础类型。所有消费 crate 平等引用。
//! `WorldClock`（运行态）在 woworld_worldgen 中，此处仅定义数据快照。
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
        // elevation = sin((dp - 0.25) * 2π), 映射到 [-π/2, π/2]
        let sun_elevation = ((dp - 0.25) * std::f64::consts::TAU).sin().asin();

        // 方位角 = -cos((dp - 0.25) * 2π) * π, 日出东(π/2)→正午南(π)→日落西(3π/2)
        let sun_azimuth = std::f64::consts::PI * (1.0 - ((dp - 0.25) * std::f64::consts::TAU).cos());

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
        assert!(wt.light_level < 0.1, "midnight should be dark, got {}", wt.light_level);
        assert!(wt.sun_elevation < 0.0, "sun should be below horizon at midnight");
        assert_eq!(wt.phase, TimeOfDay::Night);
    }

    #[test]
    fn test_noon_bright() {
        let wt = WorldTime::from_progress(0.5, 0, 120);
        assert!(wt.light_level > 0.9, "noon should be bright, got {}", wt.light_level);
        assert!(wt.sun_elevation > 1.0, "sun should be high at noon, got {}", wt.sun_elevation);
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
}
