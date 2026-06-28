//! 大气合成输出 — 17 参数，设计文档 §2 规定
//!
//! 当前 ProceduralSkyMaterial 可消费前 6 参数，
//! 其余为预留 shader uniform（待自定义 shader 消费）。

/// 一帧的大气合成结果（17 参数＝3×17＝51 floats）
///
/// 消费者：
/// - Godot ProceduralSkyMaterial（sky_zenith_color, sky_horizon_color）
/// - Godot Environment（ambient_color）
/// - Godot DirectionalLight3D（sun_color, sun_energy, sun_elevation）
/// - 未来自定义 shader（rayleigh_mult, mie_mult, exposure_mult, saturation_mult）
/// - 未来音频系统（fog_density → 混响）
/// - 未来 NPC 感知（ambient_color → 可见度）
#[derive(Copy, Clone, Debug)]
pub struct ResolvedAtmosphere {
    // ── 当前 Godot 节点可消费（6 参数）──
    /// 天空顶色（群系 zenith_tint × 时间曲线 sky_zenith）
    pub sky_zenith_color: [f32; 3],
    /// 天空地平线色（群系 horizon_tint × 时间曲线 sky_horizon）
    pub sky_horizon_color: [f32; 3],
    /// 环境光色（群系 ambient_tint × 时间曲线 ambient）
    pub ambient_color: [f32; 3],
    /// 太阳光颜色（时间曲线 sun_color）
    pub sun_color: [f32; 3],
    /// 太阳光能量（时间曲线 sun_energy）
    pub sun_energy: f32,
    /// 太阳高度角（rad，从 WorldTime 透传）
    pub sun_elevation: f32,

    // ── 预留 shader uniform（当前无消费方）──
    /// Rayleigh 散射乘数（晴天=深蓝，沙尘暴=棕黄）
    pub rayleigh_mult: [f32; 3],
    /// Mie 散射乘数（地平线色调）
    pub mie_mult: [f32; 3],
    /// 曝光乘数（阴天降低）
    pub exposure_mult: f32,
    /// 饱和度乘数（雨天降低）
    pub saturation_mult: f32,
    /// 雾颜色
    pub fog_color: [f32; 3],
    /// 雾密度（→ 音频混响 + NPC 可见度）
    pub fog_density: f32,
    /// 阴影色调
    pub shadow_tint: [f32; 3],
    /// 天空环境光贡献系数
    pub ambient_sky_contrib: f32,
    /// 夜空亮度（群系/天气调制）
    pub night_brightness: f32,
    /// 地面地平线色
    pub ground_horizon: [f32; 3],
    /// 地面曲线（ProceduralSkyMaterial ground_curve）
    pub ground_curve: f32,
}

impl ResolvedAtmosphere {
    /// 输出为 `[f32; 51]`（17 参数 × 3 分量/vec）
    ///
    /// 顺序：sky_zenith(3) → sky_horizon(3) → ambient(3) → sun_color(3) → sun_energy(1) →
    /// sun_elevation(1) → rayleigh_mult(3) → mie_mult(3) → exposure_mult(1) →
    /// saturation_mult(1) → fog_color(3) → fog_density(1) → shadow_tint(3) →
    /// ambient_sky_contrib(1) → night_brightness(1) → ground_horizon(3) → ground_curve(1)
    ///
    /// 索引按此顺序固定——消费者用命名常量访问，不依赖魔术数字。
    pub fn as_float_array(&self) -> [f32; array_index::LEN] {
        let mut a = [0.0f32; array_index::LEN];
        let mut i: usize = 0;

        macro_rules! push3 {
            ($v:expr) => {
                a[i] = $v[0];
                a[i + 1] = $v[1];
                a[i + 2] = $v[2];
                i += 3;
            };
        }
        macro_rules! push1 {
            ($v:expr) => {
                a[i] = $v;
                i += 1;
            };
        }

        push3!(self.sky_zenith_color);
        push3!(self.sky_horizon_color);
        push3!(self.ambient_color);
        push3!(self.sun_color);
        push1!(self.sun_energy);
        push1!(self.sun_elevation);
        push3!(self.rayleigh_mult);
        push3!(self.mie_mult);
        push1!(self.exposure_mult);
        push1!(self.saturation_mult);
        push3!(self.fog_color);
        push1!(self.fog_density);
        push3!(self.shadow_tint);
        push1!(self.ambient_sky_contrib);
        push1!(self.night_brightness);
        push3!(self.ground_horizon);
        push1!(self.ground_curve);

        assert_eq!(
            i,
            array_index::LEN,
            "wrote {} params, expected {}",
            i,
            array_index::LEN
        );
        a
    }
}

impl Default for ResolvedAtmosphere {
    fn default() -> Self {
        Self {
            sky_zenith_color: [0.3, 0.45, 0.65],
            sky_horizon_color: [0.6, 0.7, 0.8],
            ambient_color: [0.5, 0.5, 0.5],
            sun_color: [1.0, 0.98, 0.95],
            sun_energy: 1.0,
            sun_elevation: 1.0,
            rayleigh_mult: [1.0, 1.0, 1.0],
            mie_mult: [1.0, 1.0, 1.0],
            exposure_mult: 1.0,
            saturation_mult: 1.0,
            fog_color: [0.6, 0.6, 0.7],
            fog_density: 0.0,
            shadow_tint: [0.12, 0.18, 0.22],
            ambient_sky_contrib: 0.5,
            night_brightness: 0.12,
            ground_horizon: [0.4, 0.5, 0.5],
            ground_curve: 2.0,
        }
    }
}

// ── 命名常量：PackedFloat32Array 索引 ──────────────

/// `ResolvedAtmosphere::as_float_array()` 输出的索引常量
pub mod array_index {
    pub const SKY_ZENITH: usize = 0;
    pub const SKY_HORIZON: usize = 3;
    pub const AMBIENT: usize = 6;
    pub const SUN_COLOR: usize = 9;
    pub const SUN_ENERGY: usize = 12;
    pub const SUN_ELEVATION: usize = 13;
    pub const RAYLEIGH_MULT: usize = 14;
    pub const MIE_MULT: usize = 17;
    pub const EXPOSURE_MULT: usize = 20;
    pub const SATURATION_MULT: usize = 21;
    pub const FOG_COLOR: usize = 22;
    pub const FOG_DENSITY: usize = 25;
    pub const SHADOW_TINT: usize = 26;
    pub const AMBIENT_SKY_CONTRIB: usize = 29;
    pub const NIGHT_BRIGHTNESS: usize = 30;
    pub const GROUND_HORIZON: usize = 31;
    pub const GROUND_CURVE: usize = 34;
    pub const LEN: usize = 35;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_array_len_matches_declared() {
        let atm = ResolvedAtmosphere::default();
        let arr = atm.as_float_array();
        assert_eq!(arr.len(), array_index::LEN);
    }
}
