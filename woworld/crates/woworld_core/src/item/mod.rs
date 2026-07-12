//! 物品系统 — 核心类型与只读查询 trait
//!
//! 物品系统是 WoWorld 四大地基模块之首——定义一切可持有、可交换、可装备、
//! 可消耗物品的 ID 命名空间、属性规范和跨模块接口。
//!
//! ItemDefId 三字段布局: category(8bit) + sub_category(8bit) + def_index(48bit)
//! 参见: `WoWorld-Design/Happy Game/开发阶段/物品系统/`
//! 参见: [[CLAUDE-INTERFACES.md]]

use std::collections::BTreeMap;

use crate::id::{ItemDefId, SkillId};

// ── 子模块 ────────────────────────────────────────────

pub mod assembly;
pub mod equipment;
pub mod inventory;
pub mod inventory_tuning;

// ── 哨兵值 ──────────────────────────────────────────

/// 物品定义 ID 哨兵——表示"无物品"或"无效物品引用"。
///
/// 值为 u64::MAX，category=0xFF（Mod 保留空间，`from_u8` 返回 None）。
/// 所有 ItemQuery 实现遇到此 ID 时返回 None。
/// 匹配 CULTURE_ID_NONE / FAITH_ID_NONE 模式。
pub const ITEM_DEF_ID_NONE: ItemDefId = ItemDefId(u64::MAX);

// ── ItemCategory ────────────────────────────────────

/// 物品分类——全局恒定，`#[repr(u8)]` 编码。
///
/// 分类树分为 7 组、~44 个在用变体。
/// 0x70-0x7F 为核心保留，0x80-0xFF 为 Mod 命名空间。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum ItemCategory {
    // === 0x00-0x0F: 装备 ===
    Weapon = 0x00,
    Armor = 0x01,
    Accessory = 0x02,

    // === 0x10-0x1F: 消耗品 ===
    Consumable = 0x10,
    Potion = 0x11,
    Food = 0x12,
    Scroll = 0x13,
    Ammunition = 0x14,

    // === 0x20-0x2F: 材料 ===
    Material = 0x20,
    MineralOre = 0x21,
    Gemstone = 0x22,
    StoneMat = 0x23,
    WoodMat = 0x24,
    FiberMat = 0x25,
    LeatherMat = 0x26,
    LiquidMat = 0x27,
    OrganicMat = 0x28,
    MagicMat = 0x29,
    SoilMat = 0x2A,

    // === 0x30-0x3F: 工具 ===
    Tool = 0x30,
    Pickaxe = 0x31,
    Axe = 0x32,
    Shovel = 0x33,
    Hammer = 0x34,
    FishingRod = 0x35,
    Sickle = 0x36,
    CarvingKnife = 0x37,
    AlchemyKit = 0x38,

    // === 0x40-0x4F: 容器 ===
    Container = 0x40,
    Backpack = 0x41,
    Pouch = 0x42,
    HandCarry = 0x43,

    // === 0x50-0x5F: 特殊 ===
    Currency = 0x50,
    QuestItem = 0x51,
    Blueprint = 0x52,
    Book = 0x53,
    FurnitureItem = 0x54,
    KeyItem = 0x55,

    // === 0x60-0x6F: 魔法物品 ===
    MagicItem = 0x60,
    EnchantRune = 0x61,
    MagicImplement = 0x62,
    MagicConstruct = 0x63,
}

impl ItemCategory {
    /// 从 u8 还原——0x70-0xFF（保留/Mod 空间）返回 None。
    pub fn from_u8(v: u8) -> Option<Self> {
        Some(match v {
            0x00 => Self::Weapon,
            0x01 => Self::Armor,
            0x02 => Self::Accessory,
            0x10 => Self::Consumable,
            0x11 => Self::Potion,
            0x12 => Self::Food,
            0x13 => Self::Scroll,
            0x14 => Self::Ammunition,
            0x20 => Self::Material,
            0x21 => Self::MineralOre,
            0x22 => Self::Gemstone,
            0x23 => Self::StoneMat,
            0x24 => Self::WoodMat,
            0x25 => Self::FiberMat,
            0x26 => Self::LeatherMat,
            0x27 => Self::LiquidMat,
            0x28 => Self::OrganicMat,
            0x29 => Self::MagicMat,
            0x2A => Self::SoilMat,
            0x30 => Self::Tool,
            0x31 => Self::Pickaxe,
            0x32 => Self::Axe,
            0x33 => Self::Shovel,
            0x34 => Self::Hammer,
            0x35 => Self::FishingRod,
            0x36 => Self::Sickle,
            0x37 => Self::CarvingKnife,
            0x38 => Self::AlchemyKit,
            0x40 => Self::Container,
            0x41 => Self::Backpack,
            0x42 => Self::Pouch,
            0x43 => Self::HandCarry,
            0x50 => Self::Currency,
            0x51 => Self::QuestItem,
            0x52 => Self::Blueprint,
            0x53 => Self::Book,
            0x54 => Self::FurnitureItem,
            0x55 => Self::KeyItem,
            0x60 => Self::MagicItem,
            0x61 => Self::EnchantRune,
            0x62 => Self::MagicImplement,
            0x63 => Self::MagicConstruct,
            _ => return None,
        })
    }

