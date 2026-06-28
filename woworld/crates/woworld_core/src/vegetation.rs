//! VegetationProvider trait + 共享类型
//!
//! **Pattern D trait 反转**: trait 定义在 `woworld_core`（零依赖），
//! 实现由 `woworld_vegetation` crate 提供。
//! 消费者（建筑/NPC/经济/天气/音频/战斗）仅依赖此 trait，不依赖实现 crate。
//!
//! 参见: `WoWorld-Design/Happy Game/开发阶段/生命/010-植被群落与覆盖.md`
//! 参见: [[CLAUDE-INTERFACES.md]] CHG-046

use glam::{Vec2, Vec3};

use crate::id::SpeciesId;

// ── VegetationProvider trait ──────────────────────────────

/// 植被世界状态的统一查询接口
///
/// 消费方：建筑模块、NPC系统、经济系统、天气系统、音频系统、战斗系统、生命(动物)。
/// 与 `TerrainQuery` / `EntityIndex` / `SpatialEventBus` / `VisibilityQuery` 同层。
pub trait VegetationProvider: Send + Sync {
    /// 查询指定位置的植被群落快照（含物种组成+个体实例列表）
    fn query_community(&self, pos: Vec2, radius: f32) -> PlantCommunitySnapshot;

    /// 冠层郁闭度 0-1
    ///
    /// 消费方：天气(冠层截留)、战斗(掩体/隐身)、NPC(森林恐惧)、渲染(阴影)
    fn canopy_closure(&self, pos: Vec2) -> f32;

    /// 木材可获得性
    ///
    /// 消费方：建筑模块(BuildContext.materials)、经济系统(市场木材供给)
    fn timber_availability(&self, pos: Vec2) -> TimberAvailability;

    /// 地表覆盖（草/苔藓/落叶/裸土 四通道，和=1）
    ///
    /// 消费方：音频(脚步声选择)、NPC(移动速度修正)、渲染(地表纹理)
    fn ground_cover(&self, pos: Vec2) -> GroundCoverMap;

    /// 可采集物列表（T1药草/浆果/蘑菇等）
    ///
    /// 消费方：NPC采集行为(NPC每5s查询20m半径)
    fn query_harvestable(&self, pos: Vec2, radius: f32) -> Vec<HarvestableInfo>;

    /// 可燃物密度 0-1（火灾蔓延接口——预留）
    fn fuel_load(&self, pos: Vec2) -> f32;

    /// 根系深度影响因子 0-1（挖掘/采矿难度修正）
    fn root_interference(&self, pos: Vec3) -> f32;

    /// ★ CHG-049: 设置当前 scene_lod 上下文
    ///
    /// `LODCoordinator` 每帧派发 `scene_lod`，`VegetationProvider` 内部解释。
    fn set_scene_lod(&self, lod: u8);
}

// ── 共享类型 ────────────────────────────────────────────

/// 植被群落快照（从 `PlantCommunityTemplate` 确定性展开）
///
/// 含物种组成列表。个体实例通过 Poisson disc 展开，不存储。
#[derive(Debug, Clone)]
pub struct PlantCommunitySnapshot {
    /// 优势种及其覆盖权重 (SpeciesId, 0-1)
    pub dominant_species: Vec<(SpeciesId, f32)>,
    /// 伴生种
    pub companion_species: Vec<SpeciesId>,
    /// 冠层郁闭度 0-1
    pub canopy_closure: f32,
    /// Shannon 多样性指数
    pub shannon_diversity: f32,
}

/// 木材可获得性
#[derive(Debug, Clone)]
pub struct TimberAvailability {
    pub available: bool,
    pub quality: TimberQuality,
    /// 丰度 0-1
    pub abundance: f32,
    /// 采伐难度 0-1（坡度/密度/保护状态）
    pub harvest_difficulty: f32,
    /// 主要可用树种 IDs
    pub dominant_species: Vec<SpeciesId>,
}

/// 木材品质（从 PlantSpecies 形态学派生，非硬编码标签）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimberQuality {
    /// 松/云杉/冷杉——通用建材/造纸
    Softwood,
    /// 橡/山毛榉/枫——优质建材/家具
    Hardwood,
    /// 热带硬木——耐久/稀有
    TropicalHardwood,
    /// 巨木——船龙骨/纪念碑
    GiantWood,
    /// 魔法木——天然附魔亲和
    MagicWood,
}

/// 地表覆盖（4 通道，和 = 1.0）
#[derive(Debug, Clone, Copy)]
pub struct GroundCoverMap {
    /// 草密度
    pub grass_density: f32,
    /// 苔藓密度
    pub moss_density: f32,
    /// 落叶覆盖率
    pub leaf_litter: f32,
    /// 裸土
    pub bare_soil: f32,
}

impl Default for GroundCoverMap {
    fn default() -> Self {
        Self {
            grass_density: 0.5,
            moss_density: 0.0,
            leaf_litter: 0.0,
            bare_soil: 0.5,
        }
    }
}

/// 可采集物信息
#[derive(Debug, Clone)]
pub struct HarvestableInfo {
    /// 个体实例 ID（种子确定性派生）
    pub instance_id: u64,
    /// 物种 ID
    pub species_id: SpeciesId,
    /// 世界坐标
    pub position: Vec3,
    /// 产物类别
    pub product_category: ProductCategory,
    /// 基础产量
    pub yield_base: f32,
    /// 是否处于最佳采集季节
    pub season_optimal: bool,
}

/// 采集产物类别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductCategory {
    Herb,
    Berry,
    Mushroom,
    Nut,
    Fiber,
    Resin,
    Flower,
}

/// 再生状态
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RegenState {
    /// 未采集——完整可用
    Full,
    /// 部分采集——再生中
    Partial { days_until_full: f32 },
    /// 已被采集——等待下一季
    Depleted { season_regen: bool },
}

/// 地表覆盖类型（文化系统从 canopy_closure 派生）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandcoverType {
    /// 郁闭度 > 0.6 → 木结构偏好
    DenseForest,
    /// 郁闭度 0.2–0.6 → 混合材料
    OpenWoodland,
    /// 郁闭度 < 0.2 + 高草密度 → 土坯/帐篷
    Grassland,
    /// 几乎无植被 → 石/土坯
    Desert,
    /// 湿地植被 → 干栏式建筑偏好
    Wetland,
}
