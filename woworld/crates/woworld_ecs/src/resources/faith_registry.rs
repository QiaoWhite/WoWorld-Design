//! FaithRegistry — 信仰数据 SoA 存储 + FaithQuery 实现
//!
//! 参见: woworld_core::faith::FaithQuery

use woworld_core::faith::{
    derive_faith_label, FaithId, FaithLabel, FaithQuery, FaithTheology, ReligiousPracticeProfile,
    FAITH_ID_NONE,
};
use woworld_core::types::EntityId;

/// 信仰注册表 — SoA 列存储 + FaithQuery 实现
#[derive(Debug, Default)]
pub struct FaithRegistry {
    theologies: Vec<FaithTheology>,
    labels: Vec<FaithLabel>,
    /// NPC 实践档案: EntityId(u64) → profile
    profiles: Vec<(EntityId, ReligiousPracticeProfile)>,
    faith_ids: Vec<FaithId>,
    next_id: u32,
}

impl FaithRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册新信仰——计算派生标签并返回 FaithId
    pub fn register(&mut self, theology: FaithTheology) -> FaithId {
        if self.next_id == u32::MAX {
            panic!("FaithRegistry: FaithId overflow");
        }
        let id = FaithId(self.next_id);
        self.next_id += 1;
        let label = derive_faith_label(&theology);
        self.theologies.push(theology);
        self.labels.push(label);
        self.faith_ids.push(id);
        id
    }

    /// 注册 NPC 实践档案
    pub fn set_profile(&mut self, entity_id: EntityId, profile: ReligiousPracticeProfile) {
        if let Some(pos) = self.profiles.iter().position(|(e, _)| *e == entity_id) {
            self.profiles[pos] = (entity_id, profile);
        } else {
            self.profiles.push((entity_id, profile));
        }
    }

    /// 查询 NPC 实践档案
    pub fn get_profile(&self, entity_id: EntityId) -> Option<&ReligiousPracticeProfile> {
        self.profiles
            .iter()
            .find(|(e, _)| *e == entity_id)
            .map(|(_, p)| p)
    }

    /// 获取可变神学参数引用（用于 Phase 2 漂移）
    pub fn get_theology_mut(&mut self, id: FaithId) -> Option<&mut FaithTheology> {
        let idx = id.0 as usize;
        if idx < self.theologies.len() {
            Some(&mut self.theologies[idx])
        } else {
            None
        }
    }

    /// 重算派生标签（神学修改后调用）
    pub fn recalculate_label(&mut self, id: FaithId) {
        let idx = id.0 as usize;
        if idx < self.theologies.len() {
            self.labels[idx] = derive_faith_label(&self.theologies[idx]);
        }
    }

    pub fn len(&self) -> usize {
        self.faith_ids.len()
    }
    pub fn is_empty(&self) -> bool {
        self.faith_ids.is_empty()
    }
    pub fn profile_count(&self) -> usize {
        self.profiles.len()
    }
}

// ── FaithQuery impl ────────────────────────────────────

impl FaithQuery for FaithRegistry {
    fn theology(&self, id: FaithId) -> Option<&FaithTheology> {
        if id == FAITH_ID_NONE {
            return None;
        }
        let idx = id.0 as usize;
        if idx < self.theologies.len() {
            Some(&self.theologies[idx])
        } else {
            None
        }
    }

    fn faith_label(&self, id: FaithId) -> Option<FaithLabel> {
        if id == FAITH_ID_NONE {
            return None;
        }
        let idx = id.0 as usize;
        if idx < self.labels.len() {
            Some(self.labels[idx])
        } else {
            None
        }
    }

    fn proselytizing(&self, id: FaithId) -> f32 {
        self.theology(id).map(|t| t.proselytizing()).unwrap_or(0.0)
    }

    fn tolerance_between(&self, a: FaithId, b: FaithId) -> f32 {
        match (self.theology(a), self.theology(b)) {
            (Some(ta), Some(tb)) => woworld_core::faith::tolerance(ta, tb),
            _ => 0.5,
        }
    }

    fn hostility_between(&self, a: FaithId, b: FaithId) -> f32 {
        match (self.theology(a), self.theology(b)) {
            (Some(ta), Some(tb)) => woworld_core::faith::hostility(ta, tb),
            _ => 0.0,
        }
    }