    /// 返回顶级分组——用于粗略过滤。
    pub fn category_group(self) -> ItemCategoryGroup {
        match self {
            Self::Weapon | Self::Armor | Self::Accessory => ItemCategoryGroup::Equipment,
            Self::Consumable | Self::Potion | Self::Food | Self::Scroll | Self::Ammunition => {
                ItemCategoryGroup::Consumable
            }
            Self::Material
            | Self::MineralOre
            | Self::Gemstone
            | Self::StoneMat
            | Self::WoodMat
            | Self::FiberMat
            | Self::LeatherMat
            | Self::LiquidMat
            | Self::OrganicMat
            | Self::MagicMat
            | Self::SoilMat => ItemCategoryGroup::Material,
            Self::Tool
            | Self::Pickaxe
            | Self::Axe
            | Self::Shovel
            | Self::Hammer
            | Self::FishingRod
            | Self::Sickle
            | Self::CarvingKnife
            | Self::AlchemyKit => ItemCategoryGroup::Tool,
            Self::Container | Self::Backpack | Self::Pouch | Self::HandCarry => {
                ItemCategoryGroup::Container
            }
            Self::Currency
            | Self::QuestItem
            | Self::Blueprint
            | Self::Book
            | Self::FurnitureItem
            | Self::KeyItem => ItemCategoryGroup::Special,
            Self::MagicItem | Self::EnchantRune | Self::MagicImplement | Self::MagicConstruct => {
                ItemCategoryGroup::Magic
            }
        }
    }
}

/// 物品分类顶级分组（6 组）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemCategoryGroup {
    Equipment,
    Consumable,
    Material,
    Tool,
    Container,
    Special,
    Magic,
}

// ── Quality ──────────────────────────────────────────

/// 工艺品质——由制作技能决定，4 档。
///
/// 品质影响属性乘数（武器伤害/防具防护/耐久上限等），
/// 不同类别物品的乘数表不同。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Quality {
    /// 粗糙——学徒练手/仓促赶工
    Rough = 0,
    /// 标准——合格匠人的日常产出
    Standard = 1,
    /// 精良——熟练大师的满意作品
    Refined = 2,
    /// 完美——毕生巅峰，可能全世界仅数件
    Perfect = 3,
}

impl Quality {
    /// 耐久度乘数——跨类别共用的品质影响。
    ///
    /// Rough 0.6 / Standard 1.0 / Refined 1.3 / Perfect 1.8
    pub fn durability_multiplier(self) -> f32 {
        match self {
            Self::Rough => 0.6,
            Self::Standard => 1.0,
            Self::Refined => 1.3,
            Self::Perfect => 1.8,
        }
    }

    /// 通用属性乘数——武器伤害/防具防护等。
    ///
    /// Rough 0.7 / Standard 1.0 / Refined 1.2 / Perfect 1.5
    pub fn stat_multiplier(self) -> f32 {
        match self {
            Self::Rough => 0.7,
            Self::Standard => 1.0,
            Self::Refined => 1.2,
            Self::Perfect => 1.5,
        }
    }

    /// 触媒效率乘数——消耗品类（药水/卷轴）。
    ///
    /// Rough 0.3 / Standard 0.6 / Refined 0.825 / Perfect 0.975
    /// （取设计文档中给出的区间中点）
    pub fn consumable_multiplier(self) -> f32 {
        match self {
            Self::Rough => 0.3,
            Self::Standard => 0.6,
            Self::Refined => 0.825,
            Self::Perfect => 0.975,
        }
    }
}

// ── Rarity ───────────────────────────────────────────

