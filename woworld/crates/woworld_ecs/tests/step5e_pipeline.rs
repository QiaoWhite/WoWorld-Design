//! Step 5e 集成验证：角色控制器 6 系统管线端到端。
//!
//! 按 WorldDriver::process 的 Block A0 相同顺序运行：
//!   coyote → stamina → movement_mode → input_buffer → action(+flush) → movement
//! 断言带 CMoveIntent(+X) 的实体在若干帧后沿 +X 前进——证明连续执行层经
//! 完整管线产生位移，且六系统同序共存不 panic。
//!
//! 这是 Godot 侧冒烟测试实体的 Rust 可运行镜像（CI 可跑，无需引擎）。

use glam::Vec3;
use woworld_core::material::{Medium, SurfaceMaterial};
use woworld_core::movement::{MovementState, Pace, Stance};
use woworld_core::spatial::TerrainQuery;
use woworld_core::types::{TerrainHit, WorldPos};

use woworld_ecs::components::movement_state::{
    CMoveIntent, CMovementControl, CMovementRecovery, CMovementState, CPrevMovementState,
};
use woworld_ecs::components::transform::{Position, Velocity};

/// 平坦可行走地面（Grounded），用于隔离测试。
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

#[test]
fn step5e_full_pipeline_moves_entity_forward() {
    use woworld_core::action::ActionLifecycleEvent;
    use woworld_ecs::events::EventChannel;
    use woworld_ecs::resources::action_instance_counter::ActionInstanceCounter;
    use woworld_ecs::resources::action_registry::ActionRegistry;
    use woworld_ecs::resources::movement_profile_registry::MovementProfileRegistry;

    let mut world = hecs::World::new();
    let terrain = FlatGround;

    // 真实 TOML 资产（同时二次验证解析）
    let mut profiles = MovementProfileRegistry::new();
    profiles
        .load_from_toml(include_str!("../../../assets/movement_profiles.toml"))
        .expect("movement_profiles.toml 应能解析");
    let mut registry = ActionRegistry::new();
    registry
        .load_from_toml(include_str!("../../../assets/action_registry.toml"))
        .expect("action_registry.toml 应能解析");
    let mut counter = ActionInstanceCounter::new();
    let mut events: EventChannel<ActionLifecycleEvent> = EventChannel::new();

    let ms = MovementState {
        stance: Stance::Standing,
        pace: Pace::Running,
        ..Default::default()
    };
    let intent = CMoveIntent {
        direction: Vec3::new(1.0, 0.0, 0.0),
        ..Default::default()
    };
    let e = world.spawn((
        Position(Vec3::ZERO),
        Velocity(Vec3::ZERO),
        CMovementState(ms),
        CPrevMovementState(ms),
        CMovementRecovery::default(),
        intent,
        CMovementControl::default(),
    ));

    let dt = 0.016_f32;
    for _ in 0..60 {
        // 与 Block A0 逐字同序（此实体不带 input/action 组件 → 那两步对其 no-op，
        // 但仍执行以证明六系统同帧共存无 panic）。
        events.begin_frame();
        woworld_ecs::systems::input::coyote_time_system::coyote_time_system(
            &mut world, dt, &terrain,
        );
        woworld_ecs::systems::movement::stamina_gate_system::stamina_gate_system(&mut world, dt);
        woworld_ecs::systems::movement::movement_mode_system::movement_mode_system(
            &mut world, &terrain,
        );
        woworld_ecs::systems::input::input_buffer_system::input_buffer_system(&mut world, dt);
        woworld_ecs::systems::action::action_system::action_system(
            &mut world,
            dt,
            &registry,
            &mut counter,
            &mut events,
            &terrain,
        );
        events.mid_phase_flush();
        woworld_ecs::systems::movement::movement_system::movement_system(
            &mut world, dt, &terrain, &profiles,
        );
    }

    let p = world.get::<&Position>(e).expect("测试实体应存在 Position");
    assert!(
        p.0.x > 1.0,
        "实体应沿 +X 前进 >1m（Running），实得 x={}",
        p.0.x
    );
    // Y 应贴地（FlatGround height=0）
    assert!(p.0.y.abs() < 0.01, "应贴地 y≈0，实得 y={}", p.0.y);
}
