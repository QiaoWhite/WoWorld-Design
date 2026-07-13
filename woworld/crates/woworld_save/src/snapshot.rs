//! WorldSnapshot — 完整世界状态快照
//!
//! 包含所有需要持久化的数据：ECS entities、registries、WorldClock 等。
//! 使用手工 ComponentBag（非 hecs::serialize::column）以确保 entity ID 可控。

use serde::{Deserialize, Serialize};

use crate::header::SaveHeader;

// ── WorldSnapshot ──────────────────────────────────────────

/// 完整世界状态快照——存档文件的数据载荷
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldSnapshot {
    /// 存档文件头（magic/version/seed/timestamp/tick/name）
    pub header: SaveHeader,
    /// WorldClock 持久数据（非完整 WorldTime）
    pub clock: ClockData,
    /// 单调递增帧计数
    pub frame_count: u64,
    /// 累计游戏时间（真实秒，用于 input buffer timestamp 基线）
    pub game_time_secs: f32,
    /// 物品播种是否已完成（防重复播种）
    pub item_seeded: bool,
    /// 热栏配置（数字键 → ActionId 映射）
    pub hotbar_config: HotbarConfigData,
    /// 所有 ECS 实体快照
    pub entities: Vec<EntitySnapshot>,
    /// InventoryRegistry 库存数据
    pub inventory: InventorySnapshot,
    /// RelationStorage 关系数据
    pub relations: RelationSnapshot,
    /// EconomyRegistry 钱包数据（仅余额，非市场/订单簿）
    pub economy_wallets: EconomyWalletSnapshot,
    /// Action 实例计数器
    pub action_counter: u64,
    /// 玩家是否由 Block A0 角色控制器驱动（false = G 键自由飞行）
    pub block_a0_driving: bool,
}

// ── ClockData ──────────────────────────────────────────────

/// WorldClock 持久化数据
///
/// 仅保存权威计数器 `accumulator` 和 tuning 常量。
/// `WorldTime` 快照（day_progress/sun_elevation/light_level 等）是派生数据，
/// 加载时从 accumulator 重建。
/// `sun_orbit_radius` 是渲染常量（500.0），不保存。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClockData {
    /// 累计时间（秒）——权威计数器
    pub accumulator: f64,
    /// 现实秒 / 游戏天
    pub seconds_per_day: f64,
    /// 一年天数
    pub days_per_year: u64,
    /// 模拟速度乘数
    pub time_scale: f32,
}

// ── HotbarConfigData ──────────────────────────────────────

/// 热栏配置快照（玩家数字键 → ActionId 映射）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotbarConfigData {
    /// slots[0-9]: 每个槽位绑定的 ActionId (None = 未绑定)
    pub slots: [Option<u32>; 10],
}

// ── EntitySnapshot ─────────────────────────────────────────

/// 单个 ECS 实体的持久化快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySnapshot {
    /// 原始 hecs Entity bits（保存时的 entity ID）
    pub old_id_bits: u64,
    /// 该实体的所有持久 Component
    pub components: ComponentBag,
}

// ── ComponentBag ───────────────────────────────────────────

