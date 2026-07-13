//! 玩家系统 ECS Component — PlayerComponent + ControlModeComponent
//!
//! PlayerComponent 标记哪个实体是"当前接收人类输入的玩家角色"。
//! ControlModeComponent 存储该实体的控制模式。
//!
//! 铁律合规: Component = 纯数据（零方法）。无堆数据内联。
//!
//! 参见: `WoWorld-Design/.../玩家系统/001-玩家系统总纲.md` §一
//!       `CHG-063-玩家系统新建-20260624`

use serde::{Deserialize, Serialize};
use woworld_core::player::ControlMode;

/// 玩家标记 Component
///
/// 挂载此 Component 的实体 = 当前活跃的玩家角色。
/// 同一时刻只有一个实体拥有此 Component。
///
/// Phase 1: character_name 记录被夺舍前的名字（用于退出时恢复）。
/// Phase 2: 扩展 character_id/uuid、active_since 等字段。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerComponent {
    /// 被夺舍前的 display_name（退出时恢复）
    pub original_name_override: Option<String>,
}

/// 控制模式 Component
///
/// 存储实体的当前 ControlMode。NPC 默认 Auto，被夺舍后为 Manual。
/// 与 PlayerComponent 配对使用——PlayerComponent 标记"谁是玩家"，
/// ControlModeComponent 描述"怎么控制"。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ControlModeComponent {
    pub mode: ControlMode,
}

impl Default for ControlModeComponent {
    fn default() -> Self {
        Self {
            mode: ControlMode::Auto,
        }
    }
}

impl ControlModeComponent {
    /// 快捷构造：Manual 模式
    pub fn manual() -> Self {
        Self {
            mode: ControlMode::Manual,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_component_default() {
        let pc = PlayerComponent::default();
        assert!(pc.original_name_override.is_none());
    }

    #[test]
    fn test_control_mode_component_default_is_auto() {
        let cm = ControlModeComponent::default();
        assert_eq!(cm.mode, ControlMode::Auto);
    }

    #[test]
    fn test_control_mode_component_manual() {
        let cm = ControlModeComponent::manual();
        assert_eq!(cm.mode, ControlMode::Manual);
    }

    #[test]
    fn test_player_component_clone() {
        let pc = PlayerComponent {
            original_name_override: Some("铁匠王五".into()),
        };
        let pc2 = pc.clone();
        assert_eq!(pc2.original_name_override.unwrap(), "铁匠王五");
    }

    #[test]
    fn test_control_mode_copy() {
        let cm = ControlModeComponent::manual();
        let cm2 = cm;
        assert_eq!(cm.mode, cm2.mode);
    }
}
