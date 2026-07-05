//! ActionCategory + ActionIntent Component — ECS 铁律合规
//!
//! 参见: `开发文档/08-NPC行动涌现与分类/` + `NPC活人感开发文档ver2.0.md` §2.3
//!
//! Phase 1: 行为分类 + 意图。完整 ActionCandidate 注册表 Phase 3。

/// 行为类别——NPC 可执行的动作分类
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionCategory {
    Idle,
    Eat,
    Drink,
    Rest,
    SeekSafety,
    Socialize,
    Explore,
    Fight,
    Flee,
    Work,
    Wander,
}

/// 行为意图——action_weight_system 的产出
///
/// movement_system 消费此 Component 决定移动目标。
#[derive(Debug, Clone, Copy)]
pub struct ActionIntent {
    /// 当前选中的行为
    pub category: ActionCategory,
    /// 行为权重（越高越可能被选中）
    pub weight: f32,
}
