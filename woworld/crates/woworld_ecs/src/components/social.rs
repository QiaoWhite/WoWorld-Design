//! 社交 Component — ECS 铁律合规（纯数据·零方法·无堆）
//!
//! 参见: `开发文档/02-NPC核心/01-NPC总纲.md` §社会层
//!
//! SocialPresence 定义 NPC 的社交影响范围与恢复强度。
//! Relationships 存储 NPC 对其他实体的关系记忆（affection/familiarity/trust）。

use crate::components::bigfive::BigFive;
use woworld_core::types::EntityId;

/// 社交存在——决定 NPC 对他人的社交影响力
///
/// 外向者半径大、恢复快；内向者半径小、恢复慢。
#[derive(Debug, Clone, Copy)]
pub struct SocialPresence {
    /// 社交半径 (m)——在此范围内的其他 NPC 可恢复社交需求
    pub radius: f32,
    /// 社交恢复速率 (/s)——每秒降低对方 social 需求的基础速率
    pub recovery_rate: f32,
}

impl Default for SocialPresence {
    fn default() -> Self {
        Self {
            radius: 4.5,
            recovery_rate: 0.006,
        }
    }
}

impl SocialPresence {
    /// 从 BigFive 外向性派生社交存在
    ///
    /// - radius: 2.0 + E × 5.0 → [2m, 7m]
    /// - recovery_rate: 0.002 + E × 0.008 → [0.002/s, 0.01/s]
    pub fn derive_from_bigfive(b: &BigFive) -> Self {
        Self {
            radius: 2.0 + b.extraversion * 5.0,
            recovery_rate: 0.002 + b.extraversion * 0.008,
        }
    }
}

// ── StatusRelation ────────────────────

/// 关系中的支配/服从地位
///
/// 参见: `开发阶段/NPC活人感模块/NPC活人感开发文档ver2.0.md` §2.4
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StatusRelation {
    /// 支配方——在关系中占主导
    Dominant,
    /// 服从方——接受对方主导
    Submissive,
    /// 平等——双向自主
    #[default]
    Equal,
}

// ── Relationships ────────────────────

/// 最大记忆的关系数——更远的熟人被逐出
pub const MAX_RELATIONSHIPS: usize = 16;

/// 单个 NPC 间关系条目——纯数据
///
/// 参见: `开发阶段/NPC活人感模块/NPC活人感开发文档ver2.0.md` §2.4
///
/// affection: -1=憎恨 → 0=中性 → 1=深爱（文档原名 affection，短期波动·天级）
/// trust: -1=深度不信任 → 0=中性 → 1=完全信任（文档范围 -1~1，长期积累·年级）
/// familiarity: 0=陌生人 → 1=非常熟悉（高频互动积累）
#[derive(Debug, Clone, Copy)]
pub struct RelationshipEntry {
    pub entity: EntityId,
    /// 好感度 -1~1（文档原名 affection）
    pub affection: f32,
    /// 熟悉度 0~1
    pub familiarity: f32,
    /// 信任度 -1~1（可为负——背叛后的不信任）
    pub trust: f32,
    /// 支配/服从/平等
    pub status: StatusRelation,
    /// 总互动次数
    pub total_interactions: u32,
    /// 上次互动的游戏 tick
    pub last_interaction_tick: u64,
}

/// 社会关系记忆——固定 16 槽，ECS 铁律合规（纯值类型）
///
/// ⚠️ 容限暂定 16——文档原始设计为 BTreeMap 无上限。
/// 16 在村庄规模 (≤100 NPC) 下够用，城镇规模需升级。
///
/// 逐出策略: 满时淘汰 familiarity 最低的条目
#[derive(Debug, Clone, Copy)]
pub struct Relationships {
    pub entries: [Option<RelationshipEntry>; MAX_RELATIONSHIPS],
    pub count: u8,
}

impl Default for Relationships {
    fn default() -> Self {
        Self {
            entries: [None; MAX_RELATIONSHIPS],
            count: 0,
        }
    }
}

/// 关系衰减——好感度随时间微衰减（文档 §2.4 公式）
///
/// `days`: 自上次互动以来的游戏天数
///
/// 文档公式: `affection *= exp(-0.01 × days)`
pub fn affection_decay_over_time(affection: f32, days: f32) -> f32 {
    affection * (-0.01 * days).exp()
}

impl Relationships {
    /// 查找已知实体——O(n)，n≤16
    pub fn find(&self, entity: EntityId) -> Option<&RelationshipEntry> {
        self.entries.iter().flatten().find(|e| e.entity == entity)
    }

    /// 查找已知实体（可变引用）
    pub fn find_mut(&mut self, entity: EntityId) -> Option<&mut RelationshipEntry> {
        self.entries.iter_mut().flatten().find(|e| e.entity == entity)
    }

