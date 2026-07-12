//! ECS → EntityVisual / EntityDebugSnapshot 数据收集 System
//!
//! - entity_visual_system: 每帧运行，遍历所有 Position+EntityKind+LodLevel 实体
//! - entity_debug_system: 按需运行，为选中实体收集完整调试数据

use glam::Quat;
use std::collections::HashMap;

use woworld_core::entity_visual::{DebugField, DebugSection, EntityDebugSnapshot, EntityVisual};
use woworld_core::naming::generate_name;

use crate::components::action::ActionIntent;
use crate::components::economy::{EconomicCognition, Wallet};
use crate::components::emotion::Emotion;
use crate::components::entity_kind::EntityKind;
use crate::components::growth::GrowthNeeds;
use crate::components::lod::LodLevel;
use crate::components::transform::{Position, Rotation};
use crate::resources::inventory_registry::InventoryRegistry;
use crate::resources::speech_bubble_state::SpeechBubbleState;

/// 名字缓存：首次遇到实体时生成并缓存，后续直接返回
type NameCache = HashMap<hecs::Entity, String>;

/// 每帧收集所有实体的 (Entity, EntityVisual) 对
///
/// - `player_entity` 被**标记** `controlled=true`（不再排除；007 §九"玩家=NPC"统一路径）
/// - 从 LodLevel.render_lod 读取裁剪等级
/// - 从 Emotion PAD 计算 color_hint
/// - 从 NameCache 或 generate_name() 获取 display_name
/// - 从 SpeechBubbleState 读取当前活跃气泡文字/颜色
pub fn entity_visual_system(
    world: &hecs::World,
    player_entity: Option<hecs::Entity>,
    name_cache: &mut NameCache,
    bubble_state: &SpeechBubbleState,
) -> Vec<(hecs::Entity, EntityVisual)> {
    let mut visuals = Vec::new();

    for (entity, (pos, kind, lod)) in world.query::<(&Position, &EntityKind, &LodLevel)>().iter() {
        // ★ 007 §九：不再排除玩家——改为打标 controlled，走 entity_renderer 统一路径
        let controlled = player_entity == Some(entity);

        let rotation = world
            .get::<&Rotation>(entity)
            .map(|r| r.0)
            .unwrap_or(Quat::IDENTITY);

        let color_hint = world
            .get::<&Emotion>(entity)
            .map(|e| pad_to_color(&e))
            .unwrap_or([0.5, 0.5, 0.5]);

        let display_name = name_cache
            .entry(entity)
            .or_insert_with(|| {
                let seed = entity.to_bits().get();
                generate_name(seed).full()
            })
            .clone();

        let (bubble_text, bubble_color) = match bubble_state.active_for(entity) {
            Some(active) => (Some(active.text.clone()), Some(active.bubble_type.color())),
            None => (None, None),
        };

        visuals.push((
            entity,
            EntityVisual {
                position: pos.0,
                rotation,
                display_name,
                color_hint,
                kind: kind.to_core(),
                render_lod: lod.render_lod,
                bubble_text,
                bubble_color,
                controlled,
            },
        ));
    }

    visuals
}

