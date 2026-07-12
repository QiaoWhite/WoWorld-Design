//! 经济 Component — Wallet + EconomicCognition
//!
//! 纯数据 Component（ECS 铁律 1）。
//! EconomicCognition 从 BigFive 派生——不存储独立人格维度。
//!
//! 参见: woworld_core::economy

/// NPC 钱包 — 铜/银/金三级货币
///
/// 换算: 金:银:铜 = 1:20:400 (1 gold = 20 silver = 400 copper)
/// 24 bytes, Copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Wallet {
    pub copper: u64,
    pub silver: u64,
    pub gold: u64,
}

impl Wallet {
    /// 换算为总铜币
    pub fn total_copper(&self) -> u64 {
        self.copper + self.silver * 20 + self.gold * 400
    }

    /// 从总铜币创建
    pub fn from_copper(total: u64) -> Self {
        let gold = total / 400;
        let remainder = total % 400;
        let silver = remainder / 20;
        let copper = remainder % 20;
        Self {
            copper,
            silver,
            gold,
        }
    }

    /// 从种子生成初始钱包（Pareto 分布）
    ///
    /// 对齐设计文档 001 §5-B——财富分配遵循幂律：
    /// - x_min = 50 copper（最低）
    /// - α = 1.5（中等不平等）
    /// - 上限 5000 copper
    ///
    /// 结果：50% 人口 <79 copper，90% <232 copper，99% <1077 copper。
    pub fn from_seed(seed: u64) -> Self {
        // 确定性 hash seed → uniform [0, 1)
        let hash = seed
            .wrapping_mul(0x9E37_79B9_7F4A_7C15)
            .wrapping_add(0xDA3C_BF47);
        let u = ((hash >> 32) as f64) / (u32::MAX as f64 + 1.0);
        // 避免 u=1.0（导致除零）
        let u = if u >= 0.999999 { 0.999999 } else { u };

        // Pareto Type I 逆 CDF: x = x_min / (1-u)^(1/α)
        let x_min: f64 = 50.0;
        let alpha: f64 = 1.5;
        let amount = (x_min / (1.0 - u).powf(1.0 / alpha)) as u64;

        // 截断到 [50, 5000]
        let amount = amount.clamp(50, 5000);
        Self::from_copper(amount)
    }
}

impl From<Wallet> for woworld_core::economy::WalletSnapshot {
    fn from(w: Wallet) -> Self {
        Self {
            copper: w.copper,
            silver: w.silver,
            gold: w.gold,
        }
    }
}

impl From<woworld_core::economy::WalletSnapshot> for Wallet {
    fn from(s: woworld_core::economy::WalletSnapshot) -> Self {
        Self {
            copper: s.copper,
            silver: s.silver,
            gold: s.gold,
        }
    }
}

/// 经济认知 — 6 维经济决策缓存
///
/// 所有字段从 BigFive + 技能 + 经验派生（纯函数，零新人格维度）。
/// 每次 Personality/LifeStage 变更时重算。
/// 24 bytes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EconomicCognition {
    /// 金融素养 [0,1] — 理解价格/利率/货币
    pub financial_literacy: f32,
    /// 市场理解 [0,1] — 供需/套利/竞争动态
    pub market_understanding: f32,
    /// 价格记忆准确性 [0,1] — 1=过目不忘, 0=完全扭曲
    pub price_memory_accuracy: f32,
    /// 时间偏好率 [0.05, 0.50] — 年化, 0.05=极度耐心, 0.50=极度短视
    pub time_preference_rate: f32,
    /// 市场搜索广度 [1, 5] — 做决定前比较几个市场
    pub market_search_breadth: u8,
    /// 满意化阈值 [0,1] — 达到即停止搜索
    pub satisficing_threshold: f32,
}

impl Default for EconomicCognition {
    fn default() -> Self {
        Self {
            financial_literacy: 0.5,
            market_understanding: 0.5,
            price_memory_accuracy: 0.5,
            time_preference_rate: 0.10,
            market_search_breadth: 1,
            satisficing_threshold: 0.5,
        }
    }
}

impl EconomicCognition {
    /// 从 BigFive 人格派生经济认知
    ///
    /// 所有公式来自设计文档 007:
    /// - financial_literacy: C×0.4 + O×0.3 + intelligence_proxy×0.3
    /// - market_understanding: O×0.5 + E×0.2 + intelligence_proxy×0.3
    /// - price_memory_accuracy: C×0.5 + (1-N)×0.5
    /// - time_preference_rate: (1-C)×0.3 + N×0.15 + base_0.05, clamped [0.05, 0.50]
    /// - market_search_breadth: 1 + floor(O×3 + E×1.5), clamped [1, 5]
    /// - satisficing_threshold: 0.5 + (1-C)×0.3 + N×0.2, clamped [0.1, 1.0]
    pub fn derive_from_bigfive(
        openness: f32,
        conscientiousness: f32,
        extraversion: f32,
        _agreeableness: f32,
        neuroticism: f32,
        // 智力代理——消费方自行提供（默认从 C+O 派生）
        intelligence_proxy: Option<f32>,
    ) -> Self {
        let intel = intelligence_proxy.unwrap_or((conscientiousness + openness) / 2.0);

        let financial_literacy =
            (conscientiousness * 0.4 + openness * 0.3 + intel * 0.3).clamp(0.0, 1.0);

        let market_understanding =
            (openness * 0.5 + extraversion * 0.2 + intel * 0.3).clamp(0.0, 1.0);

        let price_memory_accuracy =
            (conscientiousness * 0.5 + (1.0 - neuroticism) * 0.5).clamp(0.0, 1.0);

        let time_preference_rate =
            ((1.0 - conscientiousness) * 0.3 + neuroticism * 0.15 + 0.05).clamp(0.05, 0.50);

        let market_search_breadth_raw = 1.0 + openness * 3.0 + extraversion * 1.5;
        let market_search_breadth = (market_search_breadth_raw.round() as u8).clamp(1, 5);

        let satisficing_threshold =
            (0.5 + (1.0 - conscientiousness) * 0.3 + neuroticism * 0.2).clamp(0.1, 1.0);

        Self {
            financial_literacy,
            market_understanding,
            price_memory_accuracy,
            time_preference_rate,
            market_search_breadth,
            satisficing_threshold,
        }
    }
}

