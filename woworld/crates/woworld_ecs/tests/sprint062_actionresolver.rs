//! Sprint-062 集成验证：ActionResolver 完整管线端到端。
//!
//! 按 WorldDriver::process Block A0 相同顺序运行（含 Sprint-062 三新系统）：
//!   player_input → coyote → stamina → movement_mode → input_buffer
//!   → action_resolver → interact_context → action(+flush) → movement
//!
//! 断言（激活休眠的 Block A0）：
//!   A. 玩家方向输入 → player_input 写 CMoveIntent → 实体沿世界空间前进（真实位移）。
//!   B. 玩家按跳跃键 → action_resolver 缓冲 → input_buffer drain → action_system
//!      启动动作（CActiveAction 变 Some）——证明 InputAction→ActionRequest→ActionController 端到端。
//!
//! Godot 输入桥接（input_bridge.gd）属下一冲刺——此处直接构造 InputState 驱动。

use glam::{Quat, Vec2, Vec3};
use woworld_core::input::{HotbarConfig, InputAction, InputState};
use woworld_core::material::{Medium, SurfaceMaterial};
use woworld_core::movement::{MovementState, Pace, Stance};
use woworld_core::player::ControlMode;
use woworld_core::spatial::TerrainQuery;
use woworld_core::types::{TerrainHit, WorldPos};

use woworld_ecs::components::action_state::{CActionRequestBuf, CActiveAction};
use woworld_ecs::components::input_state::CInputBuffer;
use woworld_ecs::components::movement_state::{
    CMoveIntent, CMovementControl, CMovementRecovery, CMovementState, CPrevMovementState,
};
use woworld_ecs::components::player::{ControlModeComponent, PlayerComponent};
use woworld_ecs::components::transform::{Position, Rotation, Velocity};
use woworld_ecs::resources::action_instance_counter::ActionInstanceCounter;
use woworld_ecs::resources::action_registry::ActionRegistry;
use woworld_ecs::resources::interact::{ActionWheelData, NearbyInteractables};
use woworld_ecs::resources::movement_profile_registry::MovementProfileRegistry;

struct FlatGround;
impl TerrainQuery for FlatGround {
    fn height_at(&self, _p: WorldPos) -> f32 {
        0.0
    }
    fn normal_at(&self, _p: WorldPos) -> Vec3 {
        Vec3::Y
    }
    fn terrain_raycast(&self, _o: WorldPos, _d: Vec3, _m: f32) -> Option<TerrainHit> {
        None
    }
    fn density_at(&self, _p: WorldPos) -> f32 {
        0.0
    }
    fn is_walkable(&self, _p: WorldPos) -> bool {
        true
    }
    fn surface_material_at(&self, _p: WorldPos) -> SurfaceMaterial {
        SurfaceMaterial::Grass
    }
    fn medium_at(&self, _p: WorldPos) -> Medium {
        Medium::Air
    }
    fn light_level_at(&self, _p: WorldPos) -> f32 {
        1.0
    }
    fn sample_horizon(&self, _p: WorldPos, _d: &[Vec3]) -> Vec<f32> {
        vec![]
    }
}

/// 完整的角色控制器玩家实体——挂 Block A0 全系统所需组件。
fn spawn_player(world: &mut hecs::World) -> hecs::Entity {
    let ms = MovementState {
        stance: Stance::Standing,
        pace: Pace::Running,
        ..Default::default()
    };
    world.spawn((
        PlayerComponent::default(),
        ControlModeComponent {
            mode: ControlMode::Manual,
        },
        CActiveAction::default(),
        CActionRequestBuf::default(),
        CInputBuffer::default(),
        CMovementControl::default(),
        CMoveIntent::default(),
        CMovementState(ms),
        CPrevMovementState(ms),
        CMovementRecovery::default(),
        Position(Vec3::ZERO),
        Velocity(Vec3::ZERO),
        Rotation(Quat::IDENTITY),
    ))
}

/// 逐字复刻 Block A0 顺序（含 Sprint-062 三新系统）。
#[allow(clippy::too_many_arguments)]
fn run_frame(
    world: &mut hecs::World,
    input: &InputState,
    hotbar: &HotbarConfig,
    nearby: &NearbyInteractables,
    wheel: &mut ActionWheelData,
    registry: &ActionRegistry,
    counter: &mut ActionInstanceCounter,
    events: &mut woworld_ecs::events::EventChannel<woworld_core::action::ActionLifecycleEvent>,
    profiles: &MovementProfileRegistry,
    terrain: &dyn TerrainQuery,
    dt: f32,
    now: f32,
) {
    use woworld_ecs::systems::action::action_system::action_system;
    use woworld_ecs::systems::input::action_resolver_system::action_resolver_system;
    use woworld_ecs::systems::input::coyote_time_system::coyote_time_system;
    use woworld_ecs::systems::input::input_buffer_system::input_buffer_system;
    use woworld_ecs::systems::input::interact_context_system::interact_context_system;
    use woworld_ecs::systems::movement::movement_mode_system::movement_mode_system;
    use woworld_ecs::systems::movement::movement_system::movement_system;
    use woworld_ecs::systems::movement::stamina_gate_system::stamina_gate_system;
    use woworld_ecs::systems::player::player_input::player_input_system;

    events.begin_frame();
    player_input_system(world, input);
    coyote_time_system(world, dt, terrain);
    stamina_gate_system(world, dt);
    movement_mode_system(world, terrain);
    input_buffer_system(world, dt);
    action_resolver_system(world, input, hotbar, registry, now);
    interact_context_system(world, input, nearby, wheel);
    action_system(world, dt, registry, counter, events, terrain);
    events.mid_phase_flush();
    movement_system(world, dt, terrain, profiles);
}