/// 为单个实体收集完整调试快照（info 命令触发）
/// 为单个实体收集完整调试快照（`info` 命令 / V5 检视面板触发）
///
/// `inventory`: 可选注册表引用——console `info` 传 `None`，检视面板传 `Some(&registry)`。
///   类型系统强制调用者显式处理"无 registry"情况。
pub fn entity_debug_system(
    world: &hecs::World,
    entity: hecs::Entity,
    inventory: Option<&InventoryRegistry>,
) -> Option<EntityDebugSnapshot> {
    let pos = world.get::<&Position>(entity).ok()?;
    let kind = world.get::<&EntityKind>(entity).ok()?;

    let entity_bits = entity.to_bits().get();
    let seed = entity_bits;
    let display_name = generate_name(seed).full();

    let mut sections = Vec::new();

    // 通用字段：变换
    let rot = world
        .get::<&Rotation>(entity)
        .map(|r| r.0)
        .unwrap_or(Quat::IDENTITY);
    sections.push(DebugSection {
        title: "Transform".into(),
        fields: vec![
            DebugField {
                label: "Position".into(),
                value: format!("({:.2}, {:.2}, {:.2})", pos.0.x, pos.0.y, pos.0.z),
                color_hint: None,
            },
            DebugField {
                label: "Rotation".into(),
                value: format!("({:.3}, {:.3}, {:.3}, {:.3})", rot.x, rot.y, rot.z, rot.w),
                color_hint: None,
            },
        ],
    });

    // Creature 专属：Action, Wallet, Economy, Growth, Vitals, Emotion, BigFive, Goal, Needs,
    //   Movement, Lifecycle, Inventory (★ V5 扩展)
    if matches!(*kind, EntityKind::Creature) {
        collect_creature_debug(world, entity, inventory, &mut sections);
    }

    // Plant / DroppedItem / etc — 占位（Component 尚未定义）
    // 后续 EntityKind 变体增加时，编译器强制穷尽 match 臂追加

    Some(EntityDebugSnapshot {
        entity_bits,
        kind: kind.to_core(),
        display_name,
        position: pos.0,
        sections,
    })
}

