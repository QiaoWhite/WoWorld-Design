//! 生命周期 Component — ECS 铁律合规
//!
//! 参见: `开发文档/07-生命周期系统/` + `生命/014-生命周期时钟与事件.md`
//!
//! Phase 1: 年龄跟踪 + 7 阶段判定 + 属性修正。死亡/生育/婴儿 Phase 2+。

/// 7 阶段发育阶段（Life Module 014 定义）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifeStage {
    /// 0–2% max_lifespan — 完全依赖照顾者
    Infant,
    /// 2–15% — 身体快速成长
    Juvenile,
    /// 15–25% — 性成熟，社会角色形成
    Adolescent,
    /// 25–45% — 身体与认知巅峰
    YoungAdult,
    /// 45–75% — 稳定期
    Adult,
    /// 75–95% — 身体开始衰退
    MiddleAge,
    /// 95%+ — 晚年
    Elder,
}

impl LifeStage {
    /// 从年龄占比判定阶段（7 阶段阈值）
    pub fn from_age_ratio(ratio: f32) -> Self {
        match ratio {
            r if r < 0.02 => LifeStage::Infant,
            r if r < 0.15 => LifeStage::Juvenile,
            r if r < 0.25 => LifeStage::Adolescent,
            r if r < 0.45 => LifeStage::YoungAdult,
            r if r < 0.75 => LifeStage::Adult,
            r if r < 0.95 => LifeStage::MiddleAge,
            _ => LifeStage::Elder,
        }
    }

    /// 阶段对应的属性乘数（中点值——实际使用时结合 age_pct 线性插值）
    pub fn attribute_multiplier(&self) -> f32 {
        match self {
            LifeStage::Infant => 0.2,
            LifeStage::Juvenile => 0.65,
            LifeStage::Adolescent => 0.875,
            LifeStage::YoungAdult => 1.0,
            LifeStage::Adult => 0.95,
            LifeStage::MiddleAge => 0.8,
            LifeStage::Elder => 0.55,
        }
    }
}

/// 年龄跟踪——每帧由 age_system 推进
#[derive(Debug, Clone, Copy)]
pub struct Age {
    /// 当前年龄（游戏日）
    pub age_days: f32,
    /// 物种最大寿命（游戏日）
    pub max_lifespan_days: f32,
}

impl Age {
    /// 创建年龄组件
    ///
    /// `max_lifespan_years`: 物种最大寿命（年），转换到天（360 天/年）
    /// `initial_age_years`: 初始年龄（年），默认 18 年为 YoungAdult 入口
    pub fn new(max_lifespan_years: f32, initial_age_years: f32) -> Self {
        Self {
            age_days: initial_age_years * 360.0,
            max_lifespan_days: max_lifespan_years * 360.0,
        }
    }

    /// 年龄占比 [0, 1+]——可超过 1.0（超出 max_lifespan 存活）
    pub fn age_ratio(&self) -> f32 {
        if self.max_lifespan_days <= 0.0 {
            return 0.5;
        }
        self.age_days / self.max_lifespan_days
    }
}

impl Default for Age {
    fn default() -> Self {
        // 默认人类尺度：70 年寿命，18 岁初始
        Self::new(70.0, 18.0)
    }
}

// ── GompertzMortality ────────────────

/// Gompertz 衰老死亡模型——追踪 NPC 的衰老死亡风险。
///
/// 参见: `开发阶段/NPC活人感模块/07-生命周期系统/007-死亡与死后.md` §二
///
/// # 种族无关设计
///
/// 所有计算基于 `age_pct = age_days / max_lifespan_days`——不同物种
/// 的绝对寿命差异自动归一化。长命种族（精灵 300 年）和短命种族
/// （人类 70 年）使用相同的 Gompertz 参数，因为年龄已经相对化。
/// 未来可通过 per-species `b`（GOMPERTZ_ALPHA）参数支持不同
/// 衰老加速度（如精灵衰老更平缓）。
///
/// # 字段
/// | 字段 | 说明 |
/// |------|------|
/// | base_risk | 基准风险（从 constitution + health_history 计算） |
/// | current_risk | 最近一次月度检查的计算风险 |
/// | last_check_age_days | 上次月度检查时的 age_days，用于判断是否到下一个检查点 |
/// | constitution | 体质 0.0-1.0，越低风险越高（1.5 - constitution 项） |
/// | health_history | 一生中重大伤病累积分 0.0+ |
///
/// # 内存
/// 20 bytes = 5 × f32
#[derive(Debug, Clone, Copy)]
pub struct GompertzMortality {
    pub base_risk: f32,
    pub current_risk: f32,
    pub last_check_age_days: f32,
    /// 体质 0.0-1.0。从 BigFive + 物种基线派生（当前从 seed 随机）
    pub constitution: f32,
    /// 一生中重大伤病累计次数。每次重病/重伤 +1.0
    pub health_history: f32,
}

