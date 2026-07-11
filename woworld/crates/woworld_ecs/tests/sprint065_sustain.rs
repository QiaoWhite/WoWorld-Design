//! Sprint-065 集成验证：持续动作（block）端到端运行时。
//!
//! 用**真实 assets/action_registry.toml** 的 block 定义，走多系统管线：
//!   coyote → stamina_gate → action_system
//! 证明：请求 → controller 接受 → sustain drain 消耗 Vitals → 松键 Complete
//! 的完整生命周期在真实 TOML 数据驱动下闭环，且与其它 Block A0 系统同帧共存不 panic。
//!
//! 这是 Godot 侧防御动作的 Rust 可运行镜像（CI 可跑，无需引擎）。

use glam::Vec3;
use woworld_core::action::{
    action_priority, ActionId, ActionLifecycleEvent, ActionParams, ActionRequest, ActionSource,
};
use woworld_core::material::{Medium, SurfaceMaterial};
use woworld_core::movement::MovementState;
use woworld_core::spatial::TerrainQuery;
use woworld_core::types::{TerrainHit, WorldPos};

use woworld_ecs::components::action_state::{CActionRequestBuf, CActiveAction, CPendingFollowUp};
use woworld_ecs::components::input_state::CCoyoteTime;
use woworld_ecs::components::movement_state::{
    CMovementControl, CMovementState, CPrevMovementState,
};
use woworld_ecs::components::transform::Position;
use woworld_ecs::components::vitals::Vitals;
use woworld_ecs::events::EventChannel;
use woworld_ecs::resources::action_instance_counter::ActionInstanceCounter;
use woworld_ecs::resources::action_registry::ActionRegistry;

/// 平坦可行走地面（Grounded）。
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

fn run_frame(
    world: &mut hecs::World,
    dt: f32,
    registry: &ActionRegistry,
    counter: &mut ActionInstanceCounter,
    events: &mut EventChannel<ActionLifecycleEvent>,
    terrain: &FlatGround,
) {
    events.begin_frame();
    woworld_ecs::systems::input::coyote_time_system::coyote_time_system(world, dt, terrain);
    woworld_ecs::systems::movement::stamina_gate_system::stamina_gate_system(world, dt);
    woworld_ecs::systems::action::action_system::action_system(
        world, dt, registry, counter, events, terrain,
    );
    events.mid_phase_flush();
}

#[test]
fn sprint065_block_lifecycle_end_to_end() {
    let mut world = hecs::World::new();
    let terrain = FlatGround;

    // 真实 TOML 资产（同时二次验证 006 示例可解析）
    let mut registry = ActionRegistry::new();
    registry
        .load_from_toml(include_str!("../../../assets/action_registry.toml"))
        .expect("action_registry.toml 应能解析");
    let mut counter = ActionInstanceCounter::new();
    let mut events: EventChannel<ActionLifecycleEvent> = EventChannel::new();

    let block_id = ActionId::from_key("block");
    let ms = MovementState::default();
    let e = world.spawn((
        Position(Vec3::ZERO),
        CMovementState(ms),
        CPrevMovementState(ms),
        CCoyoteTime::default(),
        CMovementControl::default(),
        CActiveAction::default(),
        CActionRequestBuf::default(),
        Vitals::default(),
        CPendingFollowUp::default(),
    ));

    let dt = 0.016_f32;

    // ── 帧 1：注入防御按下请求 → controller 接受 ──
    world
        .get::<&mut CActionRequestBuf>(e)
        .unwrap()
        .0
        .push(ActionRequest {
            action_id: block_id,
            priority: action_priority::BLOCK,
            source: ActionSource::Player,
            params: ActionParams::default(),
        });
    run_frame(
        &mut world,
        dt,
        &registry,
        &mut counter,
        &mut events,
        &terrain,
    );
    assert!(
        world.get::<&CActiveAction>(e).unwrap().0.is_some(),
        "防御请求应被接受为活动动作"
    );

    // ── 帧 2-40：按住防御，sustain drain 消耗 Stamina ──
    for _ in 0..40 {
        run_frame(
            &mut world,
            dt,
            &registry,
            &mut counter,
            &mut events,
            &terrain,
        );
    }
    let stamina_after_hold = world.get::<&Vitals>(e).unwrap().stamina;
    assert!(
        stamina_after_hold < 100.0,
        "按住防御应消耗 Stamina，实得 {}",
        stamina_after_hold
    );
    assert!(
        world.get::<&CActiveAction>(e).unwrap().0.is_some(),
        "未松键前防御应持续"
    );

    // ── 松键：注入 RELEASE 请求 → Complete ──
    world
        .get::<&mut CActionRequestBuf>(e)
        .unwrap()
        .0
        .push(ActionRequest {
            action_id: block_id,
            priority: action_priority::RELEASE,
            source: ActionSource::Player,
            params: ActionParams::default(),
        });
    run_frame(
        &mut world,
        dt,
        &registry,
        &mut counter,
        &mut events,
        &terrain,
    );

    assert!(
        world.get::<&CActiveAction>(e).unwrap().0.is_none(),
        "松键后防御应结束"
    );
    assert!(
        events
            .read()
            .iter()
            .any(|ev| matches!(ev, ActionLifecycleEvent::Completed { .. })),
        "松键应发出 Completed 事件（ReleaseBehavior::Complete）"
    );
}
