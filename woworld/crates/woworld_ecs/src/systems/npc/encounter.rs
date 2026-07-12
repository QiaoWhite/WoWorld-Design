//! encounter_system — 遭遇感知层（邻近边沿检测 + 迟滞 + 首帧播种 + despawn 归因）
//!
//! barrier-free 定位：**感知层**（通用），产出 `EncounterEvent` 供表达层（问候气泡）+
//! 未来战斗警觉/目击/记忆消费。**不决策行为**——"见到人做什么"由 `ActionWeight` 涌现。
//!
//! `neighbors_within` 是 social/encounter 的**单一邻近扫描原语**（XZ 水平距离，与 social
//! 现行一致）。Phase-3 收敛到 `resources/spatial_grid.rs`（EntityIndex·已存在待填充）替换此处。
//!
//! 设计对齐 `语言表达/006` `Reactive::SomeoneEntered/SomeoneLeft`。

use std::collections::{HashMap, HashSet};

use glam::Vec3;

use crate::components::needs::Needs;
use crate::components::transform::Position;
use crate::components::vitals::Corpse;
use crate::resources::encounter_state::{pair_key, EncounterEvent, EncounterKind, EncounterState};

/// 进入邻近的距离阈（m）——施密特触发下沿
pub const ENTER_RADIUS: f32 = 3.0;
/// 离开邻近的距离阈（m）——施密特触发上沿（> ENTER 防边界抖动）
pub const LEAVE_RADIUS: f32 = 4.0;

/// 邻居配对原语——返回所有 **XZ 水平距离 < radius** 的索引对 `(i, j, dist)`，i<j。
///
/// 纯几何（glam），O(n²)。social/encounter 共用此单一扫描源（与 social 现行 XZ 距离一致）。
/// 村庄规模 3-5 NPC 成本可忽略；Phase-3 收敛到 SpatialGrid 于此单点替换。
pub fn neighbors_within(positions: &[Vec3], radius: f32) -> Vec<(usize, usize, f32)> {
    let mut pairs = Vec::new();
    let r2 = radius * radius;
    for i in 0..positions.len() {
        for j in (i + 1)..positions.len() {
            let dx = positions[i].x - positions[j].x;
            let dz = positions[i].z - positions[j].z;
            let d2 = dx * dx + dz * dz;
            if d2 < r2 {
                pairs.push((i, j, d2.sqrt()));
            }
        }
    }
    pairs
}

