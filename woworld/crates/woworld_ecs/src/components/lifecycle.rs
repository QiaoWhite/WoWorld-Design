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
}
