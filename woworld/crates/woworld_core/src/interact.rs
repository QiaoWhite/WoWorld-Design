//! 交互上下文解析——`Interactable` + `interact_priority` + `resolve_interact_target`
//!
//! ActionResolver 第四层（004 §三）："交互"键上下文解析。感官系统填充候选
//! （`NearbyInteractables`），本模块做纯几何/优先级仲裁——引擎无关，易测。
//!
//! 歧义（第二候选距离 < 第一 ×1.5 且同优先级）→ 不自动选择 → 交给动作轮盘。
//!
//! 参见: `WoWorld-Design/.../角色控制器/004-ActionResolver与输入解析.md` §三/§四

use crate::action::ActionId;
use crate::types::EntityId;
use glam::Vec3;

// ── InteractKind ────────────────────────────────────────────────

/// 交互类型——决定优先级（004 §三 优先级表）。
///
/// 优先级为默认值；TOML 数据可覆盖（本 sprint 用内置默认）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InteractKind {
    /// 救起倒地同伴
    ReviveAlly,
    /// 战斗中拉杆
    CombatLever,
    /// 拾取稀有物品
    PickupRare,
    /// 战斗中搜尸
    LootInCombat,
    /// 门/容器/机关
    DoorContainerMechanism,
    /// 对话
    Talk,
    /// 制作站
    CraftingStation,
    /// 非战斗搜尸
    LootOutOfCombat,
    /// 采集节点
    HarvestNode,
    /// 普通物品
    CommonItem,
    /// 阅读标识
    ReadSign,
    /// 坐下/休息
    SitRest,
}

impl InteractKind {
    /// 默认交互优先级（004 §三，越高越优先）。
    pub fn priority(self) -> u8 {
        match self {
            InteractKind::ReviveAlly => 100,
            InteractKind::CombatLever => 80,
            InteractKind::PickupRare => 60,
            InteractKind::LootInCombat => 55,
            InteractKind::DoorContainerMechanism => 50,
            InteractKind::Talk => 40,
            InteractKind::CraftingStation => 35,
            InteractKind::LootOutOfCombat => 30,
            InteractKind::HarvestNode => 25,
            InteractKind::CommonItem => 20,
            InteractKind::ReadSign => 10,
            InteractKind::SitRest => 5,
        }
    }
}

// ── Interactable ────────────────────────────────────────────────

/// 一个可交互目标——感官系统填充。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interactable {
    /// 目标实体
    pub entity: EntityId,
    /// 交互类型（决定优先级）
    pub kind: InteractKind,
    /// 世界位置（用于距离/朝向锥判定）
    pub position: Vec3,
    /// 触发的动作 ID
    pub action_id: ActionId,
}

impl Interactable {
    /// 交互优先级快捷方法。
    pub fn priority(&self) -> u8 {
        self.kind.priority()
    }
}

// ── ResolvedInteract ────────────────────────────────────────────

/// 交互解析结果。
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedInteract {
    /// 无歧义——直接执行该目标。
    Chosen(Interactable),
    /// 有歧义——不自动选择，交给动作轮盘（候选已按优先级/距离排序）。
    Ambiguous(Vec<Interactable>),
    /// 范围/锥体内无候选。
    NoTarget,
}

/// 交互范围（m）——004 §三。
pub const INTERACT_RANGE: f32 = 2.0;
/// 前方交互锥半角余弦——120° 全角 → 半角 60°，cos(60°)=0.5。
pub const INTERACT_CONE_HALF_COS: f32 = 0.5;

