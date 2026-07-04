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