/// 一个实体的所有持久 Component 的集合
///
/// 每个 field 是 `Option<T>`——None 表示该实体没有此 Component。
/// 不使用 hecs::serialize::column 以确保 entity ID 可控
/// （hecs 反序列化不保留原始 entity bits）。
///
/// **保存策略**:
/// - ✅ 保存: 持久状态 Component（Position/Vitals/BigFive/Needs/Wallet 等）
/// - ❌ 跳过: 瞬时/每帧重算 Component（LodLevel/input_state/movement_state/action_state）
/// - 🔄 重派生: 纯函数型 Component（从 BigFive 重新计算）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentBag {
    // ── Transform ──
    pub position: Option<woworld_ecs::components::transform::Position>,
    pub rotation: Option<woworld_ecs::components::transform::Rotation>,
    pub velocity: Option<woworld_ecs::components::transform::Velocity>,

    // ── Identity ──
    pub entity_kind: Option<woworld_ecs::components::entity_kind::EntityKind>,
    pub player: Option<woworld_ecs::components::player::PlayerComponent>,
    pub control_mode: Option<woworld_ecs::components::player::ControlModeComponent>,
    pub biological_sex: Option<woworld_ecs::components::gender::BiologicalSex>,
    pub culture: Option<woworld_ecs::components::culture::Culture>,
    pub faith: Option<woworld_ecs::components::faith::Faith>,

    // ── Personality (root) ──
    pub bigfive: Option<woworld_ecs::components::bigfive::BigFive>,

    // ── Personality (derived — saved for non-deterministic seeds) ──
    pub aesthetic_taste: Option<woworld_ecs::components::aesthetic::AestheticTaste>,
    pub cognitive_style: Option<woworld_ecs::components::cognitive::CognitiveStyle>,

    // ── Personality (derived — NOT saved, re-derived from BigFive on load) ──
    //   Chronotype        → BigFive::derive_chronotype()
    //   CognitiveBiases   → CognitiveBiases::derive()
    //   NeedSensitivity   → BigFive::derive_sensitivity()
    //   EconomicCognition → BigFive::derive_from_bigfive()
    //   SocialPresence    → SocialPresence::derive_from_bigfive()

    // ── State ──
    pub emotion: Option<woworld_ecs::components::emotion::Emotion>,
    pub needs: Option<woworld_ecs::components::needs::Needs>,
    pub vitals: Option<woworld_ecs::components::vitals::Vitals>,
    pub regen_state: Option<woworld_ecs::components::vitals::RegenState>,
    pub death_cause: Option<woworld_ecs::components::vitals::DeathCause>,
    pub corpse: Option<woworld_ecs::components::vitals::Corpse>,
    pub decaying_remains: Option<woworld_ecs::components::vitals::DecayingRemains>,

    // ── Lifecycle ──
    pub age: Option<woworld_ecs::components::lifecycle::Age>,
    pub life_stage: Option<woworld_ecs::components::lifecycle::LifeStage>,
    pub gompertz: Option<woworld_ecs::components::lifecycle::GompertzMortality>,

    // ── Economy ──
    pub wallet: Option<woworld_ecs::components::economy::Wallet>,

    // ── Growth ──
    pub growth_needs: Option<woworld_ecs::components::growth::GrowthNeeds>,

    // ── Movement ──
    pub movement: Option<woworld_ecs::components::movement::Movement>,
    pub goal: Option<woworld_ecs::components::goal::Goal>,

    // ── Item (dropped items) ──
    pub item: Option<woworld_ecs::components::item::Item>,
    // ── Tags (ZST — NOT saved, re-derived on load) ──
    //   HasInventory  → from InventoryRegistry
    //   HasEquipment  → from InventoryRegistry
    //   RelationHandle → from RelationStorage
}

impl ComponentBag {
    /// 创建空的 ComponentBag（所有 field 为 None）
    pub fn empty() -> Self {
        Self {
            position: None,
            rotation: None,
            velocity: None,
            entity_kind: None,
            player: None,
            control_mode: None,
            biological_sex: None,
            culture: None,
            faith: None,
            bigfive: None,
            aesthetic_taste: None,
            cognitive_style: None,
            emotion: None,
            needs: None,
            vitals: None,
            regen_state: None,
            death_cause: None,
            corpse: None,
            decaying_remains: None,
            age: None,
            life_stage: None,
            gompertz: None,
            wallet: None,
            growth_needs: None,
            movement: None,
            goal: None,
            item: None,
        }
    }
}

impl Default for ComponentBag {
    fn default() -> Self {
        Self::empty()
    }
}

// ── Registry Snapshots ─────────────────────────────────────

/// InventoryRegistry 快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventorySnapshot {
    /// (EntityId bits, PersonalInventory) 对
    pub inventories: Vec<(u64, woworld_core::item::inventory::PersonalInventory)>,
    /// (EntityId bits, CharacterEquipment) 对
    pub equipment: Vec<(u64, woworld_core::item::equipment::CharacterEquipment)>,
}

/// RelationStorage 快照
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationSnapshot {
    /// ((entity_a_bits, entity_b_bits), Relationship) 对
    pub relations: Vec<(
        (u64, u64),
        woworld_ecs::resources::relation_storage::Relationship,
    )>,
    /// 上次维护 tick
    pub last_maintenance_tick: u64,
}

