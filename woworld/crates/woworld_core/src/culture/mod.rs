//! 文化系统 — 核心类型与只读查询 trait
//!
//! CultureCoreParams 的 10 个原子参数是所有文化行为的"DNA"。
//! 所有推导类型（CommunicationNorms、BuildingStylePreferences 等）
//! 均从这 10 个参数确定性派生——零外部依赖。
//!
//! 参见: `WoWorld-Design/Happy Game/开发阶段/文化系统/`
//! 参见: [[CLAUDE-INTERFACES.md]] CHG-024

pub mod beauty;
pub mod building;
pub mod communication;
pub mod dietary;
pub mod fertility;
pub mod relationship;

// ── CultureId ──────────────────────────────────────────

/// 文化全局唯一标识符 — 扁平 u32，不编码谱系信息。
///
/// 谱系（父子、分化、融合）独立存储在 `CultureGenealogy` 中。
/// 4 bytes 使得 CultureId 可嵌入 NPC 记忆、物品 `era_hint`、事件参与者等高频路径。
///
/// 替代幽灵类型: CultureSeed、CultureStyleId、CultureClusterId
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub struct CultureId(pub u32);

/// 无文化归属哨兵值
pub const CULTURE_ID_NONE: CultureId = CultureId(u32::MAX);

// ── CultureCoreParams ──────────────────────────────────

/// 10 个不可分割的文化原子参数 (0.0-1.0)
///
/// 所有文化行为特征必须从这 10 个参数派生。新增核心参数被禁止，
/// 除非全量跨模块审计证明其不可派生。
///
/// **稳定性**: 世界中生成，以 sigma=0.003/年 的速度漂移。
/// 玩家生命周期内（~50 游戏年）变化 ≤0.02——在感知上近乎不变。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CultureCoreParams {
    /// 个人主义 (0=集体主义, 1=极端个人主义)
    pub individualism: f32,
    /// 权力距离 (0=平等, 1=严格等级)
    pub power_distance: f32,
    /// 不确定性规避 (0=容忍变化, 1=传统绑定)
    pub uncertainty_avoidance: f32,
    /// 竞争导向 (0=合作/关怀, 1=竞争/成就)
    pub competition_orientation: f32,
    /// 长期导向 (0=活在当下, 1=长期规划)
    pub long_term_orientation: f32,
    /// 放纵 (0=克制, 1=享乐追求)
    pub indulgence: f32,
    /// 对外来者开放度 (0=排外, 1=开放/好奇)
    pub openness_to_outsiders: f32,
    /// 宗教虔诚度 (0=世俗, 1=宗教渗透一切)
    pub religiosity: f32,
    /// 军国主义 (0=和平主义, 1=军事优先)
    pub militarism: f32,
    /// 艺术性 (0=功能优先, 1=美优先)
    pub artistry: f32,
}

impl Default for CultureCoreParams {
    fn default() -> Self {
        Self {
            individualism: 0.5,
            power_distance: 0.5,
            uncertainty_avoidance: 0.5,
            competition_orientation: 0.5,
            long_term_orientation: 0.5,
            indulgence: 0.5,
            openness_to_outsiders: 0.5,
            religiosity: 0.5,
            militarism: 0.5,
            artistry: 0.5,
        }
    }
}

impl CultureCoreParams {
    /// 按索引访问维度 (0..10)
    pub fn dim(&self, idx: usize) -> f32 {
        match idx {
            0 => self.individualism,
            1 => self.power_distance,
            2 => self.uncertainty_avoidance,
            3 => self.competition_orientation,
            4 => self.long_term_orientation,
            5 => self.indulgence,
            6 => self.openness_to_outsiders,
            7 => self.religiosity,
            8 => self.militarism,
            9 => self.artistry,
            _ => panic!("CultureCoreParams dim index out of range: {idx}"),
        }
    }