impl Default for GompertzMortality {
    fn default() -> Self {
        Self {
            base_risk: 0.0,
            current_risk: 0.0,
            last_check_age_days: 0.0,
            constitution: 0.5, // 默认中等体质
            health_history: 0.0,
        }
    }
}

/// Gompertz 衰老生存概率——闭式积分。
///
/// 参见: `开发阶段/NPC活人感模块/07-生命周期系统/007-死亡与死后.md` §二
///
/// # 参数
/// - `age_pct`: 时间段**开始**时的年龄占比 (0.0-1.5)
/// - `delta_pct`: 此时间段内 age_pct 的增量
/// - `constitution`: 体质 (0.0-1.0)
/// - `health_history`: 一生中重大伤病累积分 (0.0+)
///
/// # 返回
/// 此时间段内存活的概率 (0.0-1.0)
///
/// # 公式
/// ```text
/// h(t) = a × exp(b × t)    — Gompertz 危险率
/// a = GOMPERTZ_BASE_A × (1.5 - constitution) × (1.0 + health_history)
/// b = 6.0 (GOMPERTZ_ALPHA)
/// S(t, t+dt) = exp(-∫a×exp(b×s) ds) = exp(-(a/b) × (exp(b×(t+dt)) - exp(b×t)))
/// ```
///
/// # 物种差异
/// 所有时间以 `age_pct`（占本物种 max_lifespan 比例）计——公式与
/// 绝对寿命无关。30 年寿命的生物在 21 岁（70%）和 300 年寿命的
/// 生物在 210 岁（70%）面对相同的 Gompertz 曲线。
pub fn senescence_survival(
    age_pct: f32,
    delta_pct: f32,
    constitution: f32,
    health_history: f32,
) -> f32 {
    // 70% 寿命前无衰老死亡风险
    if age_pct < 0.7 {
        return 1.0;
    }

    let t = age_pct - 0.7;
    let dt = delta_pct.min(1.5 - t.max(0.0));
    if dt <= 0.0 {
        return 1.0;
    }

    let a = GOMPERTZ_BASE_A * (1.5 - constitution) * (1.0 + health_history);
    let b = GOMPERTZ_ALPHA;

    // ∫ a×exp(b×s) ds = (a/b) × (exp(b×(t+dt)) - exp(b×t))
    let integral = (a / b) * ((b * (t + dt)).exp() - (b * t).exp());
    (-integral).exp().clamp(0.0, 1.0)
}

/// 每月天数（Gompertz 检查间隔）
pub const GOMPERTZ_CHECK_INTERVAL_DAYS: f32 = 30.0;

/// Gompertz 基线乘数——控制整条曲线的绝对高度。
///
/// 体质 0.5、无病史 → a = 50.0 → 70%寿命月死亡~5-6%，100%寿命~30%。
/// 未来可升级为 per-species 参数（精灵衰减更平缓 → 更小的值）。
pub const GOMPERTZ_BASE_A: f32 = 50.0;

/// Gompertz 衰老加速度——死亡率随年龄翻倍的速度。
///
/// b=6.0 为标准人类参数：t 每增加 ~0.12（约 8 人类年）死亡率翻倍。
/// 短命种族可用更大的 b（如地精 b=10），长命种族可用更小的 b（如精灵 b=3）。
pub const GOMPERTZ_ALPHA: f32 = 6.0;

// ── elder_decay_multiplier ────────────

