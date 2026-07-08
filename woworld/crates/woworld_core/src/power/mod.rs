//! 权力系统 — 核心类型与只读查询 trait
//!
//! 权力是"控制关系物理引擎"——定义"谁可以对谁做什么、基于什么、
//! 被治者是否接受"。17 个普适原子适用于从家长-孩子到帝国法令的所有层级。
//!
//! Phase 1: 核心枚举 + PowerQuery trait + 合法性公式。
//! 延后: PowerTopology 图、Duty/Immunity、Polity 涌现、外交关系。
//!
//! 参见: `WoWorld-Design/Happy Game/开发阶段/权力系统/`
//! 参见: [[CLAUDE-INTERFACES.md]] CHG-023

use crate::types::EntityId;

// ── PowerAtom — 17 个普适权力原子 ──────────────────────

/// 权力原子类型 — 5 类别 17 变体
///
/// 规模不变：同一个原子代码路径覆盖从家规到帝国的所有层级。
/// 国王只是同时持有 10-14 个特定原子的人。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PowerAtom {
    // ── 结构类 (5) ──
    /// 创建新的权力实体/职位
    Constitute = 0,
    /// 定义成员资格
    DefineMembership = 1,
    /// 授权——将权力传递给下属
    Delegate = 2,
    /// 放弃——自愿交出权力
    Relinquish = 3,
    /// 契约——双边创建相互义务
    Contract = 4,

    // ── 自指类 (1) ──
    /// 誓言——自我约束承诺
    Pledge = 5,

    // ── 关系类 (6) ──
    /// 限制——约束行为范围
    Constrain = 6,
    /// 强制——要求执行行动
    Compel = 7,
    /// 抽取——提取资源（税收/劳役）
    Extract = 8,
    /// 访问——授予进入/使用权限
    Access = 9,
    /// 授衔——赋予地位/等级
    ConferRank = 10,
    /// 代表——代表他人行事
    Represent = 11,

    // ── 规范类 (2) ──
    /// 制定规则——创建行为规范
    PrescribeRule = 12,
    /// 废除——撤销规则/规范
    Derogate = 13,

    // ── 裁决类 (3) ──
    /// 裁决——解决争端
    Adjudicate = 14,
    /// 制裁——惩罚违规
    Sanction = 15,
    /// 豁免——免除义务/惩罚
    Remit = 16,
}

impl PowerAtom {
    pub const COUNT: usize = 17;

    /// 原子所属类别
    pub fn category(&self) -> AtomCategory {
        match self {
            PowerAtom::Constitute
            | PowerAtom::DefineMembership
            | PowerAtom::Delegate
            | PowerAtom::Relinquish
            | PowerAtom::Contract => AtomCategory::Structure,
            PowerAtom::Pledge => AtomCategory::SelfReferential,
            PowerAtom::Constrain
            | PowerAtom::Compel
            | PowerAtom::Extract
            | PowerAtom::Access
            | PowerAtom::ConferRank
            | PowerAtom::Represent => AtomCategory::Relational,
            PowerAtom::PrescribeRule | PowerAtom::Derogate => AtomCategory::Normative,
            PowerAtom::Adjudicate | PowerAtom::Sanction | PowerAtom::Remit => {
                AtomCategory::Adjudicative
            }
        }
    }
}

/// 权力原子类别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtomCategory {
    Structure,
    SelfReferential,
    Relational,
    Normative,
    Adjudicative,
}

// ── PowerSource — 8 条获取路径 ─────────────────────────

/// 权力来源——权力的获得方式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerSource {
    /// 继承——血统/家族传递
    Inherited,
    /// 任命——上级指定
    Appointed,
    /// 选举——群体投票
    Elected,
    /// 购买——金钱交易
    Purchased,
    /// 征服——武力夺取
    Conquered,
    /// 神授——宗教仪式授予
    Divine,
    /// 涌现——自发被认可
    Emergent,
    /// 契约——双方同意
    Contractual,
}

impl PowerSource {
    /// 初始合法性——按权力来源
    ///
    /// Inherited: 0.75, Appointed: 0.65, Elected: 0.70
    /// Purchased: 0.20, Conquered: 0.10, Divine: 0.80
    /// Emergent: 0.10, Contractual: 0.80
    pub fn initial_legitimacy(&self) -> f32 {
        match self {
            PowerSource::Inherited => 0.75,
            PowerSource::Appointed => 0.65,
            PowerSource::Elected => 0.70,
            PowerSource::Purchased => 0.20,
            PowerSource::Conquered => 0.10,
            PowerSource::Divine => 0.80,
            PowerSource::Emergent => 0.10,
            PowerSource::Contractual => 0.80,
        }
    }
}

// ── PowerDomain — 权力作用域 ───────────────────────────

/// 权力作用的领域/范围
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerDomain {
    /// 领土——地理范围
    Territory,
    /// 市场——经济范围
    Market,
    /// 行为——特定行为类型
    Behavior,
    /// 信息——信息控制
    Information,
    /// 身份——特定群体
    Identity,
    /// 普遍——无限制
    Universal,
}