    /// 按索引设置维度
    pub fn set_dim(&mut self, idx: usize, v: f32) {
        let target = match idx {
            0 => &mut self.individualism,
            1 => &mut self.power_distance,
            2 => &mut self.uncertainty_avoidance,
            3 => &mut self.competition_orientation,
            4 => &mut self.long_term_orientation,
            5 => &mut self.indulgence,
            6 => &mut self.openness_to_outsiders,
            7 => &mut self.religiosity,
            8 => &mut self.militarism,
            9 => &mut self.artistry,
            _ => panic!("CultureCoreParams dim index out of range: {idx}"),
        };
        *target = v;
    }

    /// DIM_COUNT == 10
    pub const DIM_COUNT: usize = 10;

    /// 将所有维度裁剪至 [0, 1]
    pub fn clamped(mut self) -> Self {
        self.individualism = self.individualism.clamp(0.0, 1.0);
        self.power_distance = self.power_distance.clamp(0.0, 1.0);
        self.uncertainty_avoidance = self.uncertainty_avoidance.clamp(0.0, 1.0);
        self.competition_orientation = self.competition_orientation.clamp(0.0, 1.0);
        self.long_term_orientation = self.long_term_orientation.clamp(0.0, 1.0);
        self.indulgence = self.indulgence.clamp(0.0, 1.0);
        self.openness_to_outsiders = self.openness_to_outsiders.clamp(0.0, 1.0);
        self.religiosity = self.religiosity.clamp(0.0, 1.0);
        self.militarism = self.militarism.clamp(0.0, 1.0);
        self.artistry = self.artistry.clamp(0.0, 1.0);
        self
    }

    /// 从种子确定性生成 CultureCoreParams
    ///
    /// 步骤:
    /// 1. 10 个维度各取独立随机值
    /// 2. 极化: 随机选择 2-4 个维度推向极端 (0.65-0.85 或 0.15-0.35)
    /// 3. 裁剪至 [0, 1]
    pub fn from_seed(seed: u64) -> Self {
        let mut params = Self {
            individualism: culture_hash(seed, 0),
            power_distance: culture_hash(seed, 1),
            uncertainty_avoidance: culture_hash(seed, 2),
            competition_orientation: culture_hash(seed, 3),
            long_term_orientation: culture_hash(seed, 4),
            indulgence: culture_hash(seed, 5),
            openness_to_outsiders: culture_hash(seed, 6),
            religiosity: culture_hash(seed, 7),
            militarism: culture_hash(seed, 8),
            artistry: culture_hash(seed, 9),
        };
        params.polarize(seed);
        params.clamped()
    }

    /// 极化: 随机选择 2-4 个维度推向极端
    ///
    /// 设计文档 002 §2.3: 极化维度数 ∈ [2,4], 幅度 ∈ [0.15, 0.35].
    /// 这确保文化之间有足够的"形状"差异——不是全 0.5 的灰色 blob。
    fn polarize(&mut self, seed: u64) {
        let count_raw = culture_hash(seed, 10);
        let polarize_count = 2 + (count_raw * 3.0) as usize; // 2, 3, or 4

        // 随机选择要极化的维度（无放回）
        let mut indices: Vec<usize> = (0..Self::DIM_COUNT).collect();
        for i in (0..Self::DIM_COUNT).rev() {
            let j = (culture_hash(seed, 20 + i as u64) * (i + 1) as f32) as usize;
            indices.swap(i, j);
        }

        for &idx in indices.iter().take(polarize_count) {
            let current = self.dim(idx);
            let magnitude_raw = culture_hash(seed, 30 + idx as u64);
            let magnitude = 0.15 + magnitude_raw * 0.20; // [0.15, 0.35]
            let target = if current >= 0.5 {
                (current + magnitude).min(1.0)
            } else {
                (current - magnitude).max(0.0)
            };
            self.set_dim(idx, target);
        }
    }
}

// ── honor_weight ───────────────────────────────────────

