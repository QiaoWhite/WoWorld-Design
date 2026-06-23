//! WoWorld WorldGen — 程序化世界生成
//!
//! 实现双层 Perlin 噪声地形生成和 TerrainQuery trait。
//! 当前阶段: 高度场地形（无体素/无 Transvoxel）。

pub mod noise_gen;
pub mod terrain;

pub use noise_gen::{NoiseParams, WorldNoise};
pub use terrain::HeightfieldTerrain;
