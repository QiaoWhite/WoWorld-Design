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
