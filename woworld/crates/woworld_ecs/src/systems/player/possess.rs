//! 夺舍 System — NPC 角色切换 + Position 同步
//!
//! 核心哲学：夺舍 ≠ 创建新实体。夺舍 = 切换 player_ecs_entity 引用
//! + 移除 Goal/Wander（movement_system 自动跳过）+ 添加 PlayerComponent/ControlModeComponent。
//!
//! 退出夺舍 = 恢复 Goal 生成 + 移除 PlayerComponent/ControlModeComponent。
//!
//! 参见: `WoWorld-Design/.../玩家系统/003-双角色与托管模式.md`

use glam::Vec3;
use hecs::CommandBuffer;

use crate::components::entity_kind::EntityKind as EcsEntityKind;
use crate::components::goal::Goal;
use crate::components::movement::Movement;
use crate::components::movement::Wander;
use crate::components::player::{ControlModeComponent, PlayerComponent};
use crate::components::transform::Position;

/// 可夺舍候选实体——用于 Tab 键循环
#[derive(Debug, Clone)]
pub struct PossessionCandidate {
    pub entity: hecs::Entity,
    pub position: Vec3,
    /// 摄像机方向与该实体方向的点积（越大越正对）
    pub facing_dot: f32,
    /// 到摄像机的距离
    pub distance: f32,
}

