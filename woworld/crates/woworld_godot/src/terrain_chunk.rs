//! WorldDriver — 世界驱动 GodotClass
//!
//! 管理多分辨率 LOD 地形渲染 + 昼夜循环 + 大气合成。
//! 消费纯 Rust ClipmapManager（引擎无关），直接操控 Godot 节点。
//!
//! 架构：Rust 权威 → Godot 纯表现（GDScript 铁律 §14.1）

use godot::classes::light_3d::Param;
use godot::classes::mesh::PrimitiveType;
use std::sync::mpsc;

use godot::classes::{
    ArrayMesh, DirectionalLight3D, Image, MeshInstance3D, ProceduralSkyMaterial, Shader,
    ShaderMaterial, WorldEnvironment,
};
use godot::prelude::*;
use woworld_atmosphere::AtmosphereSynthesizer;
use woworld_core::prelude::*;
use woworld_core::spatial::TerrainQuery;
use std::collections::HashMap;

use woworld_core::material::SurfaceMaterial;
use woworld_core::ocean::OceanProvider;
use woworld_worldgen::{
    generate_clipmap_grid, generate_heightmap_data,
    layer_tex_config, level_spacing, load_material_colors, BiomeClassifier, HeightfieldOcean,
    HeightfieldTerrain, HeightmapData, NoiseParams, TerrainBaseDensity, WorldNoise, LEVELS,
    transvoxel_extract,
};

use crate::voxel_chunk::VoxelChunk;

// ── 缺省参数 ────────────────────
/// 默认每秒对应的游戏天数（30s/天方便观察，正式 3600s/天）
const DEFAULT_SECONDS_PER_DAY: f64 = 30.0;
/// DirectionalLight3D 距离世界原点的距离（米）
const SUN_ORBIT_RADIUS: f32 = 500.0;
/// 后台线程完成的 heightmap 数据
struct HeightmapJob {
    level_idx: u8,
    data: Vec<f32>,
    material_colors: Vec<[f32; 4]>,
    hm_size: u32,
    panicked: bool,
    /// 该高度图覆盖的世界空间原点（左下角）
    grid_origin_x: f64,
    grid_origin_z: f64,
}

/// 后台 Transvoxel 块提取描述（未来 rayon 后台使用）
#[allow(dead_code)]
struct VoxelJob {
    cx: i32,
    cz: i32,
    origin_x: f64,
    origin_y: f64,
    origin_z: f64,
}

#[allow(dead_code)]
struct VoxelResult {
    cx: i32,
    cz: i32,
    mesh: woworld_worldgen::TerrainMeshData,
}

/// 单个 LOD 级别的 GPU-driven clipmap 层
struct LodLayer {
    instance: Gd<MeshInstance3D>,
    /// 当前绑定到 shader 的 R32F heightmap 纹理（被 GPU 采样）
    heightmap_tex: Gd<godot::classes::ImageTexture>,
    /// RGBA8 材质色纹理（群系驱动，被 GPU 采样）
    material_tex: Gd<godot::classes::ImageTexture>,
    /// 邻层（coarse）高度图引用——L[n] 的 neighbor = L[n+1] 的 heightmap_tex
    /// L7 指向自身（blend_factor 永为 0，不采样）
    neighbor_heightmap_tex: Gd<godot::classes::ImageTexture>,
    /// 专用 ShaderMaterial
    material: Gd<ShaderMaterial>,
    /// 上次 snap 位置（避免冗余 set_global_position）
    last_snap: (f64, f64),
    /// 顶点间距
    spacing: f64,
    /// 该层 heightmap 纹理分辨率（像素，正方形）
    hm_size: u32,
    /// 当前高度图中心（世界坐标，用于 drift 检测）
    hm_center: (f64, f64),
    /// 高度图 job 正在生成中
    hm_in_flight: bool,
    /// 漂移余量（= hm_extent/2 - max_range - spacing）
    margin: f64,
    /// 当前 heightmap 的 grid_origin（用于下层 fine_grid_origin 引用）
    grid_origin: (f64, f64),
    /// Texture pool: standby Image + ImageTexture for double-buffered heightmap update
    hm_standby_img: Gd<godot::classes::Image>,
    hm_standby_tex: Gd<godot::classes::ImageTexture>,
    /// Texture pool: standby Image + ImageTexture for material map
    mat_standby_img: Gd<godot::classes::Image>,
    mat_standby_tex: Gd<godot::classes::ImageTexture>,
}

// ── GodotClass ────────────────────

#[derive(GodotClass)]
#[class(base = Node3D)]
pub struct WorldDriver {
    terrain: HeightfieldTerrain,
    ocean: HeightfieldOcean,
    clock: WorldClock,
    atmosphere: AtmosphereSynthesizer,

