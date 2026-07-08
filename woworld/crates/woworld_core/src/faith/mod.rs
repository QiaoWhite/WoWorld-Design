//! 信仰系统 — 核心类型与只读查询 trait
//!
//! "实践优先模型"——NPC 没有"神学立场"字段，只有参与实践的行为记录。
//! "虔诚者/民间多神论者/非信徒"等标签从实践档案派生。
//!
//! Faith ≠ God 系统: 神的存在模式 (A/B/C) 属于 Life 模块。
//! Faith ≠ Culture: Culture.religiosity 决定宗教渗透度，Faith 决定拜什么。
//!
//! Phase 1: 核心类型 + FaithQuery trait + 派生函数。
//! 延后: 传播/分裂/节日/权力桥接/魔法关系/葬礼。
//!
//! 参见: `WoWorld-Design/Happy Game/开发阶段/信仰系统/`
//! 参见: [[CLAUDE-INTERFACES.md]] CHG-025

// ── FaithId ────────────────────────────────────────────

/// 信仰全局标识符 — 扁平 u32，不编码谱系信息
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FaithId(pub u32);

pub const FAITH_ID_NONE: FaithId = FaithId(u32::MAX);

// ── FaithTheology — 10 连续神学参数 ────────────────────

/// 信仰神学参数 — 10 个连续 f32，所有离散标签均从此派生
///
/// "一神教/多神教/萨满教"等分类仅 UI 显示，不参与模拟逻辑。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FaithTheology {
    /// 神灵数量 [0, 15] — 0=无神, 1=一神, 2-5=寡神, 6+=多神
    pub deity_count: f32,
    /// 祖先崇拜重要性 [0, 1]
    pub ancestor_importance: f32,
    /// 自然神圣性 [0, 1]
    pub nature_sacredness: f32,
    /// 等级制度程度 [0, 1] — 0=平信徒自主, 1=严格神职等级
    pub hierarchy_degree: f32,
    /// 经典中心性 [0, 1] — 0=口传传统, 1=文字经典至上
    pub scripture_centrality: f32,
    /// 仪式正式性 [0, 1] — 0=自发祈祷, 1=严格礼仪
    pub ritual_formality: f32,
    /// 排他性 [0, 1] — 0=包容多信仰, 1=唯我独尊
    pub exclusivity: f32,
    /// 神秘主义 [0, 1] — 0=理性神学, 1=神秘体验至上
    pub mysticism: f32,
    /// 正统 vs 正行 [0, 1] — 0=正确行为重于正确信念, 1=反之
    pub orthodoxy_vs_orthopraxy: f32,
    /// 信仰即身份 [0, 1] — 0=信仰是私事, 1=信仰定义族群
    pub faith_as_identity: f32,
}

impl Default for FaithTheology {
    fn default() -> Self {
        Self {
            deity_count: 1.0,
            ancestor_importance: 0.5,
            nature_sacredness: 0.5,
            hierarchy_degree: 0.5,
            scripture_centrality: 0.5,
            ritual_formality: 0.5,
            exclusivity: 0.5,
            mysticism: 0.5,
            orthodoxy_vs_orthopraxy: 0.5,
            faith_as_identity: 0.5,
        }
    }
}

impl FaithTheology {
    pub const DIM_COUNT: usize = 10;

    pub fn dim(&self, idx: usize) -> f32 {
        match idx {
            0 => self.deity_count,
            1 => self.ancestor_importance,
            2 => self.nature_sacredness,
            3 => self.hierarchy_degree,
            4 => self.scripture_centrality,
            5 => self.ritual_formality,
            6 => self.exclusivity,
            7 => self.mysticism,
            8 => self.orthodoxy_vs_orthopraxy,
            9 => self.faith_as_identity,
            _ => panic!("FaithTheology dim out of range: {idx}"),
        }
    }

