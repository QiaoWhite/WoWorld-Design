//! 行为经济学 — 10 个认知偏差从 BigFive 人格的纯函数派生
//!
//! 所有概念均从现有 NPC 字段派生——零新存储需求。
//! 设计文档 007 §1-10.
//!
//! 参见: `WoWorld-Design/Happy Game/开发阶段/经济系统/007-NPC经济心智.md`

/// 经济行为参数——从 BigFive + 认知输入聚合
///
/// 仅供行为经济学函数消费，不存储在 ECS Component 中。
#[derive(Debug, Clone, Copy)]
pub struct EconBehaviorParams {
    /// 开放性 (0-1)
    pub openness: f32,
    /// 尽责性 (0-1)
    pub conscientiousness: f32,
    /// 外向性 (0-1)
    pub extraversion: f32,
    /// 宜人性 (0-1)
    pub agreeableness: f32,
    /// 神经质 (0-1)
    pub neuroticism: f32,
    /// 金融素养 (0-1, 从技能+经验派生)
    pub financial_literacy: f32,
    /// 市场理解 (0-1)
    pub market_understanding: f32,
    /// 拥有天数（用于禀赋效应/现状偏差）
    pub ownership_days: f32,
}

impl Default for EconBehaviorParams {
    fn default() -> Self {
        Self {
            openness: 0.5,
            conscientiousness: 0.5,
            extraversion: 0.5,
            agreeableness: 0.5,
            neuroticism: 0.5,
            financial_literacy: 0.5,
            market_understanding: 0.5,
            ownership_days: 0.0,
        }
    }
}

// ── 1. Loss Aversion (损失厌恶) ────────────────────────

/// 损失厌恶乘数 — 损失的主观权重 vs 收益
///
/// 公式: `multiplier = 2.0 * (0.75 + N*1.25) * (1.0 - wisdom*0.4)`
/// 范围: [1.5, 2.5]
///
/// `wisdom` 近似为 `(financial_literacy + market_understanding) / 2.0`
pub fn loss_aversion_multiplier(params: &EconBehaviorParams) -> f32 {
    let wisdom = (params.financial_literacy + params.market_understanding) / 2.0;
    let raw = 2.0 * (0.75 + params.neuroticism * 1.25) * (1.0 - wisdom * 0.4);
    raw.clamp(1.5, 2.5)
}

/// 主观价值——正收益不变，负收益乘 loss_multiplier
pub fn subjective_value(objective_delta: f32, params: &EconBehaviorParams) -> f32 {
    if objective_delta >= 0.0 {
        objective_delta
    } else {
        objective_delta * loss_aversion_multiplier(params)
    }
}

// ── 2. Mental Accounting (心理账户) ────────────────────

/// 心理账户类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MentalAccount {
    /// 常规收入
    RegularIncome,
    /// 意外之财
    Windfall,
    /// 存款
    Savings,
    /// 礼物
    Gift,
}

/// 消费倾向——按心理账户类型
///
/// RegularIncome: 0.7 - C*0.3
/// Windfall: 0.9 - C*0.5
/// Savings: 0.2
/// Gift: 0.6 + E*0.2
pub fn spending_propensity(account: MentalAccount, params: &EconBehaviorParams) -> f32 {
    match account {
        MentalAccount::RegularIncome => (0.7 - params.conscientiousness * 0.3).clamp(0.1, 1.0),
        MentalAccount::Windfall => (0.9 - params.conscientiousness * 0.5).clamp(0.2, 1.0),
        MentalAccount::Savings => 0.2,
        MentalAccount::Gift => (0.6 + params.extraversion * 0.2).clamp(0.1, 1.0),
    }
}

// ── 3. Anchoring (锚定效应) ────────────────────────────

/// 价格感知——基于锚定价格的判断
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PricePerception {
    VeryCheap,
    Cheap,
    Fair,
    Expensive,
    VeryExpensive,
}

