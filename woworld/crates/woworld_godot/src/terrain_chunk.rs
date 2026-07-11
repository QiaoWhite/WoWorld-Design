//! WorldDriver — 世界驱动 GodotClass
//!
//! 管理多分辨率 LOD 地形渲染 + 昼夜循环 + 大气合成。
//! 消费纯 Rust ClipmapManager（引擎无关），直接操控 Godot 节点。
//!
//! 架构：Rust 权威 → Godot 纯表现（GDScript 铁律 §14.1）

use godot::classes::light_3d::Param;
use godot::classes::mesh::PrimitiveType;
use std::sync::mpsc;
use std::sync::Arc;
use woworld_core::density::{DensityProvider, DensityStack};

use godot::classes::{
    ArrayMesh, DirectionalLight3D, Image, Input, MeshInstance3D, ProceduralSkyMaterial, Shader,
    ShaderMaterial, WorldEnvironment,
};
use godot::prelude::*;
use hecs::World as EcsWorld;
use std::collections::HashMap;
use woworld_atmosphere::{
    AtmosphereSynthesizer, SeasonAtmosQuery, SimpleSeasonProvider, WeatherAtmosQuery, WeatherDriver,
};
use woworld_core::lod::{
    CameraState, EntityLodInput, FrameBudget, HysteresisState, LodCoordinator, LodCoordinatorInput,
    PlayerAttention, VramPressure,
};
use woworld_core::prelude::*;
use woworld_core::spatial::TerrainQuery;

use woworld_core::material::SurfaceMaterial;
use woworld_core::ocean::OceanProvider;
use woworld_worldgen::{
    generate_clipmap_grid, generate_heightmap_data, layer_tex_config, level_spacing,
    load_material_colors, transvoxel_extract, BiomeClassifier, HeightfieldOcean,
    HeightfieldTerrain, HeightmapData, NoiseParams, TerrainBaseDensity, WorldNoise, LEVELS,
};

use crate::voxel_chunk::VoxelChunk;

// ── 缺省参数 ────────────────────
/// 默认每秒对应的游戏天数（30s/天方便观察，正式 3600s/天）
/// 现实秒 / 游戏天（3600 = 60 分钟/天，设计默认；调试可临时改为 30）
const DEFAULT_SECONDS_PER_DAY: f64 = 3600.0;
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

/// Transvoxel 块提取结果（rayon → 主线程）
struct VoxelResult {
    cx: i32,
    cz: i32,
    mesh: woworld_worldgen::TerrainMeshData,
    panicked: bool,
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
    /// Dirty flags: only update cross-sampling uniforms when layer actually moved
    fine_origin_dirty: bool,
    coarse_origin_dirty: bool,
}

// ── GodotClass ────────────────────

#[derive(GodotClass)]
#[class(base = Node3D)]
pub struct WorldDriver {
    terrain: HeightfieldTerrain,
    ocean: HeightfieldOcean,
    clock: WorldClock,
    atmosphere: AtmosphereSynthesizer,

    /// Phase 2 天气驱动（连续物理参数，每帧 tick）
    weather_driver: WeatherDriver,
    /// Phase 1 季节提供者（纯函数 total_days → season）
    season_provider: SimpleSeasonProvider,
    /// 上次更新的游戏天数（检测季节变更）
    last_game_day: u64,
    /// 调试：天气快捷键防抖计时器
    debug_weather_cooldown: f64,

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

    /// 后台 voxel 提取 → 主线程收割
    vx_result_tx: mpsc::Sender<VoxelResult>,
    vx_result_rx: mpsc::Receiver<VoxelResult>,

    /// VoxelGrid 统一 y-origin（init 扫描 25 chunk 取 min，drift 复用）
    voxel_wy: f64,
    voxel_vy: u32,

    /// Voxel 块在途标记
    vx_in_flight: std::collections::HashSet<(i32, i32)>,

    /// 所有 VoxelChunk 共享的 ShaderMaterial（camera_pos 每帧更新一次）
    voxel_material: Option<Gd<ShaderMaterial>>,

    /// Phase 1 LOD: 植被提供者，每帧接收 scene_lod 更新
    vegetation_provider: Option<Arc<dyn VegetationProvider>>,

    /// ECS World — 所有 Entity/Component 的权威存储
    ecs: EcsWorld,
    /// 掉落表注册表（EntityKind → LootTable）
    loot_tables: woworld_ecs::systems::life::loot_roll::LootTableRegistry,
    /// 空间索引——EntityIndex trait 实现 (Phase 1 就位，Phase 3 System 消费)
    #[allow(dead_code)]
    spatial_index: woworld_ecs::resources::spatial_grid::SpatialGrid,
    /// 关系存储——全局 NPC 关系 BTreeMap (RelationHandle Component → 此 Resource)
    relation_storage: woworld_ecs::resources::relation_storage::RelationStorage,
    /// 经济注册表——钱包/市场/订单簿/价格（Phase 2）
    economy_registry: woworld_ecs::resources::economy_registry::EconomyRegistry,
    /// 库存注册表——NPC 物品持有量与装备（★ Phase 2 新增）
    inventory_registry: woworld_ecs::resources::inventory_registry::InventoryRegistry,
    /// 物品注册表——物品定义 TOML 数据（Phase 2）
    item_registry: woworld_ecs::resources::item_registry::ItemRegistry,
    /// 物品种子系统是否已执行（仅一次）
    item_seeded: bool,
    /// 帧计数器（ECS System 用——单调递增 tick）
    frame_count: u64,

    /// ★ Step 5e: 角色控制器动作注册表（action_registry.toml 加载）
    action_registry: woworld_ecs::resources::action_registry::ActionRegistry,
    /// ★ Step 5e: 移动 profile 注册表（movement_profiles.toml 加载）
    movement_profile_registry:
        woworld_ecs::resources::movement_profile_registry::MovementProfileRegistry,
    /// ★ Step 5e: 动作实例 ID 生成器（单调递增，因果链串联）
    action_instance_counter: woworld_ecs::resources::action_instance_counter::ActionInstanceCounter,
    /// ★ Step 5e: 动作生命周期事件通道（双缓冲：begin_frame→send→mid_phase_flush→read）
    action_events: woworld_ecs::events::EventChannel<woworld_core::action::ActionLifecycleEvent>,

    /// ★ Sprint-062: 玩家输入帧快照（Godot input_bridge 每帧填充——桥接层属下一冲刺）
    input_state: woworld_core::input::InputState,
    /// ★ Sprint-062: 热键栏配置（数字键 → ActionId）
    hotbar_config: woworld_core::input::HotbarConfig,
    /// ★ Sprint-062: 附近可交互目标（感官系统填充——stub，属未来冲刺）
    nearby_interactables: woworld_ecs::resources::interact::NearbyInteractables,
    /// ★ Sprint-062: 动作轮盘数据（Phase 0 填充 → Godot 渲染）
    action_wheel: woworld_ecs::resources::interact::ActionWheelData,
    /// ★ Sprint-062: 累计游戏时间（秒）——输入缓冲 pressed_at 基准
    game_time_secs: f32,
    /// ★ Sprint-063: Block A0 是否对玩家实体行使渲染权威（ECS Position → Godot 节点）。
    ///   true（默认）= 玩家走 Block A0 地面控制器，ECS 权威。
    ///   false = G 键自由飞行调试旁路，Godot 节点权威（节点 → ECS 同步）。
    block_a0_driving: bool,

    /// Phase 2 LODCoordinator: 上一帧 LOD 处方（迟滞比较）
    lod_prev: HashMap<EntityId, LodPrescription>,
    /// Phase 2 LODCoordinator: 每实体迟滞状态（跨帧持久）
    lod_hyst: HashMap<EntityId, HysteresisState>,

    /// ECS → Godot 视觉桥接（NPC 胶囊体等）
    entity_renderer: Option<crate::entity_renderer::EntityRenderer>,
    /// Sprint-059: 调试控制台（F3 开关）
    debug_console: Option<crate::debug_console::DebugConsole>,
    /// Sprint-059: 名字缓存（entity_visual_system 内部维护）
    name_cache: std::collections::HashMap<hecs::Entity, String>,
    /// Sprint-061: 对话气泡状态（speech_bubble_system 维护，跨帧）
    bubble_state: woworld_ecs::resources::speech_bubble_state::SpeechBubbleState,
    /// Sprint-059: ECS Player 实体（排除用）
    /// Sprint-060: 夺舍时切换为被控 NPC，自由相机时指向裸实体
    player_ecs_entity: Option<hecs::Entity>,
    /// Sprint-060: 自由相机模式的裸 Player 实体（init 时创建，永久保留）
    bare_player_entity: Option<hecs::Entity>,
    /// Sprint-060: 夺舍候选列表索引（Tab 循环用）
    possession_candidate_index: usize,

    // Sprint-059: 边缘检测变量
    f3_was_pressed: bool,
    enter_was_pressed: bool,
    up_was_pressed: bool,
    down_was_pressed: bool,
    #[allow(dead_code)]
    mouse_left_was_pressed: bool,
    /// Sprint-060: Tab 夺舍 / F 退出夺舍边缘检测
    tab_was_pressed: bool,
    f_key_was_pressed: bool,

    // ── ★ 007 第三人称相机系统 ──
    /// CameraRig 节点引用（ready() 获取——独立相机 rig）
    camera_rig_node: Option<Gd<Node3D>>,
    /// Camera3D 节点引用（位于 CameraRig/PitchArm/Camera3D）
    camera_3d_node: Option<Gd<godot::classes::Camera3D>>,
    /// 相机跟随 SmoothDamp 位置（ECS 步进后更新）
    camera_follow_position: glam::Vec3,
    /// 相机跟随 SmoothDamp 速度
    camera_follow_velocity: glam::Vec3,
    /// CameraRig 推送的缩放目标臂长
    camera_arm_target: f32,
    /// 当前平滑臂长（Zoom SmoothDamp + collision 后）
    camera_arm_distance: f32,
    /// 缩放平滑速度
    camera_arm_velocity: f32,
    /// CameraRig 发布的最近一帧相机状态（LOD 消费）
    latest_camera_state: Option<CameraState>,
    /// 被控实体变更追踪（SNAP 检测——夺舍/瞬移跳变时重置 SmoothDamp）
    last_player_ecs_entity: Option<hecs::Entity>,
    /// 近距/FP 化身隐藏标记（arm < 1.0m → 隐藏自身胶囊）
    player_avatar_hidden: bool,

    #[base]
    base: Base<Node3D>,
}

/// 具体的 LODCoordinator 实现——WorldDriver 消费。
struct WorldLodCoordinator;
impl LodCoordinator for WorldLodCoordinator {}

