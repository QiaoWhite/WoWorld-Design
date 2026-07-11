//! InputBufferSystem — 环形缓冲区管理（过期清理 + pop_if 物理重检 drain）
//!
//! 每帧：① 剔除过期条目 → ② 解算有效 LocomotionMode（含土狼 grace）→
//! ③ 按优先级链排序 → ④ **只弹出当前物理可行的条目**到 `CActionRequestBuf`；
//! 物理不可行的（如空中的 Jump）留在缓冲，待 loco 转合法（落地预输入）或过期。
//!
//! 仅玩家实体（`With<PlayerComponent>`）激活——NPC 的 GOAP 已考虑时机（008 §九）。
//!
//! 满容量优先级淘汰在**入队侧**（`CInputBuffer::push_bounded`，由 action_resolver 调用）。
//!
//! 参见: `WoWorld-Design/.../角色控制器/008-手感系统.md` §二/§三/§五
//!       物理谓词 `physics_req.is_satisfied_by` 与 action_controller 接受路径同源（单一权威）。

use crate::components::action_state::CActionRequestBuf;
use crate::components::input_state::{CCoyoteTime, CInputBuffer};
use crate::components::player::PlayerComponent;
use crate::components::transform::Position;
use crate::resources::action_registry::ActionRegistry;
use smallvec::SmallVec;
use woworld_core::input::BufferedInput;
use woworld_core::kinematics::resolve_effective_loco;
use woworld_core::spatial::TerrainQuery;