/// 价格感知分类
///
/// 阈值以 wisdom (≈(FL+MU)/2) 调节:
/// VeryCheap: ratio < 0.85 - wisdom*0.2
/// Cheap: ratio < 0.95 - wisdom*0.1
/// Fair: ratio < 1.05 + wisdom*0.1
/// Expensive: ratio < 1.20 + wisdom*0.2
/// VeryExpensive: above
pub fn price_perception(price_ratio: f32, params: &EconBehaviorParams) -> PricePerception {
    let wisdom = (params.financial_literacy + params.market_understanding) / 2.0;
    if price_ratio < 0.85 - wisdom * 0.2 {
        PricePerception::VeryCheap
    } else if price_ratio < 0.95 - wisdom * 0.1 {
        PricePerception::Cheap
    } else if price_ratio < 1.05 + wisdom * 0.1 {
        PricePerception::Fair
    } else if price_ratio < 1.20 + wisdom * 0.2 {
        PricePerception::Expensive
    } else {
        PricePerception::VeryExpensive
    }
}

// ── 4. Endowment Effect (禀赋效应) ─────────────────────

/// 禀赋效应乘数——拥有越久、开放度越低，估值越高
///
/// 公式（设计文档 007 §4）:
/// `mult = 1.0 + (ownership_days/365*0.3) * (1.5 - openness*0.5)`
/// 即 ownership 部分被 openness 调节，零拥有时恒为 1.0
pub fn endowment_multiplier(params: &EconBehaviorParams) -> f32 {
    let ownership_effect = (params.ownership_days / 365.0) * 0.3;
    let openness_mod = 1.5 - params.openness * 0.5;
    (1.0 + ownership_effect * openness_mod).clamp(1.0, 2.0)
}

// ── 5. Hyperbolic Discounting (双曲贴现) ───────────────

/// 双曲贴现——未来价值的主观现值
///
/// 公式: `V = FV / (1 + time_preference_rate * days/365)`
/// time_preference_rate 从 EconomicCognition.time_preference_rate 读取（已派生）
pub fn hyperbolic_discount(future_value: f32, days_until: f32, time_preference_rate: f32) -> f32 {
    future_value / (1.0 + time_preference_rate * days_until / 365.0)
}

// ── 6. Herd Behavior (从众行为) ────────────────────────

/// 社会证明权重——观察到他人行动后的跟从倾向
///
/// 公式: `social_proof_weight = observed_ratio * (0.5 + E*1.5) * (0.5 + A*1.5)`, clamped to 1.0
pub fn social_proof_weight(observed_ratio: f32, params: &EconBehaviorParams) -> f32 {
    (observed_ratio * (0.5 + params.extraversion * 1.5) * (0.5 + params.agreeableness * 1.5))
        .clamp(0.0, 1.0)
}

// ── 7. Satisficing (满意化) ────────────────────────────

/// 满意化阈值——达到此阈值即停止搜索
///
/// 公式与 EconomicCognition::derive_from_bigfive() 保持一致:
/// `0.5 + (1-C)*0.3 + N*0.2`
pub fn satisficing_threshold(conscientiousness: f32, neuroticism: f32) -> f32 {
    (0.5 + (1.0 - conscientiousness) * 0.3 + neuroticism * 0.2).clamp(0.1, 1.0)
}

// ── 8. Fairness (公平感知) ─────────────────────────────

/// 公平感知——价格比 vs 参考价
///
/// ratio > 1.3 → -0.5, > 1.1 → -0.2, < 0.8 → 0.3, else → 0.1
/// 缩放: `agreeableness*(0.5+A*1.0)`
pub fn fairness_judgment(price_ratio: f32, params: &EconBehaviorParams) -> f32 {
    let raw = if price_ratio > 1.3 {
        -0.5
    } else if price_ratio > 1.1 {
        -0.2
    } else if price_ratio < 0.8 {
        0.3
    } else {
        0.1
    };
    (raw * (0.5 + params.agreeableness * 1.0)).clamp(-1.0, 1.0)
}

// ── 9. Overconfidence (过度自信) ───────────────────────