    /// 从种子 + 文化 religiosity 确定性生成
    ///
    /// religiosity < 0.1 → 无信仰（所有参数接近 0）
    /// religiosity 越高 → 参数越"极端"
    pub fn from_seed(seed: u64, religiosity: f32) -> Self {
        if religiosity < 0.1 {
            return Self::secular();
        }
        let intensity = religiosity.clamp(0.1, 1.0);
        Self {
            deity_count: lerp_hash(seed, 0, 0.0, 15.0, intensity),
            ancestor_importance: lerp_hash(seed, 1, 0.0, 1.0, intensity),
            nature_sacredness: lerp_hash(seed, 2, 0.0, 1.0, intensity),
            hierarchy_degree: lerp_hash(seed, 3, 0.0, 1.0, intensity),
            scripture_centrality: lerp_hash(seed, 4, 0.0, 1.0, intensity),
            ritual_formality: lerp_hash(seed, 5, 0.0, 1.0, intensity),
            exclusivity: lerp_hash(seed, 6, 0.0, 1.0, intensity),
            mysticism: lerp_hash(seed, 7, 0.0, 1.0, intensity),
            orthodoxy_vs_orthopraxy: lerp_hash(seed, 8, 0.0, 1.0, intensity),
            faith_as_identity: lerp_hash(seed, 9, 0.0, 1.0, intensity),
        }
    }

    /// 世俗/无神社会——所有参数归零
    fn secular() -> Self {
        Self {
            deity_count: 0.0,
            ancestor_importance: 0.0,
            nature_sacredness: 0.0,
            hierarchy_degree: 0.0,
            scripture_centrality: 0.0,
            ritual_formality: 0.0,
            exclusivity: 0.0,
            mysticism: 0.0,
            orthodoxy_vs_orthopraxy: 0.0,
            faith_as_identity: 0.0,
        }
    }

    /// 传教驱动力: exclusivity×0.5 + orthodoxy×0.3 + faith_as_identity×0.2
    pub fn proselytizing(&self) -> f32 {
        (self.exclusivity * 0.5 + self.orthodoxy_vs_orthopraxy * 0.3 + self.faith_as_identity * 0.2)
            .clamp(0.0, 1.0)
    }
}

// ── FaithLabel — 派生标签（仅 UI） ─────────────────────