/// 荣誉权重 — 从 CultureCoreParams 派生的纯函数
///
/// 决定文化中"荣誉"概念的重要性。消费者:
/// - 历史系统: 纪念碑传统权重 ×2.0
/// - NPC: 怨恨持续时间调节器
/// - 权力系统: 征服后合法性恢复
/// - 战斗系统: 投降视为耻辱
///
/// 公式 (设计文档 004 §6):
/// ```text
/// honor_weight = (1-individualism)×0.25 + power_distance×0.20
///              + uncertainty_avoidance×0.20 + competition×0.15
///              + militarism×0.10 + (1-openness)×0.10
/// ```
pub fn honor_weight(core: &CultureCoreParams) -> f32 {
    let raw = (1.0 - core.individualism) * 0.25
        + core.power_distance * 0.20
        + core.uncertainty_avoidance * 0.20
        + core.competition_orientation * 0.15
        + core.militarism * 0.10
        + (1.0 - core.openness_to_outsiders) * 0.10;
    raw.clamp(0.0, 1.0)
}

// ── CultureParam ───────────────────────────────────────

/// 文化参数枚举——标识 CultureCoreParams 的 10 个维度
///
/// 用于 CultureShift 事件指定修改哪个维度。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CultureParam {
    Individualism,
    PowerDistance,
    UncertaintyAvoidance,
    CompetitionOrientation,
    LongTermOrientation,
    Indulgence,
    OpennessToOutsiders,
    Religiosity,
    Militarism,
    Artistry,
}

// ── CultureName ────────────────────────────────────────

/// 文化名称——自名 + 外名
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CultureName {
    /// 自名（文化内部的名称）
    pub endonym: String,
    /// 自名含义
    pub endonym_meaning: String,
    /// 常见外名（其他文化怎么称呼）
    pub common_exonyms: std::collections::HashMap<CultureId, String>,
    /// 默认外名
    pub fallback_exonym: String,
}

// ── CultureGenealogy ───────────────────────────────────

/// 文化谱系——亲子/分化/融合/灭绝关系
///
/// 独立于 CultureId 存储——查询低频。
#[derive(Debug, Clone, Default)]
pub struct CultureGenealogy {
    /// 父文化 → 子文化列表
    pub children: std::collections::HashMap<CultureId, Vec<CultureId>>,
    /// 子文化 → 父文化列表
    pub parents: std::collections::HashMap<CultureId, Vec<CultureId>>,
    /// 分化来源: 子文化 → (父文化, 分化时间 tick)
    pub diverged_from: std::collections::HashMap<CultureId, (CultureId, u64)>,
    /// 融合来源: 子文化 → (父A, 父B, 融合时间 tick)
    pub fused_from: std::collections::HashMap<CultureId, (CultureId, CultureId, u64)>,
    /// 已灭绝文化
    pub extinct: std::collections::HashSet<CultureId>,
}

// ── CultureQuery trait ─────────────────────────────────

/// 文化只读查询接口 — 所有模块读取文化数据的唯一路径
///
/// 实现者: CultureRegistry (woworld_ecs)
/// 高频方法由 SoA 缓存 + 空间索引支持；零分配。
///
/// Phase 1: 无空间查询（culture_at 等 Phase 2 世界生成 P2.5 就位后加入）
pub trait CultureQuery: Send + Sync {
    /// 查询文化的核心参数
    fn core_params(&self, id: CultureId) -> Option<&CultureCoreParams>;

    /// 查询沟通规范
    fn communication_norms(&self, id: CultureId) -> Option<&communication::CommunicationNorms>;

    /// 查询敬语系统（从 CommunicationNorms 提取的便捷方法）
    fn honorific_system(&self, id: CultureId) -> Option<&communication::HonorificSystem> {
        self.communication_norms(id).map(|n| &n.honorifics)
    }

    /// 查询建筑风格偏好
    fn building_style(&self, id: CultureId) -> Option<&building::BuildingStylePreferences>;

    /// 查询审美标准
    fn beauty_standard(&self, id: CultureId) -> Option<&beauty::CulturalBeautyStandard>;

