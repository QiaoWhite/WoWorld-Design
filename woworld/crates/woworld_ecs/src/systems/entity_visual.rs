//! ECS → EntityVisual / EntityDebugSnapshot 数据收集 System
//!
//! - entity_visual_system: 每帧运行，遍历所有 Position+EntityKind+LodLevel 实体
//! - entity_debug_system: 按需运行，为选中实体收集完整调试数据

use glam::Quat;
use std::collections::HashMap;

use woworld_core::entity_visual::{DebugField, DebugSection, EntityDebugSnapshot, EntityVisual};
use woworld_core::naming::generate_name;

use crate::components::emotion::Emotion;
use crate::components::entity_kind::EntityKind;
use crate::components::lod::LodLevel;
use crate::components::transform::{Position, Rotation};

/// 名字缓存：首次遇到实体时生成并缓存，后续直接返回
type NameCache = HashMap<hecs::Entity, String>;

/// 每帧收集所有实体的 (Entity, EntityVisual) 对
///
/// - 排除 `player_entity`（有独立 Godot CharacterBody3D 渲染）
/// - 从 LodLevel.render_lod 读取裁剪等级
/// - 从 Emotion PAD 计算 color_hint
/// - 从 NameCache 或 generate_name() 获取 display_name
pub fn entity_visual_system(
    world: &hecs::World,
    player_entity: Option<hecs::Entity>,
    name_cache: &mut NameCache,
) -> Vec<(hecs::Entity, EntityVisual)> {
    let mut visuals = Vec::new();

    for (entity, (pos, kind, lod)) in world.query::<(&Position, &EntityKind, &LodLevel)>().iter() {
        // 排除 Player ECS 实体
        if player_entity == Some(entity) {
            continue;
        }

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

        visuals.push((
            entity,
            EntityVisual {
                position: pos.0,
                rotation,
                display_name,
                color_hint,
                kind: kind.to_core(),
                render_lod: lod.render_lod,
            },
        ));
    }

    visuals
}

/// 为单个实体收集完整调试快照（info 命令触发）
pub fn entity_debug_system(
    world: &hecs::World,
    entity: hecs::Entity,
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

    // Creature 专属：Vitals, Emotion, BigFive, Goal, Needs, Movement, Lifecycle, Social, Economy
    if matches!(*kind, EntityKind::Creature) {
        collect_creature_debug(world, entity, &mut sections);
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

/// 收集 Creature 实体的全部 NPC Component 数据
fn collect_creature_debug(
    world: &hecs::World,
    entity: hecs::Entity,
    sections: &mut Vec<DebugSection>,
) {
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
        let visuals = entity_visual_system(&world, None, &mut cache);
        assert_eq!(visuals.len(), 2);
    }

    #[test]
    fn test_visual_system_excludes_player() {
        let world = test_world();
        let player = world.query::<&Position>().iter().next().unwrap().0;
        let mut cache = NameCache::new();
        let visuals = entity_visual_system(&world, Some(player), &mut cache);
        assert_eq!(visuals.len(), 1);
    }

    #[test]
    fn test_visual_lod_preserved() {
        let world = test_world();
        let mut cache = NameCache::new();
        let visuals = entity_visual_system(&world, None, &mut cache);
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
        assert!(entity_debug_system(&world_mut, entity).is_none());
    }

    #[test]
    fn test_debug_system_creature() {
        let mut world = hecs::World::new();
        let e = world.spawn((
            Position(Vec3::new(1.0, 2.0, 3.0)),
            EcsKind::Creature,
            Rotation::default(),
            crate::components::vitals::Vitals::default(),
            Emotion::default(),
            crate::components::bigfive::BigFive::default(),
            crate::components::goal::Goal::default(),
            crate::components::needs::Needs::default(),
            crate::components::movement::Movement::default(),
            crate::components::lifecycle::Age::new(80.0, 30.0),
            crate::components::lifecycle::LifeStage::YoungAdult,
        ));
        let snap = entity_debug_system(&world, e).unwrap();
        assert_eq!(snap.entity_bits, e.to_bits().get());
        // 应包含 Transform + Vitals + Emotion + BigFive + Goal + Needs + Movement + Lifecycle
        assert!(
            snap.sections.len() >= 8,
            "expected >=8 sections, got {}",
            snap.sections.len()
        );
    }

    #[test]
    fn test_name_cache_reuse() {
        let mut world = hecs::World::new();
        let e = world.spawn((Position(Vec3::ZERO), EcsKind::Creature, LodLevel::default()));
        let mut cache = NameCache::new();
        let v1 = entity_visual_system(&world, None, &mut cache);
        let name1 = v1[0].1.display_name.clone();
        let v2 = entity_visual_system(&world, None, &mut cache);
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
        let visuals = entity_visual_system(&world, None, &mut cache);
        assert_eq!(visuals.len(), 1);
        assert_eq!(visuals[0].1.kind, woworld_core::types::EntityKind::Plant);
    }

    #[test]
    fn test_empty_world() {
        let world = hecs::World::new();
        let mut cache = NameCache::new();
        let visuals = entity_visual_system(&world, None, &mut cache);
        assert!(visuals.is_empty());
    }
}
