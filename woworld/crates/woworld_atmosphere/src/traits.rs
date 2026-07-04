//! 大气调制 trait — 群系/天气/季节
//!
//! Phase 1 重建天气和季节调制接口。群系 trait 推迟至世界生成气候场就绪。
//! 参见: CHG-016 天气与季节系统 v1.0 · CHG-064 昼夜循环 + 5群系系统

/// 天气→大气调制查询（每帧调用）
pub trait WeatherAtmosQuery: Send + Sync {
    /// 天空/雾色乘数（RGB，0-1）
    fn sky_mult(&self) -> [f32; 3];
    /// 雾密度（0=无雾，1=完全不透明）
    fn fog_density(&self) -> f32;
    /// 曝光乘数（0=全黑，1=正常）
    fn exposure_mult(&self) -> f32;
    /// 饱和度乘数（0=灰度，1=正常）
    fn saturation_mult(&self) -> f32;
}

/// 季节→大气调制查询（每帧调用）
pub trait SeasonAtmosQuery: Send + Sync {
    /// 饱和度乘数（0=灰度，1=正常）
    fn saturation_mult(&self) -> f32;
    /// 色温偏移（-1=极冷蓝，0=中性，+1=极暖橙）
    fn warmth(&self) -> f32;
}
