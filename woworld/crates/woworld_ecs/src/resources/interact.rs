//! 交互上下文资源 — NearbyInteractables + ActionWheelData
//!
//! - `NearbyInteractables`：感官系统每帧填充控制角色附近的可交互目标。
//!   ⚠️ **Stub 资源**——感官与知觉系统未建，本 sprint 由测试/占位填充。
//!   感官系统建好后由其填充，`interact_context_system` 逻辑零改动即接线。
//! - `ActionWheelData`：Phase 0 填充 → Godot 读取渲染动作轮盘（004 §四）。
//!
//! 参见: `WoWorld-Design/.../角色控制器/004-ActionResolver与输入解析.md` §三/§四

use woworld_core::action::ActionId;
use woworld_core::interact::Interactable;
use woworld_core::types::EntityId;

/// 控制角色附近的可交互目标——感官系统填充。
#[derive(Debug, Clone, Default)]
pub struct NearbyInteractables {
    /// 候选目标（未过滤——`resolve_interact_target` 做范围/锥体/优先级仲裁）。
    pub candidates: Vec<Interactable>,
}

impl NearbyInteractables {
    /// 空——无附近可交互目标。
    pub fn new() -> Self {
        Self {
            candidates: Vec::new(),
        }
    }

    /// 清空（每帧感官系统重填前调用）。
    pub fn clear(&mut self) {
        self.candidates.clear();
    }
}

/// 动作轮盘单条目（004 §四）。
#[derive(Debug, Clone, PartialEq)]
pub struct WheelActionEntry {
    /// 触发的动作
    pub action_id: ActionId,
    /// 显示标签
    pub label: String,
    /// 目标实体
    pub target: Option<EntityId>,
    /// 不可用原因（"体力不足"/"需要稿子"）——None 表示可用
    pub disabled_reason: Option<String>,
}

/// 动作轮盘数据——Phase 0 填充，Godot 读取渲染。
///
/// 中键按住→弹轮盘→右摇杆选择→释放执行（004 §四 Elin 方案）。
#[derive(Debug, Clone, Default)]
pub struct ActionWheelData {
    /// 可选动作条目
    pub entries: Vec<WheelActionEntry>,
    /// 是否因歧义触发（vs 玩家主动按中键）
    pub has_ambiguity: bool,
    /// 是否显示（Godot 据此渲染/隐藏）
    pub visible: bool,
}

impl ActionWheelData {
    /// 空且隐藏。
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            has_ambiguity: false,
            visible: false,
        }
    }

    /// 隐藏并清空。
    pub fn hide(&mut self) {
        self.entries.clear();
        self.has_ambiguity = false;
        self.visible = false;
    }
}