/// EconomyRegistry 钱包快照（仅余额，非市场/订单簿——MVP）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomyWalletSnapshot {
    /// (EntityId bits, copper, silver, gold) 对
    pub wallets: Vec<(u64, u64, u64, u64)>,
}

// ── Entity 收集 / 恢复 ────────────────────────────────────

/// 从 ECS World 收集所有持久实体的快照
///
/// 只收集有 `Position` 的实体（所有持久实体都有 Position）。
/// 跳过瞬时 Component（LodLevel/input_state/movement_state/action_state）。
/// 标记为 re-derive 的 Component 不保存（load 时从 BigFive 重建）。
pub fn collect_entities(ecs: &hecs::World) -> Vec<EntitySnapshot> {
    use woworld_ecs::components::*;
    let mut result = Vec::new();

    for (entity, _pos) in ecs.query::<&transform::Position>().iter() {
        let old_id_bits = entity.to_bits().get();
        let bag = ComponentBag {
            position: ecs.get::<&transform::Position>(entity).ok().map(|r| *r),
            rotation: ecs.get::<&transform::Rotation>(entity).ok().map(|r| *r),
            velocity: ecs.get::<&transform::Velocity>(entity).ok().map(|r| *r),
            entity_kind: ecs.get::<&entity_kind::EntityKind>(entity).ok().map(|r| *r),
            player: ecs
                .get::<&player::PlayerComponent>(entity)
                .ok()
                .map(|r| Clone::clone(&*r)),
            control_mode: ecs
                .get::<&player::ControlModeComponent>(entity)
                .ok()
                .map(|r| *r),
            biological_sex: ecs.get::<&gender::BiologicalSex>(entity).ok().map(|r| *r),
            culture: ecs.get::<&culture::Culture>(entity).ok().map(|r| *r),
            faith: ecs.get::<&faith::Faith>(entity).ok().map(|r| *r),
            bigfive: ecs.get::<&bigfive::BigFive>(entity).ok().map(|r| *r),
            aesthetic_taste: ecs
                .get::<&aesthetic::AestheticTaste>(entity)
                .ok()
                .map(|r| *r),
            cognitive_style: ecs
                .get::<&cognitive::CognitiveStyle>(entity)
                .ok()
                .map(|r| *r),
            emotion: ecs.get::<&emotion::Emotion>(entity).ok().map(|r| *r),
            needs: ecs.get::<&needs::Needs>(entity).ok().map(|r| *r),
            vitals: ecs.get::<&vitals::Vitals>(entity).ok().map(|r| *r),
            regen_state: ecs.get::<&vitals::RegenState>(entity).ok().map(|r| *r),
            death_cause: ecs.get::<&vitals::DeathCause>(entity).ok().map(|r| *r),
            corpse: ecs.get::<&vitals::Corpse>(entity).ok().map(|r| *r),
            decaying_remains: ecs.get::<&vitals::DecayingRemains>(entity).ok().map(|r| *r),
            age: ecs.get::<&lifecycle::Age>(entity).ok().map(|r| *r),
            life_stage: ecs.get::<&lifecycle::LifeStage>(entity).ok().map(|r| *r),
            gompertz: ecs
                .get::<&lifecycle::GompertzMortality>(entity)
                .ok()
                .map(|r| *r),
            wallet: ecs.get::<&economy::Wallet>(entity).ok().map(|r| *r),
            growth_needs: ecs.get::<&growth::GrowthNeeds>(entity).ok().map(|r| *r),
            movement: ecs.get::<&movement::Movement>(entity).ok().map(|r| *r),
            goal: ecs.get::<&goal::Goal>(entity).ok().map(|r| *r),
            item: ecs.get::<&item::Item>(entity).ok().map(|r| *r),
        };
        result.push(EntitySnapshot {
            old_id_bits,
            components: bag,
        });
    }
    result
}

