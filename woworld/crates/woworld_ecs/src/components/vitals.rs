//! 生命相关 Component — ECS 铁律合规（纯数据·零方法·无堆·Send+Sync+'static）
//!
//! 参见: `开发文档/01-世界框架/02-生命系统.md`

use woworld_core::id::ItemDefId;
use woworld_core::types::EntityId;

// ── Vitals ────────────────────────────

/// 生命体征——"这个实体还活着"
#[derive(Debug, Clone, Copy)]
pub struct Vitals {
    pub hp: f32,
    pub max_hp: f32,
    pub stamina: f32,
    pub hunger: f32,   // 0=饱腹, 100=饿死
    pub thirst: f32,   // 0=不渴, 100=渴死
    pub body_temp: f32, // 摄氏度, 37.0 正常
    pub oxygen: f32,   // 0=窒息, 100=正常
}

impl Default for Vitals {
    fn default() -> Self {
        Self {
            hp: 100.0,
            max_hp: 100.0,
            stamina: 100.0,
            hunger: 0.0,
            thirst: 0.0,
            body_temp: 37.0,
            oxygen: 100.0,
        }
    }
}

// ── DeathCause ────────────────────────

/// 死亡原因分类——6 大类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeathCategory {
    /// 暴力致死（武器/徒手/爆炸/坠落等）
    Violent = 0,
    /// 疾病致死（感染/瘟疫/中毒/寄生虫等）
    Disease = 1,
    /// 衰老致死（自然寿命耗尽）
    Senescence = 2,
    /// 环境致死（冻死/热死/溺水/窒息/雷击等）
    Environmental = 3,
    /// 精神耗竭致死（灵能耗尽/疯狂/灵魂破碎等）
    SpiritExhaustion = 4,
    /// 意志性死亡（自杀/主动放弃生命/献祭等）
    Volition = 5,
}

/// 死亡原因记录——仅在 Entity 死亡时装上，作为永久死亡记录保留
#[derive(Debug, Clone, Copy)]
pub struct DeathCause {
    pub category: DeathCategory,
    /// 30 种细分死因编码（见 `开发阶段/生命/004-身体状态与生命过程` §九）
    /// 当前 Phase 1 占位为 0
    pub specific: u8,
    /// 谁/什么造成的死亡（None = 自然/环境原因）
    pub source_entity: Option<EntityId>,
}

impl Default for DeathCause {
    fn default() -> Self {
        Self {
            category: DeathCategory::Senescence,
            specific: 0,
            source_entity: None,
        }
    }
}

// ── Corpse ────────────────────────────

/// 尸体状态——死亡时间 + 尸温，涌现数据源
///
/// `corpse_temperature` 从 37°C 趋近环境温度——感官 System 可据此推断死亡时间。
/// "尸体还温热"→凶手在附近。"白骨冰冷"→已死数百年。
#[derive(Debug, Clone, Copy)]
pub struct Corpse {
    /// 死亡时的游戏 tick（WorldClock 帧计数）
    pub death_tick: u64,
    /// 尸温（摄氏度），37.0 为刚死，趋近环境温度
    pub corpse_temperature: f32,
}

impl Default for Corpse {
    fn default() -> Self {
        Self {
            death_tick: 0,
            corpse_temperature: 37.0,
        }
    }
}

// ── PendingLoot ───────────────────────

/// 标记 Entity 等待掉落判定——零字段 tag Component。
///
/// DeathWatch 只标记"这个尸体需要掉落"，不指定掉落表。
/// LootRoll 自己通过 EntityKind（或未来 SpeciesId）决定查哪个表。
#[derive(Debug, Clone, Copy)]
pub struct PendingLoot;

impl Default for PendingLoot {
    fn default() -> Self {
        Self
    }
}

// ── LootResult ────────────────────────

/// 已确定的掉落物列表——固定大小数组，铁律 2 合规
#[derive(Debug, Clone, Copy)]
pub struct LootResult {
    /// 最多 8 个掉落物品
    pub items: [Option<ItemDefId>; 8],
    /// 实际物品数量（≤8）
    pub count: u8,
}

#[allow(clippy::derivable_impls)]
impl Default for LootResult {
    fn default() -> Self {
        Self {
            items: [None; 8],
            count: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vitals_default_is_alive() {
        let v = Vitals::default();
        assert!(v.hp > 0.0);
        assert_eq!(v.body_temp, 37.0);
        assert_eq!(v.oxygen, 100.0);
    }

    #[test]
    fn test_loot_result_fixed_array() {
        let lr = LootResult::default();
        assert_eq!(lr.count, 0);
        assert!(lr.items.iter().all(|i| i.is_none()));
    }

    #[test]
    fn test_corpse_default_temperature() {
        let c = Corpse::default();
        assert_eq!(c.corpse_temperature, 37.0);
    }

    #[test]
    fn test_pending_loot_is_zst() {
        // PendingLoot 是零字段 tag——验证它是 ZST
        assert_eq!(std::mem::size_of::<PendingLoot>(), 0);
    }
}