/// 自我评估的议价能力——通常高于实际
///
/// 公式: `self_assessed = actual_bargaining + (1-intelligence)*0.3 + E*0.2`
/// `intelligence` 近似为 `(financial_literacy + market_understanding) / 2.0`
pub fn overconfident_bargaining(actual_skill: f32, params: &EconBehaviorParams) -> f32 {
    let intelligence = (params.financial_literacy + params.market_understanding) / 2.0;
    (actual_skill + (1.0 - intelligence) * 0.3 + params.extraversion * 0.2).clamp(0.0, 1.0)
}

// ── 10. Status Quo Bias (维持现状偏差) ─────────────────

/// 维持现状偏差——变更供应商/习惯的阻力
///
/// 公式: `(1-O)*0.5 + C*0.3 + min(ownership_days/365, 1.0)*0.3`
pub fn status_quo_bias(params: &EconBehaviorParams) -> f32 {
    ((1.0 - params.openness) * 0.5
        + params.conscientiousness * 0.3
        + (params.ownership_days / 365.0).min(1.0) * 0.3)
        .clamp(0.0, 1.0)
}

// ── 综合: 消费倾向 ─────────────────────────────────────

/// 综合消费倾向——从 BigFive + 情绪 + 认知聚合
///
/// 公式（设计文档 007）:
/// ```text
/// weight = 0.5
///   + (C-0.5)*-0.15 + (N-0.5)*-0.10 + (E-0.5)*0.10
///   + (O-0.5)*0.08 + pleasure*0.12 + mood*0.08
///   + (FL-0.5)*-0.05 + (habit-0.5)*0.10
/// ```
pub fn consumption_propensity(
    params: &EconBehaviorParams,
    pleasure: f32,
    mood: f32,
    habit_weight: f32,
) -> f32 {
    let raw = 0.5
        + (params.conscientiousness - 0.5) * -0.15
        + (params.neuroticism - 0.5) * -0.10
        + (params.extraversion - 0.5) * 0.10
        + (params.openness - 0.5) * 0.08
        + pleasure * 0.12
        + mood * 0.08
        + (params.financial_literacy - 0.5) * -0.05
        + (habit_weight - 0.5) * 0.10;
    raw.clamp(0.05, 0.95)
}