/// 每帧检测遭遇边沿，产 `EncounterEvent`。
///
/// - agent = 有 `Position` + `Needs`（NPC/被夺舍玩家），排除 `Corpse`（死者不遭遇）
/// - 迟滞：进入 <3m、离开 >4m，中间保持原态防抖动
/// - 首帧播种：已邻近对标记"在范围内"、**不发 Enter**（N2 防开局群体问候）
/// - Leave 归因：对方 despawn → `by_despawn=true`（表达层据此不冒告别·G3）
pub fn encounter_system(world: &hecs::World, state: &mut EncounterState) {
    state.events.clear();

    // 采集活体 agent（有 Position+Needs，非 Corpse）——排除物品/地形/尸体
    let agents: Vec<(hecs::Entity, Vec3)> = world
        .query::<(&Position, &Needs)>()
        .without::<&Corpse>()
        .iter()
        .map(|(e, (p, _))| (e, p.0))
        .collect();

    let positions: Vec<Vec3> = agents.iter().map(|(_, p)| *p).collect();
    // LEAVE 半径超集——迟滞需要 [ENTER, LEAVE) 区间的对也在候选里
    let near = neighbors_within(&positions, LEAVE_RADIUS);

    // 存活 bits 集（Leave 的 despawn 归因）
    let alive: HashSet<u64> = agents.iter().map(|(e, _)| e.to_bits().get()).collect();

    // near 对：键 → (entA, entB, dist)
    let near_map: HashMap<(u64, u64), (hecs::Entity, hecs::Entity, f32)> = near
        .iter()
        .map(|&(i, j, d)| {
            let (ea, eb) = (agents[i].0, agents[j].0);
            (pair_key(ea, eb), (ea, eb, d))
        })
        .collect();

    // ── 首帧播种（N2）：已 <ENTER 的对入范围、不发事件 ──
    if !state.seeded {
        for (&key, &(_, _, d)) in &near_map {
            if d < ENTER_RADIUS {
                state.in_range.insert(key);
            }
        }
        state.seeded = true;
        return;
    }

    let old = std::mem::take(&mut state.in_range);
    let mut new_in_range: HashSet<(u64, u64)> = HashSet::new();

    // ① 旧的在范围对：仍 <LEAVE 且双方存活 → 保持（迟滞）；否则 Leave
    for key in old.iter().copied() {
        let both_alive = alive.contains(&key.0) && alive.contains(&key.1);
        if both_alive && near_map.contains_key(&key) {
            new_in_range.insert(key);
            continue;
        }
        if let (Some(ea), Some(eb)) = (
            hecs::Entity::from_bits(key.0),
            hecs::Entity::from_bits(key.1),
        ) {
            state.events.push(EncounterEvent {
                a: ea,
                b: eb,
                kind: EncounterKind::Leave,
                by_despawn: !both_alive,
            });
        }
    }

    // ② 新进入对：<ENTER 且此前不在范围 → Enter
    for (&key, &(ea, eb, d)) in &near_map {
        if d < ENTER_RADIUS && !old.contains(&key) && !new_in_range.contains(&key) {
            new_in_range.insert(key);
            state.events.push(EncounterEvent {
                a: ea,
                b: eb,
                kind: EncounterKind::Enter,
                by_despawn: false,
            });
        }
    }

    state.in_range = new_in_range;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_agent(world: &mut hecs::World, x: f32, z: f32) -> hecs::Entity {
        world.spawn((Position(Vec3::new(x, 0.0, z)), Needs::default()))
    }

    #[test]
    fn test_neighbors_within_xz() {
        let ps = [
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(2.0, 100.0, 0.0), // Y 远但 XZ 近
            Vec3::new(50.0, 0.0, 0.0),
        ];
        let n = neighbors_within(&ps, 3.0);
        assert_eq!(n.len(), 1, "只有 0-1 对 XZ<3");
        assert_eq!((n[0].0, n[0].1), (0, 1));
    }

    #[test]
    fn test_seed_no_enter_events() {
        let mut world = hecs::World::new();
        spawn_agent(&mut world, 0.0, 0.0);
        spawn_agent(&mut world, 1.0, 0.0); // 1m 内
        let mut state = EncounterState::new();
        encounter_system(&world, &mut state); // 首帧播种
        assert!(state.events.is_empty(), "首帧不应发 Enter（防爆发）");
        assert_eq!(state.in_range.len(), 1, "已邻近对应入范围");
    }

    #[test]
    fn test_enter_event_on_new_proximity() {
        let mut world = hecs::World::new();
        let a = spawn_agent(&mut world, 0.0, 0.0);
        let b = spawn_agent(&mut world, 10.0, 0.0); // 初始远
        let mut state = EncounterState::new();
        encounter_system(&world, &mut state); // 播种（远·无对）
                                              // 移近 b 到 2m
        *world.get::<&mut Position>(b).unwrap() = Position(Vec3::new(2.0, 0.0, 0.0));
        encounter_system(&world, &mut state);
        assert_eq!(state.events.len(), 1);
        assert_eq!(state.events[0].kind, EncounterKind::Enter);
        let e = state.events[0];
        assert!((e.a == a && e.b == b) || (e.a == b && e.b == a));
    }

    #[test]
    fn test_hysteresis_no_flicker() {
        let mut world = hecs::World::new();
        let _a = spawn_agent(&mut world, 0.0, 0.0);
        let b = spawn_agent(&mut world, 10.0, 0.0);
        let mut state = EncounterState::new();
        encounter_system(&world, &mut state); // 播种
                                              // 进入 2m → Enter
        *world.get::<&mut Position>(b).unwrap() = Position(Vec3::new(2.0, 0.0, 0.0));
        encounter_system(&world, &mut state);
        assert_eq!(state.events.len(), 1);
        // 退到 3.5m（ENTER<3.5<LEAVE）→ 迟滞保持，无事件
        *world.get::<&mut Position>(b).unwrap() = Position(Vec3::new(3.5, 0.0, 0.0));
        encounter_system(&world, &mut state);
        assert!(state.events.is_empty(), "迟滞区间不应抖动");
        assert_eq!(state.in_range.len(), 1);
        // 退到 5m（>LEAVE）→ Leave
        *world.get::<&mut Position>(b).unwrap() = Position(Vec3::new(5.0, 0.0, 0.0));
        encounter_system(&world, &mut state);
        assert_eq!(state.events.len(), 1);
        assert_eq!(state.events[0].kind, EncounterKind::Leave);
        assert!(!state.events[0].by_despawn);
    }

    #[test]
    fn test_leave_by_despawn_flagged() {
        let mut world = hecs::World::new();
        let _a = spawn_agent(&mut world, 0.0, 0.0);
        let b = spawn_agent(&mut world, 1.0, 0.0);
        let mut state = EncounterState::new();
        // 播种时已邻近 → in_range，但需先 Enter 才能 Leave；播种后 despawn
        encounter_system(&world, &mut state); // 播种（已 in_range）
        world.despawn(b).unwrap();
        encounter_system(&world, &mut state);
        assert_eq!(state.events.len(), 1);
        assert_eq!(state.events[0].kind, EncounterKind::Leave);
        assert!(state.events[0].by_despawn, "despawn 离场应标记");
    }

    #[test]
    fn test_corpse_excluded() {
        let mut world = hecs::World::new();
        let _a = spawn_agent(&mut world, 0.0, 0.0);
        let b = spawn_agent(&mut world, 1.0, 0.0);
        world.insert_one(b, Corpse::default()).unwrap();
        let mut state = EncounterState::new();
        encounter_system(&world, &mut state);
        assert!(state.in_range.is_empty(), "尸体不参与遭遇");
    }
}
