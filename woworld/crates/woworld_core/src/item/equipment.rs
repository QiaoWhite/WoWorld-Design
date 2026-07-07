//! 装备系统
//!
//! 双套 Outfit（战斗/常服，BG3 式 0 成本即时切换）。
//! BodyPlan 自动派生装备槽位——Phase 2 使用固定人形格式。
//! Phase 3: BodyPlan 派生 + SlotId 参数化。
//!
//! 参见: `WoWorld-Design/Happy Game/开发阶段/物品系统/004-装备系统.md`

use crate::id::ItemDefId;

// ── SlotId ────────────────────────────────────────────

/// 装备槽位标识。
///
/// Phase 2 固定人形格式。
/// Phase 3: BodyPlan 派生为 `Head(u8)` / `Leg(AppendageLabel, u8)` 等参数化变体。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SlotId {
    // === Outfit 槽位 ===
    /// 头盔（每头 1 槽）
    Head,
    /// 躯干甲
    Torso,
    /// ★ v1.2 — 围巾/斗篷/披风（躯干有悬挂结构则有）
    Shoulder,
    /// 腿甲
    Legs,
    /// 靴
    Feet,
    /// 主手——武器/盾/工具
    Mainhand,
    /// 副手——盾/副武器/工具
    Offhand,

    // === 饰品槽（不随 Outfit 模式切换）===
    /// 右手中指戒指
    RingRightMiddle,
    /// 右手无名指戒指
    RingRightRing,
    /// 左手中指戒指
    RingLeftMiddle,
    /// 左手无名指戒指
    RingLeftRing,
    /// 项链（≤1）
    Necklace,
    /// 手镯（最多 4）
    Bracelet(u8),
    /// 耳环（最多 2）
    Earring(u8),
    /// 头环
    Circlet,

    // === 便携容器槽（纯数据层，无 3D 渲染）===
    /// 背槽——背包/背囊/次元袋
    ContainerBack,
    /// 肩槽——挎包/肩挂包
    ContainerShoulder,
    /// 左腰槽——腰包/采集袋
    ContainerWaistLeft,
    /// 右腰槽——弹药包/药剂腰包
    ContainerWaistRight,
    /// 左手提——麻袋/水桶/工具箱
    ContainerHandLeft,
    /// 右手提——同上
    ContainerHandRight,
}

// ── OutfitMode ────────────────────────────────────────

/// 服饰模式——NPC 自主决定当前穿哪套。
///
/// 切换 0 成本（数据指针交换），切换逻辑在 NPC 模块。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum OutfitMode {
    /// 战甲——战斗/巡逻/护卫
    #[default]
    Combat = 0,
    /// 常服——日常/交易/社交/吃饭
    Civilian = 1,
    /// 睡衣（通常裸睡或简单内衣）
    Sleeping = 2,
    /// 礼服——节日/婚礼/葬礼
    Ceremonial = 3,
}

// ── OutfitSet ─────────────────────────────────────────

/// 一套服饰的完整配置。
///
/// Phase 3: 物品迁移为 ItemEntId；新增 `appendage_armor: BTreeMap<SlotId, Option<ItemDefId>>`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OutfitSet {
    /// Phase 3: ItemEntId
    pub head: Option<ItemDefId>,
    /// Phase 3: ItemEntId
    pub torso: Option<ItemDefId>,
    /// Phase 3: ItemEntId — ★ v1.2 肩部装备
    pub shoulder: Option<ItemDefId>,
    /// Phase 3: ItemEntId
    pub legs: Option<ItemDefId>,
    /// Phase 3: ItemEntId
    pub feet: Option<ItemDefId>,
    /// Phase 3: ItemEntId — 主手武器/盾/工具
    pub mainhand: Option<ItemDefId>,
    /// Phase 3: ItemEntId — 副手
    pub offhand: Option<ItemDefId>,
}

// ── AccessorySet ──────────────────────────────────────

