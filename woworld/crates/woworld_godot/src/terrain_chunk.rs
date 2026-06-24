//! WorldDriver — 世界驱动 GodotClass
//!
//! 管理多分辨率 LOD 地形渲染 + 昼夜循环 + 大气合成。
//! 消费纯 Rust ClipmapManager（引擎无关），直接操控 Godot 节点。
//!
//! 架构：Rust 权威 → Godot 纯表现（GDScript 铁律 §14.1）

use std::collections::HashMap;

use godot::classes::base_material_3d::{CullMode, Flags, ShadingMode};
use godot::classes::light_3d::Param;
use godot::classes::mesh::PrimitiveType;
use godot::classes::{
    ArrayMesh, DirectionalLight3D, MeshInstance3D, ProceduralSkyMaterial, StandardMaterial3D,
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

// ── GodotClass ────────────────────

#[derive(GodotClass)]
#[class(base = Node3D)]
pub struct WorldDriver {
    terrain: HeightfieldTerrain,
    clock: WorldClock,
    atmosphere: AtmosphereSynthesizer,
    clipmap: ClipmapManager,

    /// 活跃 Tile → MeshInstance3D 映射
    active_tiles: HashMap<LodKey, Gd<MeshInstance3D>>,
    /// 空闲 MeshInstance3D + ArrayMesh 池
    free_pool: Vec<(Gd<MeshInstance3D>, Gd<ArrayMesh>)>,
    /// 所有 Tile 共享一份材质
    shared_material: Option<Gd<StandardMaterial3D>>,

    // 缓存的 Godot 节点引用
    sun: Option<Gd<DirectionalLight3D>>,
    world_env: Option<Gd<WorldEnvironment>>,
    terrain_parent: Option<Gd<Node3D>>,
    player_node: Option<Gd<Node3D>>,

    #[base]
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for WorldDriver {
    fn init(base: Base<Node3D>) -> Self {
        let terrain = HeightfieldTerrain::default();
        let clipmap = ClipmapManager::new(terrain.clone());
        Self {
            terrain,
            clock: WorldClock::new(DEFAULT_SECONDS_PER_DAY),
            atmosphere: AtmosphereSynthesizer::from_embedded_toml()
                .expect("embedded time_curve.toml must be valid"),
            clipmap,
            active_tiles: HashMap::new(),
            free_pool: Vec::new(),
            shared_material: None,
            sun: None,
            world_env: None,
            terrain_parent: None,
            player_node: None,
            base,
        }
    }

    fn ready(&mut self) {
        // ── 1. 创建 HeightfieldTerrain（含群系）────
        let seed: u32 = 42;
        let params = NoiseParams {
            continent_scale: 0.001,
            detail_scale: 0.02,
            mountain_scale: 0.002,
            sea_threshold: -0.5,
            height_amplitude: 350.0,
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

        self.clipmap = ClipmapManager::new(self.terrain.clone());

        // ── 2. 缓存场景节点引用（零后续查找）──────
        self.sun = self.base().try_get_node_as::<DirectionalLight3D>("../Sun");
        self.world_env = self
            .base()
            .try_get_node_as::<WorldEnvironment>("../WorldEnvironment");
        self.terrain_parent = self
            .base()
            .try_get_node_as::<Node3D>(".")
            .or_else(|| Some(self.base().clone().cast::<Node3D>()));
        self.player_node = self.base().try_get_node_as::<Node3D>("../Player");

        match (&self.sun, &self.world_env) {
            (Some(_), Some(_)) => {
                godot_print!("WorldDriver: Sun + WorldEnvironment nodes cached")
            }
            (None, _) => godot_error!("WorldDriver: ../Sun not found — day/night disabled"),
            (_, None) => godot_error!("WorldDriver: ../WorldEnvironment not found — sky disabled"),
        }

        // ── 3. 共享材质 ──────────────────────────
        let mut mat = StandardMaterial3D::new_gd();
        mat.set_flag(Flags::ALBEDO_FROM_VERTEX_COLOR, true);
        mat.set_shading_mode(ShadingMode::UNSHADED);
        mat.set_cull_mode(CullMode::DISABLED);
        self.shared_material = Some(mat);

        // ── 4. 预分配对象池（~70 tiles）──────────
        let pool_size: usize = 100;
        self.free_pool.reserve(pool_size);
        for _ in 0..pool_size {
            let mi = MeshInstance3D::new_alloc();
            let am = ArrayMesh::new_gd();
            self.free_pool.push((mi, am));
        }
        godot_print!("WorldDriver: free pool pre-allocated with {} slots", pool_size);

        self.base_mut().set_process(true);
        godot_print!("WorldDriver: ready complete");
    }

    fn process(&mut self, delta: f64) {
        self.clock.advance(delta);
        let wt = &self.clock.current;
        self.terrain.clock = Some(self.clock.clone());

        let origin = WorldPos { x: 0.0, y: 0.0, z: 0.0 };
        let atm = self.atmosphere.resolve(wt, origin);
        self.update_sun_and_sky(&atm);

        let player_pos = self.get_player_position();
        let events = self.clipmap.poll(player_pos);

        for event in events {
            match event {
                TileEvent::Load { key, mesh } => self.load_tile(key, mesh),
                TileEvent::Unload { key } => self.unload_tile(key),
            }
        }
    }
}

// ── 内部方法 ──────────────────────────

impl WorldDriver {
    /// 读取玩家位置（使用缓存引用，零场景树查找）
    fn get_player_position(&self) -> WorldPos {
        if let Some(ref player) = self.player_node {
            let pos = player.get_position();
            WorldPos {
                x: pos.x as f64,
                y: pos.y as f64,
                z: pos.z as f64,
            }
        } else {
            WorldPos::default()
        }
    }

    /// 从对象池取出 MeshInstance3D → 更新 ArrayMesh → 挂载到场景树
    fn load_tile(&mut self, key: LodKey, mesh: TerrainMeshData) {
        let (mut mi, mut am) = self.free_pool.pop().unwrap_or_else(|| {
            godot_warn!("WorldDriver: free pool exhausted, allocating new slot");
            (MeshInstance3D::new_alloc(), ArrayMesh::new_gd())
        });

        Self::update_array_mesh(&mut am, &mesh);

        mi.set_name(&format!("L{}_{}_{}", key.level, key.gx, key.gz));
        mi.set_mesh(&am);
        if let Some(ref mat) = self.shared_material {
            mi.set_surface_override_material(0, mat);
        }

        if let Some(ref mut parent) = self.terrain_parent {
            let node = mi.clone().upcast::<Node>();
            parent.add_child(&node);
        }
        self.active_tiles.insert(key, mi);
    }

    /// 从场景树移除 → 归还对象池
    fn unload_tile(&mut self, key: LodKey) {
        if let Some(mut mi) = self.active_tiles.remove(&key) {
            if let Some(ref mut parent) = self.terrain_parent {
                parent.remove_child(&mi.clone().upcast::<Node>());
            }
            if let Some(am) = mi.get_mesh() {
                if let Ok(am) = am.try_cast::<ArrayMesh>() {
                    self.free_pool.push((mi, am));
                    return;
                }
            }
            mi.queue_free();
        }
    }

    /// 原地更新 ArrayMesh 的表面数据（复用已有 Godot 对象）
    fn update_array_mesh(am: &mut Gd<ArrayMesh>, mesh: &TerrainMeshData) {
        let mut vertices = PackedVector3Array::new();
        let mut normals = PackedVector3Array::new();
        let mut colors = PackedColorArray::new();
        for i in 0..mesh.vertices.len() {
            let v = mesh.vertices[i];
            let n = mesh.normals[i];
            let c = mesh.colors[i];
            vertices.push(Vector3::new(v.x, v.y, v.z));
            normals.push(Vector3::new(n.x, n.y, n.z));
            colors.push(Color::from_rgb(c.x, c.y, c.z));
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