/// 自然稀有度——由世界生成的地质分布决定，5 档。
///
/// 一块秘银矿石永远是 Rare——无论被谁开采、被谁锻造。
/// Rarity 不是制作出来的，是天然属性。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Rarity {
    /// 常见——地表随处可见（铁矿石、松木、砂岩）
    Common = 0,
    /// 罕见——特定群系/深度（银矿石、紫杉木）
    Uncommon = 1,
    /// 稀有——深度 >80m / 特殊条件（秘银、龙涎香）
    Rare = 2,
    /// 史诗——单一存档仅几处矿床（山铜、凤凰羽毛）
    Epic = 3,
    /// 传说——世界唯一或接近唯一（龙钢、世界树种子）
    Legendary = 4,
}

// ── ItemTag ──────────────────────────────────────────

/// 物品功能标签——Phase 1 最小集，后续 Phase 扩展。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ItemTag {
    /// 可堆叠（同 ItemDefId 的物品可合并）
    Stackable,
    /// 可食用/饮用
    Edible,
    /// 可作为燃料
    Fuel,
    /// 任务物品——不掉落/不交易/不消耗
    QuestItem,
    /// 双手物品
    TwoHanded,
}

// ── ItemProperties ──────────────────────────────────

/// 物品模板属性——每 ItemDefId 一份，全局恒定。
///
/// 所有物品共享的核心属性结构。各模块在此基础上叠加领域特定属性。
/// 复杂/跨模块字段以 Option 存在——Phase 1 中为 None，后续 Phase 填充。
#[derive(Debug, Clone, PartialEq)]
pub struct ItemProperties {
    // === 标识 ===
    pub def_id: ItemDefId,
    pub category: ItemCategory,
    /// 人类可读名称（Phase 1 使用 String；后续升级为 LocalizationKey）
    pub name: String,
    /// 人类可读描述（Phase 1 使用 String；后续升级为 LocalizationKey）
    pub description: String,

    // === 物理属性 ===
    /// 实际质量（克）
    pub weight_grams: u32,
    /// 体积折重系数（默认 1.0）——蓬松物品 >1.0，紧凑物品 <1.0
    pub bulk_factor: f32,
    /// 体积（升），用于容器容量计算
    pub volume_liters: f32,

    // === 品质与稀有度 ===
    /// "出厂默认"品质（世界生成用）
    pub base_quality: Quality,
    /// 自然稀缺度
    pub rarity: Rarity,
    /// 该物品可能的最低品质
    pub quality_range_min: Quality,
    /// 该物品可能的最高品质
    pub quality_range_max: Quality,

    // === 堆叠 ===
    /// 堆叠上限（1 = 不可堆叠）
    pub stack_size: u32,

    // === 经济 ===
    /// 基础价值（铜币）——Standard 品质下的参考价格
    pub base_value_copper: u32,

    // === 耐久 ===
    /// 最大耐久度（0 = 不可损坏，如货币/任务物品）
    pub max_durability: f32,
    /// 每次使用损耗
    pub durability_loss_per_use: f32,

    // === 元素 ===
    /// 魔力容量（刻）——0 = 非魔法物品
    pub magic_capacity_ke: u32,

    // === 标签 ===
    pub tags: Vec<ItemTag>,
    /// Mod 自定义标签
    pub mod_tags: BTreeMap<String, String>,

    // ── 以下字段 Phase 1 为 None / 默认值 ──
    /// 需求最低技能等级（Phase 2+：SkillId 类型就位后填充）
    pub min_skill: Option<(SkillId, f32)>,
    /// 需求最低力量（如重型武器）
    pub min_strength: Option<f32>,
    /// 需要特定身体部位（Phase 2+：AppendageLabel 类型就位后填充）
    pub required_body_part: Option<u8>, // placeholder: AppendageLabel discriminant

    /// 元素亲和（Phase 2+：Element 类型就位后填充）
    pub element_affinity: Option<u8>, // placeholder: Element discriminant

    /// ★ v1.0 家具与放置物品 — 放置属性
    /// None = 不可摆放（仅能作为掉落物或握持）
    pub placement: Option<ItemPlacementProps>,

    /// ★ v1.0 物品获取 — 工具功能标签
    /// 仅 Tool-category 物品填充此字段
    pub tool_tags: Option<Vec<String>>,

    /// ★ v1.0 基本需求系统 — 消耗品效果
    /// Life 模块定义 ConsumableEffect，物品模块只存储不解析
    pub consumable: Option<ConsumableEffect>,

