//! RelationStorage — 全局 NPC 关系 BTreeMap
//!
//! 参见: `开发阶段/NPC活人感模块/NPC活人感开发文档ver2.0.md` §2.4
//! 参见: `开发文档/02-NPC核心/01-NPC总纲.md` §Handle 类型
//!
//! 使用 (min, max) 有序对作为 key，保证 (A,B) 和 (B,A) 共享同一份关系数据。
//! 存储在 WorldDriver 中，作为参数传入 social_system。

use std::collections::BTreeMap;
use woworld_core::types::EntityId;

// ── RelationshipSource ─────────────────

/// 关系来源——影响关系变化速率
///
/// 文档: `NPC活人感开发文档ver2.0.md` §2.4 — RelationshipSource
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RelationshipSource {
    /// 血缘 → 信任度高，变化慢
    Bloodline,
    /// 婚姻 → 信任度高，背叛冲击力 ×2
    Marriage,
    /// 非婚姻伴侣关系
    Partnership,
    /// 同事 → 熟悉度高，情感浅
    Coworker,
    /// 朋友 → 双向
    Friendship,
    /// 泛泛之交
    #[default]
    Acquaintance,
    /// 共同经历（一起航海/战斗/施工）→ 信任加速
    SharedTrauma,
}

// ── StatusRelation ─────────────────────

/// 关系中的支配/服从地位
///
/// 文档: `NPC活人感开发文档ver2.0.md` §2.4 — StatusRelation
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum StatusRelation {
    /// 支配方——在关系中占主导
    Dominant {
        /// 权力差幅 (0-1)，越大越不对等
        power_differential: f32,
    },
    /// 服从方——接受对方主导
    Submissive {
        power_differential: f32,
    },
    /// 平等——双向自主
    #[default]
    Equal,
}

// ── Relationship ───────────────────────

/// NPC 间关系——完整字段对齐文档 §2.4
///
/// affection: -1=憎恨 → 0=中性 → 1=深爱（短期波动，天级）
/// trust: -1=深度不信任 → 0=中性 → 1=完全信任（长期积累，年级）
/// familiarity: 0=陌生人 → 1=非常熟悉（基于互动次数和质量）
#[derive(Debug, Clone)]
pub struct Relationship {
    /// 好感度 -1~1（短期波动，天级）
    pub affection: f32,
    /// 信任度 -1~1（长期积累，年级）
    pub trust: f32,
    /// 熟悉度 0~1
    pub familiarity: f32,
    /// 吸引力 0~1（来自性别与吸引力系统 §4.2）— Phase 2 接入
    pub attraction: f32,
    /// 支配/服从/平等
    pub status: StatusRelation,
    /// 关系来源
    pub source: RelationshipSource,
    /// 上次互动 tick
    pub last_interaction_tick: u64,
    /// 总互动次数
    pub total_interactions: u32,
}

impl Default for Relationship {
    fn default() -> Self {
        Self {
            affection: 0.0,
            trust: 0.0,
            familiarity: 0.0,
            attraction: 0.0,
            status: StatusRelation::default(),
            source: RelationshipSource::default(),
            last_interaction_tick: 0,
            total_interactions: 0,
        }
    }
}

impl Relationship {
    /// 关系衰减——好感度随时间微衰减（文档公式）
    ///
    /// `days`: 自上次互动以来的游戏天数
    pub fn decay_affection(&mut self, days: f32) {
        self.affection *= (-0.01 * days).exp();
    }

    /// 信任时间增长——无负面事件时，认识得久=更可信（文档公式）
    ///
    /// `days`: 自上次互动以来的游戏天数
    pub fn grow_trust_over_time(&mut self, days: f32) {
        if self.affection > 0.0 && days > 30.0 {
            self.trust = (self.trust + 0.001 * days.sqrt()).min(1.0);
        }
    }
}

// ── RelationStorage ────────────────────