    /// 每个 LOD 级别一个 GPU-driven clipmap 层
    lod_layers: [Option<LodLayer>; 8],

    /// 后台 heightmap 生成 → 主线程收割
    hm_job_tx: mpsc::Sender<HeightmapJob>,
    hm_job_rx: mpsc::Receiver<HeightmapJob>,

    /// SurfaceMaterial → RGBA 色表（编译时嵌入，rayon job 共享）
    material_colors: HashMap<SurfaceMaterial, [f32; 4]>,

    // 缓存的 Godot 节点引用
    sun: Option<Gd<DirectionalLight3D>>,
    world_env: Option<Gd<WorldEnvironment>>,
    terrain_parent: Option<Gd<Node3D>>,
    player_node: Option<Gd<Node3D>>,
    ocean_mesh: Option<Gd<MeshInstance3D>>,

    // Transvoxel 3D 等值面块（LOD 0 替换）
    voxel_chunks: HashMap<(i32, i32), Gd<VoxelChunk>>,
    voxel_center: (i32, i32),

    #[base]
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for WorldDriver {
    fn init(base: Base<Node3D>) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            terrain: HeightfieldTerrain::default(),
            ocean: HeightfieldOcean::default(),
            clock: WorldClock::new(DEFAULT_SECONDS_PER_DAY),
            atmosphere: AtmosphereSynthesizer::from_embedded_toml()
                .expect("embedded time_curve.toml must be valid"),
            lod_layers: Default::default(),
            hm_job_tx: tx,
            hm_job_rx: rx,
            material_colors: HashMap::new(),
            sun: None,
            world_env: None,
            terrain_parent: None,
            player_node: None,
            ocean_mesh: None,
            voxel_chunks: HashMap::new(),
            voxel_center: (i32::MAX, i32::MAX),
            base,
        }
    }

    fn ready(&mut self) {
        use godot::classes::image::Format;
        use godot::builtin::PackedByteArray;
        use godot::classes::ImageTexture;

        // ── 1. 创建 HeightfieldTerrain（含群系）────
        let seed: u64 = 99;
        let params = NoiseParams::from_toml_str(include_str!(
            "../../../assets/noise_params.toml"
        ))
        .expect("noise_params.toml must be valid");
        let noise = WorldNoise::with_params(seed, params);
        let biome_toml = include_str!("../../../assets/biomes.toml");
        let biome_classifier = BiomeClassifier::from_toml_str(biome_toml, noise.clone())
            .expect("Failed to parse biomes.toml");
        // 海洋——与 terrain 共享同一个 Arc<WorldNoise>
        self.ocean = HeightfieldOcean::new(noise.clone());
        let clock = WorldClock::new(DEFAULT_SECONDS_PER_DAY);
        self.terrain = HeightfieldTerrain::with_noise(noise, seed)
            .with_clock(clock.clone())
            .with_biomes(biome_classifier);
        self.clock = clock;

        // 加载材质色表（编译时嵌入——委托 woworld_worldgen 解析）
        self.material_colors = load_material_colors(include_str!(
            "../../../assets/material_colors.toml"
        ))
        .expect("material_colors.toml must be valid");

        // ── 2. 缓存场景节点引用 ──────────
        self.sun = self.base().try_get_node_as::<DirectionalLight3D>("../Sun");
        self.world_env = self
            .base()
            .try_get_node_as::<WorldEnvironment>("../WorldEnvironment");
        self.terrain_parent = Some(self.base().clone().cast::<Node3D>());
        self.player_node = self.base().try_get_node_as::<Node3D>("../Player");
        self.ocean_mesh = self
            .base()
            .try_get_node_as::<Node3D>("../OceanPlane")
            .and_then(|ocean| ocean.get_node_or_null("OceanPlane"))
            .and_then(|n| n.try_cast::<MeshInstance3D>().ok());

        // 加载 terrain.gdshader（全部 8 层共享）
        use godot::classes::ResourceLoader;
        let mut loader = ResourceLoader::singleton();
        let terrain_shader = loader
            .load("res://shaders/terrain.gdshader")
            .expect("Failed to load terrain.gdshader from res://shaders/")
            .cast::<Shader>();

        // ── 3. 为每个 LOD 层级创建 GPU-driven clipmap ──
        for level_idx in 0..8u8 {
            let level = &LEVELS[level_idx as usize];
            let spacing = level_spacing(level);
            let tex_cfg = layer_tex_config(level);
            let hm_size = tex_cfg.hm_size;
            let hm_extent = tex_cfg.hm_extent as f32;

            let grid = generate_clipmap_grid(level);
            let grid_n = (grid.vertices.len() as f64).sqrt() as u32;

            // 3b. 创建 Godot ArrayMesh（静态——不再修改）
            let mut am = ArrayMesh::new_gd();
            Self::upload_static_mesh(&mut am, &grid);
            godot_print!(
                "  LOD {}: {}×{} vertices, {} tris (hm: {}², margin: {:.0}m)",
                level_idx,
                grid_n, grid_n,
                grid.indices.len() / 3,
                hm_size,
                hm_extent as f64 / 2.0 - level.max_range - spacing,
            );

            // 3c. 创建 R32F heightmap 纹理（按层分辨率）
            let mut img = Image::create(
                hm_size as i32, hm_size as i32, false, Format::RF,
            )
            .expect("Image::create for heightmap");
            let mut bytes = PackedByteArray::new();
            for _ in 0..(hm_size * hm_size) {
                for &b in &0.0f32.to_le_bytes() {
                    bytes.push(b);
                }
            }
            img.set_data(hm_size as i32, hm_size as i32, false, Format::RF, &bytes);
            let ht_tex = ImageTexture::create_from_image(&img)
                .expect("ImageTexture::create_from_image for heightmap");

            // 3c-bis. 创建 RGBA8 材质纹理（初始化为沙色）
            let mut mat_img = Image::create(
                hm_size as i32, hm_size as i32, false, Format::RGBA8,
            )
            .expect("Image::create for material map");
            let mut mat_bytes = PackedByteArray::new();
            for _ in 0..(hm_size * hm_size) {
                for &b in &[191u8, 178, 128, 255] {
                    // sand #BFB280
                    mat_bytes.push(b);
                }
            }
            mat_img.set_data(hm_size as i32, hm_size as i32, false, Format::RGBA8, &mat_bytes);
            let mt_tex = ImageTexture::create_from_image(&mat_img)
                .expect("ImageTexture::create_from_image for material map");

            // 3d. 创建 ShaderMaterial（全部 8 层共享同一个 .gdshader）
            let mut mat = ShaderMaterial::new_gd();
            mat.set_shader(&terrain_shader);
            mat.set_shader_parameter(
                &StringName::from("heightmap"),
                &ht_tex.to_variant(),
            );
            mat.set_shader_parameter(
                &StringName::from("material_map"),
                &mt_tex.to_variant(),
            );
            mat.set_shader_parameter(
                &StringName::from("hm_extent"),
                &Variant::from(hm_extent),
            );
            mat.set_shader_parameter(
                &StringName::from("grid_origin"),
                &Variant::from(Vector2::new(0.0, 0.0)),
            );

            // 3e. 创建 MeshInstance3D
            let mut mi = MeshInstance3D::new_alloc();
            mi.set_name(&format!("LOD_{}", level_idx));
            mi.set_mesh(&am);
            mi.set_extra_cull_margin(10000.0);
            if let Some(ref mut parent) = self.terrain_parent {
                parent.add_child(&mi.clone().upcast::<Node>());
            }
            mi.set_surface_override_material(0, &mat);

            // 3f. Texture pool: create standby Image + ImageTexture (double-buffering)
            let mut hm_standby_img = Image::create(
                hm_size as i32, hm_size as i32, false, Format::RF,
            ).expect("Image::create for standby heightmap");
            hm_standby_img.set_data(hm_size as i32, hm_size as i32, false, Format::RF, &bytes);
            let hm_standby_tex = ImageTexture::create_from_image(&hm_standby_img)
                .expect("ImageTexture for standby heightmap");

            let mut mat_standby_img = Image::create(
                hm_size as i32, hm_size as i32, false, Format::RGBA8,
            ).expect("Image::create for standby material map");
            mat_standby_img.set_data(hm_size as i32, hm_size as i32, false, Format::RGBA8, &mat_bytes);
            let mat_standby_tex = ImageTexture::create_from_image(&mat_standby_img)
                .expect("ImageTexture for standby material map");

            self.lod_layers[level_idx as usize] = Some(LodLayer {
                instance: mi,
                heightmap_tex: ht_tex.clone(),
                material_tex: mt_tex,
                neighbor_heightmap_tex: ht_tex, // placeholder — updated below
                material: mat,
                last_snap: (f64::MAX, f64::MAX),
                spacing,
                hm_size,
                hm_center: (f64::MAX, f64::MAX),
                hm_in_flight: false,
                margin: tex_cfg.hm_extent / 2.0 - level.max_range - spacing,
                grid_origin: (0.0, 0.0),
                hm_standby_img,
                hm_standby_tex,
                mat_standby_img,
                mat_standby_tex,
            });
        }

        // ── 4. 设置细层交叉采样 uniform（L1-L7 引用 L0-L6 的 heightmap）──
        for level_idx in 1..8u8 {
            let (left, right) = self.lod_layers.split_at_mut(level_idx as usize);
            let finer = left[level_idx as usize - 1].as_ref().unwrap();
            let layer = right[0].as_mut().unwrap();
            let fine_level = &LEVELS[level_idx as usize - 1];
            let fine_spacing = level_spacing(fine_level);
            let inner_bound = LEVELS[level_idx as usize].min_range as f32;
            let blend_zone = (5.0 * fine_spacing) as f32;

            layer.material.set_shader_parameter(
                &StringName::from("fine_heightmap"),
                &finer.heightmap_tex.to_variant(),
            );
            layer.material.set_shader_parameter(
                &StringName::from("fine_grid_origin"),
                &Variant::from(Vector2::new(0.0, 0.0)),
            );
            layer.material.set_shader_parameter(
                &StringName::from("fine_hm_extent"),
                &Variant::from(layer_tex_config(fine_level).hm_extent as f32),
            );
            layer.material.set_shader_parameter(
                &StringName::from("inner_bound"),
                &Variant::from(inner_bound),
            );
            layer.material.set_shader_parameter(
                &StringName::from("blend_zone"),
                &Variant::from(blend_zone),
            );
        }
        // L0: 无更细层，设 inner_bound=-1 禁用内边界 morphing
        //     外边界 morphing: L0 → L1（Hoppe w=n/10）
        {
            let (l0_slice, l1_slice) = self.lod_layers.split_at_mut(1);
            let l0 = l0_slice[0].as_mut().unwrap();
            let l1 = l1_slice[0].as_ref().unwrap();
            l0.material.set_shader_parameter(
                &StringName::from("fine_heightmap"),
                &l0.heightmap_tex.to_variant(),
            );
            l0.material.set_shader_parameter(
                &StringName::from("inner_bound"),
                &Variant::from(-1.0f32),
            );
            let l0_level = &LEVELS[0];
            let l1_level = &LEVELS[1];
            let n = (2.0 * l0_level.max_range / level_spacing(l0_level)).ceil() as u32 + 1;
            let outer_w = (n as f32 / 10.0) * level_spacing(l0_level) as f32;
            l0.material.set_shader_parameter(
                &StringName::from("coarse_heightmap"),
                &l1.heightmap_tex.to_variant(),
            );
            l0.material.set_shader_parameter(
                &StringName::from("coarse_grid_origin"),
                &Variant::from(Vector2::new(0.0, 0.0)),
            );
            l0.material.set_shader_parameter(
                &StringName::from("coarse_hm_extent"),
                &Variant::from(layer_tex_config(l1_level).hm_extent as f32),
            );
            l0.material.set_shader_parameter(
                &StringName::from("outer_bound"),
                &Variant::from(l0_level.max_range as f32),
            );
            l0.material.set_shader_parameter(
                &StringName::from("outer_blend_zone"),
                &Variant::from(outer_w),
            );
        }

        // ── 5. 外边界 coarse 交叉采样（L1-L6 引用 L2-L7 的 heightmap）──
        for level_idx in 1..7u8 {
            let (left, right) = self.lod_layers.split_at_mut(level_idx as usize + 1);
            let coarser = right[0].as_ref().unwrap();
            let layer = left[level_idx as usize].as_mut().unwrap();
            let cur_level = &LEVELS[level_idx as usize];
            let next_level = &LEVELS[level_idx as usize + 1];
            let n = (2.0 * cur_level.max_range / level_spacing(cur_level)).ceil() as u32 + 1;
            let outer_w = (n as f32 / 10.0) * level_spacing(cur_level) as f32;

            layer.material.set_shader_parameter(
                &StringName::from("coarse_heightmap"),
                &coarser.heightmap_tex.to_variant(),
            );
            layer.material.set_shader_parameter(
                &StringName::from("coarse_grid_origin"),
                &Variant::from(Vector2::new(0.0, 0.0)),
            );
            layer.material.set_shader_parameter(
                &StringName::from("coarse_hm_extent"),
                &Variant::from(layer_tex_config(next_level).hm_extent as f32),
            );
            layer.material.set_shader_parameter(
                &StringName::from("outer_bound"),
                &Variant::from(cur_level.max_range as f32),
            );
            layer.material.set_shader_parameter(
                &StringName::from("outer_blend_zone"),
                &Variant::from(outer_w),
            );
        }
        // L7: 无更粗层，设 outer_bound=99999 禁用外边界 morphing
        if let Some(ref mut l7) = self.lod_layers[7] {
            l7.material.set_shader_parameter(
                &StringName::from("coarse_heightmap"),
                &l7.heightmap_tex.to_variant(),
            );
            l7.material.set_shader_parameter(
                &StringName::from("outer_bound"),
                &Variant::from(99999.0f32),
            );
        }

        // ── 6. 邻居高度图引用（L[n] → L[n+1], L7 → 自身）──
        for i in 0..7u8 {
            let (left, right) = self.lod_layers.split_at_mut(i as usize + 1);
            let neighbor_tex = right[0].as_ref().unwrap().heightmap_tex.clone();
            if let Some(ref mut layer) = left[i as usize] {
                layer.neighbor_heightmap_tex = neighbor_tex;
            }
        }
        if let Some(ref mut l7) = self.lod_layers[7] {
            l7.neighbor_heightmap_tex = l7.heightmap_tex.clone();
        }

        // ── 7. Voxel chunk MVP: single chunk at origin ──
        self.spawn_voxel_chunk(0.0, 0.0);

        self.base_mut().set_process(true);
        godot_print!("WorldDriver: 8 GPU-driven clipmap layers + 1 VoxelChunk ready");
    }

    fn process(&mut self, delta: f64) {
        self.clock.advance(delta);
        let wt = &self.clock.current;
        self.terrain.clock = Some(self.clock.clone());

        // 天空/太阳
        {
            let origin = WorldPos { x: 0.0, y: 0.0, z: 0.0 };
            let atm = self.atmosphere.resolve(wt, origin);
            self.update_sun_and_sky(&atm);
        }

        let player_pos = self.get_player_position();
        let px = player_pos.x as f32;
        let pz = player_pos.z as f32;
        let py = player_pos.y as f32;

        // ── 收割后台完成的 heightmap job ──
        // 纹理池双缓冲：每层预分配 2 套 Image+ImageTexture，harvest 用 update() 原地更新
        // 帧预算：每帧最多 1 次 harvest（削平多 job 同时完成产生的尖峰）
        use godot::classes::image::Format;
        use std::mem;
        let mut uploaded_this_frame = 0u32;
        const MAX_UPLOADS_PER_FRAME: u32 = 1;
        while uploaded_this_frame < MAX_UPLOADS_PER_FRAME {
            let job = match self.hm_job_rx.try_recv() {
                Ok(j) => j,
                Err(_) => break,
            };
            let idx = job.level_idx as usize;
            if let Some(ref mut layer) = self.lod_layers[idx] {
                if !job.data.is_empty() {
                    let size = job.hm_size as i32;

                    // Heightmap: pack → standby Image → update standby Texture → swap
                    let mut hm_bytes = PackedByteArray::new();
                    for &h in &job.data {
                        for &b in &h.to_le_bytes() {
                            hm_bytes.push(b);
                        }
                    }
                    layer.hm_standby_img.set_data(size, size, false, Format::RF, &hm_bytes);
                    layer.hm_standby_tex.update(&layer.hm_standby_img);
                    mem::swap(&mut layer.heightmap_tex, &mut layer.hm_standby_tex);

                    // Material map: pack → standby Image → update standby Texture → swap
                    let mut mat_bytes = PackedByteArray::new();
                    for &[r, g, b, a] in &job.material_colors {
                        for &v in &[
                            (r * 255.0) as u8,
                            (g * 255.0) as u8,
                            (b * 255.0) as u8,
                            (a * 255.0) as u8,
                        ] {
                            mat_bytes.push(v);
                        }
                    }
                    layer.mat_standby_img.set_data(size, size, false, Format::RGBA8, &mat_bytes);
                    layer.mat_standby_tex.update(&layer.mat_standby_img);
                    mem::swap(&mut layer.material_tex, &mut layer.mat_standby_tex);

                    // Bind the now-current textures (swapped in above) to shader
                    layer.material.set_shader_parameter(
                        &StringName::from("heightmap"),
                        &layer.heightmap_tex.to_variant(),
                    );
                    layer.material.set_shader_parameter(
                        &StringName::from("grid_origin"),
                        &Variant::from(Vector2::new(
                            job.grid_origin_x as f32,
                            job.grid_origin_z as f32,
                        )),
                    );
                    layer.grid_origin = (job.grid_origin_x, job.grid_origin_z);
                    layer.material.set_shader_parameter(
                        &StringName::from("material_map"),
                        &layer.material_tex.to_variant(),
                    );

                    // Update heightmap center
                    let half = job.hm_size as f64 * layer.spacing * 0.5;
                    layer.hm_center = (job.grid_origin_x + half, job.grid_origin_z + half);
                } else if job.panicked {
                    godot_error!("LOD {} heightmap job panicked", job.level_idx);
                }
                layer.hm_in_flight = false;
                uploaded_this_frame += 1;
            }
        }

        // ── GPU-Driven Clipmap: recenter + per-layer async heightmap jobs ──
        for level_idx in 0..8u8 {
            if let Some(ref mut layer) = self.lod_layers[level_idx as usize] {
                let spacing = layer.spacing;

                // Grid recentering (only when snap position changed)
                let snap_x = (player_pos.x / spacing).floor() * spacing;
                let snap_z = (player_pos.z / spacing).floor() * spacing;
                let snap = (snap_x, snap_z);
                let snap_changed = snap != layer.last_snap;
                if snap_changed {
                    layer.last_snap = snap;
                    layer.instance.set_global_position(Vector3::new(
                        snap_x as f32, 0.0, snap_z as f32,
                    ));
                }

                // Shader uniforms
                layer.material.set_shader_parameter(
                    &StringName::from("camera_pos"),
                    &Variant::from(Vector3::new(px, py, pz)),
                );
                if snap_changed {
                    layer.material.set_shader_parameter(
                        &StringName::from("node_pos"),
                        &Variant::from(Vector3::new(snap_x as f32, 0.0, snap_z as f32)),
                    );
                }

                // 高度图更新：该层漂移超过余量时提交异步 job
                let drift = (snap_x - layer.hm_center.0)
                    .abs()
                    .max((snap_z - layer.hm_center.1).abs());
                if drift > layer.margin && !layer.hm_in_flight {
                    layer.hm_in_flight = true;
                    let tx = self.hm_job_tx.clone();
                    let terrain = self.terrain.clone();
                    let mc = self.material_colors.clone();
                    let hm_size = layer.hm_size;
                    let half = hm_size as f64 * spacing * 0.5;
                    let grid_origin_x = snap_x - half;
                    let grid_origin_z = snap_z - half;
                    rayon::spawn(move || {
                        let result =
                            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                generate_heightmap_data(
                                    &terrain, snap_x, snap_z, spacing, hm_size, &mc,
                                )
                            }));
                        let (hd, panicked) = match result {
                            Ok(hd) => (hd, false),
                            Err(_) => {
                                eprintln!("HM PANIC: L{}", level_idx);
                                (
                                    HeightmapData {
                                        heights: Vec::new(),
                                        material_colors: Vec::new(),
                                    },
                                    true,
                                )
                            }
                        };
                        let _ = tx.send(HeightmapJob {
                            level_idx,
                            data: hd.heights,
                            material_colors: hd.material_colors,
                            hm_size,
                            panicked,
                            grid_origin_x,
                            grid_origin_z,
                        });
                    });
                }
            }
        }

        // ── 细层交叉采样同步：L1-L7 的 fine_heightmap/grid_origin 跟踪 L0-L6 ──
        for level_idx in 1..8u8 {
            let (left, right) = self.lod_layers.split_at_mut(level_idx as usize);
            if let (Some(ref finer), Some(ref mut coarser)) =
                (&left[level_idx as usize - 1], &mut right[0])
            {
                // 纹理引用：每次 swap 后 old texture 被 Godot 回收 → 必须每帧刷新
                coarser.material.set_shader_parameter(
                    &StringName::from("fine_heightmap"),
                    &finer.heightmap_tex.to_variant(),
                );
                let go = finer.grid_origin;
                coarser.material.set_shader_parameter(
                    &StringName::from("fine_grid_origin"),
                    &Variant::from(Vector2::new(go.0 as f32, go.1 as f32)),
                );
            }
        }

        // ── 粗层交叉采样同步：L0-L6 的 coarse_heightmap/grid_origin 跟踪 L1-L7 ──
        for level_idx in 0..7u8 {
            let (left, right) = self.lod_layers.split_at_mut(level_idx as usize + 1);
            if let (Some(ref mut layer), Some(ref coarser)) =
                (&mut left[level_idx as usize], &right[0])
            {
                layer.material.set_shader_parameter(
                    &StringName::from("coarse_heightmap"),
                    &coarser.heightmap_tex.to_variant(),
                );
                let go = coarser.grid_origin;
                layer.material.set_shader_parameter(
                    &StringName::from("coarse_grid_origin"),
                    &Variant::from(Vector2::new(go.0 as f32, go.1 as f32)),
                );
            }
        }

        // ── 水下 Environment 参数调制 ──
        // 摄像机入水 → 插值雾色/环境光/饱和度（2m 过渡带，无硬切）
        {
            let time = self.clock.current.day_number as f64 * self.clock.seconds_per_day
                + self.clock.current.day_progress * self.clock.seconds_per_day;
            let underwater = self.ocean.is_underwater(player_pos, time);
            let submersion = if underwater {
                (self.ocean.sea_level_at(player_pos) - player_pos.y).max(0.0)
            } else {
                0.0
            };
            let t = (submersion / 2.0).clamp(0.0, 1.0) as f32;

            fn lerp_f32(a: f32, b: f32, t: f32) -> f32 {
                a + (b - a) * t
            }

            if let Some(ref world_env) = self.world_env {
                if let Some(mut env) = world_env.get_environment() {
                    if t > 0.0 {
                        // 水下——蓝绿色雾 + 偏蓝环境光
                        env.set_fog_enabled(true);
                        env.set_fog_density(lerp_f32(0.001, 0.04, t));
                        env.set_fog_light_color(Color::from_rgb(
                            lerp_f32(0.8, 0.15, t),
                            lerp_f32(0.85, 0.35, t),
                            lerp_f32(0.7, 0.55, t),
                        ));
                        env.set_ambient_light_color(Color::from_rgb(
                            lerp_f32(0.5, 0.1, t),
                            lerp_f32(0.5, 0.25, t),
                            lerp_f32(0.5, 0.4, t),
                        ));
                        env.set_adjustment_enabled(true);
                        env.set_adjustment_saturation(lerp_f32(1.0, 0.7, t));
                    } else {
                        // 水上——关雾，让 update_sun_and_sky 的昼夜 ambient 生效
                        env.set_fog_enabled(false);
                        env.set_adjustment_enabled(false);
                    }
                }
            }
        }
    }
}

