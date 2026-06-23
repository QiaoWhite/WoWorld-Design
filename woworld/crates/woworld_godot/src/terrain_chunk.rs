//! TerrainChunk — Godot Node3D GDExtension 类
//!
//! 最小存根——地形网格生成已由 terrain_mesh 模块完成（纯 Rust，可测试）。
//! Godot ArrayMesh 构建逻辑将在后续冲刺完善。

use godot::prelude::*;

use crate::terrain_mesh::generate_terrain_mesh;
use woworld_worldgen::HeightfieldTerrain;

/// Godot GDExtension 类：地形块
///
/// 在 `ready()` 时生成网格数据并打印统计信息。
/// ArrayMesh 构建将在 Godot 0.5.x API 稳定后完善。
#[derive(GodotClass)]
#[class(base = Node3D, init)]
pub struct TerrainChunk {
    #[base]
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for TerrainChunk {
    fn ready(&mut self) {
        let terrain = HeightfieldTerrain::new(42);
        let mesh_data = generate_terrain_mesh(&terrain, -128.0, -128.0, 128, 2.0);

        godot_print!(
            "TerrainChunk: {} vertices, {} triangles",
            mesh_data.vertices.len(),
            mesh_data.indices.len() / 3
        );
    }
}
