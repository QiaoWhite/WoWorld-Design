//! 输入缓冲 ECS Component — CInputBuffer, CCoyoteTime
//!
//! CInputBuffer 仅在带 PlayerComponent 的实体上激活。
//! NPC 的 GOAP 产出 ActionRequest 时已考虑时机——不需要缓冲。
//!
//! 参见: `WoWorld-Design/.../角色控制器/008-手感系统.md` §二/§四

use glam::Vec3;
use smallvec::SmallVec;
use woworld_core::input::BufferedInput;

/// 输入缓冲——环形缓冲区，容量 4，6 级优先级淘汰。
///
/// 仅玩家实体激活（`With<PlayerComponent>`）。
#[derive(Debug, Clone)]
pub struct CInputBuffer {
    /// 缓冲条目（栈分配，SmallVec 容量 4）
    pub entries: SmallVec<[BufferedInput; 4]>,
    /// 上一帧的输入位集——用于检测"刚按下"vs"持续按住"
    pub prev_frame_inputs: u64,
}

impl Default for CInputBuffer {
    fn default() -> Self {
        Self {
            entries: SmallVec::new(),
            prev_frame_inputs: 0,
        }
    }
}

/// 土狼时间——"踩空后短暂时间仍可起跳"。
///
/// 触发: was_grounded → not_grounded 且非主动跳跃。
/// 消费: 在 remaining > 0 时按跳跃→接受。
/// 过期: 每帧减 dt，着地归零。
#[derive(Debug, Clone, Copy)]
pub struct CCoyoteTime {
    /// 剩余时间 (s)，> 0 表示仍在窗口内
    pub remaining: f32,
    /// 离地位置（用于边缘吸附判断）
    pub left_ground_at: Vec3,
}

impl Default for CCoyoteTime {
    fn default() -> Self {
        Self {
            remaining: 0.0,
            left_ground_at: Vec3::ZERO,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cinput_buffer_default_empty() {
        let buf = CInputBuffer::default();
        assert!(buf.entries.is_empty());
        assert_eq!(buf.prev_frame_inputs, 0);
    }

    #[test]
    fn test_ccoyote_time_default_expired() {
        let c = CCoyoteTime::default();
        assert_eq!(c.remaining, 0.0);
    }

    #[test]
    fn test_ccoyote_time_active() {
        let c = CCoyoteTime {
            remaining: 0.15,
            left_ground_at: Vec3::new(1.0, 0.0, 2.0),
        };
        assert!(c.remaining > 0.0);
        assert!((c.left_ground_at.x - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cinput_buffer_capacity() {
        let buf = CInputBuffer::default();
        assert!(buf.entries.capacity() >= 4);
    }
}
