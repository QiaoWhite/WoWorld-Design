//! 大气合成器 — 组装四层调制 → ResolvedAtmosphere
//!
//! 层一（大气曲线）由太阳高度角驱动——与天文学严格同步。
//! 层二～四（群系/天气/季节）为 stub。
//!
//! 合成链：AtmosCurve(sun_elevation) → Biome × Weather × Season → ResolvedAtmosphere

use woworld_core::prelude::*;

use crate::resolved_atmosphere::ResolvedAtmosphere;
use crate::time_curve::AtmosCurve;

/// 大气合成器
///
/// 组装四层调制并输出 `ResolvedAtmosphere`。
/// 层一（大气曲线）由太阳高度角驱动——与天文学同步。
/// 层三（天气）和层四（季节）由调用方传入；层二（群系）推迟至气候场就绪。
pub struct AtmosphereSynthesizer {
    pub(crate) curve: AtmosCurve,
}

impl Default for AtmosphereSynthesizer {
    fn default() -> Self {
        Self::from_embedded_toml().expect("embedded atmos_curve.toml must be valid")
    }
}

impl AtmosphereSynthesizer {
    /// 从嵌入的 TOML 创建（编译时嵌入）
    pub fn from_embedded_toml() -> Result<Self, toml::de::Error> {
        Self::from_toml_str(include_str!("../assets/atmos_curve.toml"))
    }

    /// 从 TOML 字符串创建
    pub fn from_toml_str(toml: &str) -> Result<Self, toml::de::Error> {
        let curve = AtmosCurve::from_toml_str(toml)?;
        Ok(Self { curve })
    }

    /// 每帧调用 —— 合成一帧的 17 参数大气输出（无天气/季节调制）
    pub fn resolve(&self, time: &WorldTime, pos: WorldPos) -> ResolvedAtmosphere {
        self.resolve_with_weather(time, pos, [1.0; 3], 0.0, 1.0, 1.0, 1.0, 0.0)
    }

    /// 每帧调用 —— 含天气/季节调制参数。
    ///
    /// 性能预算: <0.02ms（设计文档规定）
    #[allow(clippy::too_many_arguments)]
    pub fn resolve_with_weather(
        &self,
        time: &WorldTime,
        pos: WorldPos,
        weather_sky_mult: [f32; 3],
        weather_fog: f32,
        weather_exposure: f32,
        weather_saturation: f32,
        season_saturation: f32,
        season_warmth: f32,
    ) -> ResolvedAtmosphere {
        // 层一：大气曲线（太阳高度角驱动——与天文学同步）
        let ts = self.curve.evaluate(time.sun_elevation);

        // 层二：群系调制 — stub（恒等，待气候场就绪）
        let bio_zenith = stub_biome_zenith_tint(pos);
        let bio_horizon = stub_biome_horizon_tint(pos);
        let bio_ambient = stub_biome_ambient_tint(pos);
        let bio_shadow = stub_biome_shadow_tint(pos);
        let bio_night = stub_biome_night_brightness(pos);

        // 层三：天气调制（调用方传入）
        let wea_sky_mult = weather_sky_mult;
        let wea_fog = weather_fog;
        let wea_exp = weather_exposure;
        let wea_sat = weather_saturation;

        // 层四：季节调制（调用方传入）
        let sea_sat = season_saturation;
        let sea_warmth = season_warmth;

        // 合成最终颜色（逐通道乘数）
        let final_sat = wea_sat * sea_sat;

        ResolvedAtmosphere {
            sky_zenith_color: mul3(ts.sky_zenith, mul3(bio_zenith, wea_sky_mult)),
            sky_horizon_color: mul3(ts.sky_horizon, mul3(bio_horizon, wea_sky_mult)),
            ambient_color: mul3(
                ts.ambient,
                mul3(
                    bio_ambient,
                    [sea_warmth + 0.5, sea_warmth + 0.5, 1.0 - sea_warmth],
                ),
            ),
            sun_color: ts.sun_color,
            sun_energy: ts.sun_energy,
            sun_elevation: time.sun_elevation as f32,

            // 预留 shader 通道
            rayleigh_mult: wea_sky_mult,
            mie_mult: [1.0, 1.0, 1.0],
            exposure_mult: wea_exp,
            saturation_mult: final_sat,
            fog_color: mul3([0.6, 0.6, 0.7], wea_sky_mult),
            fog_density: wea_fog,
            shadow_tint: bio_shadow,
            ambient_sky_contrib: 0.5,
            night_brightness: bio_night,
            ground_horizon: mul3([0.4, 0.5, 0.5], bio_horizon),
            ground_curve: 2.0,
        }
    }
}

