//! 3D 密度场抽象
//!
//! Marching Cubes 消费此 trait：在 3D 空间采样密度值，
//! 等值面 threshold=0.5 处提取三角形网格。
//!
//! 当前 MVP: HeightfieldDensity — 把噪声高度场映射为平滑 3D 密度。
//! 未来: 多层密度场叠加 (L0-L10 per 007-体素设计决策 §1.4)。

use crate::noise_gen::WorldNoise;

/// 3D 密度场 trait
///
/// 返回值 ∈ [0.0, 1.0]:
/// - 0.0 = 纯空气
/// - 1.0 = 纯固体
/// - 0.5 = 等值面（Marching Cubes 提取边界）
pub trait DensityField: Send + Sync {
    fn sample(&self, x: f64, y: f64, z: f64) -> f32;
}

/// 高度场密度函数
///
/// 把 2D 噪声高度映射为 3D 密度——地表 ±1m 过渡带产生平滑等值面。
/// 不引入新的 3D 噪声——消费已有的 `WorldNoise::sample_height()`。
pub struct HeightfieldDensity {
    noise: WorldNoise,
    /// 地表过渡带半宽（米）——默认 1.0
    half_band: f64,
}

impl HeightfieldDensity {
    pub fn new(noise: WorldNoise) -> Self {
        Self {
            noise,
            half_band: 1.0,
        }
    }
}

impl DensityField for HeightfieldDensity {
    fn sample(&self, x: f64, y: f64, z: f64) -> f32 {
        let h = self.noise.sample_height(x, z);
        // dist > 0 = 地下, dist < 0 = 空中
        let dist = h - y;
        // smoothstep: 从 -half_band(空气) 到 +half_band(固体)
        let t = (dist + self.half_band) / (2.0 * self.half_band);
        t.clamp(0.0, 1.0) as f32
    }
}
