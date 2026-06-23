//! TerrainChunk — Godot Node3D GDExtension 类
//!
//! 暴露 HeightfieldTerrain 的高度/材质查询给 GDScript。
//! GDScript 侧用 SurfaceTool 构建 ArrayMesh。

use godot::prelude::*;
use woworld_core::spatial::TerrainQuery;
use woworld_worldgen::HeightfieldTerrain;

/// Godot GDExtension 类：地形块
///
/// 持有 Rust 侧 HeightfieldTerrain，暴露 `query_height(x, z)` 和
/// `query_material(x, z)` 给 GDScript。
#[derive(GodotClass)]
#[class(base = Node3D, init)]
pub struct TerrainChunk {
    terrain: HeightfieldTerrain,

    #[base]
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for TerrainChunk {
    fn ready(&mut self) {
        godot_print!("TerrainChunk: terrain ready (seed=42)");
    }
}

#[godot_api]
impl TerrainChunk {
    /// 查询 (x, z) 处的地形高度（米）
    #[func]
    fn query_height(&self, x: f64, z: f64) -> f32 {
        let pos = woworld_core::prelude::WorldPos { x, y: 0.0, z };
        self.terrain.height_at(pos)
    }

    /// 查询 (x, z) 处的地表材质
    /// 返回值: 0=Grass, 1=Sand, 2=Rock, 3=Stone, 4=Water, ...
    #[func]
    fn query_material(&self, x: f64, z: f64) -> i32 {
        let pos = woworld_core::prelude::WorldPos { x, y: 0.0, z };
        let mat = self.terrain.surface_material_at(pos);
        material_to_i32(mat)
    }
}

/// SurfaceMaterial → GDScript 材质索引
fn material_to_i32(mat: woworld_core::material::SurfaceMaterial) -> i32 {
    use woworld_core::material::SurfaceMaterial::*;
    match mat {
        Grass => 0,
        Sand => 1,
        Rock => 2,
        Stone => 3,
        Wood => 4,
        Metal => 5,
        Water => 6,
        Ice => 7,
        Mud => 8,
        Snow => 9,
        Gravel => 10,
        Clay => 11,
        Moss => 12,
        LeafLitter => 13,
        Cobblestone => 14,
        Marble => 15,
        Glass => 16,
        Fabric => 17,
        Thatch => 18,
        Bone => 19,
        Flesh => 20,
    }
}