// ── stub 函数（当前恒等，后续替换为 trait 对象调用）──

fn stub_biome_zenith_tint(_pos: WorldPos) -> [f32; 3] {
    [1.0, 1.0, 1.0]
}
fn stub_biome_horizon_tint(_pos: WorldPos) -> [f32; 3] {
    [1.0, 1.0, 1.0]
}
fn stub_biome_ambient_tint(_pos: WorldPos) -> [f32; 3] {
    [1.0, 1.0, 1.0]
}
fn stub_biome_shadow_tint(_pos: WorldPos) -> [f32; 3] {
    [0.12, 0.18, 0.22]
}
fn stub_biome_night_brightness(_pos: WorldPos) -> f32 {
    0.12
}

/// 逐通道乘法
fn mul3(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] * b[0], a[1] * b[1], a[2] * b[2]]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;
    use woworld_core::time::WorldTime;

    fn synth() -> AtmosphereSynthesizer {
        let toml = include_str!("../assets/atmos_curve.toml");
        AtmosphereSynthesizer::from_toml_str(toml).expect("Failed to parse")
    }

    #[test]
    fn test_noon_output_is_bright() {
        let s = synth();
        let time = WorldTime::from_progress(0.5, 0, 120);
        let result = s.resolve(&time, WorldPos::default());
        assert!(result.sun_energy > 0.8);
        assert!(result.sky_zenith_color[2] > result.sky_zenith_color[0]);
    }

    #[test]
    fn test_midnight_output_is_dark() {
        let s = synth();
        let time = WorldTime::from_progress(0.0, 0, 120);
        let result = s.resolve(&time, WorldPos::default());
        assert!(result.sun_energy < 0.1);
        assert!(result.ambient_color[0] < 0.1);
    }

    #[test]
    fn test_stub_layer_passthrough_at_noon() {
        let s = synth();
        let time = WorldTime::from_progress(0.5, 0, 120);
        let result = s.resolve(&time, WorldPos::default());

        let ts = s.curve.evaluate(FRAC_PI_2); // elev=90°=noon
        assert!((result.sun_energy - ts.sun_energy).abs() < 0.01);
    }

    #[test]
    fn test_field_count_is_17() {
        let s = synth();
        let time = WorldTime::from_progress(0.3, 0, 120);
        let result = s.resolve(&time, WorldPos::default());
        // ResolvedAtmosphere 有 17 个字段——直接通过访问验证（as_float_array 已删除）
        let _ = result.sky_zenith_color;
        let _ = result.sky_horizon_color;
        let _ = result.ambient_color;
        let _ = result.sun_color;
        let _ = result.sun_energy;
        let _ = result.sun_elevation;
        let _ = result.rayleigh_mult;
        let _ = result.mie_mult;
        let _ = result.exposure_mult;
        let _ = result.saturation_mult;
        let _ = result.fog_color;
        let _ = result.fog_density;
        let _ = result.shadow_tint;
        let _ = result.ambient_sky_contrib;
        let _ = result.night_brightness;
        let _ = result.ground_horizon;
        let _ = result.ground_curve;
        // 以上就是 17 个字段——如果编译不过，说明结构体字段变了
    }
}