    /// 查询生育规范
    fn fertility_norms(&self, id: CultureId) -> Option<&fertility::FertilityNorms>;

    /// 查询饮食偏好
    fn dietary_preferences(&self, id: CultureId) -> Option<&dietary::DietaryBasePreferences>;

    /// 查询关系规范
    fn relationship_norms(&self, id: CultureId) -> Option<&relationship::RelationshipNorms>;

    /// 查询荣誉权重（便捷方法，委托给 honor_weight()）
    fn honor_weight(&self, id: CultureId) -> f32 {
        self.core_params(id).map(honor_weight).unwrap_or(0.5)
    }

    /// 列出所有已注册文化
    fn all_cultures(&self) -> &[CultureId];

    /// 已注册文化数量
    fn culture_count(&self) -> usize;
}

// ── 确定性 PRNG (splitmix64 变体) ──────────────────────
//
// 复制自 woworld_ecs::prng——woworld_core 零外部依赖。
// 仅用于 from_seed 和 polarize。

/// 确定性伪随机 f32 ∈ [0, 1)
///
/// splitmix64 变体。给定相同 (seed, salt) 始终返回相同值。
pub(crate) fn culture_hash(seed: u64, salt: u64) -> f32 {
    let mut x = seed
        .wrapping_add(salt.wrapping_mul(0x9E37_79B9_7F4A_7C15))
        .wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    (x >> 40) as f32 / (1u64 << 24) as f32
}