    fn all_faiths(&self) -> &[FaithId] {
        &self.faith_ids
    }

    fn faith_count(&self) -> usize {
        self.faith_ids.len()
    }
}

// ── 测试 ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use woworld_core::faith::{MonotheismFlavor, ReligiousMotivation};

    #[test]
    fn test_new_empty() {
        let r = FaithRegistry::new();
        assert_eq!(r.len(), 0);
        assert_eq!(r.faith_count(), 0);
    }

    #[test]
    fn test_register_sequential() {
        let mut r = FaithRegistry::new();
        let a = r.register(FaithTheology::from_seed(0, 0.8));
        let b = r.register(FaithTheology::from_seed(1, 0.6));
        assert_eq!(a, FaithId(0));
        assert_eq!(b, FaithId(1));
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn test_query_theology() {
        let mut r = FaithRegistry::new();
        let t = FaithTheology::from_seed(42, 0.9);
        let id = r.register(t);
        let got = r.theology(id).unwrap();
        assert!((got.deity_count - t.deity_count).abs() < 0.001);
    }

    #[test]
    fn test_query_label() {
        let mut r = FaithRegistry::new();
        let t = FaithTheology {
            deity_count: 0.02,
            ancestor_importance: 0.1,
            nature_sacredness: 0.05,
            ..FaithTheology::default()
        };
        let id = r.register(t);
        assert_eq!(r.faith_label(id), Some(FaithLabel::NonTheistic));
    }

    #[test]
    fn test_proselytizing_delegates() {
        let mut r = FaithRegistry::new();
        let t = FaithTheology {
            exclusivity: 0.9,
            orthodoxy_vs_orthopraxy: 0.5,
            faith_as_identity: 0.5,
            ..FaithTheology::default()
        };
        let id = r.register(t);
        assert!(r.proselytizing(id) > 0.4);
    }

    #[test]
    fn test_tolerance_hostility() {
        let mut r = FaithRegistry::new();
        let a = r.register(FaithTheology {
            exclusivity: 0.1,
            ..FaithTheology::from_seed(10, 0.7)
        });
        let b = r.register(FaithTheology {
            exclusivity: 0.9,
            ..FaithTheology::from_seed(20, 0.7)
        });
        assert!(r.tolerance_between(a, b) > r.tolerance_between(b, a));
    }

    #[test]
    fn test_none_id() {
        let mut r = FaithRegistry::new();
        r.register(FaithTheology::from_seed(0, 0.5));
        assert!(r.theology(FAITH_ID_NONE).is_none());
        assert!(r.faith_label(FAITH_ID_NONE).is_none());
    }

    #[test]
    fn test_set_get_profile() {
        let mut r = FaithRegistry::new();
        let profile = ReligiousPracticeProfile::new(ReligiousMotivation::Habitual);
        r.set_profile(EntityId(42), profile.clone());
        let got = r.get_profile(EntityId(42)).unwrap();
        assert_eq!(got.motivation, ReligiousMotivation::Habitual);
        assert_eq!(r.profile_count(), 1);
    }

    #[test]
    fn test_profile_update() {
        let mut r = FaithRegistry::new();
        r.set_profile(
            EntityId(1),
            ReligiousPracticeProfile::new(ReligiousMotivation::SocialCustom),
        );
        r.set_profile(
            EntityId(1),
            ReligiousPracticeProfile::new(ReligiousMotivation::PersonalDevotion),
        );
        assert_eq!(r.profile_count(), 1);
        assert_eq!(
            r.get_profile(EntityId(1)).unwrap().motivation,
            ReligiousMotivation::PersonalDevotion
        );
    }

    #[test]
    fn test_recalculate_label() {
        let mut r = FaithRegistry::new();
        let id = r.register(FaithTheology::default());
        assert_eq!(
            r.faith_label(id),
            Some(FaithLabel::Monotheism(MonotheismFlavor::Standard))
        );
        // 修改神学 → NonTheistic
        if let Some(t) = r.get_theology_mut(id) {
            t.deity_count = 0.0;
            t.ancestor_importance = 0.1;
            t.nature_sacredness = 0.05;
        }
        r.recalculate_label(id);
        assert_eq!(r.faith_label(id), Some(FaithLabel::NonTheistic));
    }
}
