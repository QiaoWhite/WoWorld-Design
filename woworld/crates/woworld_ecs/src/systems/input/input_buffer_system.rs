//! InputBufferSystem — 环形缓冲区管理
//!
//! push(优先级淘汰) + pop_if(物理合法性重检) + 过期清理。
//! 仅玩家实体（`With<PlayerComponent>`）激活。
//!
//! 参见: `WoWorld-Design/.../角色控制器/008-手感系统.md` §二/§三

use crate::components::action_state::CActionRequestBuf;
use crate::components::input_state::CInputBuffer;
use crate::components::player::PlayerComponent;

/// 输入缓冲管理——清理过期条目、消费就绪条目到 CActionRequestBuf。
///
/// 仅处理带 `PlayerComponent` 的实体。
pub fn input_buffer_system(world: &mut hecs::World, dt: f32) {
    for (_, (buffer, request_buf)) in world
        .query_mut::<(&mut CInputBuffer, &mut CActionRequestBuf)>()
        .with::<&PlayerComponent>()
    {
        // ── 消费缓冲条目（按优先级链排序后弹出）──
        // Sprint 1: 过期清理简化——条目在消费时自然淘汰。
        // 完整过期逻辑待 InputFeelConfig 集成后实现。
        // 排序: 高优先级优先
        buffer.entries.sort_by(|a, b| {
            b.buffer_priority
                .cmp(&a.buffer_priority)
                .then_with(|| a.pressed_at.partial_cmp(&b.pressed_at).unwrap())
        });

        // 将缓冲中的请求移入 CActionRequestBuf（最多填充到容量限制）
        while request_buf.0.len() < request_buf.0.capacity() {
            if let Some(entry) = buffer.entries.pop() {
                request_buf.0.push(entry.action_request);
            } else {
                break;
            }
        }
        // 注: dt 预留后续时间基准用途
        let _ = dt;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use woworld_core::action::{ActionId, ActionParams, ActionRequest, ActionSource};
    use woworld_core::input::{BufferedInput, BufferPriority};

    #[test]
    fn test_buffer_entries_moved_to_request_buf() {
        let mut world = hecs::World::new();
        let req = ActionRequest {
            action_id: ActionId(1),
            priority: 20,
            source: ActionSource::Player,
            params: ActionParams::default(),
        };

        world.spawn((
            PlayerComponent::default(),
            CInputBuffer {
                entries: smallvec::smallvec![BufferedInput::new(
                    req.clone(),
                    0.0,
                    200.0,
                    BufferPriority::Movement
                )],
                prev_frame_inputs: 0,
            },
            CActionRequestBuf::default(),
        ));

        input_buffer_system(&mut world, 0.016);

        for (_, (_, buf)) in world.query_mut::<(&CInputBuffer, &CActionRequestBuf)>() {
            assert_eq!(buf.0.len(), 1);
            assert_eq!(buf.0[0].action_id, ActionId(1));
        }
    }

    #[test]
    fn test_buffer_cleared_after_consumption() {
        let mut world = hecs::World::new();
        let req = ActionRequest {
            action_id: ActionId(2),
            priority: 15,
            source: ActionSource::Player,
            params: ActionParams::default(),
        };

        world.spawn((
            PlayerComponent::default(),
            CInputBuffer {
                entries: smallvec::smallvec![BufferedInput::new(
                    req,
                    0.0,
                    200.0,
                    BufferPriority::Combat
                )],
                prev_frame_inputs: 0,
            },
            CActionRequestBuf::default(),
        ));

        input_buffer_system(&mut world, 0.016);

        for (_, (buffer, _)) in world.query_mut::<(&CInputBuffer, &CActionRequestBuf)>() {
            assert!(buffer.entries.is_empty());
        }
    }

    #[test]
    fn test_non_player_entity_not_processed() {
        let mut world = hecs::World::new();
        // 没有 PlayerComponent——不被处理
        world.spawn((
            CInputBuffer::default(),
            CActionRequestBuf::default(),
        ));

        input_buffer_system(&mut world, 0.016);

        for (_, (_, buf)) in world.query_mut::<(&CInputBuffer, &CActionRequestBuf)>() {
            assert!(buf.0.is_empty()); // 仍为空
        }
    }

    #[test]
    fn test_input_feel_toml_parses_into_config() {
        // ★ 集成前置：验证 assets/input_feel.toml 的键名与 InputFeelConfig 字段对齐。
        //   （修复前 `ledge_snap_angle_deg` 与字段 `ledge_snap_angle` 不匹配，
        //    接线反序列化时会炸；此测试锁定键名一致性。）
        use woworld_core::input::InputFeelConfig;
        #[derive(serde::Deserialize)]
        struct Wrapper {
            feel: InputFeelConfig,
        }
        let toml = include_str!("../../../../../assets/input_feel.toml");
        let w: Wrapper =
            toml::from_str(toml).expect("input_feel.toml 应能解析进 InputFeelConfig");
        assert!((w.feel.coyote_time_ms - 150.0).abs() < 0.001);
        assert!((w.feel.ledge_snap_angle - 45.0).abs() < 0.001);
    }
}