#[godot_api]
impl INode3D for WorldDriver {
    fn init(base: Base<Node3D>) -> Self {
        let (tx, rx) = mpsc::channel();
        let (vx_tx, vx_rx) = mpsc::channel::<VoxelResult>();
        Self {
            terrain: HeightfieldTerrain::default(),
            ocean: HeightfieldOcean::default(),
            clock: WorldClock::new(DEFAULT_SECONDS_PER_DAY),
            atmosphere: AtmosphereSynthesizer::from_embedded_toml()
                .expect("embedded time_curve.toml must be valid"),
            weather_driver: WeatherDriver::new(0),
            season_provider: SimpleSeasonProvider::new(0),
            last_game_day: 0,
            debug_weather_cooldown: 0.0,
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
            vx_result_tx: vx_tx,
            vx_result_rx: vx_rx,
            vx_in_flight: std::collections::HashSet::new(),
            voxel_wy: 0.0,
            voxel_vy: 32,
            voxel_material: None,
            vegetation_provider: None,
            ecs: EcsWorld::new(),
            spatial_index: woworld_ecs::resources::spatial_grid::SpatialGrid::new(),
            loot_tables: woworld_ecs::systems::life::loot_roll::LootTableRegistry::default(),
            relation_storage: woworld_ecs::resources::relation_storage::RelationStorage::default(),
            economy_registry: woworld_ecs::resources::economy_registry::EconomyRegistry::new(),
            inventory_registry: woworld_ecs::resources::inventory_registry::InventoryRegistry::new(
            ),
            item_registry: woworld_ecs::resources::item_registry::ItemRegistry::new(),
            item_seeded: false,
            frame_count: 0,
            action_registry: woworld_ecs::resources::action_registry::ActionRegistry::new(),
            movement_profile_registry:
                woworld_ecs::resources::movement_profile_registry::MovementProfileRegistry::new(),
            action_instance_counter:
                woworld_ecs::resources::action_instance_counter::ActionInstanceCounter::new(),
            action_events: woworld_ecs::events::EventChannel::new(),
            input_state: woworld_core::input::InputState::default(),
            hotbar_config: woworld_core::input::HotbarConfig::new(),
            nearby_interactables: woworld_ecs::resources::interact::NearbyInteractables::new(),
            action_wheel: woworld_ecs::resources::interact::ActionWheelData::new(),
            game_time_secs: 0.0,
            block_a0_driving: true,
            lod_prev: HashMap::new(),
            lod_hyst: HashMap::new(),
            entity_renderer: None,
            debug_console: None,
            name_cache: std::collections::HashMap::new(),
            bubble_state: woworld_ecs::resources::speech_bubble_state::SpeechBubbleState::new(),
            player_ecs_entity: None,
            bare_player_entity: None,
            possession_candidate_index: 0,
            f3_was_pressed: false,
            enter_was_pressed: false,
            up_was_pressed: false,
            down_was_pressed: false,
            mouse_left_was_pressed: false,
            tab_was_pressed: false,
            f_key_was_pressed: false,
            camera_rig_node: None,
            camera_3d_node: None,
            camera_follow_position: glam::Vec3::ZERO,
            camera_follow_velocity: glam::Vec3::ZERO,
            camera_arm_target: 4.0,
            camera_arm_distance: 4.0,
            camera_arm_velocity: 0.0,
            latest_camera_state: None,
            last_player_ecs_entity: None,
            player_avatar_hidden: false,
            base,
        }
    }