/// 收集 Creature 实体的全部 NPC Component 数据（★ V5 扩展：Action/Wallet/Economy/Growth/Inventory）
fn collect_creature_debug(
    world: &hecs::World,
    entity: hecs::Entity,
    inventory: Option<&InventoryRegistry>,
    sections: &mut Vec<DebugSection>,
) {
    // ★ V5: Action — NPC 正在做什么
    if let Ok(ai) = world.get::<&ActionIntent>(entity) {
        sections.push(DebugSection {
            title: "Action".into(),
            fields: vec![
                DebugField {
                    label: "Category".into(),
                    value: format!("{:?}", ai.category),
                    color_hint: None,
                },
                DebugField {
                    label: "Weight".into(),
                    value: format!("{:.3}", ai.weight),
                    color_hint: None,
                },
            ],
        });
    }

    // ★ V5: Wallet — 铜/银/金币
    if let Ok(w) = world.get::<&Wallet>(entity) {
        sections.push(DebugSection {
            title: "Wallet".into(),
            fields: vec![
                DebugField {
                    label: "Copper".into(),
                    value: format!("{}", w.copper),
                    color_hint: None,
                },
                DebugField {
                    label: "Silver".into(),
                    value: format!("{}", w.silver),
                    color_hint: None,
                },
                DebugField {
                    label: "Gold".into(),
                    value: format!("{}", w.gold),
                    color_hint: None,
                },
                DebugField {
                    label: "Total (copper)".into(),
                    value: format!("{}", w.total_copper()),
                    color_hint: None,
                },
            ],
        });
    }

    // ★ V5: Economy — 经济认知 6 维
    if let Ok(ec) = world.get::<&EconomicCognition>(entity) {
        sections.push(DebugSection {
            title: "Economy".into(),
            fields: vec![
                DebugField {
                    label: "Financial Literacy".into(),
                    value: format!("{:.3}", ec.financial_literacy),
                    color_hint: None,
                },
                DebugField {
                    label: "Market Understanding".into(),
                    value: format!("{:.3}", ec.market_understanding),
                    color_hint: None,
                },
                DebugField {
                    label: "Price Memory".into(),
                    value: format!("{:.3}", ec.price_memory_accuracy),
                    color_hint: None,
                },
                DebugField {
                    label: "Time Pref. Rate".into(),
                    value: format!("{:.3}", ec.time_preference_rate),
                    color_hint: None,
                },
                DebugField {
                    label: "Search Breadth".into(),
                    value: format!("{}", ec.market_search_breadth),
                    color_hint: None,
                },
            ],
        });
    }

    // ★ V5: Growth — 高层次需求
    if let Ok(gn) = world.get::<&GrowthNeeds>(entity) {
        sections.push(DebugSection {
            title: "Growth".into(),
            fields: vec![
                DebugField {
                    label: "Esteem Deficit".into(),
                    value: format!("{:.3}", gn.esteem_deficit),
                    color_hint: None,
                },
                DebugField {
                    label: "Competence Frustration".into(),
                    value: format!("{:.3}", gn.competence_frustration),
                    color_hint: None,
                },
                DebugField {
                    label: "Chronic Days".into(),
                    value: format!("{}", gn.chronic_days),
                    color_hint: None,
                },
            ],
        });
    }

    // ★ V5: Inventory — 通过 InventoryRegistry 查询（非 Component）
    if let Some(reg) = inventory {
        let eid = woworld_core::types::EntityId(entity.to_bits().get());
        let held_count = reg.get_holdings(eid).len();
        let has_eq = reg.has_equipment(eid);
        sections.push(DebugSection {
            title: "Inventory".into(),
            fields: vec![
                DebugField {
                    label: "Items Held".into(),
                    value: format!("{}", held_count),
                    color_hint: None,
                },
                DebugField {
                    label: "Has Equipment".into(),
                    value: if has_eq { "yes".into() } else { "no".into() },
                    color_hint: None,
                },
            ],
        });
    }

    // Vitals
    if let Ok(v) = world.get::<&crate::components::vitals::Vitals>(entity) {
        sections.push(DebugSection {
            title: "Vitals".into(),
            fields: vec![
                DebugField {
                    label: "HP".into(),
                    value: format!("{:.1} / {:.1}", v.hp, v.max_hp),
                    color_hint: hp_color(v.hp, v.max_hp),
                },
                DebugField {
                    label: "Stamina".into(),
                    value: format!("{:.1} / {:.1}", v.stamina, v.max_stamina),
                    color_hint: None,
                },
                DebugField {
                    label: "Spirit".into(),
                    value: format!("{:.2}", v.spirit),
                    color_hint: None,
                },
                DebugField {
                    label: "Body Temp".into(),
                    value: format!("{:.1}°C", v.body_temp),
                    color_hint: None,
                },
                DebugField {
                    label: "Oxygen".into(),
                    value: format!("{:.1}", v.oxygen),
                    color_hint: None,
                },
            ],
        });
    }

    // Emotion (PAD)
    if let Ok(e) = world.get::<&Emotion>(entity) {
        sections.push(DebugSection {
            title: "Emotion (PAD)".into(),
            fields: vec![
                DebugField {
                    label: "Pleasure".into(),
                    value: format!("{:.3}", e.pleasure),
                    color_hint: None,
                },
                DebugField {
                    label: "Arousal".into(),
                    value: format!("{:.3}", e.arousal),
                    color_hint: None,
                },
                DebugField {
                    label: "Control".into(),
                    value: format!("{:.3}", e.control),
                    color_hint: None,
                },
            ],
        });
    }

    // BigFive
    if let Ok(bf) = world.get::<&crate::components::bigfive::BigFive>(entity) {
        sections.push(DebugSection {
            title: "BigFive".into(),
            fields: vec![
                DebugField {
                    label: "Openness".into(),
                    value: format!("{:.3}", bf.openness),
                    color_hint: None,
                },
                DebugField {
                    label: "Conscientiousness".into(),
                    value: format!("{:.3}", bf.conscientiousness),
                    color_hint: None,
                },
                DebugField {
                    label: "Extraversion".into(),
                    value: format!("{:.3}", bf.extraversion),
                    color_hint: None,
                },
                DebugField {
                    label: "Agreeableness".into(),
                    value: format!("{:.3}", bf.agreeableness),
                    color_hint: None,
                },
                DebugField {
                    label: "Neuroticism".into(),
                    value: format!("{:.3}", bf.neuroticism),
                    color_hint: None,
                },
            ],
        });
    }

    // Goal
    if let Ok(g) = world.get::<&crate::components::goal::Goal>(entity) {
        let target_str = g
            .target_pos
            .map(|p| format!("({:.1}, {:.1}, {:.1})", p.x, p.y, p.z))
            .unwrap_or_else(|| "(none)".into());
        sections.push(DebugSection {
            title: "Goal".into(),
            fields: vec![
                DebugField {
                    label: "Type".into(),
                    value: format!("{:?}", g.goal_type),
                    color_hint: None,
                },
                DebugField {
                    label: "Urgency".into(),
                    value: format!("{:.3}", g.urgency),
                    color_hint: None,
                },
                DebugField {
                    label: "Target".into(),
                    value: target_str,
                    color_hint: None,
                },
            ],
        });
    }

    // Needs
    if let Ok(n) = world.get::<&crate::components::needs::Needs>(entity) {
        sections.push(DebugSection {
            title: "Needs".into(),
            fields: vec![
                DebugField {
                    label: "Hunger".into(),
                    value: format!("{:.3}", n.hunger),
                    color_hint: None,
                },
                DebugField {
                    label: "Thirst".into(),
                    value: format!("{:.3}", n.thirst),
                    color_hint: None,
                },
                DebugField {
                    label: "Fatigue".into(),
                    value: format!("{:.3}", n.fatigue),
                    color_hint: None,
                },
                DebugField {
                    label: "Safety".into(),
                    value: format!("{:.3}", n.safety),
                    color_hint: None,
                },
                DebugField {
                    label: "Social".into(),
                    value: format!("{:.3}", n.social),
                    color_hint: None,
                },
                DebugField {
                    label: "Libido".into(),
                    value: format!("{:.3}", n.libido),
                    color_hint: None,
                },
            ],
        });
    }

    // Movement
    if let Ok(m) = world.get::<&crate::components::movement::Movement>(entity) {
        sections.push(DebugSection {
            title: "Movement".into(),
            fields: vec![
                DebugField {
                    label: "Speed".into(),
                    value: format!("{:.1} m/s", m.speed),
                    color_hint: None,
                },
                DebugField {
                    label: "Arrival Radius".into(),
                    value: format!("{:.2} m", m.arrival_radius),
                    color_hint: None,
                },
            ],
        });
    }

    // Lifecycle
    if let Ok(a) = world.get::<&crate::components::lifecycle::Age>(entity) {
        sections.push(DebugSection {
            title: "Lifecycle".into(),
            fields: vec![
                DebugField {
                    label: "Age".into(),
                    value: format!("{:.1} years", a.age_days / 365.25),
                    color_hint: None,
                },
                DebugField {
                    label: "Max Lifespan".into(),
                    value: format!("{:.1} years", a.max_lifespan_days / 365.25),
                    color_hint: None,
                },
                DebugField {
                    label: "Ratio".into(),
                    value: format!("{:.3}", a.age_ratio()),
                    color_hint: None,
                },
            ],
        });
        if let Ok(ls) = world.get::<&crate::components::lifecycle::LifeStage>(entity) {
            sections.last_mut().unwrap().fields.push(DebugField {
                label: "Stage".into(),
                value: format!("{:?}", ls),
                color_hint: None,
            });
        }
    }
}