// ── 内部方法 ──────────────────────────

impl WorldDriver {
    /// 读取玩家位置
    fn get_player_position(&self) -> WorldPos {
        if let Some(ref player) = self.player_node {
            let pos = player.get_global_position();
            WorldPos {
                x: pos.x as f64,
                y: pos.y as f64,
                z: pos.z as f64,
            }
        } else {
            WorldPos::default()
        }
    }

    /// 将静态网格上传到 ArrayMesh（仅启动时调用一次）
    fn upload_static_mesh(am: &mut Gd<ArrayMesh>, mesh: &woworld_worldgen::TerrainMeshData) {
        use godot::builtin::PackedVector3Array;
        use godot::builtin::PackedColorArray;
        use godot::builtin::PackedInt32Array;
        use godot::builtin::Array;

        let mut vertices = PackedVector3Array::new();
        let mut normals = PackedVector3Array::new();
        let mut colors = PackedColorArray::new();

        for i in 0..mesh.vertices.len() {
            let v = mesh.vertices[i];
            vertices.push(Vector3::new(v.x, v.y, v.z));
            normals.push(Vector3::new(mesh.normals[i].x, mesh.normals[i].y, mesh.normals[i].z));
            let c = mesh.colors[i];
            colors.push(Color::from_rgb(c.x, c.y, c.z));
        }

        let mut indices = PackedInt32Array::new();
        for idx in &mesh.indices {
            indices.push(*idx as i32);
        }

        let mut arrays = Array::new();
        arrays.resize(13, &Variant::nil());
        arrays.set(0, &vertices.to_variant());
        arrays.set(1, &normals.to_variant());
        arrays.set(3, &colors.to_variant());
        arrays.set(12, &indices.to_variant());

        am.clear_surfaces();
        am.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);
    }

    /// 太阳位置 + 天空材质 + 环境光
    fn update_sun_and_sky(&mut self, atm: &woworld_atmosphere::ResolvedAtmosphere) {
        let wt = &self.clock.current;

        if let Some(ref mut sun) = self.sun {
            let radius = SUN_ORBIT_RADIUS;
            let elev = wt.sun_elevation as f32;
            let azim = wt.sun_azimuth as f32;
            let sun_pos = Vector3::new(
                elev.cos() * azim.sin() * radius,
                elev.sin() * radius,
                elev.cos() * azim.cos() * radius,
            );
            sun.set_position(sun_pos);
            sun.look_at(Vector3::ZERO);
            sun.set_color(Color::from_rgb(
                atm.sun_color[0],
                atm.sun_color[1],
                atm.sun_color[2],
            ));
            sun.set_param(Param::ENERGY, atm.sun_energy);
        }

        if let Some(ref world_env) = self.world_env {
            if let Some(mut env_res) = world_env.get_environment() {
                if let Some(sky) = env_res.get_sky() {
                    if let Some(mat) = sky.get_material() {
                        if let Ok(mut proc_sky) = mat.try_cast::<ProceduralSkyMaterial>() {
                            proc_sky.set_sky_top_color(Color::from_rgb(
                                atm.sky_zenith_color[0],
                                atm.sky_zenith_color[1],
                                atm.sky_zenith_color[2],
                            ));
                            proc_sky.set_sky_horizon_color(Color::from_rgb(
                                atm.sky_horizon_color[0],
                                atm.sky_horizon_color[1],
                                atm.sky_horizon_color[2],
                            ));
                            proc_sky.set_ground_horizon_color(Color::from_rgb(
                                atm.ground_horizon[0],
                                atm.ground_horizon[1],
                                atm.ground_horizon[2],
                            ));
                            proc_sky.set_sun_angle_max(5.0);
                            proc_sky.set_sun_curve(0.5);
                        }
                    }
                }
                env_res.set_ambient_light_color(Color::from_rgb(
                    atm.ambient_color[0],
                    atm.ambient_color[1],
                    atm.ambient_color[2],
                ));
            }
        }
    }
}