/// 全局关系存储——BTreeMap 按有序实体对索引
///
/// Key 使用 `(min(a,b), max(a,b))` 保证 (A,B) 和 (B,A) 共享同一条关系。
/// 存储在 WorldDriver 中，作为 `&mut RelationStorage` 参数传入 social_system。
#[derive(Debug, Default)]
pub struct RelationStorage {
    pub relations: BTreeMap<(EntityId, EntityId), Relationship>,
    /// 上次运行关系维护的 tick——避免每帧全表扫描
    pub last_maintenance_tick: u64,
}

impl RelationStorage {
    /// 获取两个实体间的关系（任意顺序）
    pub fn get(&self, a: EntityId, b: EntityId) -> Option<&Relationship> {
        let key = ordered_pair(a, b);
        self.relations.get(&key)
    }

    /// 获取可变引用
    pub fn get_mut(&mut self, a: EntityId, b: EntityId) -> Option<&mut Relationship> {
        let key = ordered_pair(a, b);
        self.relations.get_mut(&key)
    }

    /// 获取或创建关系——首次互动时插入默认值
    pub fn get_or_create(&mut self, a: EntityId, b: EntityId) -> &mut Relationship {
        let key = ordered_pair(a, b);
        self.relations.entry(key).or_default()
    }

    /// 更新某对关系的互动时间
    pub fn touch(&mut self, a: EntityId, b: EntityId, tick: u64) {
        let rel = self.get_or_create(a, b);
        rel.last_interaction_tick = tick;
    }
}

/// 将实体对排序为规范形式 (min, max)
#[inline]
fn ordered_pair(a: EntityId, b: EntityId) -> (EntityId, EntityId) {
    if a.0 <= b.0 {
        (a, b)
    } else {
        (b, a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ordered_pair_symmetric() {
        let a = EntityId(10);
        let b = EntityId(5);
        assert_eq!(ordered_pair(a, b), (EntityId(5), EntityId(10)));
        assert_eq!(ordered_pair(b, a), (EntityId(5), EntityId(10)));
    }

    #[test]
    fn test_get_or_create() {
        let mut storage = RelationStorage::default();
        let a = EntityId(1);
        let b = EntityId(2);

        // 首次——创建
        let rel = storage.get_or_create(a, b);
        assert_eq!(rel.total_interactions, 0);

        // 同对逆序——命中同一记录
        rel.total_interactions = 5;
        let rel2 = storage.get_or_create(b, a);
        assert_eq!(rel2.total_interactions, 5);
    }

    #[test]
    fn test_touch_updates_tick() {
        let mut storage = RelationStorage::default();
        let a = EntityId(1);
        let b = EntityId(2);

        storage.touch(a, b, 100);
        let rel = storage.get(a, b).unwrap();
        assert_eq!(rel.last_interaction_tick, 100);
    }

    #[test]
    fn test_decay_affection() {
        let mut rel = Relationship::default();
        rel.affection = 0.5;
        rel.decay_affection(30.0);
        assert!(rel.affection < 0.5, "affection should decay");
    }

    #[test]
    fn test_grow_trust_over_time() {
        let mut rel = Relationship::default();
        rel.affection = 0.3;
        rel.grow_trust_over_time(60.0);
        assert!(rel.trust > 0.0, "trust should grow with time and positive affection");
    }

    #[test]
    fn test_no_trust_growth_without_affection() {
        let mut rel = Relationship::default();
        rel.affection = -0.1;
        rel.grow_trust_over_time(60.0);
        assert_eq!(rel.trust, 0.0, "no trust growth without positive affection");
    }

    #[test]
    fn test_no_trust_growth_before_30_days() {
        let mut rel = Relationship::default();
        rel.affection = 0.5;
        rel.grow_trust_over_time(20.0);
        assert_eq!(rel.trust, 0.0, "no trust growth before 30 days");
    }

    #[test]
    fn test_relationship_default() {
        let rel = Relationship::default();
        assert_eq!(rel.affection, 0.0);
        assert_eq!(rel.trust, 0.0);
        assert_eq!(rel.total_interactions, 0);
    }
}