/// 输入缓冲管理——过期清理 + 物理重检 drain 到 `CActionRequestBuf`。
///
/// 仅处理带 `PlayerComponent` 的实体。
pub fn input_buffer_system(
    world: &mut hecs::World,
    terrain: &dyn TerrainQuery,
    registry: &ActionRegistry,
    now: f32,
) {
    for (_, (buffer, request_buf, pos, coyote)) in world
        .query_mut::<(
            &mut CInputBuffer,
            &mut CActionRequestBuf,
            &Position,
            Option<&CCoyoteTime>,
        )>()
        .with::<&PlayerComponent>()
    {
        // ── 1. 过期清理（008 §二：条目过窗自然丢弃）──
        buffer.entries.retain(|e| !e.is_expired(now));
        if buffer.entries.is_empty() {
            continue;
        }

        // ── 2. 有效 LocomotionMode（含土狼 grace）——物理重检数据源 ──
        //   与 action_controller 接受路径同源（resolve_effective_loco），单一权威。
        let loco =
            resolve_effective_loco(pos.0, terrain, coyote.map(|c| c.remaining).unwrap_or(0.0));

        // ── 3. 优先级链排序（008 §三：高优先级先消费；平级先到先得）──
        buffer.entries.sort_by(|a, b| {
            b.buffer_priority.cmp(&a.buffer_priority).then_with(|| {
                a.pressed_at
                    .partial_cmp(&b.pressed_at)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });

        // ── 4. pop_if 物理重检 drain（008 §二/§五）──
        //   只弹出当前物理可行的条目 → CActionRequestBuf；物理不可行的（空中的
        //   Jump 等）留缓冲，待 loco 转合法（落地预输入 I3）或过期自然淘汰。
        //   registry 中缺失的 ActionId 视为"不可行"→ 保留（不静默丢弃）。
        let mut kept: SmallVec<[BufferedInput; 4]> = SmallVec::new();
        for entry in buffer.entries.drain(..) {
            let possible = registry
                .get(entry.action_request.action_id)
                .is_some_and(|def| def.physics_req.is_satisfied_by(loco));
            if possible && request_buf.0.len() < request_buf.0.capacity() {
                request_buf.0.push(entry.action_request);
            } else {
                kept.push(entry);
            }
        }
        buffer.entries = kept;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::transform::Position;
    use glam::Vec3;
    use woworld_core::action::{
        ActionDef, ActionId, ActionKind, ActionParams, ActionRequest, ActionSource, CommitmentLevel,
    };
    use woworld_core::input::{BufferPriority, BufferedInput};
    use woworld_core::kinematics::PhysicsRequirement;
    use woworld_core::material::{Medium, SurfaceMaterial};
    use woworld_core::types::{TerrainHit, WorldPos};

    /// 平地 mock：处处可行走 → Grounded。
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

    /// 虚空 mock：处处不可行走 → PhysicsBody（模拟空中）。
    struct SkyVoid;
    impl TerrainQuery for SkyVoid {
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
            false
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

    /// 建单动作 registry（bufferable，physics_req 可指定）。
    fn reg_with(key: &str, physics: PhysicsRequirement) -> (ActionRegistry, ActionId) {
        let mut r = ActionRegistry::new();
        let id = ActionRegistry::id_of(key);
        let def = ActionDef {
            name: key.to_string(),
            category: "Movement".to_string(),
            kind: ActionKind::Discrete,
            priority: 40,
            commitment: CommitmentLevel::Soft,
            windup_ms: 0,
            active_ms: 0,
            recovery_ms: 0,
            cancel_set: vec![],
            cancel_window_ms: 0,
            bufferable: true,
            buffer_window_ms: 200,
            physics_req: physics,
            movement_lock: Default::default(),
            rotation_lock: Default::default(),
            interrupt_on_move: false,
            sustain_drain: None,
            release_behavior: None,
            overextend_threshold_secs: None,
            critical_threshold_secs: None,
        };
        r.register(id, def);
        (r, id)
    }

    fn buffered(id: ActionId, pressed_at: f32) -> BufferedInput {
        BufferedInput::new(
            ActionRequest {
                action_id: id,
                priority: 40,
                source: ActionSource::Player,
                params: ActionParams::default(),
            },
            pressed_at,
            200.0,
            BufferPriority::Movement,
        )
    }

    #[test]
    fn test_grounded_action_drained_on_walkable() {
        let (reg, id) = reg_with("jump", PhysicsRequirement::Grounded);
        let mut world = hecs::World::new();
        world.spawn((
            PlayerComponent::default(),
            CInputBuffer {
                entries: smallvec::smallvec![buffered(id, 0.0)],
                prev_frame_inputs: 0,
            },
            CActionRequestBuf::default(),
            Position(Vec3::ZERO),
        ));
        input_buffer_system(&mut world, &FlatGround, &reg, 0.1);
        for (_, (buffer, rb)) in world.query_mut::<(&CInputBuffer, &CActionRequestBuf)>() {
            assert!(buffer.entries.is_empty(), "grounded action should drain");
            assert_eq!(rb.0.len(), 1);
            assert_eq!(rb.0[0].action_id, id);
        }
    }

    /// I2 核心：空中的 Jump（physics_req=Grounded）不 drain，留缓冲。
    #[test]
    fn test_airborne_jump_kept_in_buffer() {
        let (reg, id) = reg_with("jump", PhysicsRequirement::Grounded);
        let mut world = hecs::World::new();
        world.spawn((
            PlayerComponent::default(),
            CInputBuffer {
                entries: smallvec::smallvec![buffered(id, 0.0)],
                prev_frame_inputs: 0,
            },
            CActionRequestBuf::default(),
            Position(Vec3::ZERO),
        ));
        input_buffer_system(&mut world, &SkyVoid, &reg, 0.1);
        for (_, (buffer, rb)) in world.query_mut::<(&CInputBuffer, &CActionRequestBuf)>() {
            assert_eq!(buffer.entries.len(), 1, "airborne Jump must stay buffered");
            assert!(
                rb.0.is_empty(),
                "must not drain physically-impossible action"
            );
        }
    }

    /// I3 单元级：空中缓冲 → 落地帧 drain 起跳。
    #[test]
    fn test_landing_drains_kept_jump() {
        let (reg, id) = reg_with("jump", PhysicsRequirement::Grounded);
        let mut world = hecs::World::new();
        let e = world.spawn((
            PlayerComponent::default(),
            CInputBuffer {
                entries: smallvec::smallvec![buffered(id, 0.0)],
                prev_frame_inputs: 0,
            },
            CActionRequestBuf::default(),
            Position(Vec3::ZERO),
        ));
        // 空中：保留
        input_buffer_system(&mut world, &SkyVoid, &reg, 0.05);
        assert_eq!(world.get::<&CInputBuffer>(e).unwrap().entries.len(), 1);
        assert!(world.get::<&CActionRequestBuf>(e).unwrap().0.is_empty());
        // 落地：drain 起跳
        input_buffer_system(&mut world, &FlatGround, &reg, 0.1);
        assert!(world.get::<&CInputBuffer>(e).unwrap().entries.is_empty());
        assert_eq!(world.get::<&CActionRequestBuf>(e).unwrap().0.len(), 1);
    }

    #[test]
    fn test_expired_entry_cleaned() {
        let (reg, id) = reg_with("jump", PhysicsRequirement::Grounded);
        let mut world = hecs::World::new();
        // pressed_at 0.0 + 200ms 窗 → expires_at 0.2；now 0.3 → 过期
        world.spawn((
            PlayerComponent::default(),
            CInputBuffer {
                entries: smallvec::smallvec![buffered(id, 0.0)],
                prev_frame_inputs: 0,
            },
            CActionRequestBuf::default(),
            Position(Vec3::ZERO),
        ));
        input_buffer_system(&mut world, &FlatGround, &reg, 0.3);
        for (_, (buffer, rb)) in world.query_mut::<(&CInputBuffer, &CActionRequestBuf)>() {
            assert!(buffer.entries.is_empty(), "expired entry removed");
            assert!(rb.0.is_empty(), "expired entry not drained");
        }
    }

    #[test]
    fn test_non_player_not_processed() {
        let (reg, id) = reg_with("jump", PhysicsRequirement::Grounded);
        let mut world = hecs::World::new();
        // 没有 PlayerComponent——不被处理
        world.spawn((
            CInputBuffer {
                entries: smallvec::smallvec![buffered(id, 0.0)],
                prev_frame_inputs: 0,
            },
            CActionRequestBuf::default(),
            Position(Vec3::ZERO),
        ));
        input_buffer_system(&mut world, &FlatGround, &reg, 0.1);
        for (_, (buffer, rb)) in world.query_mut::<(&CInputBuffer, &CActionRequestBuf)>() {
            assert_eq!(buffer.entries.len(), 1, "non-player untouched");
            assert!(rb.0.is_empty());
        }
    }

    #[test]
    fn test_input_feel_toml_parses_into_config() {
        // ★ 集成前置：验证 assets/input_feel.toml 的键名与 InputFeelConfig 字段对齐。
        use woworld_core::input::InputFeelConfig;
        #[derive(serde::Deserialize)]
        struct Wrapper {
            feel: InputFeelConfig,
        }
        let toml = include_str!("../../../../../assets/input_feel.toml");
        let w: Wrapper = toml::from_str(toml).expect("input_feel.toml 应能解析进 InputFeelConfig");
        assert!((w.feel.coyote_time_ms - 150.0).abs() < 0.001);
        assert!((w.feel.ledge_snap_angle - 45.0).abs() < 0.001);
    }
}
