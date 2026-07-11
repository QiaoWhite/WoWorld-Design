//! 动作相关 ECS Component — CActiveAction, CActionRequestBuf
//!
//! CActiveAction 是 ActionController 的状态存储（无内部状态——状态在 Component 中）。
//! CActionRequestBuf 是动作请求队列——多来源写入，ActionController 消费。
//!
//! 参见: `WoWorld-Design/.../角色控制器/003-ActionController与离散动作.md` §八

use smallvec::SmallVec;
use woworld_core::action::{ActionRequest, ActiveAction};

/// 当前执行的离散动作——ActionController 的状态存于此。
///
/// None = 空闲，可以接受新请求。
#[derive(Debug, Clone, Default)]
pub struct CActiveAction(pub Option<ActiveAction>);

/// 动作请求缓冲区——多来源（玩家/GOAP/本能/系统）写入。
///
/// 容量 4——正常帧不应超过此上限。ActionController 消费后清空。
#[derive(Debug, Clone)]
pub struct CActionRequestBuf(pub SmallVec<[ActionRequest; 4]>);

impl Default for CActionRequestBuf {
    fn default() -> Self {
        Self(SmallVec::new())
    }
}

/// 充能子动作的帧间载体——ActionController 释放充能动作时写入选定子动作请求，
/// 下一帧 `action_system` 将其注入 `CActionRequestBuf`（`source = ChargedAction`）后清空。
///
/// 一帧间隙——由 Recovery 帧或 id 帧覆盖，玩家无感（006 §五 帧间传递）。
/// `None` = 无待接续子动作。字段无堆分配（ActionRequest 全栈上）——铁律 #7 合规。
///
/// 参见: `WoWorld-Design/.../角色控制器/006-持续动作与充能动作.md` §五
#[derive(Debug, Clone, Default)]
pub struct CPendingFollowUp(pub Option<ActionRequest>);

#[cfg(test)]
mod tests {
    use super::*;
    use woworld_core::action::{ActionId, ActionParams, ActionRequest, ActionSource};

    #[test]
    fn test_cactive_action_default_none() {
        let c = CActiveAction::default();
        assert!(c.0.is_none());
    }

    #[test]
    fn test_caction_request_buf_push() {
        let mut buf = CActionRequestBuf::default();
        let req = ActionRequest {
            action_id: ActionId(1),
            priority: 15,
            source: ActionSource::Player,
            params: ActionParams::default(),
        };
        buf.0.push(req);
        assert_eq!(buf.0.len(), 1);
        assert_eq!(buf.0[0].action_id, ActionId(1));
    }

    #[test]
    fn test_caction_request_buf_clear() {
        let mut buf = CActionRequestBuf::default();
        buf.0.push(ActionRequest {
            action_id: ActionId(1),
            priority: 10,
            source: ActionSource::GOAP,
            params: ActionParams::default(),
        });
        buf.0.clear();
        assert!(buf.0.is_empty());
    }

    #[test]
    fn test_caction_request_buf_capacity() {
        let buf = CActionRequestBuf::default();
        assert!(buf.0.capacity() >= 4);
    }

    #[test]
    fn test_cpending_follow_up_default_none() {
        let f = CPendingFollowUp::default();
        assert!(f.0.is_none());
    }

    #[test]
    fn test_cpending_follow_up_holds_request() {
        let f = CPendingFollowUp(Some(ActionRequest {
            action_id: ActionId(7),
            priority: 5,
            source: ActionSource::ChargedAction,
            params: ActionParams::default(),
        }));
        assert_eq!(f.0.as_ref().unwrap().action_id, ActionId(7));
        assert_eq!(f.0.as_ref().unwrap().source, ActionSource::ChargedAction);
    }
}
