//! TerrainChunk — Godot Node3D GDExtension 类
//!
//! 在 Rust 侧生成地形网格，通过 GDExtension 直接构建 Godot ArrayMesh。
//! 管理 WorldClock（昼夜）和 BiomeClassifier（群系）。

use std::cell::RefCell;

use godot::classes::base_material_3d::{CullMode, Flags, ShadingMode};
use godot::classes::mesh::PrimitiveType;
use godot::classes::{ArrayMesh, MeshInstance3D, StandardMaterial3D};
use godot::prelude::*;
use woworld_core::prelude::WorldPos;
use woworld_core::spatial::TerrainQuery;
use woworld_worldgen::{BiomeClassifier, HeightfieldTerrain, WorldClock, WorldNoise};

#[derive(GodotClass)]
#[class(base = Node3D, init)]
pub struct TerrainChunk {
    terrain: HeightfieldTerrain,
    /// RefCell: Godot 的 #[func] 方法只能 &self，用内部可变性推进时钟
    clock: RefCell<WorldClock>,

    #[base]
    base: Base<Node3D>,
}

// ── 昼夜渲染参数（关键帧色板） ──────────────

/// 天空顶色关键帧：dawn / noon / dusk / midnight
const SKY_TOP_DAWN: (f32, f32, f32) = (0.7, 0.4, 0.3);
const SKY_TOP_NOON: (f32, f32, f32) = (0.3, 0.5, 0.9);
const SKY_TOP_DUSK: (f32, f32, f32) = (0.8, 0.35, 0.2);
const SKY_TOP_NIGHT: (f32, f32, f32) = (0.05, 0.05, 0.15);

/// 天空地平线色关键帧
const SKY_HORIZON_DAWN: (f32, f32, f32) = (0.9, 0.6, 0.3);
const SKY_HORIZON_NOON: (f32, f32, f32) = (0.6, 0.7, 0.9);
const SKY_HORIZON_DUSK: (f32, f32, f32) = (0.95, 0.5, 0.15);
const SKY_HORIZON_NIGHT: (f32, f32, f32) = (0.02, 0.02, 0.06);

/// 环境光关键帧
const AMBIENT_DAWN: (f32, f32, f32) = (0.4, 0.3, 0.25);
const AMBIENT_NOON: (f32, f32, f32) = (0.6, 0.6, 0.65);
const AMBIENT_DUSK: (f32, f32, f32) = (0.45, 0.3, 0.2);
const AMBIENT_NIGHT: (f32, f32, f32) = (0.04, 0.04, 0.08);

#[godot_api]
impl INode3D for TerrainChunk {
    fn ready(&mut self) {
        use woworld_worldgen::NoiseParams;
        let seed: u32 = 42;
        let params = NoiseParams {
            continent_scale: 0.001,
            detail_scale: 0.02,
            mountain_scale: 0.002,
            sea_threshold: -0.5,
            height_amplitude: 80.0,
            sea_depth: 30.0,
            climate_scale: 0.005,
        };
        let noise = WorldNoise::with_params(seed, params);

        // 加载群系
        let biome_toml = include_str!("../../../assets/biomes.toml");
        let biome_classifier = BiomeClassifier::from_toml_str(biome_toml, noise.clone())
            .expect("Failed to parse biomes.toml");

        // 构建地形（带昼夜时钟 + 群系）
        let clock = WorldClock::new(60.0); // 60s/天用于测试
        self.terrain = HeightfieldTerrain::with_noise(noise)
            .with_clock(clock.clone())
            .with_biomes(biome_classifier);
        self.clock = RefCell::new(clock);

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
    /// GDScript 每帧调用：推进世界时钟（用 &self + RefCell 因为 godot-rust #[func] 不支持 &mut self）
    #[func]
    fn advance_time(&self, delta: f64) {
        self.clock.borrow_mut().advance(delta);
    }

    /// GDScript 调用：查询 (x, z) 处地形高度
    #[func]
    fn query_height(&self, x: f64, z: f64) -> f32 {
        let pos = WorldPos { x, y: 0.0, z };
        self.terrain.height_at(pos)
    }

    /// 太阳在天空中的位置（世界坐标）
    #[func]
    fn get_sun_position(&self) -> Vector3 {
        let clock = self.clock.borrow();
        let wt = &clock.current;
        let radius = clock.sun_orbit_radius;
        let elev = wt.sun_elevation as f32;
        let azim = wt.sun_azimuth as f32;

        Vector3::new(
            elev.cos() * azim.sin() * radius,
            elev.sin() * radius,
            elev.cos() * azim.cos() * radius,
        )
    }

    /// 天空顶色（根据当前时间段 lerp）
    #[func]
    fn get_sky_top_color(&self) -> Color {
        let dp = self.clock.borrow().current.day_progress as f32;
        sky_color_lerp(dp, SKY_TOP_DAWN, SKY_TOP_NOON, SKY_TOP_DUSK, SKY_TOP_NIGHT)
    }

    /// 天空地平线色
    #[func]
    fn get_sky_horizon_color(&self) -> Color {
        let dp = self.clock.borrow().current.day_progress as f32;
        sky_color_lerp(
            dp,
            SKY_HORIZON_DAWN,
            SKY_HORIZON_NOON,
            SKY_HORIZON_DUSK,
            SKY_HORIZON_NIGHT,
        )
    }

    /// 环境光色
    #[func]
    fn get_ambient_light(&self) -> Color {
        let dp = self.clock.borrow().current.day_progress as f32;
        sky_color_lerp(dp, AMBIENT_DAWN, AMBIENT_NOON, AMBIENT_DUSK, AMBIENT_NIGHT)
    }
}

/// 四关键帧色板 lerp：dawn(0.25) → noon(0.5) → dusk(0.75) → night(0.0/1.0)
fn sky_color_lerp(
    day_progress: f32,
    dawn: (f32, f32, f32),
    noon: (f32, f32, f32),
    dusk: (f32, f32, f32),
    night: (f32, f32, f32),
) -> Color {
    let dp = day_progress;
    let (c1, c2, t) = if dp < 0.25 {
        // Night → Dawn
        (rgb(night), rgb(dawn), (dp + 0.75) / 0.25) // dp=0 → t=0 (night), dp=0.25 → t=1 (dawn)
    } else if dp < 0.5 {
        // Dawn → Noon
        (rgb(dawn), rgb(noon), (dp - 0.25) / 0.25)
    } else if dp < 0.75 {
        // Noon → Dusk
        (rgb(noon), rgb(dusk), (dp - 0.5) / 0.25)
    } else {
        // Dusk → Night
        (rgb(dusk), rgb(night), (dp - 0.75) / 0.25)
    };
    let t = t.clamp(0.0, 1.0);
    Color::from_rgb(
        c1.0 + (c2.0 - c1.0) * t,
        c1.1 + (c2.1 - c1.1) * t,
        c1.2 + (c2.2 - c1.2) * t,
    )
}

fn rgb(c: (f32, f32, f32)) -> (f32, f32, f32) {
    c
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