/// 查找所有可夺舍的 NPC 实体
///
/// 过滤条件:
/// - 有 Position + EntityKind::Creature
/// - 有 NPC 核心 Component（至少 BigFive 或 Emotion 或 Needs 之一）
/// - 不是当前 player entity
///
/// 返回按 (camera_forward dot entity_dir) 降序 → 距离升序 排列的候选列表。
pub fn find_possessable_entities(
    world: &hecs::World,
    camera_pos: Vec3,
    camera_forward: Vec3,
    current_player: Option<hecs::Entity>,
) -> Vec<PossessionCandidate> {
    let fwd = camera_forward.normalize_or_zero();
    let mut candidates: Vec<PossessionCandidate> = Vec::new();

    for (entity, pos) in world.query::<&Position>().iter() {
        // 排除当前玩家实体
        if current_player == Some(entity) {
            continue;
        }

        // 必须是 Creature
        let kind = match world.get::<&EcsEntityKind>(entity) {
            Ok(k) => k,
            Err(_) => continue,
        };
        if !matches!(*kind, EcsEntityKind::Creature) {
            continue;
        }

        // 必须有 NPC 核心 Component（非裸实体）
        // 检查 BigFive 或 Emotion 或 Needs 之一
        let is_npc = world
            .get::<&crate::components::bigfive::BigFive>(entity)
            .is_ok()
            || world
                .get::<&crate::components::emotion::Emotion>(entity)
                .is_ok()
            || world
                .get::<&crate::components::needs::Needs>(entity)
                .is_ok();
        if !is_npc {
            continue;
        }

        let to_entity = pos.0 - camera_pos;
        let dist = to_entity.length();
        if dist < 0.001 {
            continue; // 太近（就是自己）
        }

        let dir = to_entity / dist;
        let dot = fwd.dot(dir);

        candidates.push(PossessionCandidate {
            entity,
            position: pos.0,
            facing_dot: dot,
            distance: dist,
        });
    }

    // 排序: 面向优先 → 距离优先
    candidates.sort_by(|a, b| {
        b.facing_dot
            .partial_cmp(&a.facing_dot)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                a.distance
                    .partial_cmp(&b.distance)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    candidates
}

/// 执行夺舍：将指定 NPC 标记为玩家角色
///
/// 1. 添加 PlayerComponent + ControlModeComponent::Manual
/// 2. 移除 Goal + Wander（movement_system 自动跳过该实体）
/// 3. 保留所有其他 Component（BigFive/Emotion/Needs/Social/...继续正常运转）
///
/// 返回该实体的 Position（Godot CharacterBody3D 瞬移用）。
///
/// 调用者负责在返回后执行 `cmd.run_on(&mut world)`。
pub fn possess_entity(
    world: &hecs::World,
    cmd: &mut CommandBuffer,
    entity: hecs::Entity,
) -> Option<Vec3> {
    // 验证实体存在且是 Creature
    let pos = world.get::<&Position>(entity).ok()?.0;
    let _kind = world.get::<&EcsEntityKind>(entity).ok()?;

    // ★ 007 修复: 移除 Goal + Wander + 旧 Movement (绞杀者门控)——
    //   movement_system 用 `Without<Movement>` 过滤旧管线实体——NPC 带着它
    //   即使有其他 CC 组件也被跳过。必须移除才能进入 Block A0 CC 管线。
    cmd.remove_one::<Goal>(entity);
    cmd.remove_one::<Wander>(entity);
    cmd.remove_one::<Movement>(entity);

    // ★ 007: 添加完整 Block A0 CC + action 组件束——
    //   player_input_system: CMoveIntent + CMovementState (pace 直写)
    //   movement_system: CMoveIntent + CMovementState + CMovementControl + Velocity + Position
    //   action_system: CActiveAction + CActionRequestBuf + CMovementControl + Position
    //   jump_launch_system: CActiveAction + CMovementState + CMovementRecovery + Velocity
    //   character_facing_system: CMovementControl + CMoveIntent + Rotation
    //   input_buffer_system: CInputBuffer
    use crate::components::action_state::{CActionRequestBuf, CActiveAction};
    use crate::components::input_state::CInputBuffer;
    use crate::components::movement_state::{
        CMoveIntent, CMovementControl, CMovementRecovery, CMovementState, CPrevMovementState,
    };
    use crate::components::transform::{Rotation, Velocity};
    use woworld_core::movement::{MovementState, Pace, Stance};

    cmd.insert_one(entity, CMoveIntent::default());
    cmd.insert_one(entity, CMovementControl::default());
    cmd.insert_one(entity, CMovementRecovery::default());
    cmd.insert_one(entity, CActiveAction::default());
    cmd.insert_one(entity, CActionRequestBuf::default());
    cmd.insert_one(entity, CInputBuffer::default());
    cmd.insert_one(entity, Velocity(glam::Vec3::ZERO));
    if world.get::<&Rotation>(entity).is_err() {
        cmd.insert_one(entity, Rotation(glam::Quat::IDENTITY));
    }
    let ms = MovementState {
        stance: Stance::Standing,
        pace: Pace::Walking,
        ..Default::default()
    };
    cmd.insert_one(entity, CMovementState(ms));
    cmd.insert_one(entity, CPrevMovementState(ms));

    // 添加玩家标记
    cmd.insert_one(
        entity,
        PlayerComponent {
            original_name_override: None,
        },
    );
    cmd.insert_one(entity, ControlModeComponent::manual());

    Some(pos)
}

/// 退出夺舍：将指定实体恢复为普通 NPC
///
/// 1. 移除 PlayerComponent + ControlModeComponent
/// 2. Goal 重新由 goal_system 自然生成（Phase 2）
///
/// 调用者负责在返回后执行 `cmd.run_on(&mut world)`。
pub fn unpossess_entity(world: &hecs::World, cmd: &mut CommandBuffer, entity: hecs::Entity) {
    // 只有在实体存在且确实有 PlayerComponent 时才操作
    if world.get::<&PlayerComponent>(entity).is_err() {
        return;
    }

    cmd.remove_one::<PlayerComponent>(entity);
    cmd.remove_one::<ControlModeComponent>(entity);

    // Phase 1: Goal 重新生成由外部 goal_system 负责。
    // Phase 2: 此处直接触发 goal_system 为新恢复的 NPC 生成初始 Goal。
}

/// 同步玩家位置：CharacterBody3D.global_position → ECS Position
///
/// 由 Godot 每帧调用（physics_process 后）。
/// 仅在 Manual 模式下写入——Auto 模式忽略（避免覆盖 movement_system 的输出）。
pub fn sync_player_position(
    world: &mut hecs::World,
    player_entity: hecs::Entity,
    new_position: Vec3,
) {
    if let Ok(mut pos) = world.get::<&mut Position>(player_entity) {
        pos.0 = new_position;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::bigfive::BigFive;
    use crate::components::emotion::Emotion;
    use crate::components::needs::Needs;
    use glam::Vec3;
    use woworld_core::player::ControlMode;

    /// 创建一个完整 NPC（可被夺舍）
    fn spawn_full_npc(world: &mut hecs::World, pos: Vec3) -> hecs::Entity {
        world.spawn((
            Position(pos),
            EcsEntityKind::Creature,
            BigFive::default(),
            Emotion::default(),
            Needs::default(),
            Goal::default(),
        ))
    }

    /// 创建一个裸实体（不可被夺舍）
    fn spawn_bare_entity(world: &mut hecs::World, pos: Vec3) -> hecs::Entity {
        world.spawn((Position(pos), EcsEntityKind::Plant))
    }

    // ── find_possessable_entities ──

    #[test]
    fn test_find_empty_world() {
        let world = hecs::World::new();
        let candidates = find_possessable_entities(&world, Vec3::ZERO, Vec3::Z, None);
        assert!(candidates.is_empty());
    }

    #[test]
    fn test_find_skips_bare_entity() {
        let mut world = hecs::World::new();
        spawn_bare_entity(&mut world, Vec3::new(5.0, 0.0, 0.0));
        let candidates = find_possessable_entities(&world, Vec3::ZERO, Vec3::Z, None);
        assert!(
            candidates.is_empty(),
            "bare entity should not be possessable"
        );
    }

    #[test]
    fn test_find_full_npc() {
        let mut world = hecs::World::new();
        spawn_full_npc(&mut world, Vec3::new(10.0, 0.0, 0.0));
        let candidates = find_possessable_entities(&world, Vec3::ZERO, Vec3::X, None);
        assert_eq!(candidates.len(), 1);
        assert!((candidates[0].facing_dot - 1.0).abs() < 0.01);
        assert!((candidates[0].distance - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_find_excludes_current_player() {
        let mut world = hecs::World::new();
        let npc = spawn_full_npc(&mut world, Vec3::new(10.0, 0.0, 0.0));
        let candidates = find_possessable_entities(&world, Vec3::ZERO, Vec3::X, Some(npc));
        assert!(candidates.is_empty(), "current player should be excluded");
    }

    #[test]
    fn test_find_sorts_by_facing_then_distance() {
        let mut world = hecs::World::new();
        // NPC A: 正前方 10m
        spawn_full_npc(&mut world, Vec3::new(10.0, 0.0, 0.0));
        // NPC B: 正后方 5m
        spawn_full_npc(&mut world, Vec3::new(-5.0, 0.0, 0.0));
        // NPC C: 正前方 20m
        spawn_full_npc(&mut world, Vec3::new(20.0, 0.0, 0.0));

        let candidates = find_possessable_entities(&world, Vec3::ZERO, Vec3::X, None);
        assert_eq!(candidates.len(), 3);
        // 前方面向的 NPC 应排在后面（更近的在前）
        assert!(
            (candidates[0].position.x - 10.0).abs() < 0.01,
            "first should be closest forward"
        );
        assert!(
            (candidates[1].position.x - 20.0).abs() < 0.01,
            "second should be farther forward"
        );
        assert!(
            (candidates[2].position.x + 5.0).abs() < 0.01,
            "last should be behind"
        );
    }

    #[test]
    fn test_find_multiple_npcs_facing() {
        let mut world = hecs::World::new();
        // Camera at origin, looking +X
        // NPC A: slightly off-axis (dot ~0.98), 5m
        let pos_a = Vec3::new(5.0, 0.0, 1.0);
        // NPC B: on-axis (dot = 1.0), 8m
        let pos_b = Vec3::new(8.0, 0.0, 0.0);

        spawn_full_npc(&mut world, pos_a);
        spawn_full_npc(&mut world, pos_b);

        let camera_forward = Vec3::X;
        let candidates = find_possessable_entities(&world, Vec3::ZERO, camera_forward, None);
        assert_eq!(candidates.len(), 2);
        // B should be first (higher dot product)
        assert!((candidates[0].position.x - 8.0).abs() < 0.01);
        assert!((candidates[1].position.x - 5.0).abs() < 0.01);
    }

    // ── possess_entity ──

    #[test]
    fn test_possess_adds_components() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let npc = spawn_full_npc(&mut world, Vec3::new(10.0, 0.0, 0.0));

        let pos = possess_entity(&world, &mut cmd, npc);
        assert!(pos.is_some());
        cmd.run_on(&mut world);

        // 验证 PlayerComponent + ControlModeComponent 已添加
        assert!(world.get::<&PlayerComponent>(npc).is_ok());
        let cm = world.get::<&ControlModeComponent>(npc).unwrap();
        assert_eq!(cm.mode, ControlMode::Manual);

        // 验证 Goal 已移除
        assert!(world.get::<&Goal>(npc).is_err());
    }

    #[test]
    fn test_possess_removes_goal_wander_and_movement() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let npc = world.spawn((
            Position(Vec3::ZERO),
            EcsEntityKind::Creature,
            BigFive::default(),
            Goal::default(),
            Wander {
                direction: Vec3::X,
                remaining: 1.0,
            },
            Movement::default(),
        ));

        possess_entity(&world, &mut cmd, npc);
        cmd.run_on(&mut world);

        assert!(world.get::<&Goal>(npc).is_err());
        assert!(world.get::<&Wander>(npc).is_err());
        assert!(
            world.get::<&Movement>(npc).is_err(),
            "old Movement must be removed for CC pipeline"
        );
    }

    #[test]
    fn test_possess_preserves_other_components() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let npc = spawn_full_npc(&mut world, Vec3::ZERO);

        possess_entity(&world, &mut cmd, npc);
        cmd.run_on(&mut world);

        // BigFive/Emotion/Needs/Position 应保留
        assert!(world.get::<&BigFive>(npc).is_ok());
        assert!(world.get::<&Emotion>(npc).is_ok());
        assert!(world.get::<&Needs>(npc).is_ok());
        assert!(world.get::<&Position>(npc).is_ok());
    }

    #[test]
    fn test_possess_nonexistent_entity() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let dummy = world.spawn((Position(Vec3::ZERO),));
        world.despawn(dummy).unwrap();

        let result = possess_entity(&world, &mut cmd, dummy);
        assert!(result.is_none());
    }

    // ── unpossess_entity ──

    #[test]
    fn test_unpossess_removes_player_components() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let npc = spawn_full_npc(&mut world, Vec3::ZERO);

        // 先夺舍
        possess_entity(&world, &mut cmd, npc);
        cmd.run_on(&mut world);

        // 再退出
        let mut cmd2 = CommandBuffer::new();
        unpossess_entity(&world, &mut cmd2, npc);
        cmd2.run_on(&mut world);

        assert!(world.get::<&PlayerComponent>(npc).is_err());
        assert!(world.get::<&ControlModeComponent>(npc).is_err());
    }

    #[test]
    fn test_unpossess_not_player_is_noop() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let npc = spawn_full_npc(&mut world, Vec3::ZERO);

        // 对从未被夺舍的 NPC 调用 unpossess——不应 panic
        unpossess_entity(&world, &mut cmd, npc);
        cmd.run_on(&mut world);

        // NPC 应保持原样
        assert!(world.get::<&Position>(npc).is_ok());
        assert!(world.get::<&Goal>(npc).is_ok());
    }

    // ── sync_player_position ──

    #[test]
    fn test_sync_position_updates() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let npc = spawn_full_npc(&mut world, Vec3::ZERO);

        possess_entity(&world, &mut cmd, npc);
        cmd.run_on(&mut world);

        sync_player_position(&mut world, npc, Vec3::new(100.0, 50.0, 200.0));

        let pos = world.get::<&Position>(npc).unwrap();
        assert!((pos.0.x - 100.0).abs() < 0.01);
        assert!((pos.0.y - 50.0).abs() < 0.01);
        assert!((pos.0.z - 200.0).abs() < 0.01);
    }

    #[test]
    fn test_sync_on_nonexistent_does_not_panic() {
        let mut world = hecs::World::new();
        let dummy = world.spawn((Position(Vec3::ZERO),));
        world.despawn(dummy).unwrap();
        // 不应 panic
        sync_player_position(&mut world, dummy, Vec3::ONE);
    }

    #[test]
    fn test_find_respects_lod_visible_only() {
        // Phase 1: 不使用 LodLevel 过滤——所有 Creature 都是候选。
        // LOD 过滤由 Godot 侧 camera 距离裁剪负责。
        let mut world = hecs::World::new();
        // 远距离 NPC
        spawn_full_npc(&mut world, Vec3::new(1000.0, 0.0, 0.0));
        let candidates = find_possessable_entities(&world, Vec3::ZERO, Vec3::X, None);
        // 远距离也是候选（Tab 循环不加距离硬限制）
        assert_eq!(candidates.len(), 1);
    }
}
