//! Sprint-066 集成测试 — 落地预输入（Jump Buffer）端到端（I3）
//!
//! 验证：空中按 Jump → 缓冲经 input_buffer 物理重检存活多帧（airborne 时
//! physics_req=Grounded 不满足）→ 落地帧 effective_loco 转 Grounded → drain →
//! action_controller 真正启动跳跃（CActiveAction 变 Some）。
//!
//! 关键：I3 完全由 I2 的物理重检涌现——无独立着地特判分支。
//!
//! 参见: `WoWorld-Design/.../角色控制器/008-手感系统.md` §五/§八

use glam::{Quat, Vec3};
use woworld_core::action::{
    ActionDef, ActionKind, ActionParams, ActionRequest, ActionSource, CommitmentLevel,
};
use woworld_core::input::{BufferPriority, BufferedInput};
use woworld_core::kinematics::PhysicsRequirement;
use woworld_core::material::{Medium, SurfaceMaterial};
use woworld_core::movement::{MovementState, Pace, Stance};
use woworld_core::spatial::TerrainQuery;
use woworld_core::types::{TerrainHit, WorldPos};

use woworld_ecs::components::action_state::{CActionRequestBuf, CActiveAction};
use woworld_ecs::components::input_state::CInputBuffer;
use woworld_ecs::components::movement_state::{
    CMoveIntent, CMovementControl, CMovementRecovery, CMovementState, CPrevMovementState,
};
use woworld_ecs::components::player::PlayerComponent;
use woworld_ecs::components::transform::{Position, Rotation, Velocity};
use woworld_ecs::events::EventChannel;
use woworld_ecs::resources::action_instance_counter::ActionInstanceCounter;
use woworld_ecs::resources::action_registry::ActionRegistry;