// ── 测试 ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Wallet ──

    #[test]
    fn test_wallet_default_zero() {
        let w = Wallet::default();
        assert_eq!(w.total_copper(), 0);
    }

    #[test]
    fn test_wallet_total_copper() {
        let w = Wallet {
            copper: 15,
            silver: 3,
            gold: 1,
        };
        assert_eq!(w.total_copper(), 475);
    }

    #[test]
    fn test_wallet_from_copper_roundtrip() {
        for total in [0, 15, 85, 400, 920, 1234] {
            let w = Wallet::from_copper(total);
            assert_eq!(w.total_copper(), total);
        }
    }

    #[test]
    fn test_wallet_from_seed_deterministic() {
        let a = Wallet::from_seed(42);
        let b = Wallet::from_seed(42);
        assert_eq!(a.total_copper(), b.total_copper());
    }

    #[test]
    fn test_wallet_from_seed_in_range() {
        let mut values: Vec<u64> = Vec::new();
        for seed in 0..1000 {
            let w = Wallet::from_seed(seed);
            let total = w.total_copper();
            assert!(
                (50..=5000).contains(&total),
                "seed {seed}: {total} not in [50,5000]"
            );
            values.push(total);
        }
        values.sort();

        // Pareto 特征：中位数 < 均值（右偏）
        let median = values[values.len() / 2];
        let mean = values.iter().sum::<u64>() / values.len() as u64;
        assert!(
            median < mean,
            "Pareto: median {median} should be < mean {mean}"
        );

        // 至少 50% 人口 < 150 copper（设计目标：50% < 79）
        let below_150 = values.iter().filter(|&&v| v < 150).count();
        assert!(
            below_150 > values.len() / 2,
            "50%+ should be <150 copper, got {below_150}/{len}",
            len = values.len()
        );

        // 存在富 NPC（>1000 copper）
        let rich = values.iter().filter(|&&v| v > 1000).count();
        assert!(rich > 0, "Pareto: should have some wealthy NPCs >1000");
    }

    #[test]
    fn test_wallet_size() {
        assert_eq!(std::mem::size_of::<Wallet>(), 24);
    }

    // ── EconomicCognition ──

    #[test]
    fn test_cognition_default_reasonable() {
        let c = EconomicCognition::default();
        assert!((0.0..=1.0).contains(&c.financial_literacy));
        assert!((0.0..=1.0).contains(&c.market_understanding));
        assert!((0.05..=0.50).contains(&c.time_preference_rate));
        assert!((1..=5).contains(&c.market_search_breadth));
    }

    #[test]
    fn test_derive_from_bigfive_range() {
        for seed in 0..100 {
            let s = seed as f32 / 100.0;
            let c = EconomicCognition::derive_from_bigfive(s, s, s, s, s, None);
            assert!((0.0..=1.0).contains(&c.financial_literacy));
            assert!((0.0..=1.0).contains(&c.market_understanding));
            assert!((0.0..=1.0).contains(&c.price_memory_accuracy));
            assert!((0.05..=0.50).contains(&c.time_preference_rate));
            assert!((1..=5).contains(&c.market_search_breadth));
            assert!((0.1..=1.0).contains(&c.satisficing_threshold));
        }
    }

    #[test]
    fn test_derive_high_conscientiousness() {
        // C=1.0, N=0.0 → high financial literacy, patient, accurate memory
        let c = EconomicCognition::derive_from_bigfive(0.5, 1.0, 0.5, 0.5, 0.0, None);
        assert!(c.financial_literacy > 0.7);
        assert!(c.price_memory_accuracy > 0.8);
        assert!(
            c.time_preference_rate < 0.15,
            "conscientious → patient: {}",
            c.time_preference_rate
        );
    }

    #[test]
    fn test_derive_low_conscientiousness_neurotic() {
        let c = EconomicCognition::derive_from_bigfive(0.5, 0.0, 0.5, 0.5, 1.0, None);
        assert!(
            c.time_preference_rate > 0.25,
            "low C + high N → myopic: {}",
            c.time_preference_rate
        );
        assert!(
            c.satisficing_threshold > 0.5,
            "low C → high satisficing: {}",
            c.satisficing_threshold
        );
    }

    #[test]
    fn test_derive_high_openness_extraversion() {
        let c = EconomicCognition::derive_from_bigfive(1.0, 0.5, 1.0, 0.5, 0.5, None);
        assert!(
            c.market_search_breadth >= 4,
            "high O+E → wide search: {}",
            c.market_search_breadth
        );
        assert!(c.market_understanding > 0.6);
    }

    #[test]
    fn test_derive_deterministic() {
        let a = EconomicCognition::derive_from_bigfive(0.3, 0.7, 0.4, 0.6, 0.2, None);
        let b = EconomicCognition::derive_from_bigfive(0.3, 0.7, 0.4, 0.6, 0.2, None);
        assert!((a.financial_literacy - b.financial_literacy).abs() < f32::EPSILON);
        assert_eq!(a.market_search_breadth, b.market_search_breadth);
    }

    #[test]
    fn test_cognition_size() {
        assert_eq!(std::mem::size_of::<EconomicCognition>(), 24);
    }
}