// ── SuccessionRule ─────────────────────────────────────

/// 权力继承规则
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SuccessionRule {
    /// 指定继承人
    Designated,
    /// 长子继承
    Primogeniture,
    /// 选举继承
    ElectedBy,
    /// 回退上级
    RevertToSuperior,
    /// 随持有者消亡
    #[default]
    ExtinguishWithHolder,
    /// 未指定——触发 PowerEvent::SuccessionCrisis
    Unspecified,
}

// ── PowerEdge — 核心权力关系 ───────────────────────────

/// 权力边——"谁可以对谁做什么"的一条记录
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowerEdge {
    /// 权力持有者
    pub holder: EntityId,
    /// 权力承受者
    pub subject: EntityId,
    /// 权力原子类型
    pub atom: PowerAtom,
    /// 权力来源
    pub source: PowerSource,
    /// 作用域
    pub domain: PowerDomain,
    /// 合法性 [0,1]
    pub legitimacy: f32,
    /// 执行力度 [0,1]——0=有名无实, 1=绝对执行
    pub enforcement: f32,
    /// 建立时间 (tick)
    pub established_tick: u64,
    /// 有效期 (None=永久)
    pub valid_until_tick: Option<u64>,
    /// 最近行使时间
    pub last_exercised_tick: u64,
    /// 继承规则
    pub succession: SuccessionRule,
    /// 是否活跃
    pub active: bool,
}

impl Default for PowerEdge {
    fn default() -> Self {
        Self {
            holder: EntityId(0),
            subject: EntityId(0),
            atom: PowerAtom::Constrain,
            source: PowerSource::Emergent,
            domain: PowerDomain::Universal,
            legitimacy: 0.5,
            enforcement: 0.5,
            established_tick: 0,
            valid_until_tick: None,
            last_exercised_tick: 0,
            succession: SuccessionRule::default(),
            active: true,
        }
    }
}

// ── Legitimacy 计算 ────────────────────────────────────

/// 合法性计算参数
#[derive(Debug, Clone, Copy)]
pub struct LegitimacyParams {
    /// 权力来源的程序正当性 [0,1]
    pub procedural_basis: f32,
    /// 被治者的结果满意度 [0,1]
    pub outcome_satisfaction: f32,
    /// 文化权力距离匹配 [0,1]
    pub cultural_fit: f32,
    /// 权力存续年数
    pub age_years: f32,
    /// 距上次仪式的天数
    pub days_since_ritual: f32,
}

impl Default for LegitimacyParams {
    fn default() -> Self {
        Self {
            procedural_basis: 0.5,
            outcome_satisfaction: 0.5,
            cultural_fit: 0.5,
            age_years: 0.0,
            days_since_ritual: f32::MAX,
        }
    }
}

/// 计算合法性 — 5 因子加权公式
///
/// | 因子 | 权重 | 说明 |
/// |------|------|------|
/// | Procedural | 0.35 | 权力来源的正当性 |
/// | Outcome | 0.20 | 被治者获得的实际利益 |
/// | Cultural fit | 0.20 | 权力距离匹配度 |
/// | Time inertia | 0.15 | 持续越久越合法 |
/// | Ritual | 0.10 | 仪式加持 |
pub fn compute_legitimacy(params: &LegitimacyParams) -> f32 {
    let procedural = params.procedural_basis;
    let outcome = params.outcome_satisfaction;

    // Time inertia: age_years × 0.04, cap at 0.3
    let inertia = (params.age_years * 0.04).min(0.3);

    // Ritual boost
    let ritual = if params.days_since_ritual < 30.0 {
        0.15
    } else if params.days_since_ritual < 365.0 {
        0.08
    } else if params.days_since_ritual < 3650.0 {
        0.03
    } else {
        0.0
    };

    (procedural * 0.35
        + outcome * 0.20
        + params.cultural_fit * 0.20
        + inertia * 0.15
        + ritual * 0.10)
        .clamp(0.0, 1.0)
}

/// 计算合法性（便捷版——接受拆解参数）
pub fn compute_legitimacy_direct(
    procedural_basis: f32,
    outcome_satisfaction: f32,
    cultural_fit: f32,
    power_distance: f32,
    enforcement: f32,
    age_years: f32,
    days_since_ritual: f32,
) -> f32 {
    // cultural_fit 修正: 理想 power_distance 与 实际 enforcement 的差距
    let cultural = (1.0 - (enforcement - power_distance).abs()) * 0.8 + 0.2;
    let params = LegitimacyParams {
        procedural_basis,
        outcome_satisfaction,
        cultural_fit: (cultural_fit + cultural) * 0.5,
        age_years,
        days_since_ritual,
    };
    compute_legitimacy(&params)
}

/// 合法性危机阈值——低于此值触发革命检测
pub const LEGITIMACY_CRISIS_THRESHOLD: f32 = 0.05;

// ── PowerQuery trait ───────────────────────────────────

