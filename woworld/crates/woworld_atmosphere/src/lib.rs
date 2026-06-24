//! WoWorld Atmosphere — 大气与氛围系统
//!
//! 独立模块（设计文档：大气与氛围系统 #25）。合成 `ResolvedAtmosphere`（17 参数），
//! 输出为 `PackedFloat32Array` 供 Godot shader/节点消费。
//!
//! 依赖仅 `woworld_core`（WorldTime, WorldPos）——引擎无关。
//!
//! 当前阶段：时间曲线优先。群系/天气/季节调制预留 trait stub。

pub mod resolved_atmosphere;
pub mod synthesizer;
pub mod time_curve;
pub mod traits;

pub use resolved_atmosphere::ResolvedAtmosphere;
pub use synthesizer::AtmosphereSynthesizer;
pub use time_curve::{AtmosAnchor, AtmosCurve};
pub use traits::{BiomeAtmosQuery, SeasonAtmosQuery, WeatherAtmosQuery};