    /// ★ v1.0 音频系统 — 装备声学材质
    /// 音频模块定义 AudioMaterial 枚举，物品模块只存储不解析
    pub audio_material: Option<u8>, // placeholder: AudioMaterial discriminant

    /// 审美属性（Phase 2+：AestheticProps 类型就位后填充）
    pub aesthetic_props: Option<AestheticProps>,
}

/// 有效负担——考虑体积折算后的"感觉重量"（kg）。
///
/// 公式: (质量_g / 1000) × 体积折重系数
pub fn effective_encumbrance_kg(props: &ItemProperties) -> f32 {
    (props.weight_grams as f32 / 1000.0) * props.bulk_factor
}

// ── 占位类型（Phase 1 stub，后续模块填充） ──────────

/// 放置属性占位——Phase 1 为空壳，家具模块就位后展开。
#[derive(Debug, Clone, PartialEq)]
pub struct ItemPlacementProps {
    /// 占位：是否为可放置物品（后续扩展为家具族参数等）
    pub is_placeable: bool,
}

/// ★ V3a: Phase 1 空壳展开——生命模块 004 §14.1 定义 schema。
///
/// 命名说明：设计使用 `satiation`（Vitals.hunger 0=饿→1=饱，吃后 +satiation）。
/// 代码方向相反（Needs.hunger 0=满足→1=缺乏），故使用 `hunger_restore` 表示"减少缺乏量"。
/// Phase 3 统一方向后重命名为 `satiation`。
#[derive(Debug, Clone, PartialEq)]
pub struct ConsumableEffect {
    /// 是否为可食用/可饮用物品
    pub is_consumable: bool,
    /// ★ V3a: 食用后 Needs.hunger 减少量 (0.0-1.0)
    pub hunger_restore: f32,
    /// ★ V3a: 食用后 Vitals.hp 恢复量
    pub hp_restore: f32,
}

/// 审美属性占位——Phase 1 为空壳，后续展开。
#[derive(Debug, Clone, PartialEq)]
pub struct AestheticProps {
    /// 占位：视觉品质等级（后续扩展为完整审美参数）
    pub visual_quality: f32,
}

// ── ItemDef ──────────────────────────────────────────

/// 物品定义——TOML 反序列化的中间结构，随后展开为 ItemProperties。
#[derive(Debug, Clone, PartialEq)]
pub struct ItemDef {
    pub def_id: ItemDefId,
    pub properties: ItemProperties,
}

// ── ItemState ────────────────────────────────────────

/// 物品实例可变状态——每个 ItemEntId 一份。
///
/// 耐久/品质/自定义名称/铭文。Phase 2+ 添加附魔列表/所有权链/连续参数/材料谱系。
#[derive(Debug, Clone, PartialEq)]
pub struct ItemState {
    /// 当前耐久度（0.0 = 毁坏，1.0 = 全新）
    pub durability: f32,
    /// 当前品质
    pub quality: Quality,
    /// 玩家/NPC 自定义名称（如"杀龙剑"）
    pub custom_name: Option<String>,
    /// 铭文
    pub inscription: Option<String>,
}

impl Default for ItemState {
    fn default() -> Self {
        Self {
            durability: 1.0,
            quality: Quality::Standard,
            custom_name: None,
            inscription: None,
        }
    }
}

// ── ItemStack ────────────────────────────────────────

/// 物品堆叠——(ItemDefId, 数量) 对。
///
/// 用于背包、地面掉落物堆、商店商品列表等。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemStack {
    pub def_id: ItemDefId,
    pub count: u32,
}

impl ItemStack {
    pub fn new(def_id: ItemDefId, count: u32) -> Self {
        Self { def_id, count }
    }
}

impl Default for ItemStack {
    fn default() -> Self {
        Self {
            def_id: ITEM_DEF_ID_NONE,
            count: 0,
        }
    }
}

// ── ItemQuery trait ─────────────────────────────────

/// 物品系统对所有消费模块的只读查询入口。
///
/// 消费模块（Economy/Combat/NPC/Life/History/WorldGen）通过此 trait
/// 查询物品属性，不直接访问 ItemRegistry 内部。
///
/// 所有查询方法对 `ITEM_DEF_ID_NONE` 返回 None/0/空。
pub trait ItemQuery: Send + Sync {
    /// 获取物品完整属性——O(1) HashMap 查询。
    fn get_properties(&self, id: ItemDefId) -> Option<&ItemProperties>;

