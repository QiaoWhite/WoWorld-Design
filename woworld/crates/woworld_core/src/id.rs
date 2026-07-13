//! ID 新类型注册表
//!
//! 所有跨模块 ID 类型统一定义在此。遵循"ID 类型所有权"契约：
//! woworld_core 是唯一权威来源，各消费 crate 平等引用。
//!
//! 参见: `WoWorld-Design/开发路线图/002-轨A-正式开发.md` A.2
//! 参见: [[CLAUDE-INTERFACES.md]] CHG-014/CHG-015/CHG-061

use serde::{Deserialize, Serialize};
// ── 物品 ID ────────────────────────────────────────

/// 物品定义标识符（全局恒定）
///
/// 位布局: category(8bit) + sub_category(8bit) + def_index(48bit) = u64
///
/// category 0x00-0x7F: 核心游戏（128 类别）
/// category 0x80-0xFF: Mod 命名空间（128 类别）
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ItemDefId(pub u64);

impl ItemDefId {
    /// 从分类 + 子分类 + 实例索引构建 ItemDefId。
    ///
    /// `def_index` 仅使用低 48-bit——高位被静默截断。
    pub fn new(category: crate::item::ItemCategory, sub_category: u8, def_index: u64) -> Self {
        let cat = (category as u64) << 56;
        let sub = ((sub_category as u64) & 0xFF) << 48;
        ItemDefId(cat | sub | (def_index & 0x0000_FFFF_FFFF_FFFF))
    }

    /// 提取类别字节并还原为 ItemCategory。
    ///
    /// 0x70-0xFF（保留/Mod 空间）返回 None。
    pub fn category(&self) -> Option<crate::item::ItemCategory> {
        crate::item::ItemCategory::from_u8((self.0 >> 56) as u8)
    }

    /// 提取子类别字节（0-255）。
    pub fn sub_category(&self) -> u8 {
        ((self.0 >> 48) & 0xFF) as u8
    }

    /// 提取定义索引（低 48-bit）。
    pub fn def_index(&self) -> u64 {
        self.0 & 0x0000_FFFF_FFFF_FFFF
    }
}

/// 物品实体标识符（存档内唯一）
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ItemEntId(pub u64);

// ── 技能 ID ────────────────────────────────────────

/// 技能标识符（全局恒定）
///
/// 位布局: category(8bit) + sub_category(8bit) + group(16bit) + skill(32bit) = u64
/// 5 大分类: Combat/Magic/Artisan/Academic/Survival
/// 22 子组
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SkillId(pub u64);

// ── 职业标签 ID ────────────────────────────────────

/// 职业标签标识符（全局恒定）★ CHG-061
///
/// 位布局: category(8bit) + tag_id(24bit) = u32
/// ~80-100 个基础职业原子标签，TOML 数据驱动
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProfessionTagId(pub u32);

// ── 物种 ID ────────────────────────────────────────

/// 物种全局标识符（植物/动物/怪物/菌类共享 64-bit 空间）
///
/// 位布局: category(8bit) + def_id(56bit) = u64
/// 定义于 `woworld_core`——各消费 crate（建筑/NPC/经济/天气）平等引用。
///
/// 参见: [[CLAUDE-INTERFACES.md]] CHG-046
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SpeciesId(pub u64);

// ── 区块坐标 ───────────────────────────────────────

/// 区块坐标（世界生成的基本空间单元）
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChunkCoord {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}
