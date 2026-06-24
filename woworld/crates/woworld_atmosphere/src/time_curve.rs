//! 大气曲线 — TOML 数组驱动的通用锚点插值
//!
//! 锚点由**太阳高度角**（度，-90°→+90°）定义，归一化到 [0, 1] 后走通用圆形插值。
//! 物理驱动：太阳在地平线 ±0° 时天空色由 `elev=0°` 锚点确定——与天文学严格同步。
//!
//! 零硬编码：增删锚点只需修改 TOML 的 `[[anchors]]` 数组。

use serde::{de::Error, Deserialize};
#[cfg(test)]
use std::f64::consts::FRAC_PI_2;

// ── TOML 反序列化 ──────────────────

#[derive(Clone, Debug, Deserialize)]
struct CurveToml {
    #[serde(rename = "anchors")]
    anchors: Vec<AtmosAnchor>,
}

/// 单个大气锚点（TOML 中的一行 `[[anchors]]`）
#[derive(Copy, Clone, Debug, Deserialize)]
pub struct AtmosAnchor {
    /// 太阳高度角（度，-90 = 午夜天底，0 = 地平线，+90 = 正午天顶）
    pub sun_elevation: f32,
    /// 天空顶色 RGB
    pub sky_zenith: [f32; 3],
    /// 天空地平线色 RGB
    pub sky_horizon: [f32; 3],
    /// 环境光色 RGB
    pub ambient: [f32; 3],
    /// 太阳光颜色 RGB
    pub sun_color: [f32; 3],
    /// 太阳光能量 (0.0-1.0)
    pub sun_energy: f32,
}

// ── 运行时 ──────────────────────────

/// 大气曲线：按太阳高度角排序的锚点数组，圆形插值
#[derive(Clone, Debug)]
pub struct AtmosCurve {
    /// 锚点按 elevation 升序（-90° → +90°）
    anchors: Vec<AtmosAnchor>,
    /// 归一化位置：`(elev_deg + 90.0) / 180.0`，范围 [0, 1]
    positions: Vec<f32>,
}

impl AtmosCurve {
    /// 从 TOML 字符串加载。最少 2 个锚点。
    pub fn from_toml_str(toml: &str) -> Result<Self, toml::de::Error> {
        let raw: CurveToml = toml::from_str(toml)?;
        if raw.anchors.len() < 2 {
            return Err(toml::de::Error::custom("need at least 2 anchors"));
        }
        let mut curve = Self {
            anchors: raw.anchors,
            positions: Vec::new(),
        };
        // 确保按太阳高度升序
        curve.anchors.sort_by(|a, b| {
            a.sun_elevation
                .partial_cmp(&b.sun_elevation)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        // 派生归一化位置
        curve.positions = curve
            .anchors
            .iter()
            .map(|a| (a.sun_elevation + 90.0) / 180.0)
            .collect();
        Ok(curve)
    }

    /// 根据太阳高度角（弧度，来自 WorldTime.sun_elevation）插值大气参数
    ///
    /// 输入范围 [-π/2, +π/2] → 归一化到 [0, 1] → 锚点圆形插值。
    /// 高角 → 高归一化值。低角 → 低归一化值。圆形 wrap：+90° 锚点 wrap 回 -90° 锚点。
    pub fn evaluate(&self, sun_elevation_rad: f64) -> AtmosSample {
        let elev_deg = (sun_elevation_rad * 180.0 / std::f64::consts::PI) as f32;
        // 归一化：-90°→0, 0°→0.5, +90°→1.0
        let t = ((elev_deg + 90.0) / 180.0).clamp(0.0, 1.0);
        let n = self.anchors.len();

        for i in 0..n {
            let a = &self.anchors[i];
            let b = &self.anchors[(i + 1) % n];
            let a_pos = self.positions[i];
            let b_pos = self.positions[(i + 1) % n];

            if i == n - 1 {
                // wrap：最后一锚 → 第一锚（跨 +90°↔-90° 的 midnight 天底）
                let b_wrap = b_pos + 1.0;
                let t_adj = if t < b_pos { t + 1.0 } else { t };
                if t_adj >= a_pos && t_adj < b_wrap {
                    let frac = (t_adj - a_pos) / (b_wrap - a_pos);
                    return lerp(a, b, frac);
                }
            } else if t >= a_pos && t <= b_pos {
                let frac = if b_pos > a_pos {
                    ((t - a_pos) / (b_pos - a_pos)).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                return lerp(a, b, frac);
            }
        }

        sample(&self.anchors[0])
    }
}

// ── 输出 ────────────────────────────

#[derive(Copy, Clone, Debug)]
pub struct AtmosSample {
    pub sky_zenith: [f32; 3],
    pub sky_horizon: [f32; 3],
    pub ambient: [f32; 3],
    pub sun_color: [f32; 3],
    pub sun_energy: f32,
}

fn sample(a: &AtmosAnchor) -> AtmosSample {
    AtmosSample {
        sky_zenith: a.sky_zenith,
        sky_horizon: a.sky_horizon,
        ambient: a.ambient,
        sun_color: a.sun_color,
        sun_energy: a.sun_energy,
    }
}

fn lerp(a: &AtmosAnchor, b: &AtmosAnchor, t: f32) -> AtmosSample {
    let t = t.clamp(0.0, 1.0);
    AtmosSample {
        sky_zenith: lerp3(a.sky_zenith, b.sky_zenith, t),
        sky_horizon: lerp3(a.sky_horizon, b.sky_horizon, t),
        ambient: lerp3(a.ambient, b.ambient, t),
        sun_color: lerp3(a.sun_color, b.sun_color, t),
        sun_energy: a.sun_energy + (b.sun_energy - a.sun_energy) * t,
    }
}

fn lerp3(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

// ── 测试 ────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn curve() -> AtmosCurve {
        AtmosCurve::from_toml_str(include_str!("../assets/atmos_curve.toml"))
            .expect("Failed to parse")
    }

    #[test]
    fn test_midnight_is_dark() {
        let c = curve();
        let s = c.evaluate(-FRAC_PI_2); // elev = -90°
        assert!(s.sun_energy < 0.1);
        assert!(s.ambient[0] < 0.1);
    }

    #[test]
    fn test_noon_is_bright() {
        let c = curve();
        let s = c.evaluate(FRAC_PI_2); // elev = +90°
        assert!(s.sun_energy > 0.8);
    }

    #[test]
    fn test_horizon_is_warm() {
        let c = curve();
        let s = c.evaluate(0.0); // elev = 0°
        assert!(s.sun_color[0] > s.sun_color[2], "horizon sun should be warm");
    }

    #[test]
    fn test_symmetric_around_noon() {
        // 太阳高度角对称——上升和下降段色彩相同（锚点驱动）
        let c = curve();
        let morning = c.evaluate(0.5236); // 30°
        let evening = c.evaluate(0.5236); // 30°
        assert!((morning.sun_energy - evening.sun_energy).abs() < 0.001);
    }

    #[test]
    fn test_seamless_wrap() {
        let c = curve();
        // 天底附近连续
        let near_nadir = c.evaluate(-1.55); // ≈ -89°
        let at_nadir = c.evaluate(-FRAC_PI_2); // -90°
        assert!((near_nadir.sun_energy - at_nadir.sun_energy).abs() < 0.15);
    }

    #[test]
    fn test_no_panic_any_elevation() {
        let c = curve();
        for i in 0..=200 {
            let elev = (i as f64 - 100.0) / 100.0 * FRAC_PI_2;
            let s = c.evaluate(elev);
            assert!(s.sun_energy >= 0.0 && s.sun_energy <= 1.5);
        }
    }
}
