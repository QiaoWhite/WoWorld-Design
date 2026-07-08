//! LodCoordinatorSystem — 每帧为所有 Entity 写入 LodLevel Component
//!
//! 封装 `woworld_core::lod::LodCoordinator::compute_lod()` 的 8 步算法。
//! ⚠️ Phase 3 接入：当前仅测试使用——WorldDriver 暂时直调 `WorldLodCoordinator::compute_lod()`。
//! EntityIndex 就位后切换为 ECS query 遍历 + 批量写回 LodLevel。
//!
//! 参见: `开发文档/05-全局基础设施/03-LOD协调器.md`

use std::collections::HashMap;

use woworld_core::lod::{
    CameraState, EntityLodInput, FrameBudget, LodCoordinator, LodCoordinatorInput, LodPrescription,
    PlayerAttention, VramPressure,
};
use woworld_core::types::EntityId;

use crate::components::lod::LodLevel;
use crate::components::transform::Position;
use crate::entity_id::entity_id_from_hecs;

/// 一次 pass：读 Position → 算 LodLevel → 写回。
/// 使用 `query::<(&Position, &mut LodLevel)>` 避免 hecs 借用冲突。
pub fn lod_coordinator_system(
    ecs: &mut hecs::World,
    camera: CameraState,
    frame_budget: FrameBudget,
    vram: VramPressure,
    lod_prev: &mut HashMap<EntityId, LodPrescription>,
    lod_hyst: &mut HashMap<EntityId, woworld_core::lod::HysteresisState>,
) {
    // 1. 遍历全部 Entity，收集输入
    let mut entity_inputs: Vec<(hecs::Entity, EntityLodInput)> = Vec::new();
    for (entity, pos) in ecs.query::<&Position>().iter() {
        let eid = entity_id_from_hecs(entity);
        entity_inputs.push((
            entity,
            EntityLodInput {
                id: eid,
                position: glam::DVec3::new(pos.0.x as f64, pos.0.y as f64, pos.0.z as f64),
                is_player: eid.0 == 0,
                is_in_combat: false,
                is_landmark: false,
                relation_importance: 0.0,
            },
        ));
    }

    // 2. 构造输入
    let input = LodCoordinatorInput {
        camera,
        attention: PlayerAttention::default(),
        frame_budget,
        vram,
        entities: entity_inputs.iter().map(|(_, e)| e.clone()).collect(),
        broadcasts: Vec::new(),
        interactions: Vec::new(),
    };

    // 3. 计算
    struct Wlc;
    impl LodCoordinator for Wlc {}
    let prescriptions = Wlc::compute_lod(&input, lod_prev, lod_hyst);

    // 4. 写回
    for (entity, lod) in ecs.query::<&mut LodLevel>().iter() {
        let eid = entity_id_from_hecs(entity);
        if let Some(prescription) = prescriptions.get(&eid) {
            *lod = LodLevel::from_prescription(prescription);
        }
    }

    // 5. 更新跨帧状态
    *lod_prev = prescriptions;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::entity_kind::EntityKind;

    fn make_camera() -> CameraState {
        CameraState {
            position: glam::DVec3::ZERO,
            forward: glam::DVec3::NEG_Z,
            fov_radians: std::f32::consts::FRAC_PI_2,
        }
    }

    fn make_frame_budget() -> FrameBudget {
        FrameBudget {
            remaining_ms: 16.67,
            last_frame_ms: 8.0,
        }
    }

    #[test]
    fn test_lod_coordinator_system_basic() {
        let mut ecs = hecs::World::new();
        let mut lod_prev: HashMap<EntityId, LodPrescription> = HashMap::new();
        let mut lod_hyst = HashMap::new();

        let player = ecs.spawn((
            Position(glam::Vec3::new(0.0, 0.0, 0.0)),
            EntityKind::Creature,
            LodLevel::default(),
        ));

        let npc = ecs.spawn((
            Position(glam::Vec3::new(1000.0, 0.0, 0.0)),
            EntityKind::Creature,
            LodLevel::default(),
        ));

        lod_coordinator_system(
            &mut ecs,
            make_camera(),
            make_frame_budget(),
            VramPressure::default(),
            &mut lod_prev,
            &mut lod_hyst,
        );

        // 验证 Player LOD
        let player_lod = ecs.get::<&LodLevel>(player).expect("player has LodLevel");
        assert_eq!(
            player_lod.scene_lod, 0,
            "player at origin should be scene LOD 0"
        );
        assert_eq!(player_lod.skeleton_lod, 0);

        // 验证 NPC LOD（距离 1000m）
        let npc_lod = ecs.get::<&LodLevel>(npc).expect("npc has LodLevel");
        assert!(
            npc_lod.scene_lod > 0,
            "npc at 1000m should have scene_lod > 0"
        );

        // 验证 lod_prev 已更新
        let player_eid = entity_id_from_hecs(player);
        assert!(lod_prev.contains_key(&player_eid));
    }

    #[test]
    fn test_lod_coordinator_empty_world() {
        let mut ecs = hecs::World::new();
        let mut lod_prev: HashMap<EntityId, LodPrescription> = HashMap::new();
        let mut lod_hyst = HashMap::new();

        lod_coordinator_system(
            &mut ecs,
            make_camera(),
            make_frame_budget(),
            VramPressure::default(),
            &mut lod_prev,
            &mut lod_hyst,
        );

        assert!(lod_prev.is_empty());
    }

    #[test]
    fn test_lod_coordinator_player_always_lod_zero() {
        let mut ecs = hecs::World::new();
        let player = ecs.spawn((
            Position(glam::Vec3::new(0.0, 0.0, 0.0)),
            EntityKind::Creature,
            LodLevel::default(),
        ));

        let mut lod_prev = HashMap::new();
        let mut lod_hyst = HashMap::new();

        lod_coordinator_system(
            &mut ecs,
            make_camera(),
            make_frame_budget(),
            VramPressure::default(),
            &mut lod_prev,
            &mut lod_hyst,
        );

        let pl = ecs.get::<&LodLevel>(player).expect("has LodLevel");
        assert_eq!(pl.scene_lod, 0);
        assert_eq!(pl.skeleton_lod, 0);
    }
}
