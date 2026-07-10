//! Sprint-064 集成测试——跳跃全弧线（起跳→上升→落地→pace 恢复）
//!
//! 逐字复刻 Block A0 中跳跃相关系统顺序：`movement_mode → jump_launch → movement`。
//! 验证回归修复：
//!   A. 跳跃升到 >1m（不被上升段 1m 可行走带过早截断）。
//!   B. 落回地面。
//!   C. **落地后 pace 恢复为起跳前的 Running**（恢复栈 push/pop）——否则卡 Still、
//!      max_speed=0、无法水平移动（用户实机报告的回归）。
//!   D. 落地后能继续水平移动（pos.x 前进）。

use glam::{Quat, Vec3};
use woworld_core::action::{
    ActionInstanceId, ActionPhase, ActiveAction, CommitmentLevel, SustainPhase,
};
use woworld_core::material::{Medium, SurfaceMaterial};
use woworld_core::movement::{MovementState, Pace, Stance};
use woworld_core::spatial::TerrainQuery;
use woworld_core::types::{TerrainHit, WorldPos};

use woworld_ecs::components::action_state::CActiveAction;
use woworld_ecs::components::movement_state::{
    CMoveIntent, CMovementControl, CMovementRecovery, CMovementState, CPrevMovementState,
};
use woworld_ecs::components::transform::{Position, Rotation, Velocity};
use woworld_ecs::resources::action_registry::ActionRegistry;
use woworld_ecs::resources::movement_profile_registry::MovementProfileRegistry;
use woworld_ecs::systems::movement::jump_launch_system::jump_launch_system;
use woworld_ecs::systems::movement::movement_mode_system::movement_mode_system;
use woworld_ecs::systems::movement::movement_system::movement_system;

/// 平地：高度 0，处处可行走。
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

fn active_jump() -> CActiveAction {
    CActiveAction(Some(ActiveAction {
        instance: ActionInstanceId(0),
        action_id: ActionRegistry::id_of("jump"),
        phase: ActionPhase::Active,
        commitment: CommitmentLevel::Hard,
        elapsed: 0.06,
        cancel_window_open: false,
        resource_drain_rate: 0.0,
        sustain_phase: SustainPhase::Normal,
    }))
}

#[test]
fn sprint064_jump_full_arc_preserves_pace() {
    let mut world = hecs::World::new();
    let profiles = MovementProfileRegistry::new();
    let terrain = FlatGround;

    // 起跳前：站立慢跑（Running），向 +X 持续输入
    let ms = MovementState {
        stance: Stance::Standing,
        pace: Pace::Running,
        ..Default::default()
    };
    let mut intent = CMoveIntent::default();
    intent.direction = Vec3::new(1.0, 0.0, 0.0); // 持续前进输入

    let e = world.spawn((
        active_jump(),
        CMovementState(ms),
        CPrevMovementState(ms),
        CMovementRecovery::default(),
        intent,
        CMovementControl::default(),
        Position(Vec3::ZERO),
        Velocity(Vec3::ZERO),
        Rotation(Quat::IDENTITY),
    ));

    let dt = 1.0 / 60.0;
    let mut max_y: f32 = 0.0;
    let mut landed_pace: Option<Pace> = None;

    // 跑 1.5s——足够完成 0.7s 弧线 + 落地后行走
    for frame in 0..90 {
        movement_mode_system(&mut world, &terrain);
        jump_launch_system(&mut world, &profiles);
        movement_system(&mut world, dt, &terrain, &profiles);

        // 起跳后动作完成（真实 action_system 在 ~450ms 完成）——清 CActiveAction 防重跳
        if frame == 0 {
            world.get::<&mut CActiveAction>(e).unwrap().0 = None;
        }

        let y = world.get::<&Position>(e).unwrap().0.y;
        max_y = max_y.max(y);

        // 记录落地后（回到地面且非腾空）的 pace
        let ms_now = world.get::<&CMovementState>(e).unwrap().0;
        if frame > 5 && ms_now.special.is_none() && y.abs() < 0.05 && landed_pace.is_none() {
            landed_pace = Some(ms_now.pace);
        }
    }

    let pos = world.get::<&Position>(e).unwrap().0;
    let ms_final = world.get::<&CMovementState>(e).unwrap().0;

    // A. 升到 >1m（未被过早截断）
    assert!(max_y > 1.0, "跳跃应升到 >1m，实测峰值 {max_y:.2}m");
    // B. 落回地面
    assert!(pos.y.abs() < 0.2, "应落回地面 y≈0，实测 {:.2}", pos.y);
    // C. 落地 pace 恢复 Running（非空栈默认 Still）——回归核心断言
    assert_eq!(
        landed_pace,
        Some(Pace::Running),
        "落地后 pace 应恢复起跳前 Running（否则卡 Still 无法移动）"
    );
    assert_eq!(ms_final.pace, Pace::Running, "终态 pace 应为 Running");
    // D. 落地后能水平移动（持续 +X 输入 → 前进）
    assert!(pos.x > 0.5, "落地后应能水平移动，实测 pos.x={:.2}", pos.x);
}
