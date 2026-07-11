//! 输入缓冲 ECS Component — CInputBuffer, CCoyoteTime
//!
//! CInputBuffer 仅在带 PlayerComponent 的实体上激活。
//! NPC 的 GOAP 产出 ActionRequest 时已考虑时机——不需要缓冲。
//!
//! 参见: `WoWorld-Design/.../角色控制器/008-手感系统.md` §二/§四

use glam::Vec3;
use smallvec::SmallVec;
use woworld_core::action::ActionId;
use woworld_core::input::{BufferedInput, HeldItemKind};

/// 手持物——ActionResolver 第二层装备映射的数据源（004 §二）。
///
/// ⚠️ **Stub 组件**：装备系统未接线前，实体可不挂此组件——resolver
/// 缺省视为 `HeldItemKind::Empty`（空手）。装备系统建好后由其填充/维护，
/// resolver 逻辑零改动即接线。
#[derive(Debug, Clone, Copy, Default)]
pub struct CHeldItem(pub HeldItemKind);

/// 装备的特殊技能槽——ActionResolver 第五层（004 §二 第五层：Q/E/R/F → SpecialSkill）。
///
/// `slots[n]` = `SpecialSkill(n)` 键触发的动作（`None` = 该槽未绑定技能）。
///
/// ⚠️ **Stub 组件**：技能系统未接线前，实体可不挂此组件——resolver 第五层缺省不发请求。
/// 技能系统建好后由其填充装备栏技能→ActionId 映射，resolver 逻辑零改动即接线。
#[derive(Debug, Clone, Copy, Default)]
pub struct CEquippedSkills {
    /// 4 个技能槽（对应 SpecialSkill(0..4)）
    pub slots: [Option<ActionId>; 4],
}

impl CEquippedSkills {
    /// 查询第 n 个技能槽绑定的动作。
    pub fn get(&self, n: u8) -> Option<ActionId> {
        self.slots.get(n as usize).copied().flatten()
    }
}

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

/// 手感配置——`InputFeelConfig` 的 ECS 组件承载（008 §一）。
///
/// **M4**: 承载 `coyote_time_secs`，替换 `coyote_time_system` 中的硬编码 `0.15`。
///
/// ⚠️ 当前仅 `coyote_time_secs` 接线——跳跃/闪避/连招缓冲窗、边缘吸附等
///    其余手感字段留待手感系统 I1-5 冲刺扩展。缺此组件时系统回退默认值 0.15s。
#[derive(Debug, Clone, Copy)]
pub struct CInputFeelConfig {
    /// 土狼时间 (s)——离地后仍可起跳的宽限窗（008 §一 `coyote_time duration_ms=150`）
    pub coyote_time_secs: f32,
}

impl Default for CInputFeelConfig {
    fn default() -> Self {
        Self {
            coyote_time_secs: 0.15,
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
    fn test_cinput_feel_config_default_coyote() {
        let cfg = CInputFeelConfig::default();
        assert!((cfg.coyote_time_secs - 0.15).abs() < 1e-6);
    }

    #[test]
    fn test_cinput_buffer_capacity() {
        let buf = CInputBuffer::default();
        assert!(buf.entries.capacity() >= 4);
    }
}
