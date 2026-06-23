//! ID 新类型注册表
//!
//! 所有跨模块 ID 类型统一定义在此。遵循"ID 类型所有权"契约：
//! woworld_core 是唯一权威来源，各消费 crate 平等引用。
//!
//! 参见: `WoWorld-Design/开发路线图/002-轨A-正式开发.md` A.2
//! 参见: [[CLAUDE-INTERFACES.md]] CHG-014/CHG-015/CHG-061

// ── 物品 ID ────────────────────────────────────────

/// 物品定义标识符（全局恒定）
///
/// 位布局: category(8bit) + def_id(56bit) = u64
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ItemDefId(pub u64);

/// 物品实体标识符（存档内唯一）
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ItemEntId(pub u64);

// ── 技能 ID ────────────────────────────────────────

/// 技能标识符（全局恒定）
///
/// 位布局: category(8bit) + sub_category(8bit) + group(16bit) + skill(32bit) = u64
/// 5 大分类: Combat/Magic/Artisan/Academic/Survival
/// 22 子组
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct SkillId(pub u64);

// ── 职业标签 ID ────────────────────────────────────

/// 职业标签标识符（全局恒定）★ CHG-061
///
/// 位布局: category(8bit) + tag_id(24bit) = u32
/// ~80-100 个基础职业原子标签，TOML 数据驱动
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ProfessionTagId(pub u32);

// ── 区块坐标 ───────────────────────────────────────

/// 区块坐标（世界生成的基本空间单元）
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ChunkCoord {
    pub x: i64,
    pub y: i64,
    pub z: i64,
}