/// GDScript 接口 ──────────────────────

#[godot_api]
impl WorldDriver {
    /// GDScript 查询：(x, z) 处地形高度
    #[func]
    fn query_height(&self, x: f64, z: f64) -> f32 {
        let pos = WorldPos { x, y: 0.0, z };
        self.terrain.height_at(pos)
    }

    /// Create a single VoxelChunk, extract terrain mesh, and upload it (synchronous MVP).
    /// `wx, wz` = chunk world-space southwest corner (x, z). y origin auto-computed.
    fn spawn_voxel_chunk(&mut self, wx: f64, wz: f64) {
        use godot::builtin::{Array, PackedColorArray, PackedInt32Array, PackedVector3Array};
        use godot::classes::mesh::PrimitiveType;
        use godot::classes::ArrayMesh;

        let voxel_size = 0.5f64;
        let vx = 32u32;
        let vy = 32u32;
        let vz = 32u32;
        let chunk_size = voxel_size * vx as f64; // 16m

        // Sample terrain height at chunk center for y-origin
        let h = self.terrain.height_at(WorldPos {
            x: wx + chunk_size * 0.5,
            y: 0.0,
            z: wz + chunk_size * 0.5,
        });
        let wy = (h as f64 - chunk_size * 0.5).max(0.0);

        // Create VoxelChunk node
        let mut vc = VoxelChunk::new_alloc();
        vc.bind_mut().set_world_origin(wx, wy, wz);
        if let Some(ref mut parent) = self.terrain_parent {
            parent.add_child(&vc.clone().upcast::<Node>());
        }

        // Extract terrain mesh (synchronous)
        let noise_arc = self.terrain.noise_arc();
        let base_layer = TerrainBaseDensity::new(noise_arc);
        let stack = self.terrain.density_stack().clone();

        let mesh = transvoxel_extract(
            &stack, &base_layer,
            wx, wy, wz,
            vx, vy, vz, voxel_size,
            0, // no transition faces
            &self.material_colors,
        );

        if !mesh.vertices.is_empty() {
            let mut am = ArrayMesh::new_gd();
            let mut vertices = PackedVector3Array::new();
            let mut normals_packed = PackedVector3Array::new();
            let mut colors_packed = PackedColorArray::new();

            for i in 0..mesh.vertices.len() {
                let v = mesh.vertices[i];
                vertices.push(Vector3::new(v.x, v.y, v.z));
                let n = mesh.normals[i];
                normals_packed.push(Vector3::new(n.x, n.y, n.z));
                let c = mesh.colors[i];
                colors_packed.push(Color::from_rgb(c.x, c.y, c.z));
            }

            let mut indices = PackedInt32Array::new();
            for idx in &mesh.indices {
                indices.push(*idx as i32);
            }

            let mut arrays = Array::new();
            arrays.resize(13, &Variant::nil());
            arrays.set(0, &vertices.to_variant());
            arrays.set(1, &normals_packed.to_variant());
            arrays.set(3, &colors_packed.to_variant());
            arrays.set(12, &indices.to_variant());

            am.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);

            godot_print!(
                "VoxelChunk @ ({:.0},{:.0},{:.0}): {} verts, {} tris",
                wx, wy, wz,
                mesh.vertices.len(),
                mesh.indices.len() / 3,
            );

            vc.bind_mut().set_terrain_mesh(Some(am));
        } else {
            vc.bind_mut().set_terrain_mesh(None);
        }

        let cx = (wx / chunk_size).floor() as i32;
        let cz = (wz / chunk_size).floor() as i32;
        self.voxel_chunks.insert((cx, cz), vc);
        self.voxel_center = (cx, cz);
    }
}