/// 将实体快照恢复到 ECS World 中
///
/// 返回 old_id_bits → new hecs::Entity 的映射表。
/// 标记为 re-derive 的 Component 在此不添加——调用方在后续 system tick 中重建。
pub fn restore_entities(
    ecs: &mut hecs::World,
    snapshots: &[EntitySnapshot],
) -> std::collections::HashMap<u64, hecs::Entity> {
    let mut mapping = std::collections::HashMap::new();

    for snap in snapshots {
        let mut builder = hecs::EntityBuilder::new();
        let bag = &snap.components;

        if let Some(c) = bag.position {
            builder.add(c);
        }
        if let Some(c) = bag.rotation {
            builder.add(c);
        }
        if let Some(c) = bag.velocity {
            builder.add(c);
        }
        if let Some(c) = bag.entity_kind {
            builder.add(c);
        }
        if let Some(ref c) = bag.player {
            builder.add(c.clone());
        }
        if let Some(c) = bag.control_mode {
            builder.add(c);
        }
        if let Some(c) = bag.biological_sex {
            builder.add(c);
        }
        if let Some(c) = bag.culture {
            builder.add(c);
        }
        if let Some(c) = bag.faith {
            builder.add(c);
        }
        if let Some(c) = bag.bigfive {
            builder.add(c);
        }
        if let Some(c) = bag.aesthetic_taste {
            builder.add(c);
        }
        if let Some(c) = bag.cognitive_style {
            builder.add(c);
        }
        if let Some(c) = bag.emotion {
            builder.add(c);
        }
        if let Some(c) = bag.needs {
            builder.add(c);
        }
        if let Some(c) = bag.vitals {
            builder.add(c);
        }
        if let Some(c) = bag.regen_state {
            builder.add(c);
        }
        if let Some(c) = bag.death_cause {
            builder.add(c);
        }
        if let Some(c) = bag.corpse {
            builder.add(c);
        }
        if let Some(c) = bag.decaying_remains {
            builder.add(c);
        }
        if let Some(c) = bag.age {
            builder.add(c);
        }
        if let Some(c) = bag.life_stage {
            builder.add(c);
        }
        if let Some(c) = bag.gompertz {
            builder.add(c);
        }
        if let Some(c) = bag.wallet {
            builder.add(c);
        }
        if let Some(c) = bag.growth_needs {
            builder.add(c);
        }
        if let Some(c) = bag.movement {
            builder.add(c);
        }
        if let Some(c) = bag.goal {
            builder.add(c);
        }
        if let Some(c) = bag.item {
            builder.add(c);
        }

        // ZST tags 不保存——从 registry 反推
        //   HasInventory → InventoryRegistry 有该 entity 的条目
        //   HasEquipment → InventoryRegistry 有该 entity 的装备
        //   RelationHandle → RelationStorage 有该 entity 的关系

        let spawned = ecs.spawn(builder.build());
        mapping.insert(snap.old_id_bits, spawned);
    }

    mapping
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_component_bag_default_is_all_none() {
        let bag = ComponentBag::default();
        assert!(bag.position.is_none());
        assert!(bag.bigfive.is_none());
        assert!(bag.wallet.is_none());
    }

    #[test]
    fn test_component_bag_roundtrip_empty() {
        let bag = ComponentBag::default();
        let bytes = bincode::serialize(&bag).expect("serialize");
        let restored: ComponentBag = bincode::deserialize(&bytes).expect("deserialize");
        assert!(restored.position.is_none());
    }

    #[test]
    fn test_clock_data_roundtrip() {
        let clock = ClockData {
            accumulator: 900.0,
            seconds_per_day: 3600.0,
            days_per_year: 120,
            time_scale: 1.0,
        };
        let bytes = bincode::serialize(&clock).expect("serialize");
        let restored: ClockData = bincode::deserialize(&bytes).expect("deserialize");
        assert!((restored.accumulator - 900.0).abs() < 0.001);
        assert_eq!(restored.days_per_year, 120);
    }

    #[test]
    fn test_inventory_snapshot_roundtrip() {
        use woworld_core::item::inventory::PersonalInventory;
        let snap = InventorySnapshot {
            inventories: vec![
                (1, PersonalInventory::new(30)),
                (2, PersonalInventory::new(20)),
            ],
            equipment: vec![],
        };
        let bytes = bincode::serialize(&snap).expect("serialize");
        let _restored: InventorySnapshot = bincode::deserialize(&bytes).expect("deserialize");
    }
}
