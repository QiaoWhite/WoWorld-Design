//! GoalResolutionSystem — Desire → Goal 转换 + 漫游回落 + 目标坐标解析
//!
//! 第一遍：查询 Desire → 映射 GoalType → 解析 target_pos（FindFood→植被查询 /
//! FindWater→最近水点）→ cmd: remove Desire, insert Goal。
//! 第二遍（R4）：无 Desire 且需求已不紧迫的 NPC → 回落 Goal::Idle（漫游），
//! 修复「Goal sticky」（需求满足后仍卡在旧 FindX）。判据读 Needs 紧迫度防振荡。

use glam::Vec2;
use hecs::CommandBuffer;
use woworld_core::ocean::OceanProvider;
use woworld_core::types::WorldPos;
use woworld_core::vegetation::VegetationProvider;

use crate::components::goal::{Goal, GoalType};
use crate::components::needs::{Desire, DesireKind, NeedSensitivity, Needs};
use crate::components::transform::Position;
use crate::systems::npc::needs::{evaluate_top_urgency, URGENCY_THRESHOLD};

/// 最近水源搜索——XZ 平面极坐标网格扫描
///
/// 返回最近一个 `water_depth_at > 0.0` 的 XZ 坐标（Y 固定为 0.0）。
/// movement_system 的到达判定比较 XZ 距离并忽略 Y，地形跟随自动将 NPC
/// 放在水面边缘。深水/陡岸由 movement 坡度门自然阻挡（诚实涌现）。
fn find_nearest_water_xz(
    origin: Vec2,
    ocean: &dyn OceanProvider,
    radius: f32,
) -> Option<glam::Vec3> {
    const ANGLES: usize = 8;
    const STEPS: usize = 8;
    let step = radius / STEPS as f32;

    let mut best_dist = f32::MAX;
    let mut best_pos: Option<glam::Vec3> = None;

    for ring in 0..STEPS {
        let r = step * (ring + 1) as f32;
        for a in 0..ANGLES {
            let theta = std::f32::consts::TAU * a as f32 / ANGLES as f32;
            let x = origin.x + r * theta.cos();
            let z = origin.y + r * theta.sin(); // Vec2 用 (x, y) 存 XZ

            let wp = WorldPos {
                x: x as f64,
                y: 0.0,
                z: z as f64,
            };

            if ocean.water_depth_at(wp) > 0.0 {
                let dist = r;
                if dist < best_dist {
                    best_dist = dist;
                    best_pos = Some(glam::Vec3::new(x, 0.0, z));
                }
            }
        }
    }

    best_pos
}

