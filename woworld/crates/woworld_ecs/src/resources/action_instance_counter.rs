//! ActionInstanceCounter — 单调递增的动作实例 ID 生成器
//!
//! ActionController 在开始新动作时调用 `next()` 获取唯一追溯 ID。
//! GOAP/Memory/Animation 通过 ActionInstanceId 串联因果链。
//!
//! 参见: `WoWorld-Design/.../角色控制器/005-ActionOutcome与动作结果事件.md` §一

use serde::{Deserialize, Serialize};
use woworld_core::action::ActionInstanceId;

/// 动作实例计数器——每帧可递增多次。
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ActionInstanceCounter(pub u64);

impl ActionInstanceCounter {
    /// 新建——从 0 开始。
    pub fn new() -> Self {
        Self(0)
    }

    /// 获取下一个唯一 ID。
    ///
    /// 溢出时 wrap 到 0（u64 在合理游戏时长内不可能溢出）。
    pub fn next_id(&mut self) -> ActionInstanceId {
        let id = ActionInstanceId(self.0);
        self.0 = self.0.wrapping_add(1);
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_counter_starts_at_zero() {
        let mut c = ActionInstanceCounter::new();
        assert_eq!(c.next_id(), ActionInstanceId(0));
    }

    #[test]
    fn test_counter_monotonically_increases() {
        let mut c = ActionInstanceCounter::new();
        let a = c.next_id();
        let b = c.next_id();
        let c_val = c.next_id();
        assert!(a < b);
        assert!(b < c_val);
    }

    #[test]
    fn test_counter_new_is_default() {
        let c1 = ActionInstanceCounter::new();
        let c2 = ActionInstanceCounter::default();
        assert_eq!(c1.0, c2.0);
    }
}
