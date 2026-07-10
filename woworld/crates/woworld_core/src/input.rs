//! 输入缓冲类型 — BufferPriority + InputFeelConfig + BufferedInput
//!
//! 仅玩家实体使用。NPC 的 GOAP 产出 ActionRequest 时已考虑时机。
//!
//! 参见: `WoWorld-Design/.../角色控制器/008-手感系统.md`

use crate::action::ActionRequest;

// ── BufferPriority ──────────────────────────────────────────────

/// 输入缓冲优先级——满容量时低优先级条目被淘汰。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BufferPriority {
    /// 交互（最低优先级——可被任何条目淘汰）
    Interaction = 10,
    /// 战斗动作
    Combat = 30,
    /// 移动动作（跳跃）
    Movement = 40,
    /// 防御性动作（闪避/招架）
    Defensive = 60,
    /// 本能反应
    Instinct = 80,
    /// 系统紧急（最高优先级）
    Emergency = 100,
}

// ── InputFeelConfig ─────────────────────────────────────────────

/// 手感参数——TOML 数据驱动 `input_feel.toml`。
///
/// 所有参数值 Provisional——待实机测试调整。机制结构是最终设计。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct InputFeelConfig {
    /// 土狼时间窗口 (ms)
    pub coyote_time_ms: f32,
    /// 跳跃缓冲窗口 (ms)
    pub jump_buffer_ms: f32,
    /// 闪避缓冲窗口 (ms)
    pub dodge_buffer_ms: f32,
    /// 连招缓冲窗口 (ms)
    pub combo_buffer_ms: f32,
    /// 边缘吸附最大距离 (m)
    pub ledge_snap_distance: f32,
    /// 边缘吸附最大角度 (度)
    pub ledge_snap_angle: f32,
    /// 转身平滑时间 (s)
    pub turn_smooth_time: f32,
    /// 默认加速度 (m/s²)
    pub default_accel: f32,
    /// 默认减速度 (m/s²)
    pub default_decel: f32,
    /// 空中加速度乘数
    pub air_accel_multiplier: f32,
}

impl Default for InputFeelConfig {
    fn default() -> Self {
        Self {
            coyote_time_ms: 150.0,
            jump_buffer_ms: 200.0,
            dodge_buffer_ms: 200.0,
            combo_buffer_ms: 150.0,
            ledge_snap_distance: 0.3,
            ledge_snap_angle: 45.0,
            turn_smooth_time: 0.1,
            default_accel: 10.0,
            default_decel: 12.0,
            air_accel_multiplier: 0.3,
        }
    }
}

// ── BufferedInput ───────────────────────────────────────────────

/// 缓冲输入条目——存入 CInputBuffer 环形缓冲区。
#[derive(Debug, Clone, PartialEq)]
pub struct BufferedInput {
    /// 缓冲的动作请求
    pub action_request: ActionRequest,
    /// 按下时刻（游戏秒）
    pub pressed_at: f32,
    /// 过期时刻（游戏秒）
    pub expires_at: f32,
    /// 缓冲优先级——满容量时用于淘汰
    pub buffer_priority: BufferPriority,
}

impl BufferedInput {
    /// 新建缓冲条目。
    pub fn new(
        action_request: ActionRequest,
        pressed_at: f32,
        buffer_window_ms: f32,
        buffer_priority: BufferPriority,
    ) -> Self {
        Self {
            action_request,
            pressed_at,
            expires_at: pressed_at + buffer_window_ms / 1000.0,
            buffer_priority,
        }
    }

    /// 是否已过期。
    pub fn is_expired(&self, now: f32) -> bool {
        now >= self.expires_at
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ActionId, ActionParams, ActionSource};

    // ── BufferPriority ──

    #[test]
    fn test_buffer_priority_ordering() {
        assert!(BufferPriority::Interaction < BufferPriority::Combat);
        assert!(BufferPriority::Combat < BufferPriority::Movement);
        assert!(BufferPriority::Movement < BufferPriority::Defensive);
        assert!(BufferPriority::Defensive < BufferPriority::Instinct);
        assert!(BufferPriority::Instinct < BufferPriority::Emergency);
    }

    #[test]
    fn test_buffer_priority_discriminant() {
        assert_eq!(BufferPriority::Interaction as u8, 10);
        assert_eq!(BufferPriority::Emergency as u8, 100);
    }

    // ── InputFeelConfig ──

    #[test]
    fn test_input_feel_config_default() {
        let c = InputFeelConfig::default();
        assert!(c.coyote_time_ms > 0.0);
        assert!(c.jump_buffer_ms > 0.0);
        assert!(c.default_accel > 0.0);
        assert!(c.default_decel > 0.0);
        assert!(c.air_accel_multiplier > 0.0);
        assert!(c.air_accel_multiplier < 1.0);
    }

    #[test]
    fn test_input_feel_config_ledge_snap() {
        let c = InputFeelConfig::default();
        assert!(c.ledge_snap_distance > 0.0);
        assert!(c.ledge_snap_angle > 0.0);
        assert!(c.ledge_snap_angle < 90.0);
    }

    // ── BufferedInput ──

    #[test]
    fn test_buffered_input_expiration() {
        let req = ActionRequest {
            action_id: ActionId(1),
            priority: 20,
            source: ActionSource::Player,
            params: ActionParams::default(),
        };
        let bi = BufferedInput::new(req, 1.0, 200.0, BufferPriority::Movement);
        assert!(!bi.is_expired(1.0));
        assert!(!bi.is_expired(1.19));
        assert!(bi.is_expired(1.2)); // 200ms = 0.2s
        assert!(bi.is_expired(2.0));
    }

    #[test]
    fn test_buffered_input_fields() {
        let req = ActionRequest {
            action_id: ActionId(2),
            priority: 15,
            source: ActionSource::Player,
            params: ActionParams::default(),
        };
        let bi = BufferedInput::new(req.clone(), 5.0, 150.0, BufferPriority::Combat);
        assert_eq!(bi.action_request.action_id, ActionId(2));
        assert!((bi.pressed_at - 5.0).abs() < 0.001);
        assert!((bi.expires_at - 5.15).abs() < 0.001);
        assert_eq!(bi.buffer_priority, BufferPriority::Combat);
    }
}
