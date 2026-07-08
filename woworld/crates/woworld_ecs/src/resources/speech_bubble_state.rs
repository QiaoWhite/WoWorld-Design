//! SpeechBubbleState — 全局对话气泡状态（NPC 头顶文字气泡的跨帧状态）
//!
//! pull 管线下 EntityVisual 每帧重建，但气泡有 duration。本 Resource 持有
//! 每个 NPC 当前的活跃气泡 + 冷却计时，存储在 WorldDriver 中，作为 `&mut`
//! 参数传入 speech_bubble_system。
//!
//! ⚠️ 命名：这是 **UI 文字气泡**，与 `音频系统/007` 的 Bark（语声）正交。
//! 详见 `woworld_core::speech_bubble::BubbleType`。

use std::collections::HashMap;

use woworld_core::speech_bubble::BubbleType;

/// 单个 NPC 当前正在显示的气泡
#[derive(Debug, Clone)]
pub struct ActiveBubble {
    /// 气泡文字（桩化阶段为固定短语）
    pub text: String,
    /// 气泡内容类型——决定文字颜色
    pub bubble_type: BubbleType,
    /// 过期 tick——`current_tick > expiry_tick` 时气泡消失
    pub expiry_tick: u64,
}

/// 单个 NPC 的气泡槽——活跃气泡 + 冷却计时
#[derive(Debug, Clone, Default)]
pub struct BubbleSlot {
    /// 当前活跃气泡（None=当前无气泡）
    pub active: Option<ActiveBubble>,
    /// 下次允许触发的 tick——`current_tick >= next_allowed_tick` 才能生成新气泡
    pub next_allowed_tick: u64,
}

/// 全局对话气泡状态——每个 NPC 一个 BubbleSlot
#[derive(Debug, Clone, Default)]
pub struct SpeechBubbleState {
    pub slots: HashMap<hecs::Entity, BubbleSlot>,
}

impl SpeechBubbleState {
    pub fn new() -> Self {
        Self::default()
    }

    /// 查询某实体当前活跃气泡（供 entity_visual_system 消费）
    pub fn active_for(&self, entity: hecs::Entity) -> Option<&ActiveBubble> {
        self.slots.get(&entity).and_then(|s| s.active.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_empty() {
        let s = SpeechBubbleState::new();
        assert!(s.slots.is_empty());
    }

    #[test]
    fn test_active_for_absent() {
        let s = SpeechBubbleState::new();
        let mut world = hecs::World::new();
        let e = world.spawn((1u32,));
        assert!(s.active_for(e).is_none());
    }

    #[test]
    fn test_active_for_present() {
        let mut s = SpeechBubbleState::new();
        let mut world = hecs::World::new();
        let e = world.spawn((1u32,));
        s.slots.insert(
            e,
            BubbleSlot {
                active: Some(ActiveBubble {
                    text: "你好".into(),
                    bubble_type: BubbleType::Normal,
                    expiry_tick: 100,
                }),
                next_allowed_tick: 0,
            },
        );
        assert_eq!(s.active_for(e).unwrap().text, "你好");
    }
}