    fn ready(&mut self) {
        use godot::builtin::PackedByteArray;
        use godot::classes::image::Format;
        use godot::classes::ImageTexture;

        // ── 1. 创建 HeightfieldTerrain（含群系）────
        let seed: u64 = 99;
        let params = NoiseParams::from_toml_str(include_str!("../../../assets/noise_params.toml"))
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
        self.material_colors =
            load_material_colors(include_str!("../../../assets/material_colors.toml"))
                .expect("material_colors.toml must be valid");

        // ── 2. 缓存场景节点引用 ──────────
        self.sun = self.base().try_get_node_as::<DirectionalLight3D>("../Sun");
        self.world_env = self
            .base()
            .try_get_node_as::<WorldEnvironment>("../WorldEnvironment");
        self.terrain_parent = Some(self.base().clone().cast::<Node3D>());
        self.player_node = self.base().try_get_node_as::<Node3D>("../Player");
        // ★ 007 相机：获取独立 CameraRig 节点引用
        self.camera_rig_node = self.base().try_get_node_as::<Node3D>("../CameraRig");
        self.camera_3d_node = self
            .base()
            .try_get_node_as::<godot::classes::Camera3D>("../CameraRig/PitchArm/Camera3D");
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

        // ── 3. 为 LOD 1-7 创建 GPU-driven clipmap（LOD 0 由 VoxelChunk 覆盖）──
        for level_idx in 1..8u8 {
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
                grid_n,
                grid_n,
                grid.indices.len() / 3,
                hm_size,
                hm_extent as f64 / 2.0 - level.max_range - spacing,
            );

            // 3c. 创建 R32F heightmap 纹理（按层分辨率）
            let mut img = Image::create(hm_size as i32, hm_size as i32, false, Format::RF)
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
            let mut mat_img = Image::create(hm_size as i32, hm_size as i32, false, Format::RGBA8)
                .expect("Image::create for material map");
            let mut mat_bytes = PackedByteArray::new();
            for _ in 0..(hm_size * hm_size) {
                for &b in &[191u8, 178, 128, 255] {
                    // sand #BFB280
                    mat_bytes.push(b);
                }
            }
            mat_img.set_data(
                hm_size as i32,
                hm_size as i32,
                false,
                Format::RGBA8,
                &mat_bytes,
            );
            let mt_tex = ImageTexture::create_from_image(&mat_img)
                .expect("ImageTexture::create_from_image for material map");

            // 3d. 创建 ShaderMaterial（全部 8 层共享同一个 .gdshader）
            let mut mat = ShaderMaterial::new_gd();
            mat.set_shader(&terrain_shader);
            mat.set_shader_parameter(&StringName::from("heightmap"), &ht_tex.to_variant());
            mat.set_shader_parameter(&StringName::from("material_map"), &mt_tex.to_variant());
            mat.set_shader_parameter(&StringName::from("hm_extent"), &Variant::from(hm_extent));
            mat.set_shader_parameter(
                &StringName::from("grid_origin"),
                &Variant::from(Vector2::new(0.0, 0.0)),
            );

            // 3e. 创建 MeshInstance3D
            let mut mi = MeshInstance3D::new_alloc();
            mi.set_name(&format!("LOD_{}", level_idx));
            mi.set_mesh(&am);
            mi.set_extra_cull_margin(15000.0);
            if let Some(ref mut parent) = self.terrain_parent {
                parent.add_child(&mi.clone().upcast::<Node>());
            }
            mi.set_surface_override_material(0, &mat);

            // 3f. Texture pool: create standby Image + ImageTexture (double-buffering)
            let mut hm_standby_img =
                Image::create(hm_size as i32, hm_size as i32, false, Format::RF)
                    .expect("Image::create for standby heightmap");
            hm_standby_img.set_data(hm_size as i32, hm_size as i32, false, Format::RF, &bytes);
            let hm_standby_tex = ImageTexture::create_from_image(&hm_standby_img)
                .expect("ImageTexture for standby heightmap");

            let mut mat_standby_img =
                Image::create(hm_size as i32, hm_size as i32, false, Format::RGBA8)
                    .expect("Image::create for standby material map");
            mat_standby_img.set_data(
                hm_size as i32,
                hm_size as i32,
                false,
                Format::RGBA8,
                &mat_bytes,
            );
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
                fine_origin_dirty: true,
                coarse_origin_dirty: true,
            });
        }

        // ── 4. 设置细层交叉采样 uniform（L1-L7 引用 L0-L6 的 heightmap）──
        for level_idx in 1..8u8 {
            let (left, right) = self.lod_layers.split_at_mut(level_idx as usize);
            let Some(finer) = left[level_idx as usize - 1].as_ref() else {
                continue;
            };
            let Some(layer) = right[0].as_mut() else {
                continue;
            };
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
            layer
                .material
                .set_shader_parameter(&StringName::from("blend_zone"), &Variant::from(blend_zone));
        }
        // L1: LOD 0 由 VoxelChunk 覆盖 → L1 为最内层 clipmap, 无更细层
        //     设 inner_bound=-1 禁用内边界 morphing
        if let Some(ref mut l1) = self.lod_layers[1] {
            l1.material.set_shader_parameter(
                &StringName::from("fine_heightmap"),
                &l1.heightmap_tex.to_variant(),
            );
            l1.material
                .set_shader_parameter(&StringName::from("inner_bound"), &Variant::from(-1.0f32));
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
            l7.material
                .set_shader_parameter(&StringName::from("outer_bound"), &Variant::from(99999.0f32));
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

        // ── 7. VoxelChunk 5×5 网格初始化 ──
        // LOD 0 由 Transvoxel 3D 等值面块覆盖 (32³ @ 0.5m = 16m³/chunk)
        // 5×5 网格 = 80m×80m, 最小覆盖 32m, 与 LOD 1 (30m 起) 2m 重叠
        // 加载 voxel shader, 所有 25 chunks 共享一个 ShaderMaterial
        let voxel_shader = loader
            .load("res://shaders/voxel_terrain.gdshader")
            .expect("Failed to load voxel_terrain.gdshader")
            .cast::<Shader>();
        let mut voxel_mat = ShaderMaterial::new_gd();
        voxel_mat.set_shader(&voxel_shader);
        self.voxel_material = Some(voxel_mat);

        self.init_voxel_grid();

        // ★ Step 5e: 加载角色控制器 TOML 资产到注册表（解析已由 woworld_ecs 单测冒烟验证）
        self.action_registry
            .load_from_toml(include_str!("../../../assets/action_registry.toml"))
            .expect("action_registry.toml must be valid");
        self.movement_profile_registry
            .load_from_toml(include_str!("../../../assets/movement_profiles.toml"))
            .expect("movement_profiles.toml must be valid");

        // ECS Phase 0: spawn Player Entity (保存 hecs Entity 用于互转)
        let player_terrain_y = self.terrain.height_at(WorldPos {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        });
        let player_entity = self.ecs.spawn((
            woworld_ecs::components::transform::Position(glam::Vec3::new(
                0.0,
                player_terrain_y,
                0.0,
            )),
            woworld_ecs::prelude::EntityKind::Creature,
            woworld_ecs::prelude::LodLevel::default(),
        ));
        self.player_ecs_entity = Some(player_entity);
        self.bare_player_entity = Some(player_entity);

        // ── Sprint-063: 玩家实体挂角色控制器（Block A0）全组件配方 ──
        //   玩家=NPC 哲学（001 总纲）：玩家走同一套 Rust Controller。
        //   配方复刻集成测试 spawn_player。★ 不挂旧 `Movement`——绞杀者
        //   (movement_system `Without<Movement>`) 据此只处理新管线实体。
        //   夺舍保持轻量装拆（012 §〇），不在此处或夺舍时重复挂 CC 组件。
        {
            use woworld_core::movement::{MovementState, Pace, Stance};
            use woworld_ecs::components::action_state::{CActionRequestBuf, CActiveAction};
            use woworld_ecs::components::input_state::{
                CCoyoteTime, CInputBuffer, CInputFeelConfig,
            };
            use woworld_ecs::components::movement_state::{
                CMoveIntent, CMovementControl, CMovementRecovery, CMovementState,
                CPrevMovementState,
            };
            use woworld_ecs::components::player::{ControlModeComponent, PlayerComponent};
            use woworld_ecs::components::transform::{Rotation, Velocity};

            let ms = MovementState {
                stance: Stance::Standing,
                // ★ Pace::Running（非 Still）：desired_state 的 pace 意图当前无消费者
                //   （M1=A 前瞻契约），movement_mode_system 也不改 pace——初值即生效值。
                //   max_speed(Standing, Still)=0 会导致无法移动，故必须给移动 pace。
                //   方向为零时仍由 movement_system 的无输入摩擦分支停下。
                pace: Pace::Running,
                ..Default::default()
            };
            self.ecs
                .insert(
                    player_entity,
                    (
                        PlayerComponent::default(),
                        ControlModeComponent::manual(),
                        CActiveAction::default(),
                        CActionRequestBuf::default(),
                        CInputBuffer::default(),
                        // Sprint-066 候选A：手感组件接线——CInputFeelConfig（土狼窗可配 +
                        //   I4 边缘吸附玩家门控）、CCoyoteTime（土狼跳 grace，coyote_time_system
                        //   写 remaining，action/input_buffer 的 effective_loco 据此放宽物理门）。
                        //   ⚠️ 实机休眠：平滑 Perlin 高度场下 is_walkable 永不因走路 flip false
                        //   （y 每帧贴地/无 >1m 断崖/陡坡留原地），土狼跳与边缘吸附不可触发——
                        //   代码路径已通、ECS 集成测试证明机制正确，待体素碰撞移动消费边缘几何解锁。
                        CInputFeelConfig::default(),
                        CCoyoteTime::default(),
                        CMovementControl::default(),
                        CMoveIntent::default(),
                        CMovementState(ms),
                        CPrevMovementState(ms),
                        CMovementRecovery::default(),
                        Velocity(glam::Vec3::ZERO),
                        Rotation(glam::Quat::IDENTITY),
                    ),
                )
                .expect("player entity exists — just spawned");
        }

        // Spawn NPC 种群（20 个，半径 30m 环形分布，Y 从地形查询）
        const NPC_COUNT: usize = 20;
        for i in 0..NPC_COUNT {
            let npc_seed = 1000 + i as u64;
            let angle = (i as f32 / NPC_COUNT as f32) * std::f32::consts::TAU;
            let dist = (i as f32 + 1.0) / NPC_COUNT as f32 * 30.0;
            let x = angle.cos() * dist;
            let z = angle.sin() * dist;
            let terrain_y = self.terrain.height_at(WorldPos {
                x: x as f64,
                y: 0.0,
                z: z as f64,
            });
            let pos = glam::Vec3::new(x, terrain_y, z); // root 放地表，mesh 子节点向上偏移
            self.spawn_npc(npc_seed, pos);
        }
        godot_print!("WorldDriver: {} NPCs spawned within 30m radius", NPC_COUNT);

        // 初始化 ECS → Godot 视觉桥接
        if let Some(ref mut terrain_parent) = self.terrain_parent {
            self.entity_renderer =
                Some(crate::entity_renderer::EntityRenderer::new(terrain_parent));
            godot_print!("WorldDriver: EntityRenderer initialized");
        }

        // Sprint-059: 初始化调试控制台
        let camera: Option<Gd<godot::classes::Camera3D>> = self
            .base()
            .try_get_node_as::<godot::classes::Camera3D>("../CameraRig/PitchArm/Camera3D");
        if let Some(ref mut terrain_parent) = self.terrain_parent {
            self.debug_console = Some(crate::debug_console::DebugConsole::new(
                terrain_parent,
                camera,
            ));
            godot_print!(
                "WorldDriver: DebugConsole initialized (camera={})",
                self.debug_console
                    .as_ref()
                    .and_then(|c| c.camera())
                    .is_some()
            );
        }

        self.base_mut().set_process(true);
        godot_print!("WorldDriver: 7 GPU-driven clipmap layers + 5×5 VoxelChunk grid + ECS ready");
    }

    fn process(&mut self, delta: f64) {
        self.clock.advance(delta);

        // 调试：数字键 1-6 切换天气 / 7-0 切换时段（0.3s 防抖，必须在 wt borrow 之前）
        self.debug_weather_cooldown = (self.debug_weather_cooldown - delta).max(0.0);
        if self.debug_weather_cooldown <= 0.0 {
            let input = Input::singleton();
            use woworld_core::weather_types::WeatherState;
            let mut target_weather: Option<WeatherState> = None;
            let mut target_time: Option<f64> = None;

            if input.is_key_pressed(godot::global::Key::KEY_1) {
                target_weather = Some(WeatherState::Clear);
            }
            if input.is_key_pressed(godot::global::Key::KEY_2) {
                target_weather = Some(WeatherState::PartlyCloudy);
            }
            if input.is_key_pressed(godot::global::Key::KEY_3) {
                target_weather = Some(WeatherState::Overcast);
            }
            if input.is_key_pressed(godot::global::Key::KEY_4) {
                target_weather = Some(WeatherState::LightPrecip);
            }
            if input.is_key_pressed(godot::global::Key::KEY_5) {
                target_weather = Some(WeatherState::ModeratePrecip);
            }
            if input.is_key_pressed(godot::global::Key::KEY_6) {
                target_weather = Some(WeatherState::HeavyStorm);
            }
            if input.is_key_pressed(godot::global::Key::KEY_7) {
                target_time = Some(0.25);
            } // Dawn
            if input.is_key_pressed(godot::global::Key::KEY_8) {
                target_time = Some(0.50);
            } // Noon
            if input.is_key_pressed(godot::global::Key::KEY_9) {
                target_time = Some(0.75);
            } // Dusk
            if input.is_key_pressed(godot::global::Key::KEY_0) {
                target_time = Some(0.00);
            } // Midnight

            if let Some(state) = target_weather {
                self.weather_driver.set_preset(state);
                self.debug_weather_cooldown = 0.3;
                godot_print!(
                    "[Debug] Weather preset → {:?} | params={:?}",
                    state,
                    self.weather_driver.params,
                );
            }
            if let Some(prog) = target_time {
                self.clock.set_time(prog);
                self.debug_weather_cooldown = 0.3;
                let phase = self.clock.current.phase;
                godot_print!(
                    "[Debug] Time → day_progress={:.2} phase={:?} light={:.2}",
                    prog,
                    phase,
                    self.clock.current.light_level
                );
            }
        }

        let wt = &self.clock.current;
        self.terrain.clock = Some(self.clock.clone());

        let player_pos = self.get_player_position();

        // 天气驱动 tick
        self.weather_driver
            .tick(delta, self.season_provider.current_season());

        // 季节检测（每天一次）
        let total_days = self.clock.current.day_number;
        if total_days != self.last_game_day {
            self.season_provider.update(total_days);
            self.last_game_day = total_days;
        }

        // 天空/太阳（含天气+季节调制，使用真实玩家位置）
        {
            let ws = self.weather_driver.sky_mult();
            let wf = self.weather_driver.fog_density();
            let we = self.weather_driver.exposure_mult();
            let wsa = self.weather_driver.saturation_mult();
            let ss = self.season_provider.saturation_mult();
            let sw = self.season_provider.warmth();

            let atm = self
                .atmosphere
                .resolve_with_weather(wt, player_pos, ws, wf, we, wsa, ss, sw);
            self.update_sun_and_sky(&atm, delta);
        }

        // ── Phase 2 LODCoordinator: 完整 8 步算法 ──
        let camera_forward = if let Some(ref player) = self.player_node {
            let basis = player.get_global_basis();
            // Godot Basis row-major: rows[0]=(xx,xy,xz), rows[1]=(yx,yy,yz), rows[2]=(zx,zy,zz)
            // Forward = -Z = negate third column
            let r = &basis.rows;
            DVec3::new(-r[0].z as f64, -r[1].z as f64, -r[2].z as f64)
        } else {
            DVec3::NEG_Z
        };

        // ★ Sprint-059 修复: 收集所有实体送入 LOD 计算器
        // 之前只送入了 Player，NPC 永远 render_lod=4(不可见)
        let mut lod_entities: Vec<EntityLodInput> = Vec::new();
        for (entity, pos) in self
            .ecs
            .query::<&woworld_ecs::components::transform::Position>()
            .iter()
        {
            let eid = woworld_ecs::entity_id::entity_id_from_hecs(entity);
            lod_entities.push(EntityLodInput {
                id: eid,
                position: DVec3::new(pos.0.x as f64, pos.0.y as f64, pos.0.z as f64),
                is_player: self.player_ecs_entity == Some(entity),
                is_in_combat: false,
                is_landmark: false,
                relation_importance: 0.0,
            });
        }

        // ★ 007: 优先用 CameraRig 发布的真实 CameraState；回退 body forward（CameraRig 未就绪）
        let lod_camera = self
            .latest_camera_state
            .clone()
            .unwrap_or_else(|| CameraState {
                position: DVec3::new(player_pos.x, player_pos.y, player_pos.z),
                forward: camera_forward,
                fov_radians: 70.0_f32.to_radians(),
            });

        let lod_input = LodCoordinatorInput {
            camera: lod_camera,
            attention: PlayerAttention::default(),
            frame_budget: FrameBudget {
                remaining_ms: (16.67 - delta * 1000.0).max(0.0) as f32,
                last_frame_ms: (delta * 1000.0) as f32,
            },
            vram: VramPressure::default(),
            entities: lod_entities,
            broadcasts: vec![],
            interactions: vec![],
        };

        let prescriptions =
            WorldLodCoordinator::compute_lod(&lod_input, &self.lod_prev, &mut self.lod_hyst);

        // 提取玩家 scene_lod 驱动植被 + clipmap
        let scene_lod = prescriptions
            .get(&EntityId(0))
            .map(|p| p.scene_lod)
            .unwrap_or(0);
        // ★ Sprint-059 修复: 将 LOD 处方写回 ECS LodLevel Component
        // 否则所有实体保持 LodLevel::default() (render_lod=4=不可见)
        use woworld_ecs::prelude::LodLevel;
        let mut lod_write_count = 0u32;
        for (entity_id, presc) in prescriptions.iter() {
            if let Some(hecs_entity) = woworld_ecs::entity_id::entity_id_to_hecs(*entity_id) {
                if let Ok(mut lod) = self.ecs.get::<&mut LodLevel>(hecs_entity) {
                    *lod = LodLevel::from_prescription(presc);
                    lod_write_count += 1;
                }
            }
        }
        if lod_write_count == 0 && self.frame_count % 60 == 0 {
            godot_print!(
                "[LOD] WARN: 0 entities received LOD prescription! prescriptions.len()={}",
                prescriptions.len()
            );
        }

        self.lod_prev = prescriptions;

        if let Some(ref vp) = self.vegetation_provider {
            vp.set_scene_lod(scene_lod);
        }

        // Sprint-063: 浮点原点 shader 的 camera_pos 必须是**真实相机**（Camera3D 子节点）
        //   的世界位置，而非 body/节点位置。二者差一个 eye height(~1.7m)：用 body 位置会让
        //   `rel = world_pos - camera_pos` 偏大 eye height → 体素地形渲染偏高 → 低眼高相机被
        //   地表吞没、cull_back 下看到背面显示为透明（旧 +3.4m 高相机恰好掩盖了这 1.7m 误差）。
        //   px/py/pz 仅供 voxel + clipmap 的 camera_pos uniform 使用。
        let cam_world = self
            .base()
            .get_viewport()
            .and_then(|vp| vp.get_camera_3d())
            .map(|c| c.get_global_position())
            .unwrap_or_else(|| {
                Vector3::new(
                    player_pos.x as f32,
                    player_pos.y as f32,
                    player_pos.z as f32,
                )
            });
        let px = cam_world.x;
        let pz = cam_world.z;
        let py = cam_world.y;

        // Update shared voxel material camera_pos (1 call for all 25 chunks)
        if let Some(ref mut voxel_mat) = self.voxel_material {
            voxel_mat.set_shader_parameter(
                &StringName::from("camera_pos"),
                &Variant::from(Vector3::new(px, py, pz)),
            );
        }

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
                    layer
                        .hm_standby_img
                        .set_data(size, size, false, Format::RF, &hm_bytes);
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
                    layer
                        .mat_standby_img
                        .set_data(size, size, false, Format::RGBA8, &mat_bytes);
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

                    // Mark cross-sampling uniforms dirty — neighbors need update
                    layer.fine_origin_dirty = true;
                    layer.coarse_origin_dirty = true;
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
                        snap_x as f32,
                        0.0,
                        snap_z as f32,
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
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            generate_heightmap_data(&terrain, snap_x, snap_z, spacing, hm_size, &mc)
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
                if finer.coarse_origin_dirty {
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
        }

        // ── 粗层交叉采样同步：L0-L6 的 coarse_heightmap/grid_origin 跟踪 L1-L7 ──
        for level_idx in 0..7u8 {
            let (left, right) = self.lod_layers.split_at_mut(level_idx as usize + 1);
            if let (Some(ref mut layer), Some(ref coarser)) =
                (&mut left[level_idx as usize], &right[0])
            {
                if coarser.fine_origin_dirty {
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
        }

        // Reset dirty flags after cross-sampling sync
        for layer in self.lod_layers.iter_mut().flatten() {
            layer.fine_origin_dirty = false;
            layer.coarse_origin_dirty = false;
        }

        // ── VoxelChunk harvest（rayon 后台提取结果 → 主线程上传）──
        // Frame budget: max 1 mesh upload per frame
        {
            let mut vx_uploaded = 0u32;
            const VX_MAX_PER_FRAME: u32 = 1;
            while vx_uploaded < VX_MAX_PER_FRAME {
                let result = match self.vx_result_rx.try_recv() {
                    Ok(r) => r,
                    Err(_) => break,
                };
                self.vx_in_flight.remove(&(result.cx, result.cz));
                if !result.panicked {
                    if let Some(vc) = self.voxel_chunks.get(&(result.cx, result.cz)) {
                        let mut vc_clone = vc.clone();
                        Self::upload_voxel_mesh(&mut vc_clone, &result.mesh);
                    }
                } else {
                    godot_error!(
                        "VoxelChunk ({},{}) extraction panicked",
                        result.cx,
                        result.cz
                    );
                }
                vx_uploaded += 1;
            }
        }

        // ── VoxelChunk grid drift management ──
        // Destroy out-of-range chunks, create new ones at missing positions.
        // In-grid chunks keep their current mesh (no premature clearing).
        {
            const CHUNK_SIZE: i32 = 16;
            let pcx = (player_pos.x / CHUNK_SIZE as f64).floor() as i32;
            let pcz = (player_pos.z / CHUNK_SIZE as f64).floor() as i32;
            let grid_radius: i32 = 2;

            if (pcx, pcz) != self.voxel_center {
                self.voxel_center = (pcx, pcz);

                // Remove chunks outside the new grid
                let to_remove: Vec<(i32, i32)> = self
                    .voxel_chunks
                    .keys()
                    .filter(|(cx, cz)| {
                        (cx - pcx).abs() > grid_radius || (cz - pcz).abs() > grid_radius
                    })
                    .copied()
                    .collect();
                for key in to_remove {
                    if let Some(mut vc) = self.voxel_chunks.remove(&key) {
                        vc.bind_mut().set_terrain_mesh(None); // hide
                        vc.clone().upcast::<Node>().queue_free(); // remove from scene tree
                    }
                    self.vx_in_flight.remove(&key);
                }

                // Recompute unified y-range for new grid area
                {
                    let mut y_min = f64::MAX;
                    let mut y_max = f64::MIN;
                    for dx in -grid_radius..=grid_radius {
                        for dz in -grid_radius..=grid_radius {
                            let wx = (pcx + dx) as f64 * CHUNK_SIZE as f64;
                            let wz = (pcz + dz) as f64 * CHUNK_SIZE as f64;
                            let h = self.terrain.height_at(WorldPos {
                                x: wx + 8.0,
                                y: 0.0,
                                z: wz + 8.0,
                            }) as f64;
                            y_min = y_min.min(h);
                            y_max = y_max.max(h);
                        }
                    }
                    self.voxel_wy = y_min - 4.0;
                    let total_h = (y_max - self.voxel_wy + 12.0).max(16.0);
                    self.voxel_vy = ((total_h / 0.5).ceil() as u32).max(32);
                }

                // Create new chunks at missing positions
                for dx in -grid_radius..=grid_radius {
                    for dz in -grid_radius..=grid_radius {
                        let cx = pcx + dx;
                        let cz = pcz + dz;
                        if self.voxel_chunks.contains_key(&(cx, cz)) {
                            continue;
                        }
                        let wx = cx as f64 * CHUNK_SIZE as f64;
                        let wz = cz as f64 * CHUNK_SIZE as f64;
                        let wy = self.voxel_wy;
                        let vy = self.voxel_vy;

                        let mut vc = VoxelChunk::new_alloc();
                        vc.bind_mut().set_world_origin(wx, wy, wz);
                        if let Some(ref voxel_mat) = self.voxel_material {
                            vc.bind_mut()
                                .set_terrain_material(voxel_mat.clone().upcast());
                        }
                        if let Some(ref mut parent) = self.terrain_parent {
                            parent.add_child(&vc.clone().upcast::<Node>());
                        }
                        self.submit_voxel_job(cx, cz, wx, wy, wz, vy);
                        self.voxel_chunks.insert((cx, cz), vc);
                    }
                }
            }
        }

        // ── Ocean seabed_y uniform（替代 hint_depth_texture → 消除 Depth Pre-Pass）──
        if let Some(ref ocean_mesh) = self.ocean_mesh {
            let seabed_y = self.terrain.height_at(WorldPos {
                x: player_pos.x,
                y: 0.0,
                z: player_pos.z,
            });
            if let Some(mat) = ocean_mesh.get_surface_override_material(0) {
                if let Ok(mut sm) = mat.try_cast::<ShaderMaterial>() {
                    sm.set_shader_parameter(
                        &StringName::from("seabed_y"),
                        &Variant::from(seabed_y),
                    );
                }
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

        // ── Sprint-059: ECS → EntityVisual → Godot 视觉同步 ──
        // Sprint-061: 先更新对话气泡状态（必须在 entity_visual_system 之前，
        // 否则气泡数据滞后一帧）。跳过被夺舍 NPC，防退出夺舍时闪现残留气泡。
        woworld_ecs::systems::npc::speech_bubble::speech_bubble_system(
            &self.ecs,
            self.frame_count,
            self.player_ecs_entity,
            &mut self.bubble_state,
        );

        // 先获取玩家位置（避免 borrow 冲突）
        let player_pos = self.get_player_position();
        if let Some(ref mut renderer) = self.entity_renderer {
            renderer.set_player_pos(glam::Vec3::new(
                player_pos.x as f32,
                player_pos.y as f32,
                player_pos.z as f32,
            ));

            let entity_visuals = woworld_ecs::systems::entity_visual::entity_visual_system(
                &self.ecs,
                self.player_ecs_entity,
                &mut self.name_cache,
                &self.bubble_state,
            );
            // 每 600 帧（~10s）打印一次诊断
            if self.frame_count % 600 == 0 {
                godot_print!(
                    "[EntityRenderer] frame={} entity_count={} renderer_exists=true",
                    self.frame_count,
                    entity_visuals.len()
                );
                for (_, v) in entity_visuals.iter().take(3) {
                    godot_print!(
                        "  name={} pos=({:.1},{:.1},{:.1}) lod={}",
                        v.display_name,
                        v.position.x,
                        v.position.y,
                        v.position.z,
                        v.render_lod
                    );
                }
            }
            // ★ 007: 过滤被控实体——近距/FP (arm < 1.0m) 隐藏自身化身胶囊
            let hide_controlled = self.player_avatar_hidden;
            let filtered: Vec<(hecs::Entity, &woworld_core::entity_visual::EntityVisual)> =
                entity_visuals
                    .iter()
                    .filter(|(_, v)| !(v.controlled && hide_controlled))
                    .map(|(e, v)| (*e, v))
                    .collect();
            renderer.sync(&filtered);
        } else {
            if self.frame_count == 60 {
                godot_print!("[EntityRenderer] WARN: entity_renderer is None!");
            }
        }

        // ── Sprint-059: 调试控制台 ──
        let console_visible = self
            .debug_console
            .as_ref()
            .map(|c| c.is_visible())
            .unwrap_or(false);

        // F3 边缘检测
        let input = Input::singleton();
        let f3_pressed = input.is_key_pressed(godot::global::Key::F3);
        if f3_pressed && !self.f3_was_pressed {
            if let Some(ref mut console) = self.debug_console {
                console.toggle();
                let now_visible = console.is_visible();
                if now_visible {
                    Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::VISIBLE);
                } else {
                    Input::singleton().set_mouse_mode(godot::classes::input::MouseMode::CAPTURED);
                }
            }
        }
        self.f3_was_pressed = f3_pressed;

        // ── Sprint-060: Tab 夺舍 / F 退出夺舍 ──
        let tab_pressed = input.is_key_pressed(godot::global::Key::TAB);
        let f_key_pressed = input.is_key_pressed(godot::global::Key::F);

        // Tab 夺舍——仅在非控制台模式下生效
        if tab_pressed && !self.tab_was_pressed && !console_visible {
            self.handle_possess_tab();
        }
        self.tab_was_pressed = tab_pressed;

        // F 退出夺舍——仅在非控制台模式下生效
        if f_key_pressed && !self.f_key_was_pressed && !console_visible {
            self.handle_unpossess();
        }
        self.f_key_was_pressed = f_key_pressed;

        // Sprint-063: 裸玩家飞行旁路时，节点 → ECS（pre-ECS，保持 Block A0 起点一致）
        self.sync_bare_player_render(false);

        // 提前获取 viewport 引用（避免与 console borrow 冲突）
        let viewport = self.base().get_viewport();
        if console_visible {
            if let Some(ref mut console) = self.debug_console {
                // Enter — 提交命令
                let enter_pressed = input.is_key_pressed(godot::global::Key::ENTER);
                if enter_pressed && !self.enter_was_pressed {
                    let text = console.input_text();
                    if !text.is_empty() {
                        // 添加命令历史
                        console.state.command_history.push(text.clone());
                        console.state.history_cursor = console.state.command_history.len();
                        // 压入队列
                        console.push_command(text);
                        console.clear_input();
                    }
                }
                self.enter_was_pressed = enter_pressed;

                // ↑↓ — 命令历史导航
                let up = input.is_key_pressed(godot::global::Key::UP);
                let down = input.is_key_pressed(godot::global::Key::DOWN);
                if up && !self.up_was_pressed {
                    if console.state.history_cursor > 0 {
                        console.state.history_cursor -= 1;
                    }
                    console.fill_from_history(console.state.history_cursor);
                }
                if down
                    && !self.down_was_pressed
                    && console.state.history_cursor < console.state.command_history.len()
                {
                    console.state.history_cursor += 1;
                    if console.state.history_cursor == console.state.command_history.len() {
                        console.clear_input();
                    } else {
                        console.fill_from_history(console.state.history_cursor);
                    }
                }
                self.up_was_pressed = up;
                self.down_was_pressed = down;

                // ── 鼠标左键点击 → raycast 选中实体 ──
                let mouse_left = input.is_mouse_button_pressed(godot::global::MouseButton::LEFT);
                if mouse_left && !self.mouse_left_was_pressed {
                    // 使用提前获取的 viewport
                    let mouse_pos = viewport
                        .as_ref()
                        .map(|vp| vp.get_mouse_position())
                        .unwrap_or(Vector2::ZERO);
                    if let Some(ref camera) = console.camera().cloned() {
                        let origin = camera.project_ray_origin(mouse_pos);
                        let normal = camera.project_ray_normal(mouse_pos);
                        let o = glam::Vec3::new(origin.x, origin.y, origin.z);
                        let d = glam::Vec3::new(normal.x, normal.y, normal.z);

                        if let Some(ref renderer) = self.entity_renderer {
                            if let Some(hit_entity) = renderer.raycast_select(o, d) {
                                console.state.selected_entity = Some(hit_entity);
                                let bits = hit_entity.to_bits().get();
                                console.append_output(&format!(
                                    "[color=#88ff88]Selected entity: {bits}[/color]"
                                ));
                            }
                        }
                    }
                }
                self.mouse_left_was_pressed = mouse_left;

                // ── 高亮同步 ──
                if let Some(_changed) = console.highlight_changed() {
                    if let Some(ref mut renderer) = self.entity_renderer {
                        // 恢复旧高亮 + 设置新高亮
                        renderer.highlight_entity(console.state.selected_entity);
                    }
                }

                // 消费命令队列
                while let Some(cmd) = console.poll_command() {
                    let output = execute_console_cmd(&cmd, console, &self.ecs);
                    console.append_output(&output);
                }

                // 同步玩家位置到 ConsoleState（listnpc 距离用）
                console.state.player_pos = glam::Vec3::new(
                    player_pos.x as f32,
                    player_pos.y as f32,
                    player_pos.z as f32,
                );

                // 同步 nameshow/color 设置到 renderer
                if let Some(ref mut renderer) = self.entity_renderer {
                    renderer.set_name_visible(console.state.name_visible);
                    renderer.set_color_enhanced(console.state.color_enhanced);
                }
            }
        }

        // Sprint-060: 处理 possess 命令的请求（console borrow 已释放）
        let pending_possess = self
            .debug_console
            .as_mut()
            .and_then(|c| c.state.pending_possess_request.take());
        if let Some(target) = pending_possess {
            self.handle_possess_by_entity(target);
        }
    }
}

/// 执行控制台命令
fn execute_console_cmd(
    cmd_str: &str,
    console: &mut crate::debug_console::DebugConsole,
    world: &hecs::World,
) -> String {
    let parts: Vec<&str> = cmd_str.split_whitespace().collect();
    if parts.is_empty() {
        return String::new();
    }

    let name = parts[0].to_lowercase();
    let args = &parts[1..];

    // help 特殊处理
    if name == "help" {
        let mut out = "[color=#ffcc00]Available commands:[/color]\n".to_string();
        let names = console.command_names();
        let mut sorted = names.clone();
        sorted.sort();
        for n in &sorted {
            if let Some(entry) = console.get_command(n) {
                out.push_str(&format!("  [color=#88ff88]{n}[/color] — {}\n", entry.help));
            }
        }
        return out;
    }

    // nameshow 同步到 renderer（在 call 之前——因为 renderer 可能在命令执行后更新）
    if name == "nameshow" {
        return (console.get_command("nameshow").unwrap().func)(args, &mut console.state, world);
    }

    if let Some(entry) = console.get_command(&name) {
        (entry.func)(args, &mut console.state, world)
    } else {
        format!("[color=#ff8888]Unknown command: '{name}'. Type 'help' for list.[/color]")
    }
}

// ── 内部方法 ──────────────────────────

impl WorldDriver {
    /// 相机跟随 Phase 2：SNAP → SmoothDamp 跟随 → 碰撞夹紧 → Zoom 平滑 → 设节点 → 发布 CameraState。
    /// 参见: 玩家系统 007 §四/§五/§八/§十一
    fn camera_follow_and_publish(&mut self, delta: f64) {
        // Gd 是引用计数句柄，clone 廉价——克隆出来避免与 &self 方法调用的借用冲突。
        let (Some(mut rig), Some(mut cam)) =
            (self.camera_rig_node.clone(), self.camera_3d_node.clone())
        else {
            return;
        };
        let dt = delta as f32;

        // 1. 跟随目标
        let target = self.get_camera_target();

        // 2. SNAP 检测（夺舍/瞬移跳变时跳过平滑）
        let entity_changed = self.player_ecs_entity != self.last_player_ecs_entity;
        self.last_player_ecs_entity = self.player_ecs_entity;
        let tgt_glam = glam::Vec3::new(target.x, target.y, target.z);
        let displacement = (self.camera_follow_position - tgt_glam).length();
        if entity_changed || displacement > 5.0 {
            self.camera_follow_position = tgt_glam;
            self.camera_follow_velocity = glam::Vec3::ZERO;
        }

        // 3. SmoothDamp 跟随 (follow_smooth_time = 0.08s)
        let current = self.camera_follow_position;
        let result = woworld_core::camera::smooth_damp_vec3(
            current,
            tgt_glam,
            &mut self.camera_follow_velocity,
            0.08,
            dt,
            None,
        );
        self.camera_follow_position = result;

        // 4. 落地下沉——pending 实机调参（暂禁：SmoothDamp 恢复产生弹跳感）
        //   见 spec §XI.1；当前直接改 rig Y 会产生"角色弹一下"体感。
        //   正确实现需把 dip 叠加到 follow **target** Y（非 output Y），让 SmoothDamp
        //   同时平滑 onset 与 recovery——待后续冲刺。
        let rig_y = result.y;

        // 5. 设 CameraRig 世界位（保留旋转，仅改位置）
        rig.set_global_position(Vector3::new(result.x, rig_y, result.z));

        // 6. 碰撞夹紧 arm（射线沿 Camera3D +Z = 身后方向，含 pitch）
        let pivot_world = woworld_core::types::WorldPos {
            x: result.x as f64,
            y: rig_y as f64,
            z: result.z as f64,
        };
        let cam_basis = cam.get_global_basis();
        let r = cam_basis.rows;
        let arm_dir = glam::Vec3::new(r[0].z, r[1].z, r[2].z); // +Z column = 身后
        let arm_desired = self.camera_arm_target;
        let arm_resolved = woworld_core::camera::resolve_camera_arm(
            &self.terrain,
            pivot_world,
            arm_dir,
            arm_desired,
            0.3,
        );

        // 7. Zoom SmoothDamp (zoom_smooth_time = 0.12s)
        let arm_smooth = woworld_core::camera::smooth_damp(
            self.camera_arm_distance,
            arm_resolved,
            &mut self.camera_arm_velocity,
            0.12,
            dt,
            None,
        );
        self.camera_arm_distance = arm_smooth;

        // 8. 设 Camera3D 局部 z (= +arm, Godot +Z = 身后)
        cam.set_position(Vector3::new(0.0, 0.0, arm_smooth));

        // 9. 近距/FP 化身隐藏标记
        self.player_avatar_hidden = arm_smooth < woworld_core::camera::HIDE_ARM_THRESHOLD;

        // 10. publish CameraState（用于 LOD 块消费）
        let cam_world = cam.get_global_position();
        let cam_basis2 = cam.get_global_basis();
        let rows2 = cam_basis2.rows;
        let cam_fwd = glam::Vec3::new(-rows2[0].z, -rows2[1].z, -rows2[2].z); // forward = -Z
        self.latest_camera_state = Some(CameraState {
            position: glam::DVec3::new(cam_world.x as f64, cam_world.y as f64, cam_world.z as f64),
            forward: glam::DVec3::new(cam_fwd.x as f64, cam_fwd.y as f64, cam_fwd.z as f64),
            fov_radians: cam.get_fov().to_radians(),
        });
    }

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

    /// Sprint-060: Tab 键夺舍——循环切换到下一个可夺舍 NPC
    fn handle_possess_tab(&mut self) {
        use woworld_ecs::systems::player::possess::{find_possessable_entities, possess_entity};

        // ★ 007 修复: 使用真实 Camera3D 的位置和朝向（非 body 节点），使夺舍目标按"视野内目测最近"
        //   排列。旧代码用 body 的 world pos + body yaw forward——眼高差 ~1.5m 且无 pitch。
        let (cam_pos, cam_forward) = {
            if let Some(ref cam) = self.camera_3d_node {
                let p = cam.get_global_position();
                let basis = cam.get_global_basis();
                let rows = &basis.rows;
                // Camera3D forward = -Z column
                let fwd = glam::Vec3::new(-rows[0].z, -rows[1].z, -rows[2].z);
                (glam::Vec3::new(p.x, p.y, p.z), fwd)
            } else {
                let pp = self.get_player_position();
                (
                    glam::Vec3::new(pp.x as f32, pp.y as f32, pp.z as f32),
                    self.get_camera_forward(),
                )
            }
        };

        let camera_pos_glam = cam_pos;

        let candidates = find_possessable_entities(
            &self.ecs,
            camera_pos_glam,
            cam_forward,
            self.player_ecs_entity,
        );

        if candidates.is_empty() {
            godot_print!("[Possess] No possessable NPCs found.");
            return;
        }

        // 循环索引
        let idx = self.possession_candidate_index % candidates.len();
        let target = candidates[idx].entity;
        self.possession_candidate_index = idx + 1;

        // 先退出当前夺舍（如果正在夺舍中）
        self.handle_unpossess_internal();

        // 执行夺舍
        let mut cmd = hecs::CommandBuffer::new();
        let npc_pos = possess_entity(&self.ecs, &mut cmd, target);
        cmd.run_on(&mut self.ecs);

        if let Some(pos) = npc_pos {
            // 切换 player_ecs_entity
            self.player_ecs_entity = Some(target);
            // Sprint-063: 夺舍期驻停裸玩家（避免 Block A0 无形漂移）
            self.set_bare_player_manual(false);

            // 瞬移 CharacterBody3D 到 NPC 位置
            if let Some(ref mut player) = self.player_node {
                player.set_global_position(godot::builtin::Vector3::new(pos.x, pos.y, pos.z));
            }

            // 打印被夺舍 NPC 的信息
            let name = self
                .name_cache
                .get(&target)
                .cloned()
                .unwrap_or_else(|| "Unknown".into());
            godot_print!(
                "[Possess] Now controlling: {name} at ({:.1}, {:.1}, {:.1})",
                pos.x,
                pos.y,
                pos.z
            );
        }
    }

    /// Sprint-060: F 键退出夺舍
    fn handle_unpossess(&mut self) {
        if self.is_possessing() {
            self.handle_unpossess_internal();
            // 恢复到自由相机模式的裸实体
            self.player_ecs_entity = self.bare_player_entity;
            // Sprint-063: 恢复裸玩家 Manual 控制（Block A0 重新接管）
            self.set_bare_player_manual(true);
            // Sprint-063: 裸玩家 ECS Position 从节点重新播种——退出夺舍时相机停在
            //   被释放的身体处，避免 Block A0 权威把相机瞬移到夺舍期间漂移的旧位置。
            if let Some(entity) = self.bare_player_entity {
                let gp = self.get_player_position();
                if let Ok(mut pos) = self
                    .ecs
                    .get::<&mut woworld_ecs::components::transform::Position>(entity)
                {
                    pos.0 = glam::Vec3::new(gp.x as f32, gp.y as f32, gp.z as f32);
                }
            }
            godot_print!("[Possess] Exited possession. Free camera mode.");
        }
    }

    /// 内部：退出夺舍（不改变 player_ecs_entity）
    fn handle_unpossess_internal(&mut self) {
        if let Some(current_player) = self.player_ecs_entity {
            // 只在夺舍模式下操作（不是裸实体）
            if self.bare_player_entity != Some(current_player) {
                use woworld_ecs::systems::player::possess::unpossess_entity;
                let mut cmd = hecs::CommandBuffer::new();
                unpossess_entity(&self.ecs, &mut cmd, current_player);
                cmd.run_on(&mut self.ecs);
            }
        }
    }

    /// ★ 007: 夺舍并入 CC 管线——post-ECS 方向反转：ECS Position → CharacterBody3D。
    ///   旧方向（节点→ECS）是 legacy walk 时代的残留——player.gd 驱动节点位置，
    ///   ECS 被动跟随。现在 Block A0 CC 管线是实体位置的唯一权威，节点应跟踪 ECS。
    fn sync_possessed_position(&mut self) {
        if !self.is_possessing() {
            return;
        }
        if let Some(entity) = self.player_ecs_entity {
            use woworld_ecs::components::transform::Position;
            if let Ok(pos) = self.ecs.get::<&Position>(entity) {
                if let Some(ref mut node) = self.player_node {
                    node.set_global_position(godot::builtin::Vector3::new(
                        pos.0.x, pos.0.y, pos.0.z,
                    ));
                }
            }
        }
    }

    /// 是否正在夺舍模式（不是自由相机）
    fn is_possessing(&self) -> bool {
        self.player_ecs_entity.is_some()
            && self.bare_player_entity.is_some()
            && self.player_ecs_entity != self.bare_player_entity
    }

    /// Sprint-063: 设置裸玩家实体的 ControlMode（Manual=自由控制 / Auto=夺舍期驻停）。
    ///
    /// 夺舍 NPC 期间把裸玩家设 Auto——player_input_system 据 controls_domain 跳过它，
    /// 避免裸玩家 ECS Position 被 Block A0 无形漂移。退出夺舍恢复 Manual。
    fn set_bare_player_manual(&mut self, manual: bool) {
        use woworld_core::player::ControlMode;
        use woworld_ecs::components::player::ControlModeComponent;
        if let Some(entity) = self.bare_player_entity {
            if let Ok(mut cm) = self.ecs.get::<&mut ControlModeComponent>(entity) {
                cm.mode = if manual {
                    ControlMode::Manual
                } else {
                    ControlMode::Auto
                };
            }
        }
    }

    /// Sprint-063: 裸玩家实体（非夺舍）的 Block A0 渲染权威同步。
    ///
    /// - `block_a0_driving` = true（默认）：ECS 权威。`post_ecs=true` 时把玩家实体
    ///   Position 推给 Godot 节点（Block A0 移动结果可见）。
    /// - `block_a0_driving` = false（G 飞行旁路）：节点权威。`post_ecs=false` 时把
    ///   节点位置同步进 ECS，保持 Block A0 起点一致，落地切回时无跳变。
    ///
    /// 夺舍模式（NPC 无 CC 组件）走 `sync_possessed_position`，此处早退。
    fn sync_bare_player_render(&mut self, post_ecs: bool) {
        if self.is_possessing() {
            return;
        }
        let Some(entity) = self.bare_player_entity else {
            return;
        };
        use woworld_ecs::components::transform::Position;
        if self.block_a0_driving {
            if post_ecs {
                // ECS → 节点
                let p = match self.ecs.get::<&Position>(entity) {
                    Ok(pos) => pos.0,
                    Err(_) => return,
                };
                if let Some(ref mut node) = self.player_node {
                    node.set_global_position(godot::builtin::Vector3::new(p.x, p.y, p.z));
                }
            }
        } else if !post_ecs {
            // 节点 → ECS（飞行）
            let gp = self.get_player_position();
            if let Ok(mut pos) = self.ecs.get::<&mut Position>(entity) {
                pos.0 = glam::Vec3::new(gp.x as f32, gp.y as f32, gp.z as f32);
            }
        }
    }

    /// Sprint-060: 夺舍指定实体（控制台 possess <id> 命令使用）
    fn handle_possess_by_entity(&mut self, target: hecs::Entity) {
        use woworld_ecs::systems::player::possess::possess_entity;

        // 先退出当前夺舍
        self.handle_unpossess_internal();

        let mut cmd = hecs::CommandBuffer::new();
        let npc_pos = possess_entity(&self.ecs, &mut cmd, target);
        cmd.run_on(&mut self.ecs);

        if let Some(pos) = npc_pos {
            self.player_ecs_entity = Some(target);
            // Sprint-063: 夺舍期驻停裸玩家（避免 Block A0 无形漂移）
            self.set_bare_player_manual(false);

            // 瞬移 CharacterBody3D
            if let Some(ref mut player) = self.player_node {
                player.set_global_position(godot::builtin::Vector3::new(pos.x, pos.y, pos.z));
            }

            let bits = target.to_bits().get();
            let name = self
                .name_cache
                .get(&target)
                .cloned()
                .unwrap_or_else(|| format!("Entity_{bits}"));
            godot_print!(
                "[Possess] Console possess: {name} (id={bits}) at ({:.1}, {:.1}, {:.1})",
                pos.x,
                pos.y,
                pos.z
            );
        }
    }

    /// 获取摄像机前方向量（glam Vec3）
    fn get_camera_forward(&self) -> glam::Vec3 {
        if let Some(ref player) = self.player_node {
            let basis = player.get_global_basis();
            let r = &basis.rows;
            // Godot Basis: -Z = forward
            glam::Vec3::new(-r[0].z, -r[1].z, -r[2].z)
        } else {
            glam::Vec3::NEG_Z
        }
    }

    /// 设置植被提供者——LODCoordinator 每帧通过 `set_scene_lod` 驱动植被细节。
    #[allow(dead_code)]
    pub fn set_vegetation_provider(&mut self, vp: Arc<dyn VegetationProvider>) {
        self.vegetation_provider = Some(vp);
    }

    /// 将静态网格上传到 ArrayMesh（仅启动时调用一次）
    fn upload_static_mesh(am: &mut Gd<ArrayMesh>, mesh: &woworld_worldgen::TerrainMeshData) {
        use godot::builtin::Array;
        use godot::builtin::PackedColorArray;
        use godot::builtin::PackedInt32Array;
        use godot::builtin::PackedVector3Array;

        let mut vertices = PackedVector3Array::new();
        let mut normals = PackedVector3Array::new();
        let mut colors = PackedColorArray::new();

        for i in 0..mesh.vertices.len() {
            let v = mesh.vertices[i];
            vertices.push(Vector3::new(v.x, v.y, v.z));
            normals.push(Vector3::new(
                mesh.normals[i].x,
                mesh.normals[i].y,
                mesh.normals[i].z,
            ));
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
    fn update_sun_and_sky(&mut self, atm: &woworld_atmosphere::ResolvedAtmosphere, delta: f64) {
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

        // ── ECS tick — 全 17 System ──────────
        {
            use hecs::CommandBuffer;
            use woworld_ecs::systems::life::{
                cleanup, corpse_decay, death_watch, item_spawn, loot_roll, regen,
            };
            use woworld_ecs::systems::npc::{
                action_weight::action_weight_system,
                age::age_system,
                bigfive_derive::bigfive_derive_system,
                emotion::{emotion_drift_system, physiological_pull_system},
                movement::movement_system,
                social::social_system,
            };

            self.frame_count += 1;
            let day_progress = Some(self.clock.day_progress());

            // ── 物品种子（一次性）──
            if !self.item_seeded {
                woworld_ecs::systems::item::item_seed_system(&mut self.item_registry);
                // 创建默认经济区和市场
                let econ_id = self.economy_registry.create_economy();
                self.economy_registry.create_market_with_economy(econ_id);
                self.item_seeded = true;
            }

            // ── Block A0: 角色控制器管线（Step 5e + Sprint-062 ActionResolver）──
            //   顺序 = player_input → coyote → stamina → movement_mode
            //          → input_buffer → action_resolver → interact_context → action(+flush)
            //          → jump_launch → movement
            //   · player_input 最先：写 CMoveIntent.direction（Movement 域不经 resolver, 004 §五）
            //   · coyote 必须在 movement_mode 前：读 CPrevMovementState，后者会覆盖它
            //   · stamina 在 movement_mode 前：恢复栈快照到降级后的 pace，避免落地闪帧（D1 定论）
            //   · action_resolver 在 input_buffer 后（001 Phase 0 序）：即时→CActionRequestBuf,
            //     时敏→CInputBuffer（下一帧 input_buffer drain）；interact_context 解析交互键
            //   这些系统仅作用于带新组件的实体（绞杀者：movement 带 Without<Movement>），
            //   旧 20 NPC 与裸玩家不受影响。当前无实体挂新组件——管线休眠 no-op，
            //   待 ActionResolver 桥接（下一冲刺）/ 夺舍迁移把实体接上即激活。
            {
                use woworld_ecs::systems::action::action_system::action_system;
                use woworld_ecs::systems::input::action_resolver_system::action_resolver_system;
                use woworld_ecs::systems::input::coyote_time_system::coyote_time_system;
                use woworld_ecs::systems::input::input_buffer_system::input_buffer_system;
                use woworld_ecs::systems::input::interact_context_system::interact_context_system;
                use woworld_ecs::systems::movement::movement_mode_system::movement_mode_system;
                use woworld_ecs::systems::movement::movement_system::movement_system as cc_movement_system;
                use woworld_ecs::systems::movement::stamina_gate_system::stamina_gate_system;
                use woworld_ecs::systems::player::player_input::player_input_system;

                let cc_dt = delta as f32;
                self.game_time_secs += cc_dt;
                self.action_events.begin_frame();
                player_input_system(&mut self.ecs, &self.input_state);
                coyote_time_system(&mut self.ecs, cc_dt, &self.terrain);
                stamina_gate_system(&mut self.ecs, cc_dt);
                movement_mode_system(&mut self.ecs, &self.terrain);
                input_buffer_system(
                    &mut self.ecs,
                    &self.terrain,
                    &self.action_registry,
                    self.game_time_secs,
                );
                action_resolver_system(
                    &mut self.ecs,
                    &self.input_state,
                    &self.hotbar_config,
                    &self.action_registry,
                    self.game_time_secs,
                );
                interact_context_system(
                    &mut self.ecs,
                    &self.input_state,
                    &self.nearby_interactables,
                    &mut self.action_wheel,
                );
                action_system(
                    &mut self.ecs,
                    cc_dt,
                    &self.action_registry,
                    &mut self.action_instance_counter,
                    &mut self.action_events,
                    &self.terrain,
                );
                self.action_events.mid_phase_flush();
                // ★ 007: character_facing_system — rotation_lock → Rotation (smooth damp)
                // 位置: mid_phase_flush 之后 (action_system 已写 rotation_lock)，
                //       jump_launch 之前 (facing 不依赖跳跃结果)
                woworld_ecs::systems::movement::character_facing_system::character_facing_system(
                    &mut self.ecs,
                    cc_dt,
                );
                // Sprint-064: jump 起跳——动作激活后、移动积分前注入垂直速度 + 置腾空态
                woworld_ecs::systems::movement::jump_launch_system::jump_launch_system(
                    &mut self.ecs,
                    &self.movement_profile_registry,
                );
                cc_movement_system(
                    &mut self.ecs,
                    cc_dt,
                    &self.terrain,
                    &self.movement_profile_registry,
                );
            }
            // ── Block A1: &mut World systems (no CommandBuffer) ──
            woworld_ecs::systems::npc::needs::needs_decay_system(&mut self.ecs, day_progress);
            regen::regen_system(&mut self.ecs);
            emotion_drift_system(&mut self.ecs, delta as f32);
            physiological_pull_system(&mut self.ecs);
            social_system(
                &mut self.ecs,
                delta as f32,
                self.frame_count,
                &mut self.relation_storage,
            );

            // ── Block A2: movement_system (&mut World + active cmd) ──
            {
                let mut move_cmd = CommandBuffer::new();
                movement_system(
                    &mut self.ecs,
                    &mut move_cmd,
                    delta as f32,
                    self.frame_count,
                    &self.terrain,
                );
                move_cmd.run_on(&mut self.ecs);
            }

            // ── Block A3: age_system (&mut World + cmd) ──
            {
                let mut age_cmd = CommandBuffer::new();
                age_system(
                    &mut self.ecs,
                    &mut age_cmd,
                    delta as f32 / self.clock.seconds_per_day as f32,
                );
                age_cmd.run_on(&mut self.ecs);
            }

            // ── Block A4: &World systems via CommandBuffer batch ──
            {
                let mut cmd = CommandBuffer::new();
                bigfive_derive_system(&self.ecs, &mut cmd);
                woworld_ecs::systems::npc::needs::need_evaluation_system(&self.ecs, &mut cmd);
                woworld_ecs::systems::npc::goal::goal_resolution_system(&self.ecs, &mut cmd);
                action_weight_system(&self.ecs, &mut cmd, day_progress);
                // Economy: cognition + wallet init
                woworld_ecs::systems::economy::economic_cognition_update_system(
                    &self.ecs,
                    &mut cmd,
                    &mut self.economy_registry,
                );
                woworld_ecs::systems::economy::wallet_init_system(
                    &self.ecs,
                    &mut cmd,
                    &mut self.economy_registry,
                );
                // ★ Phase 2: 库存初始化（含 NPC 初始物品播种）
                woworld_ecs::systems::item::inventory_init_system(
                    &self.ecs,
                    &mut cmd,
                    &mut self.inventory_registry,
                    &self.item_registry,
                );
                death_watch::death_watch_system(&self.ecs, &mut cmd, self.frame_count);
                loot_roll::loot_roll_system(&self.ecs, &mut cmd, &self.loot_tables);
                item_spawn::item_spawn_system(&self.ecs, &mut cmd);
                corpse_decay::corpse_decay_system(&self.ecs, &mut cmd, self.frame_count);
                cleanup::cleanup_system(&self.ecs, &mut cmd);
                cmd.run_on(&mut self.ecs);
            }

            // ── Block A5: Economy systems (registry-only, no CommandBuffer) ──
            {
                woworld_ecs::systems::economy::order_creation_system(
                    &self.ecs,
                    &mut self.economy_registry,
                    &self.item_registry,
                    self.frame_count,
                );
                woworld_ecs::systems::economy::market_matching_system(
                    &mut self.economy_registry,
                    self.frame_count,
                );
            }
        }

        // Sprint-063: 裸玩家走 Block A0 时，ECS Position → 渲染节点（post-ECS，ECS 权威）
        self.sync_bare_player_render(true);
        // ★ 007: 夺舍并入 CC 管线——post-ECS 把 ECS Position 推给节点（方向反转）
        self.sync_possessed_position();

        // ── ★ 007 相机跟随 + 碰撞 + 发布（ECS 步进后，spec §V.3 step ③） ──
        self.camera_follow_and_publish(delta);
    }
}

// GDScript 接口 ──────────────────────

/// Godot `Transform3D` → glam `Mat4`（列主序）。
///
/// Godot `Basis.rows` 为行主序：`rows[i] = (m_i0, m_i1, m_i2)`。glam Mat4 列 j =
/// `(rows[0][j], rows[1][j], rows[2][j])`。用于 input_bridge 传相机变换。
fn transform3d_to_mat4(xform: Transform3D) -> glam::Mat4 {
    let r = &xform.basis.rows;
    let o = xform.origin;
    glam::Mat4::from_cols(
        glam::Vec4::new(r[0].x, r[1].x, r[2].x, 0.0),
        glam::Vec4::new(r[0].y, r[1].y, r[2].y, 0.0),
        glam::Vec4::new(r[0].z, r[1].z, r[2].z, 0.0),
        glam::Vec4::new(o.x, o.y, o.z, 1.0),
    )
}

#[godot_api]
impl WorldDriver {
    /// GDScript 查询：(x, z) 处地形高度
    #[func]
    fn query_height(&self, x: f64, z: f64) -> f32 {
        let pos = WorldPos { x, y: 0.0, z };
        self.terrain.height_at(pos)
    }

    /// Sprint-059: 调试控制台是否开启（player.gd 消费）
    #[func]
    fn is_console_open(&self) -> bool {
        self.debug_console
            .as_ref()
            .map(|c| c.is_visible())
            .unwrap_or(false)
    }

    // ── Sprint-063: input_bridge.gd 桥接 API ─────────────────────
    //   GDScript 每帧调用：begin_frame → set_move/set_camera_transform → press/release。
    //   InputState 是平台无关帧快照；Rust 侧 Block A0 消费（004 §一/§五）。

    /// 帧初：清空 pressed/released 边沿（held 保持）。input_bridge 每帧最先调用。
    #[func]
    fn input_begin_frame(&mut self) {
        self.input_state.begin_frame();
    }

    /// 记录一次按下（边沿 + held）。code/payload 见 `InputAction::from_code`。未知 code 忽略。
    #[func]
    fn input_press(&mut self, code: i64, payload: i64) {
        if let Some(action) = woworld_core::input::InputAction::from_code(code, payload) {
            self.input_state.press(action);
        }
    }

    /// 记录一次释放（边沿，移出 held）。未知 code 忽略。
    #[func]
    fn input_release(&mut self, code: i64, payload: i64) {
        if let Some(action) = woworld_core::input::InputAction::from_code(code, payload) {
            self.input_state.release(action);
        }
    }

    /// 设置原始移动输入（相机相对：x=左右平移, z=前后，+z 为前进）。
    /// player_input_system 用 camera_transform 转世界空间。
    #[func]
    fn input_set_move(&mut self, x: f32, z: f32) {
        self.input_state.move_direction = glam::Vec2::new(x, z);
    }

    /// 设置相机变换（Godot Transform3D → glam Mat4）——用于相机相对移动转世界空间。
    /// 传玩家 body 的 global_transform（仅 yaw，水平朝向；pitch 在 Camera 子节点）。
    #[func]
    fn input_set_camera_transform(&mut self, xform: Transform3D) {
        self.input_state.camera_transform = transform3d_to_mat4(xform);
    }

    /// Block A0 是否对玩家实体行使渲染权威（player.gd 消费——决定是否让权）。
    ///
    /// 仅当「自由裸玩家 + 非飞行」时为真：此时 Block A0 地面控制器权威，
    /// player.gd 停自身物理。夺舍 NPC（无 CC 组件，走 legacy player.gd 移动）
    /// 或 G 飞行旁路时返回 false，player.gd 自行驱动节点。
    #[func]
    fn is_block_a0_driving(&self) -> bool {
        // ★ 007: 夺舍并入 CC 管线——夺舍时也让 Block A0 权威，player.gd 保持 idle。
        //   （旧代码 `&& !is_possessing()` 会让夺舍走 player.gd legacy walk，
        //    body basis 未旋转 → WASD 固定方向 + 吸附 +1.7 → 浮空。）
        self.block_a0_driving
    }

    /// 设置 Block A0 渲染权威开关（player.gd 切 G 飞行时调用）。
    #[func]
    fn set_block_a0_driving(&mut self, driving: bool) {
        self.block_a0_driving = driving;
    }

    // ── ★ 007 相机系统 #[func] ──────────────────

    /// CameraRig.gd 消费：返回被控实体 ECS Position + 眼高（pivot）。
    #[func]
    fn get_camera_target(&self) -> Vector3 {
        const PIVOT_HEIGHT: f32 = 1.5;
        if let Some(entity) = self.player_ecs_entity {
            use woworld_ecs::components::transform::Position;
            if let Ok(pos) = self.ecs.get::<&Position>(entity) {
                return Vector3::new(pos.0.x, pos.0.y + PIVOT_HEIGHT, pos.0.z);
            }
        }
        let pp = self.get_player_position();
        Vector3::new(pp.x as f32, pp.y as f32 + PIVOT_HEIGHT, pp.z as f32)
    }

    /// CameraRig 消费：上一帧着地冲击速度（m/s），无事件则 0.0。
    #[func]
    fn get_just_landed_impact(&self) -> f32 {
        if let Some(entity) = self.player_ecs_entity {
            use woworld_ecs::components::movement_state::CJustLanded;
            if let Ok(jl) = self.ecs.get::<&CJustLanded>(entity) {
                return jl.impact_speed;
            }
        }
        0.0
    }

    /// CameraRig 消费：被控实体是否正在冲刺。
    #[func]
    fn is_player_sprinting(&self) -> bool {
        if let Some(entity) = self.player_ecs_entity {
            use woworld_ecs::components::movement_state::CMovementState;
            if let Ok(ms) = self.ecs.get::<&CMovementState>(entity) {
                return ms.0.pace == woworld_core::movement::Pace::Sprinting;
            }
        }
        false
    }

    /// CameraRig 消费：UI 是否捕获输入（阻止相机旋转）。
    /// MVP stub——始终返回 false。Phase 2 接入实际 UI 面板焦点。
    #[func]
    fn is_ui_capturing(&self) -> bool {
        false
    }

    /// CameraRig._input 推送：缩放 / FP 切换的目标臂长。
    #[func]
    fn set_target_arm_distance(&mut self, d: f32) {
        self.camera_arm_target = d;
    }

    /// 创建完整 NPC Entity（全 Component bundle），返回 hecs Entity
    fn spawn_npc(&mut self, seed: u64, position: glam::Vec3) -> hecs::Entity {
        use woworld_ecs::components::aesthetic::AestheticTaste;
        use woworld_ecs::components::biases::CognitiveBiases;
        use woworld_ecs::components::bigfive::BigFive;
        use woworld_ecs::components::cognitive::CognitiveStyle;
        use woworld_ecs::components::economy::{EconomicCognition, Wallet};
        use woworld_ecs::components::emotion::Emotion;
        use woworld_ecs::components::gender::BiologicalSex;
        use woworld_ecs::components::goal::Goal;
        use woworld_ecs::components::growth::GrowthNeeds;
        use woworld_ecs::components::lifecycle::{Age, GompertzMortality, LifeStage};
        use woworld_ecs::components::movement::Movement;
        use woworld_ecs::components::needs::Needs;
        use woworld_ecs::components::social::{RelationHandle, SocialPresence};
        use woworld_ecs::components::transform::{Position, Rotation};
        use woworld_ecs::components::vitals::{RegenState, Vitals};
        use woworld_ecs::prelude::{EntityKind, LodLevel};
        use woworld_ecs::prng::pseudo_random_f32_range;

        // 1. 人格根
        let bf = BigFive::from_seed(seed);

        // 2. 从 BigFive 派生
        let need_sens = bf.derive_sensitivity();
        let chronotype = bf.derive_chronotype();
        let social_presence = SocialPresence::derive_from_bigfive(&bf);
        let cognitive_style = CognitiveStyle::derive_from_bigfive(&bf);

        // 3. 情感（drift system 接管后向 baseline 收敛）
        let emotion = Emotion::default();

        // 4. 审美 + 偏误
        let aesthetic = AestheticTaste::derive_from_bigfive(&bf, seed);
        let biases = CognitiveBiases::derive(&cognitive_style, &bf, &emotion);

        // 5. 性别
        let sex = BiologicalSex::from_seed(seed);

        // 6. 年龄 18-65，从 seed 确定
        let age_years = 18.0 + pseudo_random_f32_range(seed, 100, 0.0, 47.0);
        let max_lifespan = 70.0 + pseudo_random_f32_range(seed, 101, -10.0, 20.0);
        let age = Age::new(max_lifespan, age_years);
        let life_stage = LifeStage::from_age_ratio(age.age_ratio());

        // 6.5 Gompertz 衰老模型——体质从 seed 派生（0.3-0.8）
        let constitution = 0.3 + pseudo_random_f32_range(seed, 200, 0.0, 0.5);
        let gmort = GompertzMortality {
            last_check_age_days: age.age_days,
            constitution,
            health_history: 0.0,
            ..GompertzMortality::default()
        };

        // 7. 分步插入 Component（hecs 平面元组 ≤16 上限）
        let entity = self.ecs.spawn((
            Position(position),
            Rotation::default(),
            EntityKind::Creature,
            LodLevel::default(),
            bf,
            need_sens,
            chronotype,
            social_presence,
            cognitive_style,
            aesthetic,
            biases,
            sex,
            age,
            life_stage,
        ));
        // 第二批：Vitals + Movement + Needs + Emotion + Goal + GrowthNeeds + Gompertz + RelationHandle
        self.ecs
            .insert(
                entity,
                (
                    Vitals::default(),
                    RegenState::default(),
                    Movement::default(),
                    Needs::default(),
                    emotion,
                    Goal::default(),
                    GrowthNeeds::default(),
                    gmort,
                    RelationHandle,
                    Wallet::from_seed(seed),
                    EconomicCognition::default(),
                ),
            )
            .expect("NPC entity should exist after spawn");
        entity
    }

    /// Initialize 5×5 VoxelChunk grid + submit first batch of rayon extraction jobs.
    fn init_voxel_grid(&mut self) {
        const CHUNK_SIZE: f64 = 16.0;
        const VOXEL_SIZE: f64 = 0.5;
        let grid_radius: i32 = 2;
        let pcx: i32 = 0;
        let pcz: i32 = 0;

        // Pass 1: scan 25 chunk centers → unified wy (all chunks share same vertical origin)
        let mut y_min = f64::MAX;
        let mut y_max = f64::MIN;
        for dx in -grid_radius..=grid_radius {
            for dz in -grid_radius..=grid_radius {
                let wx = (pcx + dx) as f64 * CHUNK_SIZE;
                let wz = (pcz + dz) as f64 * CHUNK_SIZE;
                let h = self.terrain.height_at(WorldPos {
                    x: wx + 8.0,
                    y: 0.0,
                    z: wz + 8.0,
                }) as f64;
                y_min = y_min.min(h);
                y_max = y_max.max(h);
            }
        }
        let wy = y_min - 4.0; // no .max(0.0) — allow terrain below sea level
        let total_h = (y_max - wy + 12.0).max(16.0);
        let vy = ((total_h / VOXEL_SIZE).ceil() as u32).max(32);
        godot_print!(
            "VoxelGrid unified: wy={:.0} vy={}  y=[{:.0}, {:.0}]",
            wy,
            vy,
            y_min,
            y_max
        );
        self.voxel_wy = wy;
        self.voxel_vy = vy;

        // Pass 2: create all 25 chunks with unified wy
        for dx in -grid_radius..=grid_radius {
            for dz in -grid_radius..=grid_radius {
                let cx = pcx + dx;
                let cz = pcz + dz;
                let wx = cx as f64 * CHUNK_SIZE;
                let wz = cz as f64 * CHUNK_SIZE;

                let mut vc = VoxelChunk::new_alloc();
                vc.bind_mut().set_world_origin(wx, wy, wz);
                if let Some(ref voxel_mat) = self.voxel_material {
                    vc.bind_mut()
                        .set_terrain_material(voxel_mat.clone().upcast());
                }
                if let Some(ref mut parent) = self.terrain_parent {
                    parent.add_child(&vc.clone().upcast::<Node>());
                }
                self.voxel_chunks.insert((cx, cz), vc);
                self.submit_voxel_job(cx, cz, wx, wy, wz, vy);
            }
        }
        self.voxel_center = (pcx, pcz);
    }

    /// Submit a single transvoxel_extract job to rayon background thread pool.
    fn submit_voxel_job(&mut self, cx: i32, cz: i32, wx: f64, wy: f64, wz: f64, vy: u32) {
        if self.vx_in_flight.contains(&(cx, cz)) {
            return;
        }
        self.vx_in_flight.insert((cx, cz));

        let tx = self.vx_result_tx.clone();
        let noise_arc = self.terrain.noise_arc();
        let biome = self.terrain.biome_classifier.clone();
        let material_colors = self.material_colors.clone();
        let voxel_size = 0.5f64;
        let vx = 32u32;
        let vz = 32u32;
        // ★ 捕获 EditDensity 层（若存在）——CoW 快照，Clone 成本极低
        let edit_layer = self.terrain.edit_density_layer(voxel_size);

        rayon::spawn(move || {
            // Construct surface-only density stack (no CaveDensity).
            // LOD 0 matches clipmap LOD 1-7 — both use pure 2D heightfield, no caves.
            // Use biome classifier for material_at — same color as clipmap.
            let mk_base = || -> TerrainBaseDensity {
                let b = TerrainBaseDensity::new(noise_arc.clone());
                if let Some(ref bc) = biome {
                    b.with_biomes(bc.clone())
                } else {
                    b
                }
            };
            let surface_layer: Arc<dyn DensityProvider> = Arc::new(mk_base());
            let mut stack = DensityStack::new();
            stack.push(surface_layer);
            // ★ 插入 EditDensity 层（priority 10，覆盖基底）
            if let Some(ref edit) = edit_layer {
                stack.push(Arc::new(edit.clone()));
            }
            let base_layer = mk_base();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                transvoxel_extract(
                    &stack,
                    &base_layer,
                    wx,
                    wy,
                    wz,
                    vx,
                    vy,
                    vz,
                    voxel_size,
                    0, // no transition faces
                    &material_colors,
                )
            }));
            match result {
                Ok(mesh) => {
                    let _ = tx.send(VoxelResult {
                        cx,
                        cz,
                        mesh,
                        panicked: false,
                    });
                }
                Err(_) => {
                    let _ = tx.send(VoxelResult {
                        cx,
                        cz,
                        mesh: woworld_worldgen::TerrainMeshData {
                            vertices: Vec::new(),
                            normals: Vec::new(),
                            colors: Vec::new(),
                            indices: Vec::new(),
                        },
                        panicked: true,
                    });
                }
            }
        });
    }

    /// Upload a completed TerrainMeshData to a VoxelChunk as ArrayMesh.
    fn upload_voxel_mesh(vc: &mut Gd<VoxelChunk>, mesh: &woworld_worldgen::TerrainMeshData) {
        use godot::builtin::{Array, PackedColorArray, PackedInt32Array, PackedVector3Array};
        use godot::classes::mesh::PrimitiveType;
        use godot::classes::ArrayMesh;

        if mesh.vertices.is_empty() {
            let (ox, oy, oz) = vc.bind().origin_tuple();
            godot_print!(
                "Voxel empty @ ({:.0},{:.0},{:.0}) — all-air chunk",
                ox,
                oy,
                oz
            );
            vc.bind_mut().set_terrain_mesh(None);
            return;
        }

        // transvoxel_extract returns world-space vertices; convert to local space
        // (VoxelChunk Node3D position provides the world offset)
        let (ox, oy, oz) = vc.bind().origin_tuple();

        let mut am = ArrayMesh::new_gd();
        let mut vertices = PackedVector3Array::new();
        let mut normals_packed = PackedVector3Array::new();
        let mut colors_packed = PackedColorArray::new();

        for i in 0..mesh.vertices.len() {
            let v = mesh.vertices[i];
            vertices.push(Vector3::new(
                v.x - ox as f32,
                v.y - oy as f32,
                v.z - oz as f32,
            ));
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

        // Diagnostic: NaN, Inf, index OOB (disabled — 解除注释以调试 Voxel mesh)
        // let nv = mesh.vertices.len() as u32;
        // let nan_v  = mesh.vertices.iter().filter(|v| v.x.is_nan() || v.y.is_nan() || v.z.is_nan()).count();
        // let inf_v  = mesh.vertices.iter().filter(|v| v.x.is_infinite() || v.y.is_infinite() || v.z.is_infinite()).count();
        // let nan_n  = mesh.normals.iter().filter(|n| n.x.is_nan() || n.y.is_nan() || n.z.is_nan()).count();
        // let oob    = mesh.indices.iter().filter(|&&i| i >= nv).count();
        // let mid    = mesh.vertices.len() / 2;
        // let sn     = if mid < mesh.normals.len() { (mesh.normals[mid].x, mesh.normals[mid].y, mesh.normals[mid].z) } else { (0.0f32, 0.0, 0.0) };
        // let sc     = if mid < mesh.colors.len() { (mesh.colors[mid].x, mesh.colors[mid].y, mesh.colors[mid].z) } else { (0.0f32, 0.0, 0.0) };
        // godot_print!(
        //     "Voxel @({:.0},{:.0},{:.0}) {}v {}t NaN(v={} i={} n={}) OOB={}  n[{}]=({:.3},{:.3},{:.3}) c[{}]=({:.3},{:.3},{:.3})",
        //     ox, oy, oz, mesh.vertices.len(), mesh.indices.len()/3,
        //     nan_v, inf_v, nan_n, oob, mid,
        //     sn.0, sn.1, sn.2,
        //     mid, sc.0, sc.1, sc.2,
        // );

        vc.bind_mut().set_terrain_mesh(Some(am));
    }
}
