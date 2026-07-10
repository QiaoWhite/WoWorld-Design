//! 玩家系统核心类型 — ControlMode + ActionDomain
//!
//! WoWorld 核心哲学：玩家 = NPC + I/O 适配层。
//! ControlMode 定义了人类输入如何覆盖 NPC 的 GOAP 决策引擎。
//!
//! 引擎无关——仅定义枚举，不依赖 Godot/GDExtension。
//!
//! 参见: `WoWorld-Design/Happy Game/开发阶段/玩家系统/001-玩家系统总纲.md`
//!       `CHG-063-玩家系统新建-20260624`

/// 玩家对 NPC 角色的控制模式
///
/// 设计 §1.1: Player ≠ 独立实体。Player = SapientMind + ControlMode。
///
/// Phase 1: Auto / Manual 二态。Phase 2: 增加 DomainDelegated。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ControlMode {
    /// 完全托管——GOAP 正常运转，和其他 NPC 一样。玩家旁观。
    #[default]
    Auto,
    /// 完全手操——GOAP 后台运转但不被执行。
    /// 玩家的键盘/鼠标输入直接驱动该角色的移动和行动。
    Manual,
}

impl ControlMode {
    /// 玩家是否**手动控制**该行为域——ActionResolver 域过滤（004 §五）。
    ///
    /// - `Auto` → 恒 `false`（GOAP 驱动一切，玩家旁观）
    /// - `Manual` → 恒 `true`（玩家接管全部 6 域）
    ///
    /// Phase 2 引入 `DomainDelegated` 后，此方法将按 per-域 bitmask 返回。
    pub fn controls_domain(self, _domain: ActionDomain) -> bool {
        match self {
            ControlMode::Auto => false,
            ControlMode::Manual => true,
        }
    }
}

/// 行为域——玩家可选择性地接管某些域，其余由 GOAP 自主
///
/// 用于未来的 `ControlMode::DomainDelegated { manual_domains }`。
/// Phase 1 仅定义枚举，Phase 2 接入 ControlMode。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActionDomain {
    /// 移动方向/速度、跳跃、攀爬、游泳
    Movement,
    /// 目标选择、攻击时机、风格切换、技能释放
    Combat,
    /// 对话选项选择、声量、语气
    Speech,
    /// 物品使用/装备切换/丢弃
    ItemUse,
    /// 门/箱子/机关/采集/制作
    Interaction,
    /// 法术选择与释放
    MagicUse,
}

/// ActionDomain → u8 bitmask 位置（Phase 2 DomainDelegated 用）
impl ActionDomain {
    /// 返回该域在 6-bit bitmask 中的位索引 (0-5)
    pub const fn bit(self) -> u8 {
        match self {
            Self::Movement => 0,
            Self::Combat => 1,
            Self::Speech => 2,
            Self::ItemUse => 3,
            Self::Interaction => 4,
            Self::MagicUse => 5,
        }
    }

    /// 从 bit 索引取回 ActionDomain
    pub const fn from_bit(bit: u8) -> Option<Self> {
        match bit {
            0 => Some(Self::Movement),
            1 => Some(Self::Combat),
            2 => Some(Self::Speech),
            3 => Some(Self::ItemUse),
            4 => Some(Self::Interaction),
            5 => Some(Self::MagicUse),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_control_mode_default_is_auto() {
        assert_eq!(ControlMode::default(), ControlMode::Auto);
    }

    #[test]
    fn test_control_mode_clone() {
        let m = ControlMode::Manual;
        assert_eq!(m, m.clone());
    }

    #[test]
    fn test_action_domain_bit_roundtrip() {
        let domains = [
            ActionDomain::Movement,
            ActionDomain::Combat,
            ActionDomain::Speech,
            ActionDomain::ItemUse,
            ActionDomain::Interaction,
            ActionDomain::MagicUse,
        ];
        for d in &domains {
            let bit = d.bit();
            let back = ActionDomain::from_bit(bit);
            assert_eq!(back, Some(*d), "roundtrip failed for {:?} (bit {})", d, bit);
        }
    }

    #[test]
    fn test_action_domain_bit_unique() {
        // 6 个域对应 6 个唯一的 bit 索引
        let mut bits = [false; 6];
        for d in &[
            ActionDomain::Movement,
            ActionDomain::Combat,
            ActionDomain::Speech,
            ActionDomain::ItemUse,
            ActionDomain::Interaction,
            ActionDomain::MagicUse,
        ] {
            let b = d.bit() as usize;
            assert!(!bits[b], "duplicate bit {} for {:?}", b, d);
            bits[b] = true;
        }
    }

    #[test]
    fn test_from_bit_out_of_range() {
        assert_eq!(ActionDomain::from_bit(6), None);
        assert_eq!(ActionDomain::from_bit(255), None);
    }

    #[test]
    fn test_control_mode_eq() {
        assert_eq!(ControlMode::Auto, ControlMode::Auto);
        assert_eq!(ControlMode::Manual, ControlMode::Manual);
        assert_ne!(ControlMode::Auto, ControlMode::Manual);
    }

    #[test]
    fn test_control_mode_controls_domain() {
        // Auto: 玩家不控任何域
        assert!(!ControlMode::Auto.controls_domain(ActionDomain::Combat));
        assert!(!ControlMode::Auto.controls_domain(ActionDomain::Movement));
        assert!(!ControlMode::Auto.controls_domain(ActionDomain::Interaction));
        // Manual: 玩家控全部域
        assert!(ControlMode::Manual.controls_domain(ActionDomain::Combat));
        assert!(ControlMode::Manual.controls_domain(ActionDomain::Movement));
        assert!(ControlMode::Manual.controls_domain(ActionDomain::MagicUse));
    }
}
