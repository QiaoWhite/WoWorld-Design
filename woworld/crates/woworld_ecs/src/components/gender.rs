//! BiologicalSex 生物性别 Component — ECS 铁律合规
//!
//! 参见: `开发文档/02-性别与吸引力系统.md`
//!
//! Phase 1: 仅生物性别标记。性别认同由文化规范 (NormScope) 处理，非本 Component。

use crate::prng::pseudo_random_f32;

/// 生物性别——5 变体，覆盖真实生物多样性
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BiologicalSex {
    Male,
    #[default]
    Female,
    /// 同时具有两性生殖能力
    Hermaphroditic,
    /// 生命中可变——当前阶段
    Sequential { current_phase: SequentialPhase },
    /// 无性别（亡灵/构装体等）
    Neuter,
}

/// 顺序性别的当前阶段
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SequentialPhase {
    Male,
    Female,
    /// 性别转换中（生理过渡期）
    Transitioning,
}

impl BiologicalSex {
    /// 从种子确定性生成——按设计分布
    ///
    /// Male ~48%, Female ~48%, Hermaphroditic ~2%, Sequential ~1%, Neuter ~1%
    pub fn from_seed(seed: u64) -> Self {
        let r = pseudo_random_f32(seed);
        match r {
            r if r < 0.48 => BiologicalSex::Male,
            r if r < 0.96 => BiologicalSex::Female,
            r if r < 0.98 => BiologicalSex::Hermaphroditic,
            r if r < 0.99 => {
                // Sequential: 子种子决定初始阶段
                let phase_seed = seed.wrapping_add(1);
                let p = pseudo_random_f32(phase_seed);
                let phase = if p < 0.5 {
                    SequentialPhase::Male
                } else {
                    SequentialPhase::Female
                };
                BiologicalSex::Sequential {
                    current_phase: phase,
                }
            }
            _ => BiologicalSex::Neuter,
        }
    }

    pub fn is_male(&self) -> bool {
        matches!(self, BiologicalSex::Male)
    }

    pub fn is_female(&self) -> bool {
        matches!(self, BiologicalSex::Female)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_female() {
        assert_eq!(BiologicalSex::default(), BiologicalSex::Female);
    }

    #[test]
    fn test_from_seed_deterministic() {
        let a = BiologicalSex::from_seed(42);
        let b = BiologicalSex::from_seed(42);
        assert_eq!(a, b);
    }

    #[test]
    fn test_different_seeds_different() {
        // 至少有一些不同（可能全相同但概率极低）
        let mut all_same = true;
        let first = BiologicalSex::from_seed(0);
        for s in 1..20 {
            if BiologicalSex::from_seed(s) != first {
                all_same = false;
                break;
            }
        }
        assert!(!all_same, "20 seeds should not all produce same sex");
    }

    #[test]
    fn test_from_seed_produces_all_variants() {
        let mut seen_male = false;
        let mut seen_female = false;
        let mut seen_herm = false;
        let mut seen_seq = false;
        let mut seen_neuter = false;

        for s in 0..500 {
            match BiologicalSex::from_seed(s) {
                BiologicalSex::Male => seen_male = true,
                BiologicalSex::Female => seen_female = true,
                BiologicalSex::Hermaphroditic => seen_herm = true,
                BiologicalSex::Sequential { .. } => seen_seq = true,
                BiologicalSex::Neuter => seen_neuter = true,
            }
        }
        assert!(seen_male, "should see Male");
        assert!(seen_female, "should see Female");
        assert!(seen_herm, "should see Hermaphroditic");
        assert!(seen_seq, "should see Sequential");
        assert!(seen_neuter, "should see Neuter");
    }

    #[test]
    fn test_is_methods() {
        let male = BiologicalSex::Male;
        assert!(male.is_male());
        assert!(!male.is_female());

        let female = BiologicalSex::Female;
        assert!(female.is_female());
        assert!(!female.is_male());

        let neuter = BiologicalSex::Neuter;
        assert!(!neuter.is_male());
        assert!(!neuter.is_female());
    }

    #[test]
    fn test_distribution_mostly_binary() {
        let mut male_count = 0;
        let mut female_count = 0;
        let total = 200;
        for s in 0..total {
            match BiologicalSex::from_seed(s) {
                BiologicalSex::Male => male_count += 1,
                BiologicalSex::Female => female_count += 1,
                _ => {}
            }
        }
        let binary_pct = (male_count + female_count) as f32 / total as f32;
        assert!(binary_pct > 0.80, "binary should be >80%, got {:.1}%", binary_pct * 100.0);
    }
}
