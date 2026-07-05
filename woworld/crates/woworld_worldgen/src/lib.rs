//! WoWorld WorldGen — 程序化世界生成
//!
//! 双层 Perlin 噪声地形 + GPU-Driven Clipmap LOD 地形渲染。
//! 纯 Rust 计算引擎——引擎无关，不依赖 Godot。

pub mod biome;
pub mod cave;
pub mod clipmap;
pub mod noise_gen;
pub mod ocean;
pub mod terrain;
pub mod terrain_mesh;
pub mod transition_tables;
pub mod transvoxel;
pub mod tri_table_data;
pub mod vegetation;

pub use biome::{BiomeClassifier, BiomeDef};
pub use cave::CaveDensity;
pub use clipmap::{
    generate_clipmap_grid, generate_heightmap, generate_heightmap_data, layer_tex_config,
    level_spacing, load_material_colors, sample_colors_from, sample_heightmap_from, HeightmapData,
    LayerTexConfig, LodLevel, MeshAlgorithm, LEVELS,
};
pub use noise_gen::{NoiseParams, WorldNoise};
pub use ocean::HeightfieldOcean;
pub use terrain::{HeightfieldTerrain, TerrainBaseDensity};
pub use terrain_mesh::TerrainMeshData;
pub use transvoxel::transvoxel_extract;
pub use vegetation::BiomeVegetation;
pub use woworld_core::time::WorldClock;
