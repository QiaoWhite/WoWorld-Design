//! WorldDriver — 世界驱动 GodotClass
//!
//! 管理多分辨率 LOD 地形渲染 + 昼夜循环 + 大气合成。
//! 消费纯 Rust ClipmapManager（引擎无关），直接操控 Godot 节点。
//!
//! 架构：Rust 权威 → Godot 纯表现（GDScript 铁律 §14.1）

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use godot::classes::light_3d::Param;
use godot::classes::mesh::PrimitiveType;
use godot::classes::{
    ArrayMesh, DirectionalLight3D, MeshInstance3D, ProceduralSkyMaterial, Shader, ShaderMaterial,
    WorldEnvironment,
};
use godot::prelude::*;
use woworld_atmosphere::AtmosphereSynthesizer;
use woworld_core::prelude::*;
use woworld_core::spatial::TerrainQuery;
use woworld_worldgen::{
    BiomeClassifier, ClipmapManager, HeightfieldTerrain, LodKey, NoiseParams, TerrainMeshData,
    TileEvent, WorldNoise,
};

// ── 缺省参数 ────────────────────
/// 默认每秒对应的游戏天数（30s/天方便观察，正式 3600s/天）
const DEFAULT_SECONDS_PER_DAY: f64 = 30.0;
/// DirectionalLight3D 距离世界原点的距离（米）
const SUN_ORBIT_RADIUS: f32 = 500.0;

/// 单个 LOD 级别的合并 mesh 状态
struct LodLayer {
    #[allow(dead_code)]
    instance: Gd<MeshInstance3D>,
    mesh: Gd<ArrayMesh>,
    active_keys: HashSet<LodKey>,
    completed: HashMap<LodKey, TerrainMeshData>,
    dirty: bool,  // active_keys 或 completed 有变化 → 需要重新合并上传
}

// ── GodotClass ────────────────────

#[derive(GodotClass)]
#[class(base = Node3D)]
pub struct WorldDriver {
    terrain: HeightfieldTerrain,
    clock: WorldClock,
    atmosphere: AtmosphereSynthesizer,
    clipmap: ClipmapManager,

    /// 每个 LOD 级别一个合并 MeshInstance3D（索引=level）
    lod_layers: [Option<LodLayer>; 8],
    /// 所有 Tile 共享一份材质
    shared_material: Option<Gd<ShaderMaterial>>,

    // 缓存的 Godot 节点引用
    sun: Option<Gd<DirectionalLight3D>>,
    world_env: Option<Gd<WorldEnvironment>>,
    terrain_parent: Option<Gd<Node3D>>,
    player_node: Option<Gd<Node3D>>,
    ocean_mesh: Option<Gd<MeshInstance3D>>,

