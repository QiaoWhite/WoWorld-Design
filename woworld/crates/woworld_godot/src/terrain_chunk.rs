//! TerrainChunk — Godot Node3D GDExtension 类
//!
//! 在 Rust 侧生成地形网格，通过 GDExtension 直接构建 Godot ArrayMesh。

use godot::classes::mesh::PrimitiveType;
use godot::classes::{ArrayMesh, MeshInstance3D};
use godot::prelude::*;
use woworld_core::prelude::WorldPos;
use woworld_core::spatial::TerrainQuery;
use woworld_worldgen::HeightfieldTerrain;

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
        let grid_size: i32 = 128;
        let spacing: f64 = 2.0;
        let origin_x: f64 = -128.0;
        let origin_z: f64 = -128.0;

        // 生成顶点 + 颜色
        let mut vertices = PackedVector3Array::new();
        let mut colors = PackedColorArray::new();

        for iz in 0..grid_size {
            let wz = origin_z + iz as f64 * spacing;
            for ix in 0..grid_size {
                let wx = origin_x + ix as f64 * spacing;
                let pos = WorldPos {
                    x: wx,
                    y: 0.0,
                    z: wz,
                };
                let h = self.terrain.height_at(pos);
                let mat = self.terrain.surface_material_at(pos);

                vertices.push(Vector3::new(wx as f32, h, wz as f32));
                colors.push(material_color(mat, h));
            }
        }

        // 生成索引
        let mut indices = PackedInt32Array::new();
        for iz in 0..(grid_size - 1) {
            for ix in 0..(grid_size - 1) {
                let tl = iz * grid_size + ix;
                let tr = iz * grid_size + ix + 1;
                let bl = (iz + 1) * grid_size + ix;
                let br = (iz + 1) * grid_size + ix + 1;
                indices.push(tl);
                indices.push(bl);
                indices.push(tr);
                indices.push(tr);
                indices.push(bl);
                indices.push(br);
            }
        }

        // 构建 ArrayMesh: add_surface_from_arrays(primitive, &AnyArray) — 仅 2 参数
        let mut arrays = Array::new();
        let v = vertices.to_variant();
        arrays.push(&v);
        let c = colors.to_variant();
        arrays.push(&c);
        let i = indices.to_variant();
        arrays.push(&i);

        let mut array_mesh = ArrayMesh::new_gd();
        array_mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);

        // MeshInstance3D
        let mut mesh_instance = MeshInstance3D::new_alloc();
        mesh_instance.set_name("GeneratedTerrain");

        // upcast MeshInstance3D → Node, then add_child
        let child = mesh_instance.upcast::<Node>();
        self.base_mut().add_child(&child);

        godot_print!(
            "TerrainChunk: {} vertices, {} triangles",
            vertices.len(),
            indices.len() / 3
        );
    }
}

fn material_color(mat: woworld_core::material::SurfaceMaterial, height: f32) -> Color {
    use woworld_core::material::SurfaceMaterial::*;
    match mat {
        Water => Color::from_rgb(0.1, 0.3, 0.8),
        Sand => Color::from_rgb(0.76, 0.7, 0.5),
        Grass => {
            if height > 100.0 {
                Color::from_rgb(0.2, 0.55, 0.2)
            } else {
                Color::from_rgb(0.3, 0.65, 0.25)
            }
        }
        Rock => Color::from_rgb(0.45, 0.42, 0.38),
        Stone => Color::from_rgb(0.35, 0.35, 0.35),
        Gravel => Color::from_rgb(0.5, 0.45, 0.4),
        Snow => Color::from_rgb(0.95, 0.95, 0.95),
        Ice => Color::from_rgb(0.9, 0.95, 1.0),
        Mud => Color::from_rgb(0.4, 0.3, 0.2),
        _ => Color::from_rgb(0.4, 0.5, 0.3),
    }
}