// ── 测试 ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn default_params() -> EconBehaviorParams {
        EconBehaviorParams::default()
    }

    // ── Loss Aversion ──

    #[test]
    fn test_loss_aversion_in_range() {
        for seed in 0..100 {
            let p = EconBehaviorParams {
                neuroticism: (seed as f32 % 100.0) / 100.0,
                financial_literacy: ((seed + 30) as f32 % 100.0) / 100.0,
                market_understanding: ((seed + 60) as f32 % 100.0) / 100.0,
                ..default_params()
            };
            let m = loss_aversion_multiplier(&p);
            assert!((1.5..=2.5).contains(&m), "seed {seed}: {m} not in [1.5,2.5]");
        }
    }

    #[test]
    fn test_loss_aversion_high_neuroticism() {
        let p = EconBehaviorParams { neuroticism: 1.0, financial_literacy: 0.0, market_understanding: 0.0, ..default_params() };
        let m = loss_aversion_multiplier(&p);
        assert!(m > 2.2, "high neuroticism + low wisdom → high loss aversion");
    }

    #[test]
    fn test_subjective_value_loss_amplified() {
        let p = default_params();
        let gain = subjective_value(100.0, &p);
        let loss = subjective_value(-100.0, &p);
        assert_eq!(gain, 100.0);
        assert!(loss < -150.0, "loss should be amplified: {loss}");
    }

    // ── Mental Accounting ──

    #[test]
    fn test_spending_propensity_savings_low() {
        let p = default_params();
        assert!((spending_propensity(MentalAccount::Savings, &p) - 0.2).abs() < f32::EPSILON);
    }

    #[test]
    fn test_spending_propensity_windfall_high() {
        let p = EconBehaviorParams { conscientiousness: 0.0, ..default_params() };
        assert!(spending_propensity(MentalAccount::Windfall, &p) > 0.8);
    }

    #[test]
    fn test_spending_propensity_in_range() {
        for seed in 0..100 {
            let p = EconBehaviorParams {
                conscientiousness: (seed as f32 % 100.0) / 100.0,
                extraversion: ((seed + 40) as f32 % 100.0) / 100.0,
                ..default_params()
            };
            for account in [MentalAccount::RegularIncome, MentalAccount::Windfall, MentalAccount::Savings, MentalAccount::Gift] {
                let sp = spending_propensity(account, &p);
                assert!((0.0..=1.0).contains(&sp));
            }
        }
    }

    // ── Anchoring (Price Perception) ──

    #[test]
    fn test_price_perception_extremes() {
        let p = default_params();
        assert_eq!(price_perception(0.5, &p), PricePerception::VeryCheap);
        assert_eq!(price_perception(1.0, &p), PricePerception::Fair);
        assert_eq!(price_perception(2.0, &p), PricePerception::VeryExpensive);
    }

    #[test]
    fn test_price_perception_wisdom_narrows_fair_range() {
        let p = EconBehaviorParams { financial_literacy: 1.0, market_understanding: 1.0, ..default_params() };
        // wisdom=1.0: VeryCheap < 0.65, Cheap < 0.85, Fair < 1.15, Expensive < 1.40
        assert_eq!(price_perception(0.5, &p), PricePerception::VeryCheap);
        assert_eq!(price_perception(0.75, &p), PricePerception::Cheap);
        assert_eq!(price_perception(1.00, &p), PricePerception::Fair);
    }

    // ── Endowment Effect ──

    #[test]
    fn test_endowment_no_ownership() {
        let p = EconBehaviorParams { ownership_days: 0.0, openness: 1.0, ..default_params() };
        assert!((endowment_multiplier(&p) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_endowment_long_ownership() {
        let p = EconBehaviorParams { ownership_days: 365.0, openness: 0.0, ..default_params() };
        let m = endowment_multiplier(&p);
        // 1.0 + 0.3 * (1.5 - 0*0.5) = 1.0 + 0.45 = 1.45
        assert!(m > 1.3, "1 year ownership + low openness → endowment: {m}");
    }

    #[test]
    fn test_endowment_in_range() {
        for seed in 0..100 {
            let p = EconBehaviorParams {
                ownership_days: ((seed % 50) as f32) * 20.0,
                openness: (seed as f32 % 100.0) / 100.0,
                ..default_params()
            };
            let m = endowment_multiplier(&p);
            assert!((1.0..=2.0).contains(&m));
        }
    }

    // ── Hyperbolic Discounting ──

    #[test]
    fn test_hyperbolic_no_delay() {
        let v = hyperbolic_discount(100.0, 0.0, 0.10);
        assert!((v - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_hyperbolic_far_future_discounted() {
        let v = hyperbolic_discount(100.0, 365.0, 0.10);
        assert!(v < 100.0, "future value should be discounted: {v}");
    }

    // ── Herd Behavior ──

    #[test]
    fn test_herd_no_observers() {
        let p = default_params();
        assert!((social_proof_weight(0.0, &p) - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_herd_extrovert_agreeable() {
        let p = EconBehaviorParams { extraversion: 1.0, agreeableness: 1.0, ..default_params() };
        let w = social_proof_weight(0.5, &p);
        assert!(w > 0.8, "extrovert+agreeable should follow herd: {w}");
    }

    // ── Satisficing ──

    #[test]
    fn test_satisficing_high_conscientiousness() {
        let t = satisficing_threshold(1.0, 0.0);
        assert!(t < 0.6, "high C → stricter threshold: {t}");
    }

    #[test]
    fn test_satisficing_in_range() {
        for seed in 0..100 {
            let c = (seed as f32 % 100.0) / 100.0;
            let n = ((seed + 50) as f32 % 100.0) / 100.0;
            let t = satisficing_threshold(c, n);
            assert!((0.1..=1.0).contains(&t));
        }
    }

    // ── Fairness ──

    #[test]
    fn test_fairness_very_expensive() {
        let p = default_params();
        let f = fairness_judgment(1.5, &p);
        assert!(f < -0.3, "price gouging should feel unfair: {f}");
    }

    #[test]
    fn test_fairness_bargain() {
        let p = default_params();
        let f = fairness_judgment(0.7, &p);
        assert!(f > 0.0, "bargain should feel positive: {f}");
    }

    #[test]
    fn test_fairness_in_range() {
        for seed in 0..100 {
            let p = EconBehaviorParams {
                agreeableness: (seed as f32 % 100.0) / 100.0,
                ..default_params()
            };
            for ratio in [0.5, 0.9, 1.0, 1.15, 1.5] {
                let f = fairness_judgment(ratio, &p);
                assert!((-1.0..=1.0).contains(&f));
            }
        }
    }

    // ── Overconfidence ──

    #[test]
    fn test_overconfidence_boosts() {
        let p = EconBehaviorParams { financial_literacy: 0.2, market_understanding: 0.2, extraversion: 0.8, ..default_params() };
        let self_assess = overconfident_bargaining(0.5, &p);
        assert!(self_assess > 0.5, "should overestimate: {self_assess}");
    }

    #[test]
    fn test_overconfidence_expert_accurate() {
        let p = EconBehaviorParams { financial_literacy: 1.0, market_understanding: 1.0, extraversion: 0.0, ..default_params() };
        let self_assess = overconfident_bargaining(0.5, &p);
        assert!((self_assess - 0.5).abs() < 0.09, "expert should be accurate: {self_assess}");
    }

    // ── Status Quo Bias ──

    #[test]
    fn test_status_quo_open_adventurous() {
        let p = EconBehaviorParams { openness: 1.0, conscientiousness: 0.0, ownership_days: 0.0, ..default_params() };
        let b = status_quo_bias(&p);
        assert!(b < 0.2, "high openness should resist bias: {b}");
    }

    #[test]
    fn test_status_quo_conservative() {
        let p = EconBehaviorParams { openness: 0.0, conscientiousness: 1.0, ownership_days: 365.0, ..default_params() };
        let b = status_quo_bias(&p);
        assert!(b > 0.7, "conservative should resist change: {b}");
    }

    // ── Consumption Propensity ──

    #[test]
    fn test_consumption_propensity_midpoint() {
        let p = default_params();
        let cp = consumption_propensity(&p, 0.0, 0.0, 0.5);
        // all 0.5 → all deviations = 0, only base 0.5
        assert!((cp - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_consumption_propensity_high_pleasure() {
        let p = default_params();
        let cp = consumption_propensity(&p, 1.0, 1.0, 0.5);
        assert!(cp > 0.6, "high pleasure+mood → more spending: {cp}");
    }

    #[test]
    fn test_consumption_propensity_conservative() {
        let p = EconBehaviorParams { conscientiousness: 1.0, neuroticism: 1.0, financial_literacy: 1.0, ..default_params() };
        let cp = consumption_propensity(&p, 0.0, 0.0, 0.5);
        assert!(cp < 0.4, "conservative → less spending: {cp}");
    }

    #[test]
    fn test_consumption_propensity_in_range() {
        for seed in 0..100 {
            let p = EconBehaviorParams {
                openness: (seed as f32 % 100.0) / 100.0,
                conscientiousness: ((seed + 20) as f32 % 100.0) / 100.0,
                extraversion: ((seed + 40) as f32 % 100.0) / 100.0,
                agreeableness: ((seed + 60) as f32 % 100.0) / 100.0,
                neuroticism: ((seed + 80) as f32 % 100.0) / 100.0,
                financial_literacy: ((seed + 15) as f32 % 100.0) / 100.0,
                market_understanding: ((seed + 35) as f32 % 100.0) / 100.0,
                ..default_params()
            };
            let cp = consumption_propensity(&p, (seed % 3) as f32 * 0.4 - 0.4, (seed % 3) as f32 * 0.3 - 0.3, 0.5);
            assert!((0.05..=0.95).contains(&cp), "seed {seed}: {cp}");
        }
    }
}
