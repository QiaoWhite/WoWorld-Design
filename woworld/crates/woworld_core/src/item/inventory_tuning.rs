//! 库存调节常量
//!
//! 所有槽位数量和重量限制均可通过 TOML 调整。
//! 以下为设计默认值（来源: 005-背包与库存 §2.2）。
//!
//! Phase 3: 迁移为 TOML 数据文件加载。

/// [TUNING] 基础随身槽位数。
pub const BASE_SLOTS: u16 = 30;

/// [TUNING] 基础最大负重（kg）——力量乘以此系数。
pub const BASE_MAX_WEIGHT_KG: f32 = 10.0;

/// [TUNING] 种族负重差异系数——默认 1.0。
/// 龙 = 3.0，半身人 = 0.7。
pub const RACE_CARRY_MULTIPLIER_DEFAULT: f32 = 1.0;

// ── 背槽容器 ──────────────────────────────────────────

/// [TUNING] 小背包——额外槽位。
pub const SMALL_BACKPACK: u16 = 20;
/// [TUNING] 中背包——额外槽位。
pub const MEDIUM_BACKPACK: u16 = 35;
/// [TUNING] 大背包——额外槽位。
pub const LARGE_BACKPACK: u16 = 50;
/// [TUNING] 军用背囊——额外槽位。
pub const MILITARY_BACKPACK: u16 = 60;
/// [TUNING] 次元袋——额外槽位（但 weight_reduction = 0.5）。
pub const DIMENSIONAL_BAG: u16 = 30;

// ── 肩槽容器 ──────────────────────────────────────────

/// [TUNING] 肩挂包——额外槽位。
pub const SHOULDER_BAG: u16 = 15;
/// [TUNING] 信使包——额外槽位。
pub const MESSENGER_BAG: u16 = 25;

// ── 腰槽容器 ──────────────────────────────────────────

/// [TUNING] 腰包——额外槽位。
pub const WAIST_POUCH: u16 = 8;
/// [TUNING] 采集袋——额外槽位。
pub const GATHERING_BAG: u16 = 15;
/// [TUNING] 弹药包——额外槽位。
pub const AMMO_POUCH: u16 = 15;

// ── 手提容器 ──────────────────────────────────────────

/// [TUNING] 麻袋——额外槽位。
pub const SACK: u16 = 40;
/// [TUNING] 工具箱——额外槽位。
pub const TOOLBOX: u16 = 30;
/// [TUNING] 大搬运箱——额外槽位。
pub const LARGE_CARRY_BOX: u16 = 80;

// ── 极值 ──────────────────────────────────────────────

/// 满载 6 个容器时的理论最大槽位数:
/// BASE(30) + MILITARY_BACKPACK(60) + MESSENGER_BAG(25) + 2×GATHERING_BAG(15+15) + SACK(40) + TOOLBOX(30) = 215
pub const MAX_THEORETICAL_SLOTS: u16 =
    BASE_SLOTS + MILITARY_BACKPACK + MESSENGER_BAG + GATHERING_BAG + AMMO_POUCH + SACK + TOOLBOX;

// ── tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_theoretical_slots() {
        // 对齐设计 005 §2.3
        assert_eq!(MAX_THEORETICAL_SLOTS, 215);
    }

    #[test]
    fn test_all_positive() {
        assert!(BASE_SLOTS > 0);
        assert!(BASE_MAX_WEIGHT_KG > 0.0);
        assert!(SMALL_BACKPACK > 0);
        assert!(MEDIUM_BACKPACK > 0);
        assert!(LARGE_BACKPACK > 0);
        assert!(MILITARY_BACKPACK > 0);
        assert!(DIMENSIONAL_BAG > 0);
        assert!(SHOULDER_BAG > 0);
        assert!(MESSENGER_BAG > 0);
        assert!(WAIST_POUCH > 0);
        assert!(GATHERING_BAG > 0);
        assert!(AMMO_POUCH > 0);
        assert!(SACK > 0);
        assert!(TOOLBOX > 0);
        assert!(LARGE_CARRY_BOX > 0);
    }

    #[test]
    fn test_base_less_than_max() {
        assert!(BASE_SLOTS < MAX_THEORETICAL_SLOTS);
    }

    #[test]
    fn test_weight_constant_valid() {
        assert!(BASE_MAX_WEIGHT_KG > 0.0);
        assert!(RACE_CARRY_MULTIPLIER_DEFAULT > 0.0);
    }

    #[test]
    fn test_container_bonuses_non_overlapping_sum() {
        // 验证满载场景的计算公式: 30 + 60 + 25 + 15 + 15 + 40 + 30
        let max = BASE_SLOTS
            + MILITARY_BACKPACK
            + MESSENGER_BAG
            + GATHERING_BAG
            + AMMO_POUCH
            + SACK
            + TOOLBOX;
        assert_eq!(max, 215);
    }
}
