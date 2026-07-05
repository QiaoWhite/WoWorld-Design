//! 天气与季节基础类型 — CHG-016 Phase 1
//!
//! 仅包含核心枚举。完整 `WeatherSample`（~120 字节，14 消费者）推迟至天气系统独立 crate。
//! 参见: `WoWorld-Design/Change/CHG-016-天气与季节系统v1.0创建-20260612.md`

/// Markov 6-state 天气状态（梯度模型）。
///
/// 状态按严重程度递增排列：Clear → … → HeavyStorm。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherState {
    Clear,
    PartlyCloudy,
    Overcast,
    LightPrecip,
    ModeratePrecip,
    HeavyStorm,
}

impl WeatherState {
    /// 状态总数（用于随机选择）
    pub const COUNT: usize = 6;

    /// 从 u8 索引解码（0-5）
    pub fn from_index(i: u8) -> Self {
        match i {
            0 => Self::Clear,
            1 => Self::PartlyCloudy,
            2 => Self::Overcast,
            3 => Self::LightPrecip,
            4 => Self::ModeratePrecip,
            _ => Self::HeavyStorm,
        }
    }

    /// 状态在梯度链中的位置 (0-5)
    pub fn index(self) -> u8 {
        match self {
            Self::Clear => 0,
            Self::PartlyCloudy => 1,
            Self::Overcast => 2,
            Self::LightPrecip => 3,
            Self::ModeratePrecip => 4,
            Self::HeavyStorm => 5,
        }
    }
}

// ── WeatherParams ─────────────────────

/// 连续物理天气参数——替代离散 WeatherState。
///
/// 所有值连续演化，无跳变。视觉效果通过 lerp 从参数计算，
/// 而非查表——产生无限种中间天气。
#[derive(Debug, Clone, Copy)]
pub struct WeatherParams {
    /// 云量 0=晴空 → 1=全阴
    pub cloud_cover: f32,
    /// 降水强度 0=无 → 1=暴雨
    pub precipitation: f32,
    /// 风速 m/s
    pub wind_speed: f32,
    /// 相对湿度 0→1
    pub humidity: f32,
    /// 气温 °C（基线=季节温度 + 天气扰动）
    pub temperature: f32,
    /// 气压 hPa（1013 标准海平面）
    pub pressure: f32,
}

impl Default for WeatherParams {
    fn default() -> Self {
        Self {
            cloud_cover: 0.0,
            precipitation: 0.0,
            wind_speed: 0.0,
            humidity: 0.5,
            temperature: 20.0,
            pressure: 1013.0,
        }
    }
}

impl WeatherParams {
    /// 线性插值——0-1 之间的 t 映射到 a-b 之间
    #[inline]
    pub fn lerp(a: f32, b: f32, t: f32) -> f32 {
        a + (b - a) * t.clamp(0.0, 1.0)
    }

    /// RGB 三元插值
    #[inline]
    pub fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
        [Self::lerp(a[0], b[0], t), Self::lerp(a[1], b[1], t), Self::lerp(a[2], b[2], t)]
    }

    /// 连续参数 → 离散 WeatherState（调试快捷键兼容）
    pub fn to_weather_state(&self) -> WeatherState {
        let c = self.cloud_cover;
        let p = self.precipitation;
        // 优先按降水分类，无降水时按云量分类
        if p >= 0.7 {
            WeatherState::HeavyStorm
        } else if p >= 0.35 {
            WeatherState::ModeratePrecip
        } else if p >= 0.10 {
            WeatherState::LightPrecip
        } else if c >= 0.7 {
            WeatherState::Overcast
        } else if c >= 0.3 {
            WeatherState::PartlyCloudy
        } else {
            WeatherState::Clear
        }
    }

    /// 从 WeatherState 预设连续参数（调试快捷键 → 近似旧视觉）
    pub fn from_weather_state(state: WeatherState) -> Self {
        match state {
            WeatherState::Clear => Self {
                cloud_cover: 0.0, precipitation: 0.0, wind_speed: 0.5,
                humidity: 0.3, temperature: 20.0, pressure: 1015.0,
            },
            WeatherState::PartlyCloudy => Self {
                cloud_cover: 0.35, precipitation: 0.0, wind_speed: 2.0,
                humidity: 0.4, temperature: 19.0, pressure: 1013.0,
            },
            WeatherState::Overcast => Self {
                cloud_cover: 0.75, precipitation: 0.02, wind_speed: 5.0,
                humidity: 0.6, temperature: 17.0, pressure: 1010.0,
            },
            WeatherState::LightPrecip => Self {
                cloud_cover: 0.85, precipitation: 0.15, wind_speed: 8.0,
                humidity: 0.75, temperature: 15.0, pressure: 1008.0,
            },
            WeatherState::ModeratePrecip => Self {
                cloud_cover: 0.93, precipitation: 0.40, wind_speed: 12.0,
                humidity: 0.85, temperature: 12.0, pressure: 1004.0,
            },
            WeatherState::HeavyStorm => Self {
                cloud_cover: 0.98, precipitation: 0.80, wind_speed: 20.0,
                humidity: 0.95, temperature: 8.0, pressure: 998.0,
            },
        }
    }
}

// ── Season ────────────────────────────

/// 四季枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Season {
    Spring,
    Summer,
    Autumn,
    Winter,
}

impl Season {
    /// 从总游戏天数计算当前季节（120 天/年，每季 30 天）
    pub fn from_total_days(total_days: u64) -> Self {
        match (total_days / 30) % 4 {
            0 => Self::Spring,
            1 => Self::Summer,
            2 => Self::Autumn,
            _ => Self::Winter,
        }
    }
}
