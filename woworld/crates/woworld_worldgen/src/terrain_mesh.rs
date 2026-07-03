//! 地形网格数据 — 纯 Rust，引擎无关
//!
//! TerrainMeshData 是 clipmap 网格生成和 heightmap 数据传递的载体。
//! 不依赖 Godot。输出适合传递给 Godot ArrayMesh 的原始数组。

use glam::Vec3;

/// 网格数据——可直接转换为 Godot PackedArray
#[derive(Debug)]
pub struct TerrainMeshData {
    pub vertices: Vec<Vec3>,
    pub normals: Vec<Vec3>,
    pub indices: Vec<u32>,
    pub colors: Vec<Vec3>,
}