/// 上下文解析——从候选中挑选最佳交互目标（004 §三）。
///
/// - `player_pos`：玩家世界位置
/// - `facing`：玩家朝向（水平，已归一化；零向量视为不作锥体过滤）
///
/// 过滤：范围 `INTERACT_RANGE` 内 + 前方 120° 锥体。
/// 排序：优先级降序 → 距离升序。
/// 歧义：第二候选与第一同优先级且距离 < 第一 ×1.5 → `Ambiguous`（→轮盘）。
pub fn resolve_interact_target(
    candidates: &[Interactable],
    player_pos: Vec3,
    facing: Vec3,
) -> ResolvedInteract {
    let facing_flat = Vec3::new(facing.x, 0.0, facing.z);
    let use_cone = facing_flat.length_squared() > 1e-6;
    let facing_n = facing_flat.normalize_or_zero();

    // ── 过滤：范围 + 朝向锥 ──
    let mut in_reach: Vec<(Interactable, f32)> = candidates
        .iter()
        .filter_map(|c| {
            let to = c.position - player_pos;
            let dist = to.length();
            if dist > INTERACT_RANGE {
                return None;
            }
            if use_cone && dist > 1e-6 {
                let to_flat = Vec3::new(to.x, 0.0, to.z).normalize_or_zero();
                if to_flat.dot(facing_n) < INTERACT_CONE_HALF_COS {
                    return None;
                }
            }
            Some((*c, dist))
        })
        .collect();

    if in_reach.is_empty() {
        return ResolvedInteract::NoTarget;
    }

    // ── 排序：优先级降序 → 距离升序 ──
    in_reach.sort_by(|a, b| {
        b.0.priority()
            .cmp(&a.0.priority())
            .then(a.1.total_cmp(&b.1))
    });

    let best = in_reach[0];
    // ── 歧义检测 ──
    if in_reach.len() > 1 {
        let second = in_reach[1];
        if second.0.priority() == best.0.priority() && second.1 < best.1 * 1.5 {
            return ResolvedInteract::Ambiguous(in_reach.into_iter().map(|(c, _)| c).collect());
        }
    }
    ResolvedInteract::Chosen(best.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(kind: InteractKind, pos: Vec3) -> Interactable {
        Interactable {
            entity: EntityId(1),
            kind,
            position: pos,
            action_id: ActionId(42),
        }
    }

    #[test]
    fn test_priority_table_ordering() {
        assert!(InteractKind::ReviveAlly.priority() > InteractKind::CombatLever.priority());
        assert!(InteractKind::DoorContainerMechanism.priority() > InteractKind::Talk.priority());
        assert!(InteractKind::CommonItem.priority() > InteractKind::SitRest.priority());
        assert_eq!(InteractKind::ReviveAlly.priority(), 100);
        assert_eq!(InteractKind::SitRest.priority(), 5);
    }

    #[test]
    fn test_no_candidates_is_no_target() {
        let r = resolve_interact_target(&[], Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0));
        assert_eq!(r, ResolvedInteract::NoTarget);
    }

    #[test]
    fn test_out_of_range_filtered() {
        let far = mk(InteractKind::Talk, Vec3::new(0.0, 0.0, -5.0)); // 5m > 2m
        let r = resolve_interact_target(&[far], Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0));
        assert_eq!(r, ResolvedInteract::NoTarget);
    }

    #[test]
    fn test_behind_player_filtered_by_cone() {
        // 目标在身后（+Z），朝向 -Z
        let behind = mk(InteractKind::Talk, Vec3::new(0.0, 0.0, 1.0));
        let r = resolve_interact_target(&[behind], Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0));
        assert_eq!(r, ResolvedInteract::NoTarget);
    }

    #[test]
    fn test_higher_priority_wins() {
        let door = mk(
            InteractKind::DoorContainerMechanism,
            Vec3::new(0.0, 0.0, -1.5),
        );
        let talk = mk(InteractKind::Talk, Vec3::new(0.0, 0.0, -1.0));
        let r = resolve_interact_target(&[talk, door], Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0));
        match r {
            ResolvedInteract::Chosen(c) => assert_eq!(c.kind, InteractKind::DoorContainerMechanism),
            _ => panic!("应选高优先级门，got {:?}", r),
        }
    }

    #[test]
    fn test_same_priority_close_is_ambiguous() {
        // 两个对话，距离接近（1.0 vs 1.2 < 1.0×1.5）→ 歧义
        let a = mk(InteractKind::Talk, Vec3::new(0.0, 0.0, -1.0));
        let b = mk(InteractKind::Talk, Vec3::new(0.2, 0.0, -1.18));
        let r = resolve_interact_target(&[a, b], Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0));
        assert!(matches!(r, ResolvedInteract::Ambiguous(_)));
    }

    #[test]
    fn test_same_priority_far_apart_chooses_nearest() {
        // 两个对话，近的 0.5m 远的 1.9m（1.9 > 0.5×1.5=0.75）→ 不歧义，选近的
        let near = mk(InteractKind::Talk, Vec3::new(0.0, 0.0, -0.5));
        let far = mk(InteractKind::Talk, Vec3::new(0.0, 0.0, -1.9));
        let r = resolve_interact_target(&[far, near], Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0));
        match r {
            ResolvedInteract::Chosen(c) => {
                assert!((c.position.z - (-0.5)).abs() < 1e-5)
            }
            _ => panic!("应选最近，got {:?}", r),
        }
    }
}
