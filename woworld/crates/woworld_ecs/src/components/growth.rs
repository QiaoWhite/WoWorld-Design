//! GrowthNeeds 进阶心理需求 Component — ECS 铁律合规
//!
//! 参见: `开发文档/04-进阶需求系统/` + `进阶需求系统概览.md` §GrowthColumn
//!
//! esteem_deficit（尊重赤字）和 competence_frustration（胜任挫折）属于
//! 心理成长层——不放 Needs struct，独立为 GrowthColumn SoA。

/// 进阶心理需求——尊重 + 胜任
///
/// 设计规定: 不放入 Needs struct，独立 SoA 列族 (GrowthColumn)。
/// esteem: passive +0.01/天，通过社交恢复。
/// competence: 每游戏日按技能 aspiration gap 重算。
#[derive(Debug, Clone, Copy)]
pub struct GrowthNeeds {
    /// 尊重赤字 [0, 1]——0=被充分认可，1=无人知晓我的价值
    pub esteem_deficit: f32,
    /// 胜任挫折 [0, 1]——0=技能如预期增长，1=努力无回报
    pub competence_frustration: f32,
    /// 慢性挫折天数——连续 gap>0.3 的天数（触发慢性放大）
    pub chronic_days: u16,
}

impl Default for GrowthNeeds {
    fn default() -> Self {
        Self {
            esteem_deficit: 0.0,
            competence_frustration: 0.0,
            chronic_days: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_growth_needs_default() {
        let g = GrowthNeeds::default();
        assert!((g.esteem_deficit - 0.0).abs() < f32::EPSILON);
        assert!((g.competence_frustration - 0.0).abs() < f32::EPSILON);
        assert_eq!(g.chronic_days, 0);
    }

    #[test]
    fn test_growth_needs_in_range() {
        let g = GrowthNeeds {
            esteem_deficit: 0.5,
            competence_frustration: 0.3,
            chronic_days: 15,
        };
        assert!((0.0..=1.0).contains(&g.esteem_deficit));
        assert!((0.0..=1.0).contains(&g.competence_frustration));
    }
}