    /// 获取物品分类。
    fn get_category(&self, id: ItemDefId) -> Option<ItemCategory>;

    /// 获取堆叠上限。
    fn get_stack_size(&self, id: ItemDefId) -> Option<u32>;

    /// 获取基础价值（铜币）。
    fn get_base_value(&self, id: ItemDefId) -> Option<u32>;

    /// 获取自然稀有度。
    fn get_rarity(&self, id: ItemDefId) -> Option<Rarity>;

    /// 获取名称。
    fn get_name(&self, id: ItemDefId) -> Option<&str>;

    /// 所有已注册 ItemDefId 的切片。
    fn all_def_ids(&self) -> &[ItemDefId];

    /// 已注册物品定义数量。
    fn def_count(&self) -> usize;
}

// ── tests ────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── ItemCategory ──────────────────────────────────

    #[test]
    fn test_category_discriminants_unique() {
        let mut seen = std::collections::HashSet::new();
        let variants: &[ItemCategory] = &[
            ItemCategory::Weapon,
            ItemCategory::Armor,
            ItemCategory::Accessory,
            ItemCategory::Consumable,
            ItemCategory::Potion,
            ItemCategory::Food,
            ItemCategory::Scroll,
            ItemCategory::Ammunition,
            ItemCategory::Material,
            ItemCategory::MineralOre,
            ItemCategory::Gemstone,
            ItemCategory::StoneMat,
            ItemCategory::WoodMat,
            ItemCategory::FiberMat,
            ItemCategory::LeatherMat,
            ItemCategory::LiquidMat,
            ItemCategory::OrganicMat,
            ItemCategory::MagicMat,
            ItemCategory::SoilMat,
            ItemCategory::Tool,
            ItemCategory::Pickaxe,
            ItemCategory::Axe,
            ItemCategory::Shovel,
            ItemCategory::Hammer,
            ItemCategory::FishingRod,
            ItemCategory::Sickle,
            ItemCategory::CarvingKnife,
            ItemCategory::AlchemyKit,
            ItemCategory::Container,
            ItemCategory::Backpack,
            ItemCategory::Pouch,
            ItemCategory::HandCarry,
            ItemCategory::Currency,
            ItemCategory::QuestItem,
            ItemCategory::Blueprint,
            ItemCategory::Book,
            ItemCategory::FurnitureItem,
            ItemCategory::KeyItem,
            ItemCategory::MagicItem,
            ItemCategory::EnchantRune,
            ItemCategory::MagicImplement,
            ItemCategory::MagicConstruct,
        ];
        for v in variants {
            let disc = *v as u8;
            assert!(
                seen.insert(disc),
                "duplicate discriminant: {disc:#04x} for {v:?}"
            );
        }
        assert_eq!(seen.len(), variants.len());
    }

    #[test]
    fn test_category_from_u8_roundtrip() {
        let variants: &[ItemCategory] = &[
            ItemCategory::Weapon,
            ItemCategory::Food,
            ItemCategory::MineralOre,
            ItemCategory::Pickaxe,
            ItemCategory::Backpack,
            ItemCategory::Currency,
            ItemCategory::MagicItem,
        ];
        for v in variants {
            let disc = *v as u8;
            let back = ItemCategory::from_u8(disc);
            assert_eq!(back, Some(*v), "roundtrip failed for {v:?} ({disc:#04x})");
        }
    }

    #[test]
    fn test_category_from_u8_reserved_returns_none() {
        assert_eq!(ItemCategory::from_u8(0x70), None);
        assert_eq!(ItemCategory::from_u8(0x7F), None);
        assert_eq!(ItemCategory::from_u8(0x80), None);
        assert_eq!(ItemCategory::from_u8(0xFF), None);
    }

    #[test]
    fn test_category_from_u8_unallocated_gap_returns_none() {
        // 0x03-0x0F: 装备组内未分配
        assert_eq!(ItemCategory::from_u8(0x03), None);
        // 0x15-0x1F: 消耗品组内未分配
        assert_eq!(ItemCategory::from_u8(0x15), None);
    }

    #[test]
    fn test_category_group() {
        assert_eq!(
            ItemCategory::Weapon.category_group(),
            ItemCategoryGroup::Equipment
        );
        assert_eq!(
            ItemCategory::Food.category_group(),
            ItemCategoryGroup::Consumable
        );
        assert_eq!(
            ItemCategory::MineralOre.category_group(),
            ItemCategoryGroup::Material
        );
        assert_eq!(
            ItemCategory::Pickaxe.category_group(),
            ItemCategoryGroup::Tool
        );
        assert_eq!(
            ItemCategory::Backpack.category_group(),
            ItemCategoryGroup::Container
        );
        assert_eq!(
            ItemCategory::Currency.category_group(),
            ItemCategoryGroup::Special
        );
        assert_eq!(
            ItemCategory::MagicImplement.category_group(),
            ItemCategoryGroup::Magic
        );
    }

    // ── Quality ───────────────────────────────────────

    #[test]
    fn test_quality_durability_multiplier() {
        assert!((Quality::Rough.durability_multiplier() - 0.6).abs() < f32::EPSILON);
        assert!((Quality::Standard.durability_multiplier() - 1.0).abs() < f32::EPSILON);
        assert!((Quality::Refined.durability_multiplier() - 1.3).abs() < f32::EPSILON);
        assert!((Quality::Perfect.durability_multiplier() - 1.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_quality_stat_multiplier() {
        assert!((Quality::Rough.stat_multiplier() - 0.7).abs() < f32::EPSILON);
        assert!((Quality::Standard.stat_multiplier() - 1.0).abs() < f32::EPSILON);
        assert!((Quality::Refined.stat_multiplier() - 1.2).abs() < f32::EPSILON);
        assert!((Quality::Perfect.stat_multiplier() - 1.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_quality_ordering() {
        assert!(Quality::Rough < Quality::Standard);
        assert!(Quality::Standard < Quality::Refined);
        assert!(Quality::Refined < Quality::Perfect);
    }

    // ── Rarity ────────────────────────────────────────

    #[test]
    fn test_rarity_ordering() {
        assert!(Rarity::Common < Rarity::Uncommon);
        assert!(Rarity::Uncommon < Rarity::Rare);
        assert!(Rarity::Rare < Rarity::Epic);
        assert!(Rarity::Epic < Rarity::Legendary);
    }

    // ── ItemState ─────────────────────────────────────

    #[test]
    fn test_item_state_default() {
        let st = ItemState::default();
        assert!((st.durability - 1.0).abs() < f32::EPSILON);
        assert_eq!(st.quality, Quality::Standard);
        assert!(st.custom_name.is_none());
        assert!(st.inscription.is_none());
    }

    // ── ItemStack ─────────────────────────────────────

    #[test]
    fn test_item_stack_new() {
        let stack = ItemStack::new(ItemDefId(42), 5);
        assert_eq!(stack.def_id, ItemDefId(42));
        assert_eq!(stack.count, 5);
    }

    #[test]
    fn test_item_stack_default_is_none() {
        let stack = ItemStack::default();
        assert_eq!(stack.def_id, ITEM_DEF_ID_NONE);
        assert_eq!(stack.count, 0);
    }

    // ── effective_encumbrance_kg ─────────────────────

    #[test]
    fn test_effective_encumbrance() {
        let props = ItemProperties {
            weight_grams: 1000,
            bulk_factor: 2.0,
            ..make_test_props()
        };
        let enc = effective_encumbrance_kg(&props);
        assert!((enc - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_effective_encumbrance_bulk_one() {
        let props = ItemProperties {
            weight_grams: 500,
            bulk_factor: 1.0,
            ..make_test_props()
        };
        let enc = effective_encumbrance_kg(&props);
        assert!((enc - 0.5).abs() < f32::EPSILON);
    }

    // ── helpers ───────────────────────────────────────

    fn make_test_props() -> ItemProperties {
        ItemProperties {
            def_id: ITEM_DEF_ID_NONE,
            category: ItemCategory::Material,
            name: String::new(),
            description: String::new(),
            weight_grams: 0,
            bulk_factor: 1.0,
            volume_liters: 0.0,
            base_quality: Quality::Standard,
            rarity: Rarity::Common,
            quality_range_min: Quality::Rough,
            quality_range_max: Quality::Perfect,
            stack_size: 1,
            base_value_copper: 0,
            max_durability: 0.0,
            durability_loss_per_use: 0.0,
            magic_capacity_ke: 0,
            tags: vec![],
            mod_tags: BTreeMap::new(),
            min_skill: None,
            min_strength: None,
            required_body_part: None,
            element_affinity: None,
            placement: None,
            tool_tags: None,
            consumable: None,
            audio_material: None,
            aesthetic_props: None,
        }
    }
}