// ── 测试 ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── CultureId ──

    #[test]
    fn test_culture_id_none_is_max() {
        assert_eq!(CULTURE_ID_NONE.0, u32::MAX);
    }

    #[test]
    fn test_culture_id_equality() {
        assert_eq!(CultureId(0), CultureId(0));
        assert_ne!(CultureId(0), CultureId(1));
    }

    // ── CultureCoreParams::default ──

    #[test]
    fn test_default_midpoints() {
        let p = CultureCoreParams::default();
        for i in 0..CultureCoreParams::DIM_COUNT {
            assert!(
                (p.dim(i) - 0.5).abs() < f32::EPSILON,
                "dim {i} not at midpoint"
            );
        }
    }

    // ── dim / set_dim ──

    #[test]
    fn test_dim_set_dim_roundtrip() {
        let mut p = CultureCoreParams::default();
        p.set_dim(3, 0.77);
        assert!((p.dim(3) - 0.77).abs() < f32::EPSILON);
        assert_eq!(p.competition_orientation, 0.77);
    }

    #[test]
    #[should_panic]
    fn test_dim_out_of_range_panics() {
        CultureCoreParams::default().dim(10);
    }

    // ── clamped ──

    #[test]
    fn test_clamped_brings_into_range() {
        let p = CultureCoreParams {
            individualism: 1.5,
            power_distance: -0.3,
            ..CultureCoreParams::default()
        };
        let c = p.clamped();
        assert!((c.individualism - 1.0).abs() < f32::EPSILON);
        assert!((c.power_distance - 0.0).abs() < f32::EPSILON);
    }

    // ── from_seed ──

    #[test]
    fn test_from_seed_deterministic() {
        let a = CultureCoreParams::from_seed(42);
        let b = CultureCoreParams::from_seed(42);
        for i in 0..CultureCoreParams::DIM_COUNT {
            assert!(
                (a.dim(i) - b.dim(i)).abs() < f32::EPSILON,
                "dim {i}: {} vs {}",
                a.dim(i),
                b.dim(i)
            );
        }
    }

    #[test]
    fn test_from_seed_values_in_range() {
        for seed in 0..100 {
            let p = CultureCoreParams::from_seed(seed);
            for i in 0..CultureCoreParams::DIM_COUNT {
                let v = p.dim(i);
                assert!(
                    (0.0..=1.0).contains(&v),
                    "seed {seed} dim {i}: {v} not in [0,1]"
                );
            }
        }
    }

    #[test]
    fn test_from_seed_diverse() {
        let a = CultureCoreParams::from_seed(0);
        let b = CultureCoreParams::from_seed(1);
        let c = CultureCoreParams::from_seed(2);
        // 3 个种子至少产出 2 组不同参数
        let same_ab =
            (0..CultureCoreParams::DIM_COUNT).all(|i| (a.dim(i) - b.dim(i)).abs() < f32::EPSILON);
        let same_bc =
            (0..CultureCoreParams::DIM_COUNT).all(|i| (b.dim(i) - c.dim(i)).abs() < f32::EPSILON);
        assert!(
            !same_ab || !same_bc,
            "different seeds should produce diverse params"
        );
    }

    #[test]
    fn test_from_seed_has_polarization() {
        // 极化后至少 2 个维度应偏离中点 ≥ 0.1
        for seed in 0..100 {
            let p = CultureCoreParams::from_seed(seed);
            let deviant_count = (0..CultureCoreParams::DIM_COUNT)
                .filter(|&i| (p.dim(i) - 0.5).abs() >= 0.1)
                .count();
            assert!(
                deviant_count >= 2,
                "seed {seed}: only {deviant_count} deviant dims (need ≥2)"
            );
        }
    }

    // ── honor_weight ──

    #[test]
    fn test_honor_weight_all_midpoint() {
        let core = CultureCoreParams::default();
        let hw = honor_weight(&core);
        // 手动计算:
        // (1-0.5)*0.25 + 0.5*0.20 + 0.5*0.20 + 0.5*0.15 + 0.5*0.10 + (1-0.5)*0.10
        // = 0.125 + 0.10 + 0.10 + 0.075 + 0.05 + 0.05 = 0.50
        assert!((hw - 0.50).abs() < 0.01);
    }

    #[test]
    fn test_honor_weight_extreme_high() {
        // 高集体主义 + 高权力距离 + 高不确定性规避 + 高竞争 + 高军国主义 + 低开放性
        let core = CultureCoreParams {
            individualism: 0.0,
            power_distance: 1.0,
            uncertainty_avoidance: 1.0,
            competition_orientation: 1.0,
            militarism: 1.0,
            openness_to_outsiders: 0.0,
            ..CultureCoreParams::default()
        };
        let hw = honor_weight(&core);
        assert!(hw > 0.75, "expected high honor weight, got {hw}");
    }

    #[test]
    fn test_honor_weight_extreme_low() {
        let core = CultureCoreParams {
            individualism: 1.0,
            power_distance: 0.0,
            uncertainty_avoidance: 0.0,
            competition_orientation: 0.0,
            militarism: 0.0,
            openness_to_outsiders: 1.0,
            ..CultureCoreParams::default()
        };
        let hw = honor_weight(&core);
        assert!(hw < 0.25, "expected low honor weight, got {hw}");
    }

    #[test]
    fn test_honor_weight_in_range() {
        for seed in 0..100 {
            let core = CultureCoreParams::from_seed(seed);
            let hw = honor_weight(&core);
            assert!(
                (0.0..=1.0).contains(&hw),
                "seed {seed}: honor_weight {hw} not in [0,1]"
            );
        }
    }

    // ── culture_hash ──

    #[test]
    fn test_hash_deterministic() {
        let a = culture_hash(42, 7);
        let b = culture_hash(42, 7);
        assert!((a - b).abs() < f32::EPSILON);
    }

    #[test]
    fn test_hash_different_salt_different_output() {
        let a = culture_hash(42, 0);
        let b = culture_hash(42, 1);
        assert!((a - b).abs() > 0.001);
    }

    #[test]
    fn test_hash_in_range() {
        for seed in 0..100 {
            for salt in 0..15 {
                let v = culture_hash(seed, salt);
                assert!(
                    (0.0..=1.0).contains(&v),
                    "seed {seed} salt {salt}: {v} not in [0,1]"
                );
            }
        }
    }
}