/// 权力只读查询接口
pub trait PowerQuery: Send + Sync {
    /// 查询实体的所有权出边（持有人视角）
    fn powers_of(&self, holder: EntityId) -> Vec<PowerEdge>;
    /// 查询实体的所有入边（被治者视角）
    fn constraints_on(&self, subject: EntityId) -> Vec<PowerEdge>;
    /// 查询特定原子类型的出边
    fn powers_by_atom(&self, holder: EntityId, atom: PowerAtom) -> Vec<PowerEdge>;
    /// 被治者对持有人的感知合法性
    fn perceived_legitimacy(&self, subject: EntityId, holder: EntityId) -> f32;
    /// 所有活跃边数
    fn edge_count(&self) -> usize;
}

// ── 测试 ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_atom_count() {
        assert_eq!(PowerAtom::COUNT, 17);
    }

    #[test]
    fn test_atom_category() {
        assert_eq!(PowerAtom::Constitute.category(), AtomCategory::Structure);
        assert_eq!(PowerAtom::Pledge.category(), AtomCategory::SelfReferential);
        assert_eq!(PowerAtom::Compel.category(), AtomCategory::Relational);
        assert_eq!(PowerAtom::PrescribeRule.category(), AtomCategory::Normative);
        assert_eq!(PowerAtom::Sanction.category(), AtomCategory::Adjudicative);
    }

    #[test]
    fn test_all_atoms_unique_discriminant() {
        let mut seen = [false; 17];
        let atoms = [
            PowerAtom::Constitute,
            PowerAtom::DefineMembership,
            PowerAtom::Delegate,
            PowerAtom::Relinquish,
            PowerAtom::Contract,
            PowerAtom::Pledge,
            PowerAtom::Constrain,
            PowerAtom::Compel,
            PowerAtom::Extract,
            PowerAtom::Access,
            PowerAtom::ConferRank,
            PowerAtom::Represent,
            PowerAtom::PrescribeRule,
            PowerAtom::Derogate,
            PowerAtom::Adjudicate,
            PowerAtom::Sanction,
            PowerAtom::Remit,
        ];
        for atom in atoms {
            let d = atom as u8;
            assert!(!seen[d as usize], "duplicate discriminant {d}");
            seen[d as usize] = true;
        }
    }

    #[test]
    fn test_initial_legitimacy_in_range() {
        let sources = [
            PowerSource::Inherited,
            PowerSource::Appointed,
            PowerSource::Elected,
            PowerSource::Purchased,
            PowerSource::Conquered,
            PowerSource::Divine,
            PowerSource::Emergent,
            PowerSource::Contractual,
        ];
        for s in sources {
            let l = s.initial_legitimacy();
            assert!((0.0..=1.0).contains(&l), "{s:?}: {l}");
        }
    }

    #[test]
    fn test_initial_legitimacy_ranking() {
        assert!(
            PowerSource::Divine.initial_legitimacy() > PowerSource::Purchased.initial_legitimacy()
        );
        assert!(
            PowerSource::Inherited.initial_legitimacy()
                > PowerSource::Conquered.initial_legitimacy()
        );
    }

    #[test]
    fn test_compute_legitimacy_all_midpoint() {
        let params = LegitimacyParams::default();
        let l = compute_legitimacy(&params);
        // 0.5*0.35 + 0.5*0.20 + 0.5*0.20 + 0*0.15 + 0*0.10 = 0.175 + 0.10 + 0.10 = 0.375
        assert!((l - 0.375).abs() < 0.02);
    }

    #[test]
    fn test_compute_legitimacy_high() {
        let params = LegitimacyParams {
            procedural_basis: 0.9,
            outcome_satisfaction: 0.8,
            cultural_fit: 0.8,
            age_years: 10.0,
            days_since_ritual: 15.0,
        };
        let l = compute_legitimacy(&params);
        assert!(l > 0.65, "high legitimacy expected, got {l}");
    }

    #[test]
    fn test_compute_legitimacy_low() {
        let params = LegitimacyParams {
            procedural_basis: 0.1,
            outcome_satisfaction: 0.2,
            cultural_fit: 0.1,
            age_years: 0.0,
            days_since_ritual: 5000.0,
        };
        let l = compute_legitimacy(&params);
        assert!(l < 0.2, "low legitimacy expected, got {l}");
    }

    #[test]
    fn test_compute_legitimacy_in_range() {
        for seed in 0..100 {
            let s = seed as f32 / 100.0;
            let params = LegitimacyParams {
                procedural_basis: s,
                outcome_satisfaction: 1.0 - s,
                cultural_fit: (s + 0.5) % 1.0,
                age_years: (seed % 30) as f32,
                days_since_ritual: (seed % 4000) as f32,
            };
            let l = compute_legitimacy(&params);
            assert!((0.0..=1.0).contains(&l), "seed {seed}: {l}");
        }
    }

    #[test]
    fn test_crisis_threshold() {
        assert!(LEGITIMACY_CRISIS_THRESHOLD < 0.1);
    }
}
