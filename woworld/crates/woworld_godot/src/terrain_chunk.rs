//! TerrainChunk — Godot Node3D GDExtension 类
//!
//! 在 Rust 侧生成地形网格，通过 GDExtension 直接构建 Godot ArrayMesh。

use godot::classes::base_material_3d::{CullMode, Flags, ShadingMode};
use godot::classes::mesh::PrimitiveType;
use godot::classes::{ArrayMesh, MeshInstance3D, StandardMaterial3D};
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
        // Demo 参数：缩短噪声波长使 512m 网格内可见起伏
        use woworld_worldgen::{NoiseParams, WorldNoise};
        let params = NoiseParams {
            continent_scale: 0.001,
            detail_scale: 0.02,
            mountain_scale: 0.002,
            sea_threshold: -0.5,
            height_amplitude: 80.0,
            sea_depth: 30.0,
        };
        self.terrain = HeightfieldTerrain::with_noise(WorldNoise::with_params(42, params));

        let grid_size: i32 = 256;
        let spacing: f64 = 2.0;
        let origin_x: f64 = -256.0;
        let origin_z: f64 = -256.0;

        // 生成顶点、法线、颜色
        let mut vertices = PackedVector3Array::new();
        let mut normals = PackedVector3Array::new();
        let mut colors = PackedColorArray::new();
        let mut min_h: f32 = f32::MAX;
        let mut max_h: f32 = f32::MIN;

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
                let n = self.terrain.normal_at(pos);

                vertices.push(Vector3::new(wx as f32, h, wz as f32));
                normals.push(Vector3::new(n.x, n.y, n.z));
                colors.push(material_color(mat, h));
                min_h = min_h.min(h);
                max_h = max_h.max(h);
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

        // ArrayMesh: arrays 按 ARRAY_* 索引排列
        // [0]=VERTEX, [1]=NORMAL, [3]=COLOR, [12]=INDEX
        let mut arrays = Array::new();
        let nil = Variant::nil();
        arrays.resize(13, &nil);
        let v = vertices.to_variant();
        arrays.set(0, &v);
        let n = normals.to_variant();
        arrays.set(1, &n);
        let c = colors.to_variant();
        arrays.set(3, &c);
        let i = indices.to_variant();
        arrays.set(12, &i);

        let mut array_mesh = ArrayMesh::new_gd();
        array_mesh.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);

        // 顶点色材质
        let mut mat = StandardMaterial3D::new_gd();
        mat.set_flag(Flags::ALBEDO_FROM_VERTEX_COLOR, true);
        mat.set_shading_mode(ShadingMode::UNSHADED);
        mat.set_cull_mode(CullMode::DISABLED);

        let mut terrain_instance = MeshInstance3D::new_alloc();
        terrain_instance.set_name("GeneratedTerrain");
        terrain_instance.set_mesh(&array_mesh);
        terrain_instance.set_surface_override_material(0, &mat);
        let t = terrain_instance.upcast::<Node>();
        self.base_mut().add_child(&t);

        godot_print!(
            "TerrainChunk: {} vertices, {} triangles, height [{:.1}, {:.1}]",
            vertices.len(),
            indices.len() / 3,
            min_h,
            max_h
        );
    }
}

#[godot_api]
impl TerrainChunk {
    /// GDScript 调用：查询 (x, z) 处地形高度
    #[func]
    fn query_height(&self, x: f64, z: f64) -> f32 {
        let pos = WorldPos { x, y: 0.0, z };
        self.terrain.height_at(pos)
    }
}

fn material_color(mat: woworld_core::material::SurfaceMaterial, height: f32) -> Color {
    use woworld_core::material::SurfaceMaterial::*;
    match mat {
        Water => {
            // 无真实水面时，水材质映射为湿地色
            if height < -20.0 {
                Color::from_rgb(0.3, 0.4, 0.5) // 深海蓝灰
            } else {
                Color::from_rgb(0.55, 0.6, 0.45) // 浅滩湿地绿
            }
        }
        Sand => Color::from_rgb(0.76, 0.7, 0.5),
        Grass => {
            if height > 150.0 {
                Color::from_rgb(0.25, 0.5, 0.2) // 高山暗绿
            } else if height > 50.0 {
                Color::from_rgb(0.3, 0.6, 0.25) // 丘陵
            } else {
                Color::from_rgb(0.35, 0.7, 0.3) // 低地鲜绿
            }
        }
        Rock => Color::from_rgb(0.5, 0.45, 0.4),
        Stone => Color::from_rgb(0.4, 0.4, 0.4),
        Gravel => Color::from_rgb(0.55, 0.5, 0.45),
        Snow => Color::from_rgb(0.95, 0.95, 0.95),
        Ice => Color::from_rgb(0.85, 0.9, 0.95),
        Mud => Color::from_rgb(0.45, 0.35, 0.25),
        _ => Color::from_rgb(0.4, 0.5, 0.3),
    }
}