/// 可参数化 walkable 的地形 mock（walkable=false 模拟空中）。
struct Ground {
    walkable: bool,
}
impl TerrainQuery for Ground {
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
        self.walkable
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

/// registry：单个 "jump" 动作——bufferable、physics_req=Grounded、windup 可观测。
fn jump_registry() -> ActionRegistry {
    let mut r = ActionRegistry::new();
    let def = ActionDef {
        name: "jump".to_string(),
        category: "Movement".to_string(),
        kind: ActionKind::Discrete,
        priority: 40,
        commitment: CommitmentLevel::Soft,
        windup_ms: 50,
        active_ms: 100,
        recovery_ms: 50,
        cancel_set: vec![],
        cancel_window_ms: 0,
        bufferable: true,
        buffer_window_ms: 500, // 足够长，不在测试的空中帧内过期
        physics_req: PhysicsRequirement::Grounded,
        movement_lock: Default::default(),
        rotation_lock: Default::default(),
        interrupt_on_move: false,
        sustain_drain: None,
        release_behavior: None,
        overextend_threshold_secs: None,
        critical_threshold_secs: None,
    };
    r.register(ActionRegistry::id_of("jump"), def);
    r
}

/// 完整玩家实体——action_system + input_buffer_system 所需组件。
fn spawn_player(world: &mut hecs::World) -> hecs::Entity {
    let ms = MovementState {
        stance: Stance::Standing,
        pace: Pace::Running,
        ..Default::default()
    };
    world.spawn((
        PlayerComponent::default(),
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

/// 直接向缓冲注入一个 Jump（模拟 action_resolver 在空中按键时的入队结果）。
fn push_jump_to_buffer(world: &mut hecs::World, e: hecs::Entity, now: f32) {
    let mut buf = world.get::<&mut CInputBuffer>(e).unwrap();
    buf.push_bounded(BufferedInput::new(
        ActionRequest {
            action_id: ActionRegistry::id_of("jump"),
            priority: 40,
            source: ActionSource::Player,
            params: ActionParams::default(),
        },
        now,
        500.0,
        BufferPriority::Movement,
    ));
}

/// 跑一帧 input_buffer + action_system（省略与本测试无关的其余系统）。
fn tick(
    world: &mut hecs::World,
    registry: &ActionRegistry,
    counter: &mut ActionInstanceCounter,
    events: &mut EventChannel<woworld_core::action::ActionLifecycleEvent>,
    terrain: &dyn TerrainQuery,
    dt: f32,
    now: f32,
) {
    events.begin_frame();
    woworld_ecs::systems::input::input_buffer_system::input_buffer_system(
        world, terrain, registry, now,
    );
    woworld_ecs::systems::action::action_system::action_system(
        world, dt, registry, counter, events, terrain,
    );
}

#[test]
fn jump_buffered_in_air_fires_on_landing() {
    let registry = jump_registry();
    let mut counter = ActionInstanceCounter::default();
    let mut events = EventChannel::default();
    let airborne = Ground { walkable: false };
    let grounded = Ground { walkable: true };

    let mut world = hecs::World::new();
    let e = spawn_player(&mut world);

    let dt = 0.016_f32;
    let mut now = 0.0_f32;

    // ── 空中按 Jump（缓冲注入）──
    push_jump_to_buffer(&mut world, e, now);

    // ── 空中飞行若干帧：Jump 物理不可行 → 留缓冲，不启动动作 ──
    for _ in 0..3 {
        now += dt;
        tick(
            &mut world,
            &registry,
            &mut counter,
            &mut events,
            &airborne,
            dt,
            now,
        );
    }
    assert!(
        world.get::<&CActiveAction>(e).unwrap().0.is_none(),
        "空中：Jump 不应启动（physics_req=Grounded 不满足）"
    );
    assert_eq!(
        world.get::<&CInputBuffer>(e).unwrap().entries.len(),
        1,
        "空中：Jump 必须留在缓冲等待落地"
    );

    // ── 落地帧：effective_loco 转 Grounded → drain → 启动跳跃 ──
    now += dt;
    tick(
        &mut world,
        &registry,
        &mut counter,
        &mut events,
        &grounded,
        dt,
        now,
    );

    let active = world.get::<&CActiveAction>(e).unwrap();
    assert!(
        active.0.is_some(),
        "落地：缓冲的 Jump 应被消费并启动 CActiveAction（落地预输入 I3）"
    );
    assert_eq!(
        active.0.as_ref().unwrap().action_id,
        ActionRegistry::id_of("jump"),
        "启动的应是 jump 动作"
    );
    assert!(
        world.get::<&CInputBuffer>(e).unwrap().entries.is_empty(),
        "落地：缓冲应已清空（Jump 已消费）"
    );
}

#[test]
fn jump_expires_if_never_lands() {
    // 落地预输入有窗口：空中久到超过 buffer_window（500ms）→ Jump 过期丢弃，不再起跳。
    let registry = jump_registry();
    let mut counter = ActionInstanceCounter::default();
    let mut events = EventChannel::default();
    let airborne = Ground { walkable: false };
    let grounded = Ground { walkable: true };

    let mut world = hecs::World::new();
    let e = spawn_player(&mut world);

    let dt = 0.05_f32;
    let mut now = 0.0_f32;
    push_jump_to_buffer(&mut world, e, now); // expires_at = 0.5

    // 空中 12 帧 × 0.05 = 0.6s > 0.5s 窗 → 过期
    for _ in 0..12 {
        now += dt;
        tick(
            &mut world,
            &registry,
            &mut counter,
            &mut events,
            &airborne,
            dt,
            now,
        );
    }
    assert!(
        world.get::<&CInputBuffer>(e).unwrap().entries.is_empty(),
        "超过缓冲窗 → Jump 应过期丢弃"
    );

    // 此时落地——不应再起跳（缓冲已空）
    now += dt;
    tick(
        &mut world,
        &registry,
        &mut counter,
        &mut events,
        &grounded,
        dt,
        now,
    );
    assert!(
        world.get::<&CActiveAction>(e).unwrap().0.is_none(),
        "过期后落地不应起跳"
    );
}

/// 候选A 激活验证——玩家配方带 CCoyoteTime + CInputFeelConfig → 踩空后土狼窗内仍可起跳。
/// 复刻 Godot 玩家配方（terrain_chunk.rs），守护"配方漏挂手感组件"回归。
#[test]
fn coyote_grace_jump_after_walking_off_edge() {
    use woworld_ecs::components::input_state::{CCoyoteTime, CInputFeelConfig};
    use woworld_ecs::systems::input::coyote_time_system::coyote_time_system;

    let registry = jump_registry();
    let mut counter = ActionInstanceCounter::default();
    let mut events = EventChannel::default();
    let airborne = Ground { walkable: false };

    let mut world = hecs::World::new();
    let e = spawn_player(&mut world);
    // 补齐候选A 手感组件（与 Godot 玩家 spawn 配方一致）
    world
        .insert(e, (CCoyoteTime::default(), CInputFeelConfig::default()))
        .unwrap();

    let dt = 0.016_f32;

    // ── 踩空：prev grounded(special=None) + 当前 airborne(SkyVoid=不可行走) + special 非 Jumping ──
    //   coyote_time_system 检测 was_grounded→not_walkable → 设 remaining = coyote_time(0.15)。
    coyote_time_system(&mut world, dt, &airborne);
    assert!(
        world.get::<&CCoyoteTime>(e).unwrap().remaining > 0.0,
        "踩空应触发土狼窗（remaining>0）"
    );

    // ── 土狼窗内按跳跃：input_buffer/action 的 effective_loco 上调 Grounded → 空中仍接受起跳 ──
    push_jump_to_buffer(&mut world, e, 0.0);
    tick(
        &mut world,
        &registry,
        &mut counter,
        &mut events,
        &airborne,
        dt,
        0.05,
    );

    assert!(
        world.get::<&CActiveAction>(e).unwrap().0.is_some(),
        "土狼窗内应接受起跳（coyote grace-jump，虽在空中）"
    );
}
