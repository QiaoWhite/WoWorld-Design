//! 跨模块查询 trait stubs
//!
//! 预留群系/天气/季节模块的调制接口。当前全部返回恒等值（不影响输出），
//! 后续模块就绪后在各自 crate 中实现这些 trait。

use woworld_core::prelude::*;

/// 群系大气查询 — 由群系系统实现
///
/// 位置 → 群系特定的颜色调制参数。
/// 当前 stub 返回恒等值。
pub trait BiomeAtmosQuery {
    /// 天顶色调制（恒等 = 无调制）
    fn zenith_tint(&self, _pos: WorldPos) -> [f32; 3] {
        [1.0, 1.0, 1.0]
    }
    /// 地平线色调制
    fn horizon_tint(&self, _pos: WorldPos) -> [f32; 3] {
        [1.0, 1.0, 1.0]
    }
    /// 环境光色调制
    fn ambient_tint(&self, _pos: WorldPos) -> [f32; 3] {
        [1.0, 1.0, 1.0]
    }
    /// 阴影色调
    fn shadow_tint(&self, _pos: WorldPos) -> [f32; 3] {
        [0.12, 0.18, 0.22]
    }
    /// 夜空亮度
    fn night_brightness(&self, _pos: WorldPos) -> f32 {
        0.12
    }
}

/// 天气大气查询 — 由天气系统实现
///
/// 位置 + 时间 → 天气调制参数。
/// 当前 stub 返回恒等值。
pub trait WeatherAtmosQuery {
    /// 天空散射色调制（正常=白，沙尘暴=棕黄）
    fn sky_color_mult(&self, _pos: WorldPos, _time: &WorldTime) -> [f32; 3] {
        [1.0, 1.0, 1.0]
    }
    /// 雾密度 (0.0 = 清澈)
    fn fog_density(&self, _pos: WorldPos, _time: &WorldTime) -> f32 {
        0.0
    }
    /// 曝光乘数（阴天降低）
    fn exposure_mult(&self, _pos: WorldPos, _time: &WorldTime) -> f32 {
        1.0
    }
    /// 饱和度乘数（雨天降低）
    fn saturation_mult(&self, _pos: WorldPos, _time: &WorldTime) -> f32 {
        1.0
    }
}

/// 季节大气查询 — 由季节系统实现
///
/// 季节进度 (0-1) → 季节调制参数。
/// 当前 stub 返回恒等值。
pub trait SeasonAtmosQuery {
    /// 饱和度乘数（秋季=0.90, 冬季=0.70）
    fn saturation_mult(&self, _season_progress: f32) -> f32 {
        1.0
    }
    /// 环境色温暖度偏移量（金秋=+0.08, 冷白=-0.05）
    fn warmth_add(&self, _season_progress: f32) -> f32 {
        0.0
    }
}