/// 饰品集合——不随 Outfit 模式切换。
///
/// 戒指绑定到具体手指（设计 004 §二·3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AccessorySet {
    /// 右手中指戒指
    pub ring_right_middle: Option<ItemDefId>,
    /// 右手无名指戒指
    pub ring_right_ring: Option<ItemDefId>,
    /// 左手中指戒指
    pub ring_left_middle: Option<ItemDefId>,
    /// 左手无名指戒指
    pub ring_left_ring: Option<ItemDefId>,
    /// 项链（≤1）
    pub necklace: Option<ItemDefId>,
    /// 手镯（最多 4）
    pub bracelets: [Option<ItemDefId>; 4],
    /// 耳环（最多 2）
    pub earrings: [Option<ItemDefId>; 2],
    /// 头环
    pub circlet: Option<ItemDefId>,
}

// ── ContainerSet ──────────────────────────────────────

/// 便携容器槽——纯数据层，无 3D 实体渲染。
///
/// 设计 004 §三：容器槽位提供额外库存容量，但无角色模型上的视觉表示。
/// 手提槽战斗规则：非战斗→移速惩罚；进入战斗→惩罚消除；离开战斗 5s→恢复。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ContainerSet {
    /// 背槽 ×1——背包/背囊/次元袋
    pub back: Option<ItemDefId>,
    /// 肩槽 ×1——挎包/肩挂包
    pub shoulder: Option<ItemDefId>,
    /// 左腰槽 ×1——腰包/采集袋
    pub waist_left: Option<ItemDefId>,
    /// 右腰槽 ×1——弹药包/药剂腰包
    pub waist_right: Option<ItemDefId>,
    /// 左手提 ×1——麻袋/水桶/工具箱
    pub hand_left: Option<ItemDefId>,
    /// 右手提 ×1——同上
    pub hand_right: Option<ItemDefId>,
}

// ── EquipmentVisualToggles ────────────────────────────

/// 每槽位视觉显隐——纯客户端开关，不影响 gameplay。
///
/// ★ v1.2 新增。仅对玩家角色生效。NPC 始终全显示。
/// 设计意图：玩家想隐藏头盔/斗篷但保留属性。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EquipmentVisualToggles {
    pub head: bool,
    pub shoulder: bool,
    pub torso: bool,
    pub legs: bool,
    pub feet: bool,
    pub mainhand: bool,
    pub offhand: bool,
    /// 全部 4 枚戒指统一开关
    pub rings: bool,
    pub necklace: bool,
    pub bracelets: bool,
    pub earrings: bool,
    pub circlet: bool,
}

impl Default for EquipmentVisualToggles {
    fn default() -> Self {
        Self {
            head: true,
            shoulder: true,
            torso: true,
            legs: true,
            feet: true,
            mainhand: true,
            offhand: true,
            rings: true,
            necklace: true,
            bracelets: true,
            earrings: true,
            circlet: true,
        }
    }
}

// ── CharacterEquipment ────────────────────────────────

/// 角色完整装备状态。
///
/// 存储于 InventoryRegistry（非 ECS Component——含 Vec/非 Copy 字段）。
/// ECS 侧使用 `HasEquipment` ZST 标签标记。
#[derive(Debug, Clone, Default)]
pub struct CharacterEquipment {
    /// 战甲——战斗/巡逻/护卫时的穿着
    pub combat: OutfitSet,
    /// 常服——日常/交易/社交/吃饭时的穿着
    pub civilian: OutfitSet,
    /// 当前激活模式
    pub mode: OutfitMode,
    /// 饰品（不随模式切换）
    pub accessories: AccessorySet,
    /// 便携容器（纯数据层）
    pub containers: ContainerSet,
    /// 玩家专属——每槽位视觉开关
    pub visual_toggles: EquipmentVisualToggles,
}