/// Resolve target_pos for a given goal type.
///
/// FindFood → nearest harvestable via VegetationProvider
/// FindWater → nearest water point via OceanProvider
/// All others → None (Phase 3+)
fn resolve_target_pos(
    npc_pos: glam::Vec3,
    goal_type: GoalType,
    vegetation: Option<&dyn VegetationProvider>,
    ocean: &dyn OceanProvider,
    search_radius: f32,
) -> Option<glam::Vec3> {
    match goal_type {
        GoalType::FindFood => {
            let vp = vegetation?;
            let npc_xz = Vec2::new(npc_pos.x, npc_pos.z);
            let harvestables = vp.query_harvestable(npc_xz, search_radius);
            harvestables
                .into_iter()
                .min_by(|a, b| {
                    let da = npc_pos.distance_squared(a.position);
                    let db = npc_pos.distance_squared(b.position);
                    da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|h| h.position)
        }
        GoalType::FindWater => {
            let npc_xz = Vec2::new(npc_pos.x, npc_pos.z);
            find_nearest_water_xz(npc_xz, ocean, search_radius)
        }
        // Phase 3+: FindRest, FindSafePlace, FindSocialContact,
        // BalanceElements, ExpressLibido — 暂不解析
        _ => None,
    }
}

/// Desire → GoalType 映射
fn goal_for_desire(kind: DesireKind) -> GoalType {
    match kind {
        DesireKind::Eat => GoalType::FindFood,
        DesireKind::Drink => GoalType::FindWater,
        DesireKind::Rest => GoalType::FindRest,
        DesireKind::SeekSafety => GoalType::FindSafePlace,
        DesireKind::Socialize => GoalType::FindSocialContact,
        DesireKind::BalanceElements => GoalType::BalanceElements,
        DesireKind::ExpressLibido => GoalType::ExpressLibido,
    }
}

/// 每帧执行——Desire → Goal（含 target_pos 解析）+ 漫游回落
pub fn goal_resolution_system(
    world: &hecs::World,
    cmd: &mut CommandBuffer,
    vegetation: Option<&dyn VegetationProvider>,
    ocean: &dyn OceanProvider,
    search_radius: f32,
) {
    // ── 第一遍：有 Desire → 转 Goal（消费 Desire + 同步填充 target_pos）──
    for (entity, (pos, desire)) in world.query::<(&Position, &Desire)>().iter() {
        let goal_type = goal_for_desire(desire.kind);
        let target_pos = resolve_target_pos(pos.0, goal_type, vegetation, ocean, search_radius);

        let goal = Goal {
            goal_type,
            urgency: desire.urgency,
            target_pos,
        };

        cmd.remove_one::<Desire>(entity);
        cmd.insert_one(entity, goal);
    }

    // ── 第二遍：漫游回落（R4）──
    // 无 Desire 且当前 Goal 非 Idle 的 NPC：最高需求已不紧迫 → 回落 Idle（漫游）。
    // ⚠️ 判据必须读 Needs 紧迫度，不能靠「无 Desire」——Desire 被本系统即时消费，
    //    持续紧迫的 NPC 也存在「当帧无 Desire」窗口（needs/goal 同 Block A4 共用 cmd），
    //    若按「无 Desire → Idle」回落会致 Goal 每帧 FindX↔Idle 振荡。
    for (entity, (goal, needs, sens)) in world
        .query::<(&Goal, &Needs, &NeedSensitivity)>()
        .without::<&Desire>()
        .iter()
    {
        if goal.goal_type == GoalType::Idle {
            continue;
        }
        let (_, urgency) = evaluate_top_urgency(needs, sens);
        if urgency < URGENCY_THRESHOLD {
            cmd.insert_one(entity, Goal::default()); // Idle——漫游
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;
    use hecs::World;
    use woworld_core::id::SpeciesId;
    use woworld_core::vegetation::{
        GroundCoverMap, HarvestableInfo, PlantCommunitySnapshot, ProductCategory, RegenState,
        TimberAvailability, TimberQuality,
    };

    /// 测试用默认搜索半径
    const SEARCH_RADIUS: f32 = 150.0;

    // ── Mock providers ──

    /// 测试用 OceanProvider——默认无水（所有查询返回 0.0 / false）
    struct MockNoWater;
    impl OceanProvider for MockNoWater {
        fn sea_level_at(&self, _pos: WorldPos) -> f64 {
            0.0
        }
        fn wave_height_at(&self, _pos: WorldPos, _time: f64) -> f64 {
            0.0
        }
        fn water_depth_at(&self, _pos: WorldPos) -> f64 {
            0.0
        }
        fn is_underwater(&self, _pos: WorldPos, _time: f64) -> bool {
            false
        }
    }

    /// 测试用 OceanProvider——固定深水（所有点水深 5m）
    struct MockAllWater;
    impl OceanProvider for MockAllWater {
        fn sea_level_at(&self, _pos: WorldPos) -> f64 {
            0.0
        }
        fn wave_height_at(&self, _pos: WorldPos, _time: f64) -> f64 {
            0.0
        }
        fn water_depth_at(&self, _pos: WorldPos) -> f64 {
            5.0
        }
        fn is_underwater(&self, _pos: WorldPos, _time: f64) -> bool {
            false
        }
    }

    /// 测试用 VegetationProvider——返回预设的可采集物列表
    struct MockVegetation {
        harvestables: Vec<HarvestableInfo>,
    }
    impl VegetationProvider for MockVegetation {
        fn query_harvestable(&self, _pos: Vec2, radius: f32) -> Vec<HarvestableInfo> {
            // Mock 尊重 radius：只返回距离内的采集物（使用 x 坐标作为简化的距离代理）
            self.harvestables
                .iter()
                .filter(|h| h.position.x.abs() <= radius && h.position.z.abs() <= radius)
                .cloned()
                .collect()
        }
        fn query_community(&self, _pos: Vec2, _radius: f32) -> PlantCommunitySnapshot {
            PlantCommunitySnapshot {
                dominant_species: vec![],
                companion_species: vec![],
                canopy_closure: 0.0,
                shannon_diversity: 0.0,
            }
        }
        fn canopy_closure(&self, _pos: Vec2) -> f32 {
            0.0
        }
        fn timber_availability(&self, _pos: Vec2) -> TimberAvailability {
            TimberAvailability {
                available: false,
                quality: TimberQuality::Softwood,
                abundance: 0.0,
                harvest_difficulty: 0.0,
                dominant_species: vec![],
            }
        }
        fn ground_cover(&self, _pos: Vec2) -> GroundCoverMap {
            GroundCoverMap::default()
        }
        fn fuel_load(&self, _pos: Vec2) -> f32 {
            0.0
        }
        fn root_interference(&self, _pos: glam::Vec3) -> f32 {
            0.0
        }
        fn set_scene_lod(&self, _lod: u8) {}
    }

    /// 创建一个简单的 HarvestableInfo 用于测试
    fn make_harvestable(x: f32, y: f32, z: f32) -> HarvestableInfo {
        HarvestableInfo {
            instance_id: 1,
            species_id: SpeciesId(1),
            position: Vec3::new(x, y, z),
            product_category: ProductCategory::Berry,
            yield_base: 1.0,
            season_optimal: true,
            regen_state: RegenState::Full,
        }
    }

    fn run_and_flush(
        world: &mut World,
        vegetation: Option<&dyn VegetationProvider>,
        ocean: &dyn OceanProvider,
    ) {
        let mut cmd = CommandBuffer::new();
        goal_resolution_system(world, &mut cmd, vegetation, ocean, SEARCH_RADIUS);
        cmd.run_on(world);
    }

    // ── 原有测试（适配新签名）──

    #[test]
    fn test_hunger_desire_becomes_find_food() {
        let mut world = World::new();
        let ocean = MockNoWater;

        let e = world.spawn((
            Position(Vec3::ZERO),
            Desire {
                kind: DesireKind::Eat,
                urgency: 0.9,
            },
        ));

        run_and_flush(&mut world, None::<&dyn VegetationProvider>, &ocean);

        let goal = world.get::<&Goal>(e).expect("should have Goal");
        assert_eq!(goal.goal_type, GoalType::FindFood);
        assert_eq!(goal.urgency, 0.9);
        assert!(goal.target_pos.is_none(), "no vegetation → target_pos=None");
        assert!(world.get::<&Desire>(e).is_err());
    }

    #[test]
    fn test_thirst_desire_becomes_find_water() {
        let mut world = World::new();
        let ocean = MockNoWater;

        let e = world.spawn((
            Position(Vec3::ZERO),
            Desire {
                kind: DesireKind::Drink,
                urgency: 0.85,
            },
        ));

        run_and_flush(&mut world, None::<&dyn VegetationProvider>, &ocean);

        let goal = world.get::<&Goal>(e).expect("should have Goal");
        assert_eq!(goal.goal_type, GoalType::FindWater);
        // 无水 → target_pos = None（漫游）
        assert!(goal.target_pos.is_none());
    }

    #[test]
    fn test_fatigue_desire_becomes_find_rest() {
        let mut world = World::new();
        let ocean = MockNoWater;

        let e = world.spawn((
            Position(Vec3::ZERO),
            Desire {
                kind: DesireKind::Rest,
                urgency: 0.88,
            },
        ));

        run_and_flush(&mut world, None::<&dyn VegetationProvider>, &ocean);

        let goal = world.get::<&Goal>(e).expect("should have Goal");
        assert_eq!(goal.goal_type, GoalType::FindRest);
        assert!(
            goal.target_pos.is_none(),
            "FindRest not resolved (Phase 3+)"
        );
    }

    #[test]
    fn test_no_desire_no_goal() {
        let mut world = World::new();
        let ocean = MockNoWater;

        // 空 World——不 panic
        run_and_flush(&mut world, None::<&dyn VegetationProvider>, &ocean);
    }

    // ── R4: 漫游回落（适配 Position 不存在的情况——Pass 2 不读 Position）──

    #[test]
    fn test_satisfied_need_falls_back_to_idle() {
        let mut world = World::new();
        let ocean = MockNoWater;
        let e = world.spawn((
            Goal {
                goal_type: GoalType::FindFood,
                urgency: 0.9,
                target_pos: None,
            },
            Needs::default(),
            NeedSensitivity::default(),
        ));

        run_and_flush(&mut world, None::<&dyn VegetationProvider>, &ocean);

        let goal = world.get::<&Goal>(e).expect("should have Goal");
        assert_eq!(goal.goal_type, GoalType::Idle);
    }

    #[test]
    fn test_urgent_need_no_idle_flicker() {
        let mut world = World::new();
        let ocean = MockNoWater;
        let e = world.spawn((
            Goal {
                goal_type: GoalType::FindFood,
                urgency: 0.9,
                target_pos: None,
            },
            Needs {
                hunger: 1.0,
                ..Needs::default()
            },
            NeedSensitivity::default(),
        ));

        run_and_flush(&mut world, None::<&dyn VegetationProvider>, &ocean);

        let goal = world.get::<&Goal>(e).expect("should have Goal");
        assert_eq!(
            goal.goal_type,
            GoalType::FindFood,
            "urgent need must not flicker to Idle"
        );
    }

    #[test]
    fn test_desire_entity_takes_first_pass_not_idle() {
        let mut world = World::new();
        let ocean = MockNoWater;
        let e = world.spawn((
            Position(Vec3::ZERO),
            Desire {
                kind: DesireKind::Eat,
                urgency: 0.9,
            },
            Goal::default(),
            Needs::default(),
            NeedSensitivity::default(),
        ));

        run_and_flush(&mut world, None::<&dyn VegetationProvider>, &ocean);

        let goal = world.get::<&Goal>(e).expect("should have Goal");
        assert_eq!(goal.goal_type, GoalType::FindFood);
    }

    // ── 新测试：target_pos 解析 ──

    #[test]
    fn test_eat_desire_fills_target_with_nearest_food() {
        let mut world = World::new();
        let ocean = MockNoWater;
        let veg = MockVegetation {
            harvestables: vec![
                make_harvestable(50.0, 2.0, 0.0), // dist 50
                make_harvestable(10.0, 3.0, 0.0), // dist 10 ← nearest
                make_harvestable(30.0, 1.0, 0.0), // dist 30
            ],
        };

        let e = world.spawn((
            Position(Vec3::ZERO),
            Desire {
                kind: DesireKind::Eat,
                urgency: 0.9,
            },
        ));

        run_and_flush(&mut world, Some(&veg as &dyn VegetationProvider), &ocean);

        let goal = world.get::<&Goal>(e).expect("should have Goal");
        assert_eq!(goal.goal_type, GoalType::FindFood);
        let tp = goal.target_pos.expect("should have target with vegetation");
        assert!(
            (tp - Vec3::new(10.0, 3.0, 0.0)).length() < 0.01,
            "should choose nearest harvestable, got {:?}",
            tp
        );
    }

    #[test]
    fn test_eat_no_vegetation_target_none() {
        let mut world = World::new();
        let ocean = MockNoWater;

        let e = world.spawn((
            Position(Vec3::ZERO),
            Desire {
                kind: DesireKind::Eat,
                urgency: 0.9,
            },
        ));

        run_and_flush(&mut world, None::<&dyn VegetationProvider>, &ocean);

        let goal = world.get::<&Goal>(e).expect("should have Goal");
        assert_eq!(goal.goal_type, GoalType::FindFood);
        assert!(goal.target_pos.is_none(), "no vegetation → no target");
    }

    #[test]
    fn test_eat_empty_harvestables_target_none() {
        let mut world = World::new();
        let ocean = MockNoWater;
        let veg = MockVegetation {
            harvestables: vec![],
        };

        let e = world.spawn((
            Position(Vec3::ZERO),
            Desire {
                kind: DesireKind::Eat,
                urgency: 0.9,
            },
        ));

        run_and_flush(&mut world, Some(&veg as &dyn VegetationProvider), &ocean);

        let goal = world.get::<&Goal>(e).expect("should have Goal");
        assert!(
            goal.target_pos.is_none(),
            "no harvestables nearby → no target"
        );
    }

    #[test]
    fn test_drink_desire_fills_target_with_water() {
        let mut world = World::new();
        let ocean = MockAllWater;

        let e = world.spawn((
            Position(Vec3::ZERO),
            Desire {
                kind: DesireKind::Drink,
                urgency: 0.85,
            },
        ));

        run_and_flush(&mut world, None::<&dyn VegetationProvider>, &ocean);

        let goal = world.get::<&Goal>(e).expect("should have Goal");
        assert_eq!(goal.goal_type, GoalType::FindWater);
        assert!(
            goal.target_pos.is_some(),
            "all water → should find nearest water point"
        );
    }

    #[test]
    fn test_drink_no_water_nearby_target_none() {
        let mut world = World::new();
        let ocean = MockNoWater;

        let e = world.spawn((
            Position(Vec3::ZERO),
            Desire {
                kind: DesireKind::Drink,
                urgency: 0.85,
            },
        ));

        run_and_flush(&mut world, None::<&dyn VegetationProvider>, &ocean);

        let goal = world.get::<&Goal>(e).expect("should have Goal");
        assert!(goal.target_pos.is_none(), "no water → no target (wander)");
    }

    #[test]
    fn test_find_rest_keeps_target_none() {
        let mut world = World::new();
        let ocean = MockNoWater;
        let veg = MockVegetation {
            harvestables: vec![make_harvestable(5.0, 1.0, 0.0)],
        };

        let e = world.spawn((
            Position(Vec3::ZERO),
            Desire {
                kind: DesireKind::Rest,
                urgency: 0.9,
            },
        ));

        run_and_flush(&mut world, Some(&veg as &dyn VegetationProvider), &ocean);

        let goal = world.get::<&Goal>(e).expect("should have Goal");
        assert_eq!(goal.goal_type, GoalType::FindRest);
        assert!(
            goal.target_pos.is_none(),
            "FindRest not resolved (Phase 3+)"
        );
    }

    #[test]
    fn test_harvestable_outside_radius_not_found() {
        let mut world = World::new();
        let ocean = MockNoWater;
        let veg = MockVegetation {
            harvestables: vec![
                // 所有采集物都在 150m 搜索半径之外
                make_harvestable(200.0, 1.0, 200.0),
            ],
        };

        let e = world.spawn((
            Position(Vec3::ZERO),
            Desire {
                kind: DesireKind::Eat,
                urgency: 0.9,
            },
        ));

        // 使用小搜索半径
        let mut cmd = CommandBuffer::new();
        goal_resolution_system(
            &world,
            &mut cmd,
            Some(&veg as &dyn VegetationProvider),
            &ocean,
            50.0, // 小半径——200m 外的采集物搜不到
        );
        cmd.run_on(&mut world);

        let goal = world.get::<&Goal>(e).expect("should have Goal");
        assert!(
            goal.target_pos.is_none(),
            "harvestable at 200m > 50m radius"
        );
    }

    #[test]
    fn test_water_search_deterministic() {
        // 同位置+同 provider → 同结果（确定性）
        let ocean = MockAllWater;

        let result1 = find_nearest_water_xz(Vec2::new(10.0, 20.0), &ocean, 150.0);
        let result2 = find_nearest_water_xz(Vec2::new(10.0, 20.0), &ocean, 150.0);

        match (result1, result2) {
            (Some(a), Some(b)) => {
                assert!((a - b).length() < 0.01, "deterministic: {:?} vs {:?}", a, b);
            }
            (None, None) => {} // 都是 None 也是确定性的
            _ => panic!("mismatched Some/None"),
        }
    }
}
