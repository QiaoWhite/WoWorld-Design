//! 通用组件装配框架
//!
//! 物品可以是单件（Simple）或多组件组合（Composite）。
//! Combat 和 Magic 各自注册 slot_type——物品系统只提供框架，不定义组件语义。
//!
//! Phase 2: Stub 类型。Phase 3: 完整注册系统 + ItemEntId 迁移。
//!
//! 参见: `WoWorld-Design/Happy Game/开发阶段/物品系统/001-物品系统总纲.md §三`
//! 参见: `WoWorld-Design/Happy Game/开发阶段/物品系统/003-物品属性与品质.md §四`

use std::collections::BTreeMap;

use glam::Vec3;

use crate::id::ItemDefId;

// ── SlotInstanceId ────────────────────────────────────

/// 装配体内——每个组件槽位有独立身份。
///
/// `SlotInstanceId(0..components.len())` —— 槽位索引。
/// 换了 occupant 后槽位身份不变——root ItemEntId 也不变。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SlotInstanceId(pub u16);

// ── ItemAssembly ──────────────────────────────────────

/// 物品可以是单件或组合件。
///
/// Phase 3: `root_ent_id` → `ItemEntId`。
#[derive(Debug, Clone)]
pub enum ItemAssembly {
    /// 单件物品（大多数物品——矿石/药水/书籍/货币）
    Simple {
        /// Phase 3: ItemEntId
        def: ItemDefId,
    },

    /// 多组件装配（武器/工具/复杂物品）
    Composite {
        /// 装配体根节点的身份（History 追踪此 ID）
        /// Phase 3: ItemEntId
        root_ent_id: ItemDefId,
        /// 子组件列表
        components: Vec<ComponentSlot>,
        /// 组件之间的连接方式
        connections: Vec<ComponentConnection>,
    },
}

// ── ComponentSlot ─────────────────────────────────────

/// 组件槽——装配体中的一个位置。
///
/// Phase 3: `occupant` → `ItemEntId`。
#[derive(Debug, Clone)]
pub struct ComponentSlot {
    /// 此槽位的身份（换了 occupant 后槽位身份不变）
    pub slot_id: SlotInstanceId,
    /// 组件类型标签（Combat/Magic 各自注册——物品系统不解析）
    pub slot_type: u16,
    /// 占用的组件（自身也是物品）
    /// Phase 3: ItemEntId
    pub occupant: Option<ItemDefId>,
    /// 连续参数值——由 ParamSchema 验证
    pub params: ItemParams,
    /// 指向 registered ParamSchema 的 ID
    pub param_schema_id: u16,
}

// ── ComponentConnection ───────────────────────────────

/// 组件之间的连接方式。
#[derive(Debug, Clone)]
pub struct ComponentConnection {
    /// 源槽位索引
    pub from_slot: usize,
    /// 目标槽位索引
    pub to_slot: usize,
    /// 关节类型
    pub joint: JointType,
}

// ── JointType ─────────────────────────────────────────

/// 装配连接方式——对应四种力学模型。
///
/// 设计 001 §3.1。
#[derive(Debug, Clone)]
pub enum JointType {
    /// 刚性——99% 的武器（柄+刃=一根棍子）
    Rigid,
    /// 铰链——双节棍、折叠弩
    Hinge {
        /// 旋转轴
        axis: Vec3,
        /// 角度限制（度）
        limits_deg: (f32, f32),
    },
    /// 链连接——连枷/锁镰/九节鞭
    Chain {
        /// 链节数
        links: u16,
        /// 单节长度（毫米）
        link_length_mm: f32,
    },
    /// 柔性——鞭子/绳镖
    Flexible {
        /// 刚度（0=完全柔软, 1=完全刚性）
        stiffness: f32,
        /// 粗细渐变（0=均匀, 1=从粗到细完全渐变）
        taper_ratio: f32,
    },
}

// ── ItemParams ────────────────────────────────────────

/// 连续参数运行时值——由 Combat/Magic 定义的 ParamSchema 验证。
///
/// 设计 003 §四。
#[derive(Debug, Clone, Default)]
pub struct ItemParams {
    /// 浮点参数——参数ID → 值
    pub floats: BTreeMap<u16, f32>,
    /// 整数参数——参数ID → 值
    pub ints: BTreeMap<u16, u32>,
}

// ── ParamSchema / ParamDef ────────────────────────────

/// 参数 schema——Combat/Magic 注册 slot_type 时提供。
///
/// 物品系统据此验证 ComponentSlot.params 的合法性。
#[derive(Debug, Clone)]
pub struct ParamSchema {
    pub params: Vec<ParamDef>,
}