/// 信仰分类标签——纯 UI 显示，不参与模拟逻辑
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaithLabel {
    NonTheistic,
    Monotheism(MonotheismFlavor),
    Polytheism(PolytheismFlavor),
    Shamanism,
    Animism,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonotheismFlavor {
    Exclusive,
    Mystical,
    Standard,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolytheismFlavor {
    Pantheon,
    NatureForces,
    Functional,
}

/// 从 FaithTheology 派生标签（对齐设计文档 002 §2.2）
pub fn derive_faith_label(t: &FaithTheology) -> FaithLabel {
    let dc = t.deity_count;
    // NonTheistic: 严格阈值
    if dc < 0.05 && t.ancestor_importance < 0.3 && t.nature_sacredness < 0.1 {
        return FaithLabel::NonTheistic;
    }
    // Monotheism: dc ∈ [0.8, 1.2]
    if (0.8..=1.2).contains(&dc) {
        let flavor = if t.exclusivity > 0.7 {
            MonotheismFlavor::Exclusive
        } else if t.mysticism > 0.6 {
            MonotheismFlavor::Mystical
        } else {
            MonotheismFlavor::Standard
        };
        return FaithLabel::Monotheism(flavor);
    }
    // Polytheism: dc > 1.2
    if dc > 1.2 {
        let flavor = if t.nature_sacredness > 0.6 {
            PolytheismFlavor::NatureForces
        } else if t.hierarchy_degree > 0.5 {
            PolytheismFlavor::Pantheon
        } else {
            PolytheismFlavor::Functional
        };
        return FaithLabel::Polytheism(flavor);
    }
    // dc ∈ [0.05, 0.8): Shamanism / Animism / Other
    if dc < 0.5 {
        if t.mysticism > 0.5 && t.hierarchy_degree < 0.3 {
            return FaithLabel::Shamanism;
        } else {
            return FaithLabel::Animism;
        }
    }
    // dc ∈ [0.5, 0.8): 非典型小范围
    FaithLabel::Other
}

// ── ReligiousMotivation ────────────────────────────────

/// 宗教参与动机
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ReligiousMotivation {
    /// "村里人都这样"（多数 NPC 默认）
    #[default]
    SocialCustom,
    /// 真诚的自主信仰（罕有）
    PersonalDevotion,
    /// 危机驱动的临时信仰——since=tick, intensity=0-1
    CrisisDriven { since_tick: u64, intensity: f32 },
    /// "这是我们是谁"（高 faith_as_identity 文化）
    IdentityBased,
    /// "拜财神求财，拜药神求健康"
    Pragmatic,
    /// "我在寻找真理" (<1% NPC)
    SpiritualSeeker,
    /// "从没想过，一直就这样做"
    Habitual,
    /// "不去会被罚款/羞辱"
    SociallyCompelled,
    /// 主动排斥所有宗教实践（罕有）
    AntiReligious,
}

// ── ReligiousPracticeProfile ───────────────────────────

/// NPC 宗教实践档案 — 存储 NPC 参与哪些信仰及深度
///
/// Phase 1 存储在 FaithRegistry。Phase 2 迁入 NPC 模块本地字段（~40B）。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ReligiousPracticeProfile {
    /// 参与权重: (FaithId, 权重) — 通常 1-3 条
    pub participation: Vec<(FaithId, f32)>,
    /// 神学深度: 0=民间信仰, 1=神学家
    pub theological_depth: f32,
    /// 参与动机
    pub motivation: ReligiousMotivation,
}

impl ReligiousPracticeProfile {
    /// 总参与强度——所有信仰权重之和，上限 1.0
    pub fn total_participation(&self) -> f32 {
        self.participation
            .iter()
            .map(|(_, w)| w)
            .sum::<f32>()
            .min(1.0)
    }

    /// 新建——从默认动机开始
    pub fn new(motivation: ReligiousMotivation) -> Self {
        Self {
            participation: Vec::new(),
            theological_depth: 0.0,
            motivation,
        }
    }
}

// ── DerivedReligiosity ─────────────────────────────────

/// 派生虔诚度——实时计算，不存储
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DerivedReligiosity {
    /// 行为强度: 所有参与权重的几何和，上限 1.0
    pub behavioral_intensity: f32,
    /// 神学深度
    pub theological_depth: f32,
    /// 动机自主性: 0=纯被动, 1=纯自主
    pub motivational_autonomy: f32,
}

/// 从实践档案派生虔诚度
pub fn derived_religiosity(profile: &ReligiousPracticeProfile) -> DerivedReligiosity {
    let behavioral_intensity = profile.total_participation();
    let motivational_autonomy = match profile.motivation {
        ReligiousMotivation::PersonalDevotion => 0.9,
        ReligiousMotivation::SpiritualSeeker => 0.8,
        ReligiousMotivation::IdentityBased => 0.7,
        ReligiousMotivation::Pragmatic => 0.5,
        ReligiousMotivation::Habitual => 0.3,
        ReligiousMotivation::SocialCustom => 0.2,
        ReligiousMotivation::SociallyCompelled => 0.1,
        ReligiousMotivation::CrisisDriven { .. } => 0.3,
        ReligiousMotivation::AntiReligious => 0.0,
    };
    DerivedReligiosity {
        behavioral_intensity,
        theological_depth: profile.theological_depth,
        motivational_autonomy,
    }
}

// ── 信仰间关系 ─────────────────────────────────────────

/// 神学距离: 10 维的 RMSE × intensity 调节
pub fn theology_distance(a: &FaithTheology, b: &FaithTheology) -> f32 {
    let sum_sq: f32 = (0..FaithTheology::DIM_COUNT)
        .map(|i| {
            let diff = (a.dim(i) - b.dim(i)) / if i == 0 { 15.0 } else { 1.0 };
            diff * diff
        })
        .sum();
    (sum_sq / FaithTheology::DIM_COUNT as f32)
        .sqrt()
        .clamp(0.0, 1.0)
}

/// A 对 B 的宽容度
///
/// 公式: `clamp((1 - A.exclusivity) - distance*0.3, 0, 1)`
pub fn tolerance(a: &FaithTheology, b: &FaithTheology) -> f32 {
    ((1.0 - a.exclusivity) - theology_distance(a, b) * 0.3).clamp(0.0, 1.0)
}

/// A 与 B 之间的敌意——取双向宽容度的最小值反转
pub fn hostility(a: &FaithTheology, b: &FaithTheology) -> f32 {
    let tol_ab = tolerance(a, b);
    let tol_ba = tolerance(b, a);
    (1.0 - tol_ab.min(tol_ba)).clamp(0.0, 1.0)
}

/// 共享信仰的社交亲和加成
pub fn faith_affinity_bonus(weight_a: f32, weight_b: f32, theology: &FaithTheology) -> f32 {
    (weight_a.min(weight_b) * (0.15 + theology.exclusivity * 0.15)).clamp(0.0, 0.3)
}

// ── FaithQuery trait ───────────────────────────────────

/// 信仰只读查询接口
pub trait FaithQuery: Send + Sync {
    /// 查询神学参数
    fn theology(&self, id: FaithId) -> Option<&FaithTheology>;
    /// 查询派生标签
    fn faith_label(&self, id: FaithId) -> Option<FaithLabel>;
    /// 传教驱动力
    fn proselytizing(&self, id: FaithId) -> f32;
    /// 信仰间宽容度
    fn tolerance_between(&self, a: FaithId, b: FaithId) -> f32;
    /// 信仰间敌意
    fn hostility_between(&self, a: FaithId, b: FaithId) -> f32;
    /// 所有信仰列表
    fn all_faiths(&self) -> &[FaithId];
    /// 信仰数量
    fn faith_count(&self) -> usize;
}

// ── 确定性 PRNG 辅助 ───────────────────────────────────

fn culture_hash(seed: u64, salt: u64) -> f32 {
    let mut x = seed
        .wrapping_add(salt.wrapping_mul(0x9E37_79B9_7F4A_7C15))
        .wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    (x >> 40) as f32 / (1u64 << 24) as f32
}

fn lerp_hash(seed: u64, salt: u64, min: f32, max: f32, intensity: f32) -> f32 {
    let t = culture_hash(seed, salt);
    // intensity 越高 → 越接近最大值（而非均匀分布的中点）
    let biased = t.powf(1.0 / intensity.max(0.1));
    min + biased * (max - min)
}

// ── 测试 ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── FaithId ──

    #[test]
    fn test_faith_id_none() {
        assert_eq!(FAITH_ID_NONE.0, u32::MAX);
    }

    // ── FaithTheology ──

    #[test]
    fn test_default() {
        let t = FaithTheology::default();
        for i in 0..FaithTheology::DIM_COUNT {
            let v = t.dim(i);
            assert!((0.0..=15.0).contains(&v), "dim {i}: {v}");
        }
    }

    #[test]
    fn test_secular_zero() {
        let t = FaithTheology::from_seed(42, 0.05);
        assert_eq!(t.deity_count, 0.0);
        assert_eq!(t.ritual_formality, 0.0);
    }

    #[test]
    fn test_from_seed_deterministic() {
        let a = FaithTheology::from_seed(42, 0.8);
        let b = FaithTheology::from_seed(42, 0.8);
        for i in 0..FaithTheology::DIM_COUNT {
            assert!(
                (a.dim(i) - b.dim(i)).abs() < 0.001,
                "dim {i}: {} vs {}",
                a.dim(i),
                b.dim(i)
            );
        }
    }

    #[test]
    fn test_from_seed_high_religiosity_bigger_range() {
        let low = FaithTheology::from_seed(0, 0.2);
        let high = FaithTheology::from_seed(0, 0.9);
        // 高 religiosity 应产生更极端的值
        assert!(high.deity_count != low.deity_count || high.exclusivity != low.exclusivity);
    }

    #[test]
    fn test_proselytizing_in_range() {
        for seed in 0..50 {
            let t = FaithTheology::from_seed(seed, 0.5 + (seed as f32 % 50.0) / 100.0);
            let p = t.proselytizing();
            assert!((0.0..=1.0).contains(&p), "seed {seed}: {p}");
        }
    }

    #[test]
    fn test_proselytizing_high_exclusive() {
        let t = FaithTheology {
            exclusivity: 1.0,
            orthodoxy_vs_orthopraxy: 1.0,
            faith_as_identity: 1.0,
            ..FaithTheology::default()
        };
        assert!(t.proselytizing() > 0.8);
    }

    // ── FaithLabel ──

    #[test]
    fn test_label_nontheistic() {
        let t = FaithTheology {
            deity_count: 0.02,
            ancestor_importance: 0.1,
            nature_sacredness: 0.05,
            ..FaithTheology::default()
        };
        assert_eq!(derive_faith_label(&t), FaithLabel::NonTheistic);
    }

    #[test]
    fn test_label_shamanism() {
        // dc < 0.5, mysticism > 0.5, hierarchy < 0.3
        let t = FaithTheology {
            deity_count: 0.3,
            mysticism: 0.8,
            hierarchy_degree: 0.1,
            ..FaithTheology::default()
        };
        assert_eq!(derive_faith_label(&t), FaithLabel::Shamanism);
    }

    #[test]
    fn test_label_monotheism_exclusive() {
        let t = FaithTheology {
            deity_count: 1.0,
            exclusivity: 0.9,
            mysticism: 0.3,
            ..FaithTheology::default()
        };
        assert_eq!(
            derive_faith_label(&t),
            FaithLabel::Monotheism(MonotheismFlavor::Exclusive)
        );
    }

    #[test]
    fn test_label_polytheism() {
        let t = FaithTheology {
            deity_count: 4.0,
            nature_sacredness: 0.3,
            hierarchy_degree: 0.7,
            ..FaithTheology::default()
        };
        assert_eq!(
            derive_faith_label(&t),
            FaithLabel::Polytheism(PolytheismFlavor::Pantheon)
        );
    }

    // ── tolerance / hostility ──

    #[test]
    fn test_same_faith_tolerance() {
        let t = FaithTheology::default();
        assert!(tolerance(&t, &t) > 0.4);
    }

    #[test]
    fn test_high_exclusivity_low_tolerance() {
        let a = FaithTheology {
            exclusivity: 0.9,
            ..FaithTheology::default()
        };
        let b = FaithTheology::default();
        assert!(tolerance(&a, &b) < 0.3);
    }

    #[test]
    fn test_hostility_symmetric() {
        let a = FaithTheology::from_seed(1, 0.8);
        let b = FaithTheology::from_seed(2, 0.8);
        assert!((hostility(&a, &b) - hostility(&b, &a)).abs() < 0.001);
    }

    #[test]
    fn test_tolerance_hostility_in_range() {
        for s1 in 0..30 {
            for s2 in 0..30 {
                if s1 == s2 {
                    continue;
                }
                let a = FaithTheology::from_seed(s1, 0.7);
                let b = FaithTheology::from_seed(s2, 0.7);
                assert!((0.0..=1.0).contains(&tolerance(&a, &b)));
                assert!((0.0..=1.0).contains(&hostility(&a, &b)));
            }
        }
    }

    // ── derived_religiosity ──

    #[test]
    fn test_derived_devout() {
        let profile = ReligiousPracticeProfile {
            participation: vec![(FaithId(0), 0.9)],
            theological_depth: 0.8,
            motivation: ReligiousMotivation::PersonalDevotion,
        };
        let d = derived_religiosity(&profile);
        assert!(d.behavioral_intensity > 0.8);
        assert!(d.motivational_autonomy > 0.8);
    }

    #[test]
    fn test_derived_habitual() {
        let profile = ReligiousPracticeProfile {
            participation: vec![(FaithId(0), 0.4)],
            theological_depth: 0.1,
            motivation: ReligiousMotivation::Habitual,
        };
        let d = derived_religiosity(&profile);
        assert!(d.motivational_autonomy < 0.4);
    }

    // ── ReligiousPracticeProfile ──

    #[test]
    fn test_total_participation_capped() {
        let profile = ReligiousPracticeProfile {
            participation: vec![(FaithId(0), 0.6), (FaithId(1), 0.5), (FaithId(2), 0.4)],
            theological_depth: 0.3,
            motivation: ReligiousMotivation::Pragmatic,
        };
        assert!((profile.total_participation() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_faith_affinity_bonus() {
        let t = FaithTheology {
            exclusivity: 0.8,
            ..FaithTheology::default()
        };
        let bonus = faith_affinity_bonus(0.7, 0.7, &t);
        assert!(bonus > 0.15);
    }
}