impl CharacterEquipment {
    /// 根据指定模式获取对应 OutfitSet 的可变引用。
    pub fn outfit_mut(&mut self, mode: OutfitMode) -> &mut OutfitSet {
        match mode {
            OutfitMode::Combat => &mut self.combat,
            OutfitMode::Civilian => &mut self.civilian,
            OutfitMode::Sleeping | OutfitMode::Ceremonial => {
                // Sleeping 和 Ceremonial 共享 civilian 槽位（Phase 2 简化）
                // Phase 3: 扩展独立槽位
                &mut self.civilian
            }
        }
    }

    /// 根据指定模式获取对应 OutfitSet 的只读引用。
    pub fn outfit(&self, mode: OutfitMode) -> &OutfitSet {
        match mode {
            OutfitMode::Combat => &self.combat,
            OutfitMode::Civilian => &self.civilian,
            OutfitMode::Sleeping | OutfitMode::Ceremonial => &self.civilian,
        }
    }

    /// 获取当前激活的 OutfitSet。
    pub fn active_outfit(&self) -> &OutfitSet {
        self.outfit(self.mode)
    }

    /// 在指定槽位装备物品。
    ///
    /// 返回被替换的旧装备（如有）。
    /// 调用方负责：① 从库存消费物品 ② 将旧装备返回库存。
    /// 对齐设计 004 §四——装备物品同时也是库存物品。
    pub fn set_slot(&mut self, slot: SlotId, mode: OutfitMode, item: Option<ItemDefId>) -> Option<ItemDefId> {
        match slot {
            SlotId::Head => self.set_outfit_field(mode, |o| &mut o.head, item),
            SlotId::Torso => self.set_outfit_field(mode, |o| &mut o.torso, item),
            SlotId::Shoulder => self.set_outfit_field(mode, |o| &mut o.shoulder, item),
            SlotId::Legs => self.set_outfit_field(mode, |o| &mut o.legs, item),
            SlotId::Feet => self.set_outfit_field(mode, |o| &mut o.feet, item),
            SlotId::Mainhand => self.set_outfit_field(mode, |o| &mut o.mainhand, item),
            SlotId::Offhand => self.set_outfit_field(mode, |o| &mut o.offhand, item),
            SlotId::RingRightMiddle => Self::replace(&mut self.accessories.ring_right_middle, item),
            SlotId::RingRightRing => Self::replace(&mut self.accessories.ring_right_ring, item),
            SlotId::RingLeftMiddle => Self::replace(&mut self.accessories.ring_left_middle, item),
            SlotId::RingLeftRing => Self::replace(&mut self.accessories.ring_left_ring, item),
            SlotId::Necklace => Self::replace(&mut self.accessories.necklace, item),
            SlotId::Bracelet(i) => {
                let idx = i as usize;
                if idx < self.accessories.bracelets.len() {
                    Self::replace(&mut self.accessories.bracelets[idx], item)
                } else {
                    None
                }
            }
            SlotId::Earring(i) => {
                let idx = i as usize;
                if idx < self.accessories.earrings.len() {
                    Self::replace(&mut self.accessories.earrings[idx], item)
                } else {
                    None
                }
            }
            SlotId::Circlet => Self::replace(&mut self.accessories.circlet, item),
            SlotId::ContainerBack => Self::replace(&mut self.containers.back, item),
            SlotId::ContainerShoulder => Self::replace(&mut self.containers.shoulder, item),
            SlotId::ContainerWaistLeft => Self::replace(&mut self.containers.waist_left, item),
            SlotId::ContainerWaistRight => Self::replace(&mut self.containers.waist_right, item),
            SlotId::ContainerHandLeft => Self::replace(&mut self.containers.hand_left, item),
            SlotId::ContainerHandRight => Self::replace(&mut self.containers.hand_right, item),
        }
    }