/// 单个参数定义——名称/单位/范围/步进。
///
/// 设计 003 §四。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamDef {
    /// 参数 ID（在所属 ParamSchema 内唯一）
    pub id: u16,
    /// 参数名称（如 "长度"）
    pub name: &'static str,
    /// 单位（如 "m"）
    pub unit: &'static str,
    /// 最小值
    pub min: f32,
    /// 最大值
    pub max: f32,
    /// 默认值
    pub default: f32,
    /// 调节步进
    pub step: f32,
}

// ── tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── SlotInstanceId ─────────────────────────────────

    #[test]
    fn test_slot_instance_id_equality() {
        let a = SlotInstanceId(0);
        let b = SlotInstanceId(0);
        let c = SlotInstanceId(1);
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    // ── ItemAssembly ───────────────────────────────────

    #[test]
    fn test_simple_assembly() {
        let def = crate::id::ItemDefId(42);
        let a = ItemAssembly::Simple { def };
        match a {
            ItemAssembly::Simple { def: d } => assert_eq!(d, crate::id::ItemDefId(42)),
            _ => panic!("expected Simple"),
        }
    }

    #[test]
    fn test_composite_assembly() {
        let root = crate::id::ItemDefId(100);
        let components = vec![ComponentSlot {
            slot_id: SlotInstanceId(0),
            slot_type: 1,
            occupant: None,
            params: ItemParams::default(),
            param_schema_id: 0,
        }];
        let connections = vec![];
        let a = ItemAssembly::Composite {
            root_ent_id: root,
            components,
            connections,
        };
        match a {
            ItemAssembly::Composite { root_ent_id, .. } => {
                assert_eq!(root_ent_id, crate::id::ItemDefId(100));
            }
            _ => panic!("expected Composite"),
        }
    }

    // ── JointType ──────────────────────────────────────

    #[test]
    fn test_joint_rigid() {
        let j = JointType::Rigid;
        match j {
            JointType::Rigid => {}
            _ => panic!("expected Rigid"),
        }
    }

    #[test]
    fn test_joint_hinge() {
        let axis = Vec3::new(0.0, 1.0, 0.0);
        let j = JointType::Hinge {
            axis,
            limits_deg: (-90.0, 90.0),
        };
        match &j {
            JointType::Hinge {
                axis: a,
                limits_deg: (min, max),
            } => {
                assert!((a.x - 0.0).abs() < f32::EPSILON);
                assert!((a.y - 1.0).abs() < f32::EPSILON);
                assert!((*min + 90.0).abs() < f32::EPSILON);
                assert!((*max - 90.0).abs() < f32::EPSILON);
            }
            _ => panic!("expected Hinge"),
        }
    }

    #[test]
    fn test_joint_chain() {
        let j = JointType::Chain {
            links: 8,
            link_length_mm: 12.0,
        };
        match j {
            JointType::Chain {
                links,
                link_length_mm,
            } => {
                assert_eq!(links, 8);
                assert!((link_length_mm - 12.0).abs() < f32::EPSILON);
            }
            _ => panic!("expected Chain"),
        }
    }

    #[test]
    fn test_joint_flexible() {
        let j = JointType::Flexible {
            stiffness: 0.1,
            taper_ratio: 0.05,
        };
        match j {
            JointType::Flexible {
                stiffness,
                taper_ratio,
            } => {
                assert!((stiffness - 0.1).abs() < f32::EPSILON);
                assert!((taper_ratio - 0.05).abs() < f32::EPSILON);
            }
            _ => panic!("expected Flexible"),
        }
    }

    #[test]
    fn test_joint_values_roundtrip() {
        // Hinge
        let j = JointType::Hinge {
            axis: Vec3::new(1.0, 0.0, 0.0),
            limits_deg: (-180.0, 180.0),
        };
        match j {
            JointType::Hinge { axis, limits_deg } => {
                assert_eq!(axis, Vec3::new(1.0, 0.0, 0.0));
                assert_eq!(limits_deg, (-180.0, 180.0));
            }
            _ => panic!(),
        }
    }

    // ── ParamDef ───────────────────────────────────────

    #[test]
    fn test_param_def_bounds() {
        let pd = ParamDef {
            id: 1,
            name: "长度",
            unit: "m",
            min: 0.3,
            max: 2.0,
            default: 0.7,
            step: 0.01,
        };
        assert!(pd.min <= pd.default && pd.default <= pd.max);
        assert!(pd.step > 0.0);
    }

    // ── ItemParams ─────────────────────────────────────

    #[test]
    fn test_item_params_default_empty() {
        let p = ItemParams::default();
        assert!(p.floats.is_empty());
        assert!(p.ints.is_empty());
    }

    #[test]
    fn test_item_params_insert() {
        let mut p = ItemParams::default();
        p.floats.insert(1, 0.7);
        p.ints.insert(2, 42);
        assert_eq!(p.floats.get(&1), Some(&0.7));
        assert_eq!(p.ints.get(&2), Some(&42));
    }
}