    #[base]
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for WorldDriver {
    fn init(base: Base<Node3D>) -> Self {
        let terrain = HeightfieldTerrain::default();
        let clipmap = ClipmapManager::new_async(Arc::new(terrain.clone()));
        Self {
            terrain,
            clock: WorldClock::new(DEFAULT_SECONDS_PER_DAY),
            atmosphere: AtmosphereSynthesizer::from_embedded_toml()
                .expect("embedded time_curve.toml must be valid"),
            clipmap,
            lod_layers: Default::default(),
            shared_material: None,
            sun: None,
            world_env: None,
            terrain_parent: None,
            player_node: None,
            ocean_mesh: None,
            base,
        }
    }

    fn ready(&mut self) {
        // ── 1. 创建 HeightfieldTerrain（含群系）────
        let seed: u64 = 42;
        let params = NoiseParams {
            continent_scale: 0.001,
            detail_scale: 0.005,
            mountain_scale: 0.001,
            sea_threshold: -0.5,
            height_amplitude: 120.0,
            sea_depth: 400.0,
            climate_scale: 0.005,
        };
        let noise = WorldNoise::with_params(seed, params);

        let biome_toml = include_str!("../../../assets/biomes.toml");
        let biome_classifier = BiomeClassifier::from_toml_str(biome_toml, noise.clone())
            .expect("Failed to parse biomes.toml");

        let clock = WorldClock::new(DEFAULT_SECONDS_PER_DAY);
        self.terrain = HeightfieldTerrain::with_noise(noise)
            .with_clock(clock.clone())
            .with_biomes(biome_classifier);
        self.clock = clock;

        self.clipmap = ClipmapManager::new_async(Arc::new(self.terrain.clone()));

        // ── 2. 缓存场景节点引用（零后续查找）──────
        self.sun = self.base().try_get_node_as::<DirectionalLight3D>("../Sun");
        self.world_env = self
            .base()
            .try_get_node_as::<WorldEnvironment>("../WorldEnvironment");
        self.terrain_parent = Some(self.base().clone().cast::<Node3D>());
        self.player_node = self.base().try_get_node_as::<Node3D>("../Player");

        // OceanPlane 的 MeshInstance3D 子节点 —— 用于传水深 uniform
        self.ocean_mesh = self
            .base()
            .try_get_node_as::<Node3D>("../OceanPlane")
            .and_then(|ocean| ocean.get_node_or_null("OceanPlane"))
            .and_then(|n| n.try_cast::<MeshInstance3D>().ok());

        match (&self.sun, &self.world_env) {
            (Some(_), Some(_)) => {
                godot_print!("WorldDriver: Sun + WorldEnvironment nodes cached")
            }
            (None, _) => godot_error!("WorldDriver: ../Sun not found — day/night disabled"),
            (_, None) => godot_error!("WorldDriver: ../WorldEnvironment not found — sky disabled"),
        }

        // ── 3. 共享 ShaderMaterial（vertex shader camera-relative）──
        let mut mat = ShaderMaterial::new_gd();
        let mut shader = Shader::new_gd();
        shader.set_code("shader_type spatial;
render_mode unshaded, cull_disabled;
uniform vec3 camera_pos = vec3(0.0);
void vertex() {
    vec3 rel = VERTEX - camera_pos;
    vec3 view_pos = mat3(VIEW_MATRIX) * rel;
    POSITION = PROJECTION_MATRIX * vec4(view_pos, 1.0);
}
void fragment() {
    ALBEDO = COLOR.rgb;
}");
        mat.set_shader(&shader);
        self.shared_material = Some(mat);

        // ── 4. 创建每 LOD 级别的合并 MeshInstance3D ──
        for level in 0..8u8 {
            let mut mi = MeshInstance3D::new_alloc();
            let am = ArrayMesh::new_gd();
            mi.set_name(&format!("LOD_{}", level));
            mi.set_mesh(&am);
            // 材质在首次 terrain mesh 上传时应用（此时 mesh 为空，无表面可用）
            if let Some(ref mut parent) = self.terrain_parent {
                parent.add_child(&mi.clone().upcast::<Node>());
            }
            self.lod_layers[level as usize] = Some(LodLayer {
                instance: mi,
                mesh: am,
                active_keys: HashSet::new(),
                completed: HashMap::new(),
                dirty: true,  // 新创建 → 首次需合并上传
            });
        }

        self.base_mut().set_process(true);
        godot_print!("WorldDriver: 8 LOD layers ready");
    }

    fn process(&mut self, delta: f64) {
        self.clock.advance(delta);
        let wt = &self.clock.current;
        self.terrain.clock = Some(self.clock.clone());

        // 天空/太阳：每帧更新（保持太阳运动连续）
        {
            let origin = WorldPos { x: 0.0, y: 0.0, z: 0.0 };
            let atm = self.atmosphere.resolve(wt, origin);
            self.update_sun_and_sky(&atm);
        }

        let player_pos = self.get_player_position();

        // ── Floating Origin (Shader) ──
        // Vertex shader: VERTEX - camera_pos → GPU 坐标 < 500m
        if let Some(ref mut mat) = self.shared_material {
            let value = Variant::from(Vector3::new(
                player_pos.x as f32,
                player_pos.y as f32,
                player_pos.z as f32,
            ));
            mat.set_shader_parameter("camera_pos", &value);
        }

        // 更新海水深度
        if let Some(ref ocean_mesh) = self.ocean_mesh {
            let terrain_h = self.terrain.height_at(player_pos);
            let water_depth = if terrain_h < 0.0 {
                (-terrain_h).max(0.1)
            } else {
                0.1
            };
            if let Some(mat) = ocean_mesh.get_surface_override_material(0) {
                if let Ok(mut shader_mat) = mat.try_cast::<ShaderMaterial>() {
                    shader_mat.set_shader_parameter("water_depth", &Variant::from(water_depth));
                }
            }
        }

        let events = self.clipmap.poll(player_pos);

        for event in events {
            match event {
                TileEvent::Load { key, mesh } => {
                    if let Some(ref mut layer) = self.lod_layers[key.level as usize] {
                        if !mesh.vertices.is_empty() {
                            layer.completed.insert(key, mesh);
                        }
                        layer.active_keys.insert(key);
                        layer.dirty = true;
                    }
                }
                TileEvent::Unload { key } => {
                    if let Some(ref mut layer) = self.lod_layers[key.level as usize] {
                        layer.active_keys.remove(&key);
                        layer.completed.remove(&key);
                        layer.dirty = true;
                    }
                }
            }
        }

        // 合并各 LOD 级别的 mesh 并上传
        for level in 0..8u8 {
            if let Some(ref mut layer) = self.lod_layers[level as usize] {
                Self::merge_and_upload(layer, self.shared_material.as_ref());
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

    /// 合并某 LOD 级别所有已完成 tile mesh，上传到该级别的 ArrayMesh
    fn merge_and_upload(layer: &mut LodLayer, material: Option<&Gd<ShaderMaterial>>) {
        // 无变化 → 跳过昂贵的合并和上传
        if !layer.dirty {
            return;
        }

        // 清理已离开活跃集的 tile 的旧 mesh
        layer
            .completed
            .retain(|k, _| layer.active_keys.contains(k));

        if layer.active_keys.is_empty() {
            layer.mesh.clear_surfaces();
            layer.dirty = false;
            return;
        }

        // 检查是否所有活跃 tile 都已完成
        if !layer
            .active_keys
            .iter()
            .all(|k| layer.completed.contains_key(k))
        {
            return; // 仍待生成——保留 dirty 等待完成
        }

        let entries: Vec<(&LodKey, &TerrainMeshData)> = layer.completed.iter().collect();
        if entries.is_empty() {
            return;
        }
        let meshes: Vec<&TerrainMeshData> = entries.iter().map(|(_, m)| *m).collect();
        let origins: Vec<(f64, f64, f64)> = entries
            .iter()
            .map(|(key, _)| {
                let ts = match key.level {
                    0 => 16.0,
                    1 => 32.0,
                    2 => 64.0,
                    3 => 128.0,
                    4 => 256.0,
                    5 => 512.0,
                    6 => 1024.0,
                    7 => 2048.0,
                    _ => 0.0,
                };
                (key.gx as f64 * ts, key.gz as f64 * ts, ts)
            })
            .collect();

        let merged = Self::merge_meshes(&meshes, &origins);
        Self::update_array_mesh(&mut layer.mesh, &merged);
        if let Some(mat) = material {
            layer.instance.set_surface_override_material(0, mat);
        }
        layer.dirty = false;
    }

    /// 合并多个 TerrainMeshData 为一个（边界顶点焊接）
    ///
    /// 仅对 tile 边界附近（≤1m）的顶点进行焊接去重，
    /// 消除相邻 tile 独立等值面提取产生的边界裂缝。
    /// 内部顶点不焊接——避免意外合并不同位置的顶点。
    fn merge_meshes(
        meshes: &[&TerrainMeshData],
        origins: &[(f64, f64, f64)], // (ox, oz, tile_size) per mesh
    ) -> TerrainMeshData {
        use glam::Vec3;
        let weld_eps: f32 = 0.2; // 20cm — Transvoxel 边界顶点偏差可达 10-15cm
        let boundary_margin: f32 = 1.0; // 距 tile 边界 1m 内 = 边界顶点
        let cell_size = weld_eps;

        let mut vertices: Vec<Vec3> = Vec::new();
        let mut normals: Vec<Vec3> = Vec::new();
        let mut colors: Vec<Vec3> = Vec::new();
        let mut indices: Vec<u32> = Vec::new();
        // 仅存储边界顶点的空间哈希（内部顶点不参与焊接）
        let mut boundary_grid: HashMap<(i32, i32, i32), Vec<(u32, Vec3)>> = HashMap::new();

        for (mi, mesh) in meshes.iter().enumerate() {
            let (ox, oz, tile_sz) = origins[mi];
            let min_x = ox as f32;
            let max_x = (ox + tile_sz) as f32;
            let min_z = oz as f32;
            let max_z = (oz + tile_sz) as f32;

            let mut local_to_global: Vec<u32> = Vec::with_capacity(mesh.vertices.len());

            for vi in 0..mesh.vertices.len() {
                let v = mesh.vertices[vi];
                let on_boundary = (v.x - min_x).abs() < boundary_margin
                    || (v.x - max_x).abs() < boundary_margin
                    || (v.z - min_z).abs() < boundary_margin
                    || (v.z - max_z).abs() < boundary_margin;

                let g_idx = if on_boundary {
                    let cell = (
                        (v.x / cell_size).floor() as i32,
                        (v.y / cell_size).floor() as i32,
                        (v.z / cell_size).floor() as i32,
                    );
                    let mut found = None;
                    'search: for dx in -1..=1 {
                        for dy in -1..=1 {
                            for dz in -1..=1 {
                                let key = (cell.0 + dx, cell.1 + dy, cell.2 + dz);
                                if let Some(candidates) = boundary_grid.get(&key) {
                                    for &(idx, cv) in candidates {
                                        if (v - cv).length_squared() < weld_eps * weld_eps {
                                            found = Some(idx);
                                            break 'search;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    if let Some(existing) = found {
                        existing
                    } else {
                        let new_idx = vertices.len() as u32;
                        vertices.push(v);
                        normals.push(mesh.normals[vi]);
                        colors.push(mesh.colors[vi]);
                        boundary_grid
                            .entry(cell)
                            .or_default()
                            .push((new_idx, v));
                        new_idx
                    }
                } else {
                    // 内部顶点——直接添加，不焊接
                    let new_idx = vertices.len() as u32;
                    vertices.push(v);
                    normals.push(mesh.normals[vi]);
                    colors.push(mesh.colors[vi]);
                    new_idx
                };
                local_to_global.push(g_idx);
            }

            for &local_idx in &mesh.indices {
                indices.push(local_to_global[local_idx as usize]);
            }
        }

        TerrainMeshData {
            vertices,
            normals,
            indices,
            colors,
        }
    }

    /// 原地更新 ArrayMesh 的表面数据（复用已有 Godot 对象）
    fn update_array_mesh(am: &mut Gd<ArrayMesh>, mesh: &TerrainMeshData) {
        let mut vertices = PackedVector3Array::new();
        let mut normals = PackedVector3Array::new();
        let mut colors = PackedColorArray::new();

        // 单次遍历：同时构建数组 + 计算 AABB（消除双次遍历）
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut min_z = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        let mut max_z = f32::MIN;

        for i in 0..mesh.vertices.len() {
            let v = mesh.vertices[i];
            vertices.push(Vector3::new(v.x, v.y, v.z));
            normals.push(Vector3::new(mesh.normals[i].x, mesh.normals[i].y, mesh.normals[i].z));
            let c = mesh.colors[i];
            colors.push(Color::from_rgb(c.x, c.y, c.z));

            min_x = min_x.min(v.x);
            min_y = min_y.min(v.y);
            min_z = min_z.min(v.z);
            max_x = max_x.max(v.x);
            max_y = max_y.max(v.y);
            max_z = max_z.max(v.z);
        }

        let mut indices = PackedInt32Array::new();
        for idx in &mesh.indices {
            indices.push(*idx as i32);
        }

        let mut arrays = Array::new();
        let nil = Variant::nil();
        arrays.resize(13, &nil);
        arrays.set(0, &vertices.to_variant());
        arrays.set(1, &normals.to_variant());
        arrays.set(3, &colors.to_variant());
        arrays.set(12, &indices.to_variant());

        am.clear_surfaces();
        am.add_surface_from_arrays(PrimitiveType::TRIANGLES, &arrays);

        // 膨胀 AABB 防视锥体裁剪误裁边缘三角形
        {
            let margin: f32 = 1.0;
            let aabb = godot::builtin::Aabb::new(
                Vector3::new(min_x - margin, min_y - margin, min_z - margin),
                Vector3::new(
                    max_x - min_x + 2.0 * margin,
                    max_y - min_y + 2.0 * margin,
                    max_z - min_z + 2.0 * margin,
                ),
            );
            am.set_custom_aabb(aabb);
        }
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

// ── GDScript 接口 ──────────────────────

#[godot_api]
impl WorldDriver {
    /// GDScript 查询：(x, z) 处地形高度
    #[func]
    fn query_height(&self, x: f64, z: f64) -> f32 {
        let pos = WorldPos { x, y: 0.0, z };
        self.terrain.height_at(pos)
    }
}