    /// 读取指定槽位的当前装备。
    pub fn get_slot(&self, slot: SlotId, mode: OutfitMode) -> Option<ItemDefId> {
        match slot {
            SlotId::Head => self.outfit(mode).head,
            SlotId::Torso => self.outfit(mode).torso,
            SlotId::Shoulder => self.outfit(mode).shoulder,
            SlotId::Legs => self.outfit(mode).legs,
            SlotId::Feet => self.outfit(mode).feet,
            SlotId::Mainhand => self.outfit(mode).mainhand,
            SlotId::Offhand => self.outfit(mode).offhand,
            SlotId::RingRightMiddle => self.accessories.ring_right_middle,
            SlotId::RingRightRing => self.accessories.ring_right_ring,
            SlotId::RingLeftMiddle => self.accessories.ring_left_middle,
            SlotId::RingLeftRing => self.accessories.ring_left_ring,
            SlotId::Necklace => self.accessories.necklace,
            SlotId::Bracelet(i) => {
                let idx = i as usize;
                if idx < self.accessories.bracelets.len() {
                    self.accessories.bracelets[idx]
                } else {
                    None
                }
            }
            SlotId::Earring(i) => {
                let idx = i as usize;
                if idx < self.accessories.earrings.len() {
                    self.accessories.earrings[idx]
                } else {
                    None
                }
            }
            SlotId::Circlet => self.accessories.circlet,
            SlotId::ContainerBack => self.containers.back,
            SlotId::ContainerShoulder => self.containers.shoulder,
            SlotId::ContainerWaistLeft => self.containers.waist_left,
            SlotId::ContainerWaistRight => self.containers.waist_right,
            SlotId::ContainerHandLeft => self.containers.hand_left,
            SlotId::ContainerHandRight => self.containers.hand_right,
        }
    }

    /// 获取当前激活模式下的槽位装备。
    pub fn get_active_slot(&self, slot: SlotId) -> Option<ItemDefId> {
        self.get_slot(slot, self.mode)
    }

    // ── 私有 ──────────────────────────────────────────

    fn set_outfit_field<F>(
        &mut self,
        mode: OutfitMode,
        accessor: F,
        item: Option<ItemDefId>,
    ) -> Option<ItemDefId>
    where
        F: Fn(&mut OutfitSet) -> &mut Option<ItemDefId>,
    {
        let outfit = match mode {
            OutfitMode::Combat => &mut self.combat,
            OutfitMode::Civilian => &mut self.civilian,
            OutfitMode::Sleeping | OutfitMode::Ceremonial => &mut self.civilian,
        };
        let field = accessor(outfit);
        Self::replace(field, item)
    }

    fn replace(slot: &mut Option<ItemDefId>, item: Option<ItemDefId>) -> Option<ItemDefId> {
        let old = *slot;
        *slot = item;
        old
    }
}

// ── tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sword_id() -> ItemDefId {
        crate::id::ItemDefId(100)
    }

    fn shield_id() -> ItemDefId {
        crate::id::ItemDefId(200)
    }

    fn helmet_id() -> ItemDefId {
        crate::id::ItemDefId(300)
    }

    // ── OutfitSet ──────────────────────────────────────

    #[test]
    fn test_outfit_default_all_none() {
        let o = OutfitSet::default();
        assert!(o.head.is_none());
        assert!(o.torso.is_none());
        assert!(o.shoulder.is_none());
        assert!(o.legs.is_none());
        assert!(o.feet.is_none());
        assert!(o.mainhand.is_none());
        assert!(o.offhand.is_none());
    }

    #[test]
    fn test_outfit_copy() {
        let mut a = OutfitSet::default();
        a.head = Some(helmet_id());
        let b = a;
        assert_eq!(a.head, b.head);
    }

    // ── AccessorySet ───────────────────────────────────

    #[test]
    fn test_accessory_default_all_none() {
        let a = AccessorySet::default();
        assert!(a.ring_right_middle.is_none());
        assert!(a.ring_right_ring.is_none());
        assert!(a.ring_left_middle.is_none());
        assert!(a.ring_left_ring.is_none());
        assert!(a.necklace.is_none());
        assert!(a.bracelets.iter().all(|b| b.is_none()));
        assert!(a.earrings.iter().all(|e| e.is_none()));
        assert!(a.circlet.is_none());
    }

    // ── ContainerSet ───────────────────────────────────

    #[test]
    fn test_container_default_all_none() {
        let c = ContainerSet::default();
        assert!(c.back.is_none());
        assert!(c.shoulder.is_none());
        assert!(c.waist_left.is_none());
        assert!(c.waist_right.is_none());
        assert!(c.hand_left.is_none());
        assert!(c.hand_right.is_none());
    }

    #[test]
    fn test_container_copy() {
        let mut a = ContainerSet::default();
        a.back = Some(crate::id::ItemDefId(400));
        let b = a;
        assert_eq!(a.back, b.back);
    }

    // ── EquipmentVisualToggles ─────────────────────────

    #[test]
    fn test_visual_toggles_default_all_true() {
        let t = EquipmentVisualToggles::default();
        assert!(t.head);
        assert!(t.shoulder);
        assert!(t.torso);
        assert!(t.legs);
        assert!(t.feet);
        assert!(t.mainhand);
        assert!(t.offhand);
        assert!(t.rings);
        assert!(t.necklace);
        assert!(t.bracelets);
        assert!(t.earrings);
        assert!(t.circlet);
    }

    // ── OutfitMode ─────────────────────────────────────

    #[test]
    fn test_outfit_mode_discriminants_unique() {
        let variants = [
            OutfitMode::Combat,
            OutfitMode::Civilian,
            OutfitMode::Sleeping,
            OutfitMode::Ceremonial,
        ];
        let mut seen = std::collections::HashSet::new();
        for v in variants {
            assert!(seen.insert(v as u8));
        }
    }

    // ── CharacterEquipment ─────────────────────────────

    #[test]
    fn test_equipment_default() {
        let e = CharacterEquipment::default();
        assert_eq!(e.mode, OutfitMode::Combat);
        assert!(e.combat.mainhand.is_none());
        assert!(e.accessories.necklace.is_none());
    }

    #[test]
    fn test_set_slot_weapon() {
        let mut eq = CharacterEquipment::default();
        let old = eq.set_slot(SlotId::Mainhand, OutfitMode::Combat, Some(sword_id()));
        assert!(old.is_none());
        assert_eq!(eq.combat.mainhand, Some(sword_id()));
        // 常服不受影响
        assert!(eq.civilian.mainhand.is_none());
    }

    #[test]
    fn test_set_slot_swap_old() {
        let mut eq = CharacterEquipment::default();
        eq.combat.mainhand = Some(sword_id());
        let old = eq.set_slot(SlotId::Mainhand, OutfitMode::Combat, Some(shield_id()));
        assert_eq!(old, Some(sword_id()));
        assert_eq!(eq.combat.mainhand, Some(shield_id()));
    }

    #[test]
    fn test_set_slot_unequip() {
        let mut eq = CharacterEquipment::default();
        eq.combat.mainhand = Some(sword_id());
        let old = eq.set_slot(SlotId::Mainhand, OutfitMode::Combat, None);
        assert_eq!(old, Some(sword_id()));
        assert!(eq.combat.mainhand.is_none());
    }

    #[test]
    fn test_set_slot_accessory() {
        let mut eq = CharacterEquipment::default();
        let ring_id = crate::id::ItemDefId(500);
        let old = eq.set_slot(SlotId::RingRightMiddle, OutfitMode::Combat, Some(ring_id));
        assert!(old.is_none());
        assert_eq!(eq.accessories.ring_right_middle, Some(ring_id));
    }

    #[test]
    fn test_get_slot_current_mode() {
        let mut eq = CharacterEquipment::default();
        eq.combat.mainhand = Some(sword_id());
        eq.mode = OutfitMode::Combat;
        assert_eq!(eq.get_active_slot(SlotId::Mainhand), Some(sword_id()));
    }

    #[test]
    fn test_get_slot_different_mode() {
        let mut eq = CharacterEquipment::default();
        eq.combat.mainhand = Some(sword_id());
        // Civilian 模式下查 Mainhand → civilian Outfit，应该是 None
        assert!(eq.get_slot(SlotId::Mainhand, OutfitMode::Civilian).is_none());
    }
}