/// 老年衰减乘数——MiddleAge 后 sigmoid 下降
///
/// 公式来源: `07-生命周期系统/006-老年与衰退.md` §elder_attribute_multiplier
/// age_pct < 0.85 → 1.0
/// age_pct >= 0.85 → sigmoid_decay × 0.9 + 0.1（floor at 0.1）
pub fn elder_decay_multiplier(age_pct: f32) -> f32 {
    if age_pct < 0.85 {
        return 1.0;
    }
    let x = (age_pct - 0.85) / 0.15;
    let sigmoid = 1.0 / (1.0 + (x * 3.0).exp());
    sigmoid * 0.9 + 0.1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_ratio_infant() {
        assert_eq!(LifeStage::from_age_ratio(0.01), LifeStage::Infant);
        assert_eq!(LifeStage::from_age_ratio(0.019), LifeStage::Infant);
    }

    #[test]
    fn test_from_ratio_juvenile() {
        assert_eq!(LifeStage::from_age_ratio(0.05), LifeStage::Juvenile);
    }

    #[test]
    fn test_from_ratio_adolescent() {
        assert_eq!(LifeStage::from_age_ratio(0.20), LifeStage::Adolescent);
    }

    #[test]
    fn test_from_ratio_young_adult() {
        assert_eq!(LifeStage::from_age_ratio(0.35), LifeStage::YoungAdult);
    }

    #[test]
    fn test_from_ratio_adult() {
        assert_eq!(LifeStage::from_age_ratio(0.60), LifeStage::Adult);
    }

    #[test]
    fn test_from_ratio_middle_age() {
        assert_eq!(LifeStage::from_age_ratio(0.85), LifeStage::MiddleAge);
    }

    #[test]
    fn test_from_ratio_elder() {
        assert_eq!(LifeStage::from_age_ratio(0.97), LifeStage::Elder);
        assert_eq!(LifeStage::from_age_ratio(1.2), LifeStage::Elder); // 超长寿
    }

    #[test]
    fn test_from_ratio_boundaries() {
        // 边界值测试
        assert_eq!(LifeStage::from_age_ratio(0.0), LifeStage::Infant);
        assert_eq!(LifeStage::from_age_ratio(0.02), LifeStage::Juvenile);
        assert_eq!(LifeStage::from_age_ratio(0.15), LifeStage::Adolescent);
        assert_eq!(LifeStage::from_age_ratio(0.25), LifeStage::YoungAdult);
        assert_eq!(LifeStage::from_age_ratio(0.45), LifeStage::Adult);
        assert_eq!(LifeStage::from_age_ratio(0.75), LifeStage::MiddleAge);
        assert_eq!(LifeStage::from_age_ratio(0.95), LifeStage::Elder);
    }

    #[test]
    fn test_age_ratio_calculation() {
        let a = Age::new(80.0, 40.0); // 80年寿命, 40岁
        assert!((a.age_ratio() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_age_default_is_young_adult() {
        let a = Age::default();
        let ratio = a.age_ratio();
        assert!(ratio > 0.25 && ratio < 0.45); // YoungAdult range
    }

    #[test]
    fn test_attribute_multiplier_infant() {
        assert!((LifeStage::Infant.attribute_multiplier() - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_attribute_multiplier_peak() {
        assert_eq!(LifeStage::YoungAdult.attribute_multiplier(), 1.0);
    }

    #[test]
    fn test_attribute_multiplier_elder_low() {
        assert!(LifeStage::Elder.attribute_multiplier() < 0.7);
    }

    #[test]
    fn test_attribute_multiplier_monotonic_rise_then_fall() {
        let stages = [
            LifeStage::Infant, LifeStage::Juvenile, LifeStage::Adolescent,
            LifeStage::YoungAdult, LifeStage::Adult, LifeStage::MiddleAge, LifeStage::Elder,
        ];
        let mults: Vec<f32> = stages.iter().map(|s| s.attribute_multiplier()).collect();
        // 上升阶段
        assert!(mults[0] < mults[1]);
        assert!(mults[1] < mults[2]);
        assert!(mults[2] < mults[3]); // peak at YoungAdult
        // 下降阶段
        assert!(mults[3] > mults[4]);
        assert!(mults[4] > mults[5]);
        assert!(mults[5] > mults[6]);
    }

    #[test]
    fn test_elder_multiplier_no_decay_before_middle_age() {
        assert!((elder_decay_multiplier(0.5) - 1.0).abs() < f32::EPSILON);
        assert!((elder_decay_multiplier(0.84) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_elder_multiplier_decays() {
        let m = elder_decay_multiplier(0.95);
        assert!(m < 0.6, "elder at 95% should have low multiplier, got {m}");
        assert!(m > 0.1, "should not go below floor");
    }

    #[test]
    fn test_elder_multiplier_floor() {
        let m = elder_decay_multiplier(1.5); // way past max
        assert!(m >= 0.1, "floor should be 0.1, got {m}");
    }

    #[test]
    fn test_elder_multiplier_monotonic() {
        let mut prev = 1.0;
        for i in 85..=120 {
            let pct = i as f32 / 100.0;
            let m = elder_decay_multiplier(pct);
            assert!(m <= prev, "not monotonic at {pct}: {m} > {prev}");
            prev = m;
        }
    }

    // ── Gompertz senescence_survival tests ──

    #[test]
    fn test_senescence_survival_below_70_pct() {
        // 70% 寿命前无衰老风险
        assert!(
            (senescence_survival(0.5, 0.01, 0.5, 0.0) - 1.0).abs() < f32::EPSILON
        );
        assert!(
            (senescence_survival(0.69, 0.01, 0.5, 0.0) - 1.0).abs() < f32::EPSILON
        );
    }

    #[test]
    fn test_senescence_survival_at_70_pct() {
        // 刚好 70%——风险初现，月存活率仍很高（~95%）
        let s = senescence_survival(0.70, 0.001, 0.5, 0.0);
        assert!(s > 0.90, "survival at 70% with small dt should be >0.90, got {s}");
    }

    #[test]
    fn test_senescence_survival_declines_with_age() {
        // 越老存活率越低
        let s_young = senescence_survival(0.75, 0.01, 0.5, 0.0);
        let s_old = senescence_survival(0.95, 0.01, 0.5, 0.0);
        assert!(s_young > s_old, "younger should have higher survival");
    }

    #[test]
    fn test_senescence_survival_at_max_lifespan() {
        // max 年龄——月度存活率 ~70%（对应 ~30% 月死亡率，~98% 年死亡率）
        let delta_monthly = 30.0 / (70.0 * 360.0); // ~0.00119
        let s = senescence_survival(1.0, delta_monthly, 0.5, 0.0);
        assert!(s < 0.80, "monthly survival at max lifespan should be <0.80, got {s}");
        assert!(s > 0.50, "should not be certain death in a single month, got {s}");
    }

    #[test]
    fn test_senescence_low_constitution_higher_risk() {
        // 低体质 → 更高死亡率
        let s_low_con = senescence_survival(0.85, 0.002, 0.2, 0.0);
        let s_high_con = senescence_survival(0.85, 0.002, 0.8, 0.0);
        assert!(s_low_con < s_high_con, "low constitution should mean lower survival");
    }

    #[test]
    fn test_senescence_health_history_increases_risk() {
        // 有病史 → 更高死亡率
        let s_clean = senescence_survival(0.85, 0.002, 0.5, 0.0);
        let s_burdened = senescence_survival(0.85, 0.002, 0.5, 2.0);
        assert!(s_burdened < s_clean, "health history should lower survival");
    }

    #[test]
    fn test_senescence_survival_zero_delta() {
        // 零时间增量 → 100% 存活
        assert!(
            (senescence_survival(0.85, 0.0, 0.5, 0.0) - 1.0).abs() < f32::EPSILON
        );
    }

    #[test]
    fn test_senescence_survival_clamped() {
        // 结果在 [0, 1] 范围内
        for age_pct in [0.7, 0.8, 0.9, 1.0, 1.2, 1.5] {
            let s = senescence_survival(age_pct, 0.01, 0.1, 5.0);
            assert!(s >= 0.0 && s <= 1.0, "survival {s} out of [0,1] at age_pct={age_pct}");
        }
    }

    #[test]
    fn test_senescence_survival_over_long_period() {
        // 长时间段 → 更低存活率
        let s_short = senescence_survival(0.85, 0.001, 0.5, 0.0);
        let s_long = senescence_survival(0.85, 0.01, 0.5, 0.0);
        assert!(s_long < s_short, "longer period should have lower survival");
    }

    #[test]
    fn test_gompertz_mortality_default() {
        let gm = GompertzMortality::default();
        assert_eq!(gm.base_risk, 0.0);
        assert_eq!(gm.current_risk, 0.0);
        assert_eq!(gm.last_check_age_days, 0.0);
        assert_eq!(gm.constitution, 0.5);
        assert_eq!(gm.health_history, 0.0);
    }

    /// 验证公式——age_pct=0.95, 月度检查, 中等体质。
    /// GOMPERTZ_BASE_A=50, b=6.0 → ~23% 月死亡率 → ~77% 存活率
    #[test]
    fn test_senescence_doc_example_095() {
        // 人类 70 年寿命, 30 天 = 30/(70*360) ≈ 0.00119 delta_pct
        let delta_monthly = 30.0 / (70.0 * 360.0);
        let s = senescence_survival(0.95, delta_monthly, 0.5, 0.0);
        // 23.5% 月死亡概率 → 76.5% 存活率
        let expected = 0.765;
        assert!(
            (s - expected).abs() < 0.01,
            "expected ~{expected} survival at 0.95, got {s}"
        );
    }
}
