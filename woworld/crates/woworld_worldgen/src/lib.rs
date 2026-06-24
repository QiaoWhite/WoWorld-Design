//! WoWorld WorldGen — 程序化世界生成
//!
//! 实现双层 Perlin 噪声地形生成和 TerrainQuery trait。
//! 当前阶段: 高度场地形（无体素/无 Transvoxel）。

pub mod biome;
pub mod chunk_manager;
pub mod clipmap;
pub mod density;
pub mod marching_cubes;
pub mod noise_gen;
pub mod terrain;
pub mod terrain_mesh;

pub use biome::BiomeClassifier;
pub use chunk_manager::{ChunkEvent, ChunkManager};
pub use clipmap::{ClipmapManager, LodKey, TileEvent};
// density 模块保留用于未来多层密度场——当前 MC 直接消费 HeightfieldTerrain
pub use noise_gen::{NoiseParams, WorldNoise};
pub use terrain::HeightfieldTerrain;
pub use terrain_mesh::{generate_terrain_mesh, TerrainMeshData};
pub use woworld_core::time::WorldClock;