fn load_registries() -> (ActionRegistry, MovementProfileRegistry) {
    let mut registry = ActionRegistry::new();
    registry
        .load_from_toml(include_str!("../../../assets/action_registry.toml"))
        .expect("action_registry.toml 应能解析");
    let mut profiles = MovementProfileRegistry::new();
    profiles
        .load_from_toml(include_str!("../../../assets/movement_profiles.toml"))
        .expect("movement_profiles.toml 应能解析");
    (registry, profiles)
}

#[test]
fn sprint062_player_input_moves_entity() {
    let mut world = hecs::World::new();
    let terrain = FlatGround;
    let (registry, profiles) = load_registries();
    let mut counter = ActionInstanceCounter::new();
    let mut events = woworld_ecs::events::EventChannel::new();
    let hotbar = HotbarConfig::new();
    let nearby = NearbyInteractables::new();
    let mut wheel = ActionWheelData::new();

    let e = spawn_player(&mut world);

    // 玩家推方向 +X（相机 IDENTITY，move_direction.x=1 → 世界 +X）
    let mut input = InputState::default();
    input.move_direction = Vec2::new(1.0, 0.0);

    let dt = 0.016_f32;
    let mut now = 0.0_f32;
    for _ in 0..60 {
        now += dt;
        run_frame(
            &mut world,
            &input,
            &hotbar,
            &nearby,
            &mut wheel,
            &registry,
            &mut counter,
            &mut events,
            &profiles,
            &terrain,
            dt,
            now,
        );
    }

    let p = world.get::<&Position>(e).expect("玩家实体应存在 Position");
    assert!(
        p.0.x > 1.0,
        "玩家方向输入应使实体沿 +X 前进 >1m，实得 x={}",
        p.0.x
    );
}

#[test]
fn sprint062_jump_input_starts_action() {
    let mut world = hecs::World::new();
    let terrain = FlatGround;
    let (registry, profiles) = load_registries();
    let mut counter = ActionInstanceCounter::new();
    let mut events = woworld_ecs::events::EventChannel::new();
    let hotbar = HotbarConfig::new();
    let nearby = NearbyInteractables::new();
    let mut wheel = ActionWheelData::new();

    let e = spawn_player(&mut world);

    // 玩家按住跳跃键（无 begin_frame → 每帧视为按下，简化测试）
    let mut input = InputState::default();
    input.press(InputAction::Jump);

    let dt = 0.016_f32;
    let mut now = 0.0_f32;
    // jump 可缓冲：resolver→CInputBuffer→(下一帧)input_buffer→action_system。
    // 数帧后应启动动作。
    for _ in 0..5 {
        now += dt;
        run_frame(
            &mut world,
            &input,
            &hotbar,
            &nearby,
            &mut wheel,
            &registry,
            &mut counter,
            &mut events,
            &profiles,
            &terrain,
            dt,
            now,
        );
    }

    let active = world
        .get::<&CActiveAction>(e)
        .expect("玩家实体应存在 CActiveAction");
    assert!(
        active.0.is_some(),
        "跳跃输入应经 resolver→buffer→action 启动一个动作（CActiveAction=Some），实得 None"
    );
    assert_eq!(
        active.0.as_ref().unwrap().action_id,
        ActionRegistry::id_of("jump"),
        "启动的动作应为 jump"
    );
}

#[test]
fn sprint062_auto_mode_no_action_from_input() {
    // Auto 模式：玩家输入不应产生动作/位移（GOAP 驱动，绞杀者）
    let mut world = hecs::World::new();
    let terrain = FlatGround;
    let (registry, profiles) = load_registries();
    let mut counter = ActionInstanceCounter::new();
    let mut events = woworld_ecs::events::EventChannel::new();
    let hotbar = HotbarConfig::new();
    let nearby = NearbyInteractables::new();
    let mut wheel = ActionWheelData::new();

    let e = spawn_player(&mut world);
    // 改为 Auto
    world.get::<&mut ControlModeComponent>(e).unwrap().mode = ControlMode::Auto;

    let mut input = InputState::default();
    input.move_direction = Vec2::new(1.0, 0.0);
    input.press(InputAction::Jump);

    let dt = 0.016_f32;
    let mut now = 0.0_f32;
    for _ in 0..5 {
        now += dt;
        run_frame(
            &mut world,
            &input,
            &hotbar,
            &nearby,
            &mut wheel,
            &registry,
            &mut counter,
            &mut events,
            &profiles,
            &terrain,
            dt,
            now,
        );
    }

    let active = world.get::<&CActiveAction>(e).unwrap();
    assert!(
        active.0.is_none(),
        "Auto 模式玩家输入不应启动动作（GOAP 驱动）"
    );
}
