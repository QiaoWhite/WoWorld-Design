//! WoWorld WorldGen — 程序化世界生成
//!
//! 实现双层 Perlin 噪声地形生成和 TerrainQuery trait。
//! 体素: Transvoxel 等值面提取（顶点共享 + 过渡单元）。MC 查找表为 Transvoxel 数学地基。
//!
//! ## 密度场多层架构
//!
//! Decorator 链 — `DensityStack::new(base).with_cave_layer(seed, params).as_density()`
//! → 高 priority 先答，低 priority 兜底。

pub mod biome;
pub mod clipmap;
pub mod density;
pub mod marching_cubes;
pub mod noise_gen;
pub mod seed;
pub mod terrain;
pub mod terrain_mesh;
pub mod transition_tables;
pub mod transvoxel;

pub use biome::BiomeClassifier;
pub use clipmap::{ClipmapManager, LodKey, TileEvent};
pub use density::{
    CaveDensity, CaveParams, DensityField, DensityStack, HeightfieldDensity, VOXEL_AIR,
    VOXEL_DIRT, VOXEL_GRANITE, VOXEL_GRASS, VOXEL_GRAVEL, VOXEL_ICE, VOXEL_SAND, VOXEL_SNOW,
    VOXEL_STONE, VOXEL_WATER,
};
pub use noise_gen::{
    derive_noise_seed, worley_3d_f2f1, NoiseParams, WorldNoise,
    NOISE_DISCRIMINANT_WORLEY_CAVE,
};
pub use seed::{hash_chunk_seed, hash_stage_seed, mix64};
pub use terrain::HeightfieldTerrain;
pub use terrain_mesh::{generate_sh_mesh, TerrainMeshData};
pub use transvoxel::{extract_isosurface_transvoxel, IsoSurfaceParams};
pub use woworld_core::time::WorldClock;