// ── 辅助函数 ────────────────────────────

/// PAD 三维→ RGB 颜色映射
fn pad_to_color(e: &Emotion) -> [f32; 3] {
    let r = 0.5 + (1.0 - e.pleasure) * 0.5;
    let g = 0.5 + e.pleasure * 0.5;
    let b = 0.5 + e.control * 0.5;
    let factor = 0.4 + e.arousal * 0.6;
    [
        (r * factor).clamp(0.0, 1.0),
        (g * factor).clamp(0.0, 1.0),
        (b * factor).clamp(0.0, 1.0),
    ]
}

/// HP 颜色：高=绿, 低=红
fn hp_color(hp: f32, max_hp: f32) -> Option<String> {
    let ratio = if max_hp > 0.0 { hp / max_hp } else { 0.0 };
    if ratio > 0.5 {
        None // 绿色默认
    } else if ratio > 0.25 {
        Some("#ffaa00".into()) // 橙色
    } else {
        Some("#ff4444".into()) // 红色
    }
}

// ── 测试 ────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::emotion::Emotion;
    use crate::components::entity_kind::EntityKind as EcsKind;
    use crate::components::lod::LodLevel;
    use crate::components::transform::Position;
    use glam::Vec3;

    fn test_world() -> hecs::World {
        let mut world = hecs::World::new();
        // NPC 1 — 愉快、活跃
        world.spawn((
            Position(Vec3::new(10.0, 0.0, 5.0)),
            EcsKind::Creature,
            LodLevel::default(),
            Emotion {
                pleasure: 0.7,
                arousal: 0.8,
                control: 0.5,
            },
        ));
        // NPC 2 — 不愉快、平静
        world.spawn((
            Position(Vec3::new(20.0, 0.0, 5.0)),
            EcsKind::Creature,
            LodLevel {
                render_lod: 3,
                ..LodLevel::default()
            },
            Emotion {
                pleasure: -0.5,
                arousal: 0.2,
                control: -0.3,
            },
        ));
        world
    }

    #[test]
    fn test_visual_system_collects_all() {
        let world = test_world();
        let mut cache = NameCache::new();
        let visuals = entity_visual_system(&world, None, &mut cache, &SpeechBubbleState::default());
        assert_eq!(visuals.len(), 2);
        // player_entity=None → both should be controlled=false
        assert!(visuals.iter().all(|(_, v)| !v.controlled));
    }

    #[test]
    fn test_visual_system_marks_controlled() {
        // 007 §九: player entity is now included, marked controlled=true
        let world = test_world();
        let player = world.query::<&Position>().iter().next().unwrap().0;
        let mut cache = NameCache::new();
        let visuals = entity_visual_system(
            &world,
            Some(player),
            &mut cache,
            &SpeechBubbleState::default(),
        );
        // 2 entities total: player is now included
        assert_eq!(visuals.len(), 2);
        // Exactly one is controlled
        let controlled_count = visuals.iter().filter(|(_, v)| v.controlled).count();
        assert_eq!(controlled_count, 1);
        // The controlled entity should be the player entity
        let (controlled_entity, _) = visuals.iter().find(|(_, v)| v.controlled).unwrap();
        assert_eq!(*controlled_entity, player);
    }

    #[test]
    fn test_visual_lod_preserved() {
        let world = test_world();
        let mut cache = NameCache::new();
        let visuals = entity_visual_system(&world, None, &mut cache, &SpeechBubbleState::default());
        let npc2 = visuals.iter().find(|(_, v)| v.render_lod == 3).unwrap();
        assert!(!npc2.1.show_label());
        assert!(npc2.1.is_visible());
    }

    #[test]
    fn test_debug_system_nonexistent() {
        let mut world = hecs::World::new();
        let entity = world.spawn((Position(Vec3::ZERO),));
        // Entity 已 despawn → 查询不到
        let mut world_mut = world;
        world_mut.despawn(entity).unwrap();
        // 用无效 entity 查询
        assert!(entity_debug_system(&world_mut, entity, None).is_none());
    }

    #[test]
    fn test_debug_system_creature() {
        let mut world = hecs::World::new();
        let e = world.spawn((
            Position(Vec3::new(1.0, 2.0, 3.0)),
            EcsKind::Creature,
            Rotation::default(),
            crate::components::action::ActionIntent {
                category: crate::components::action::ActionCategory::Eat,
                weight: 0.72,
            },
            crate::components::economy::Wallet {
                copper: 15,
                silver: 3,
                gold: 0,
            },
            crate::components::economy::EconomicCognition::default(),
            crate::components::growth::GrowthNeeds::default(),
            crate::components::vitals::Vitals::default(),
            Emotion::default(),
            crate::components::bigfive::BigFive::default(),
            crate::components::goal::Goal::default(),
            crate::components::needs::Needs::default(),
            crate::components::movement::Movement::default(),
            crate::components::lifecycle::Age::new(80.0, 30.0),
            crate::components::lifecycle::LifeStage::YoungAdult,
        ));
        let snap = entity_debug_system(&world, e, None).unwrap();
        assert_eq!(snap.entity_bits, e.to_bits().get());
        // ★ V5: Transform + Action + Wallet + Economy + Growth + Vitals +
        //   Emotion + BigFive + Goal + Needs + Movement + Lifecycle (12 sections min)
        //   Inventory section absent because None was passed
        assert!(
            snap.sections.len() >= 12,
            "expected >=12 sections, got {}",
            snap.sections.len()
        );

        // 验证新 section 存在且值正确
        let has_action = snap.sections.iter().any(|s| s.title == "Action");
        let has_wallet = snap.sections.iter().any(|s| s.title == "Wallet");
        let has_economy = snap.sections.iter().any(|s| s.title == "Economy");
        let has_growth = snap.sections.iter().any(|s| s.title == "Growth");
        assert!(has_action, "missing Action section");
        assert!(has_wallet, "missing Wallet section");
        assert!(has_economy, "missing Economy section");
        assert!(has_growth, "missing Growth section");
    }

    /// ★ V5: Inventory section 存在当 registry 传入
    #[test]
    fn test_debug_system_with_inventory_registry() {
        use crate::resources::inventory_registry::InventoryRegistry;
        use woworld_core::types::EntityId;

        let mut world = hecs::World::new();
        let e = world.spawn((Position(Vec3::ZERO), EcsKind::Creature, Rotation::default()));
        let mut reg = InventoryRegistry::new();
        let eid = EntityId(e.to_bits().get());
        reg.init_inventory(eid, 10);

        let snap = entity_debug_system(&world, e, Some(&reg)).unwrap();
        let has_inv = snap.sections.iter().any(|s| s.title == "Inventory");
        assert!(has_inv, "missing Inventory section when registry is Some");

        // None 时不出现 Inventory
        let snap_none = entity_debug_system(&world, e, None).unwrap();
        let has_inv_none = snap_none.sections.iter().any(|s| s.title == "Inventory");
        assert!(
            !has_inv_none,
            "Inventory section should be absent when registry is None"
        );
    }

    #[test]
    fn test_name_cache_reuse() {
        let mut world = hecs::World::new();
        let _e = world.spawn((Position(Vec3::ZERO), EcsKind::Creature, LodLevel::default()));
        let mut cache = NameCache::new();
        let v1 = entity_visual_system(&world, None, &mut cache, &SpeechBubbleState::default());
        let name1 = v1[0].1.display_name.clone();
        let v2 = entity_visual_system(&world, None, &mut cache, &SpeechBubbleState::default());
        assert_eq!(name1, v2[0].1.display_name, "name should be cached");
    }

    #[test]
    fn test_pad_to_color_happy() {
        let e = Emotion {
            pleasure: 1.0,
            arousal: 1.0,
            control: 1.0,
        };
        let c = pad_to_color(&e);
        // 愉快→绿色高，高唤醒→亮
        assert!(c[1] > 0.7, "green should be high for happy, got {}", c[1]);
        assert!(
            c[0] <= 0.5,
            "red should be neutral/low for happy, got {}",
            c[0]
        );
    }

    #[test]
    fn test_pad_to_color_unhappy() {
        let e = Emotion {
            pleasure: -1.0,
            arousal: 0.8,
            control: -1.0,
        };
        let c = pad_to_color(&e);
        // 不愉快→红色高
        assert!(c[0] > 0.7, "red should be high for unhappy, got {}", c[0]);
    }

    #[test]
    fn test_world_no_creatures() {
        let mut world = hecs::World::new();
        world.spawn((Position(Vec3::ZERO), EcsKind::Plant, LodLevel::default()));
        let mut cache = NameCache::new();
        let visuals = entity_visual_system(&world, None, &mut cache, &SpeechBubbleState::default());
        assert_eq!(visuals.len(), 1);
        assert_eq!(visuals[0].1.kind, woworld_core::types::EntityKind::Plant);
    }

    #[test]
    fn test_empty_world() {
        let world = hecs::World::new();
        let mut cache = NameCache::new();
        let visuals = entity_visual_system(&world, None, &mut cache, &SpeechBubbleState::default());
        assert!(visuals.is_empty());
    }
}