    /// 添加或更新关系。满时淘汰最不熟悉的条目。
    pub fn upsert(&mut self, entity: EntityId, current_tick: u64) -> &mut RelationshipEntry {
        // 已存在 → 直接更新并返回索引
        let existing_idx = self.entries.iter().position(|opt| {
            opt.as_ref().is_some_and(|e| e.entity == entity)
        });
        if let Some(idx) = existing_idx {
            let entry = self.entries[idx].as_mut().unwrap();
            entry.last_interaction_tick = current_tick;
            // SAFETY: we return a mutable reference into self.entries. The caller
            // must drop the reference before calling upsert again, which is the
            // natural usage pattern.
            return entry;
        }

        // 有空槽 → 直接插入
        if (self.count as usize) < MAX_RELATIONSHIPS {
            let empty_idx = self.entries.iter().position(|opt| opt.is_none()).unwrap();
            self.entries[empty_idx] = Some(RelationshipEntry {
                entity,
                affection: 0.0,
                familiarity: 0.0,
                trust: 0.0,
                status: StatusRelation::default(),
                total_interactions: 0,
                last_interaction_tick: current_tick,
            });
            self.count += 1;
            return self.entries[empty_idx].as_mut().unwrap();
        }

        // 满 → 淘汰 familiarity 最低的
        let evict_idx = {
            let mut min_fam = f32::MAX;
            let mut min_idx = 0;
            for (i, opt) in self.entries.iter().enumerate() {
                if let Some(ref e) = opt {
                    if e.familiarity < min_fam {
                        min_fam = e.familiarity;
                        min_idx = i;
                    }
                }
            }
            min_idx
        };

        self.entries[evict_idx] = Some(RelationshipEntry {
            entity,
            affection: 0.0,
            familiarity: 0.0,
            trust: 0.0,
            status: StatusRelation::default(),
            total_interactions: 0,
            last_interaction_tick: current_tick,
        });
        self.entries[evict_idx].as_mut().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_bigfive_extravert() {
        let b = BigFive { extraversion: 1.0, ..BigFive::default() };
        let sp = SocialPresence::derive_from_bigfive(&b);
        assert!((sp.radius - 7.0).abs() < 0.01);
        assert!((sp.recovery_rate - 0.01).abs() < 0.001);
    }

    #[test]
    fn test_from_bigfive_introvert() {
        let b = BigFive { extraversion: 0.0, ..BigFive::default() };
        let sp = SocialPresence::derive_from_bigfive(&b);
        assert!((sp.radius - 2.0).abs() < 0.01);
        assert!((sp.recovery_rate - 0.002).abs() < 0.001);
    }

    #[test]
    fn test_default_in_range() {
        let sp = SocialPresence::default();
        assert!(sp.radius > 0.0);
        assert!(sp.recovery_rate > 0.0);
    }

    // ── Relationships tests ──

    #[test]
    fn test_relationships_default_empty() {
        let r = Relationships::default();
        assert_eq!(r.count, 0);
        assert!(r.entries.iter().all(|e| e.is_none()));
    }

    #[test]
    fn test_relationships_upsert_new() {
        let mut r = Relationships::default();
        let e1 = EntityId(1);

        let entry = r.upsert(e1, 100);
        assert_eq!(entry.entity, e1);
        assert_eq!(entry.familiarity, 0.0);
        assert_eq!(r.count, 1);
    }

    #[test]
    fn test_relationships_upsert_existing() {
        let mut r = Relationships::default();
        let e1 = EntityId(1);

        // First interaction
        let entry = r.upsert(e1, 100);
        entry.familiarity = 0.5;
        entry.affection = 0.3;

        // Second interaction — same entity
        let entry2 = r.upsert(e1, 200);
        assert_eq!(entry2.familiarity, 0.5, "should keep accumulated familiarity");
        assert_eq!(entry2.affection, 0.3, "should keep accumulated affection");
        assert_eq!(entry2.last_interaction_tick, 200, "should update tick");
        assert_eq!(r.count, 1, "should not increase count");
    }

    #[test]
    fn test_relationships_find() {
        let mut r = Relationships::default();
        let e1 = EntityId(1);
        let e2 = EntityId(2);

        r.upsert(e1, 100);

        assert!(r.find(e1).is_some());
        assert!(r.find(e2).is_none());
    }

    #[test]
    fn test_relationships_evict_least_familiar() {
        let mut r = Relationships::default();

        // Fill all 16 slots
        for i in 0..MAX_RELATIONSHIPS {
            let e = EntityId(i as u64);
            let entry = r.upsert(e, 100);
            entry.familiarity = i as f32 * 0.05; // 0.0, 0.05, ..., 0.75
        }
        assert_eq!(r.count as usize, MAX_RELATIONSHIPS);

        // Slot 0 has familiarity 0.0 — it should be evicted
        let new_e = EntityId(999);
        r.upsert(new_e, 200);

        assert!(r.find(EntityId(0)).is_none(), "least familiar should be evicted");
        assert!(r.find(new_e).is_some(), "new entity should be present");
        assert_eq!(r.count as usize, MAX_RELATIONSHIPS);
    }

    #[test]
    fn test_affection_decay_over_time() {
        let a = affection_decay_over_time(0.5, 30.0); // 30 天无互动
        assert!(a < 0.5, "affection should decay");
        assert!(a > 0.3, "30 days shouldn't erase all affection");
    }

    #[test]
    fn test_affection_decay_zero_days() {
        let a = affection_decay_over_time(0.5, 0.0);
        assert!((a - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_affection_decay_long_time() {
        let a = affection_decay_over_time(0.5, 365.0); // 一年无互动
        assert!(a < 0.05, "one year without contact should nearly erase affection");
    }

    #[test]
    fn test_relationships_find_mut() {
        let mut r = Relationships::default();
        let e1 = EntityId(1);
        r.upsert(e1, 100);

        if let Some(entry) = r.find_mut(e1) {
            entry.familiarity += 0.1;
        }
        assert!((r.find(e1).unwrap().familiarity - 0.1).abs() < f32::EPSILON);
    }
}
