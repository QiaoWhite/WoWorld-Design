//! 环形缓冲区事件总线 — SpatialEventBus trait 实现
//!
//! 使用环形缓冲区存储空间事件，支持时间窗口过滤查询。
//! 气味源目前返回空——待生命/战斗模块接入后实现。

use std::time::Duration;
use woworld_core::prelude::*;
use woworld_core::spatial::SpatialEventBus;

/// 默认事件容量
const DEFAULT_CAPACITY: usize = 10_000;

/// 环形缓冲区事件总线
#[derive(Clone, Debug)]
pub struct RingEventBus {
    events: Vec<Option<SpatialEvent>>,
    capacity: usize,
    write_index: usize,
    count: usize,
}

impl RingEventBus {
    pub fn new(capacity: usize) -> Self {
        Self {
            events: vec![None; capacity],
            capacity,
            write_index: 0,
            count: 0,
        }
    }

    /// 已存储的事件数量
    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// 检查 AABB 是否包含点
    fn aabb_contains(aabb: &Aabb, point: WorldPos) -> bool {
        point.x >= aabb.min.x
            && point.x <= aabb.max.x
            && point.y >= aabb.min.y
            && point.y <= aabb.max.y
            && point.z >= aabb.min.z
            && point.z <= aabb.max.z
    }

    /// 遍历所有有效事件
    fn iter(&self) -> impl Iterator<Item = &SpatialEvent> {
        self.events.iter().filter_map(|e| e.as_ref())
    }
}

impl Default for RingEventBus {
    fn default() -> Self {
        Self::new(DEFAULT_CAPACITY)
    }
}

impl SpatialEventBus for RingEventBus {
    fn push_event(&mut self, event: SpatialEvent) {
        self.events[self.write_index] = Some(event);
        self.write_index = (self.write_index + 1) % self.capacity;
        if self.count < self.capacity {
            self.count += 1;
        }
    }

    fn recent_events_in(&self, aabb: &Aabb, time_window: Duration) -> Vec<SpatialEvent> {
        let now = self
            .events
            .iter()
            .filter_map(|e| e.as_ref())
            .map(|e| e.timestamp)
            .fold(0.0f64, f64::max);
        let min_time = now - time_window.as_secs_f64();

        self.iter()
            .filter(|e| e.timestamp >= min_time && Self::aabb_contains(aabb, e.position))
            .cloned()
            .collect()
    }

    fn scent_sources_in(&self, aabb: &Aabb) -> Vec<ScentSource> {
        // 当前未集成气味系统——返回空。
        // 气味跟踪将在生命/战斗模块接入后通过独立的 ScentTracker 组件实现。
        let _ = aabb;
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_event(id: u64, x: f64, y: f64, z: f64, timestamp: f64) -> SpatialEvent {
        SpatialEvent {
            event_type: "test",
            position: WorldPos { x, y, z },
            intensity: 0.5,
            timestamp,
            source_entity: Some(EntityId(id)),
        }
    }

    #[test]
    fn test_push_and_query() {
        let mut bus = RingEventBus::new(100);
        bus.push_event(make_event(1, 10.0, 0.0, 10.0, 1.0));
        bus.push_event(make_event(2, 15.0, 0.0, 15.0, 2.0));
        bus.push_event(make_event(3, 50.0, 0.0, 50.0, 3.0));

        let aabb = Aabb {
            min: WorldPos {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
            max: WorldPos {
                x: 20.0,
                y: 1.0,
                z: 20.0,
            },
        };
        let found = bus.recent_events_in(&aabb, Duration::from_secs(10));
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn test_time_window_filter() {
        let mut bus = RingEventBus::new(100);
        bus.push_event(make_event(1, 10.0, 0.0, 10.0, 1.0));
        bus.push_event(make_event(2, 12.0, 0.0, 12.0, 100.0)); // 很新

        let aabb = Aabb {
            min: WorldPos {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
            max: WorldPos {
                x: 20.0,
                y: 1.0,
                z: 20.0,
            },
        };
        // 1 秒窗口——now=100.0, min_time=99.0，仅 t=100.0 的事件通过
        let found = bus.recent_events_in(&aabb, Duration::from_secs(1));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].source_entity, Some(EntityId(2)));
    }

    #[test]
    fn test_ring_buffer_wrap() {
        let mut bus = RingEventBus::new(4); // 小容量强制环绕
        for i in 0..6 {
            bus.push_event(make_event(i, 10.0, 0.0, 10.0, i as f64));
        }
        // 只保留最后 4 个
        assert_eq!(bus.len(), 4);
    }

    #[test]
    fn test_scent_sources_empty() {
        let bus = RingEventBus::new(100);
        let aabb = Aabb {
            min: WorldPos {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
            max: WorldPos {
                x: 100.0,
                y: 1.0,
                z: 100.0,
            },
        };
        assert!(bus.scent_sources_in(&aabb).is_empty());
    }
}
