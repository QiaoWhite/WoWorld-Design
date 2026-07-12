//! EncounterState — 遭遇感知层状态（成对邻近边沿·迟滞·问候冷却）
//!
//! 遭遇是**成对、对称、有状态的关系边**——独立于 `SpatialEvent`（广播点事件）与
//! `RelationStorage`（长期关系值）。设计对齐 `语言表达/006` `Reactive::SomeoneEntered/Left`。
//!
//! barrier-free 定位：这是**感知层**（通用），产出 `EncounterEvent` 供表达层（问候气泡）、
//! 未来战斗警觉/目击/记忆消费——**非语音专属**。
//!
//! ⚠️ 感知赝品：3m proximity（+ 表达层朝向门）是 `感官系统`/`VisibilityQuery` 的临时替身，
//! 就位后替换（非扩展）。存 `WorldDriver`，`&mut` 传入 `encounter_system`。

use std::collections::{HashMap, HashSet};

/// 遭遇边沿类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncounterKind {
    /// 一对 agent 首次进入邻近（dist < ENTER_RADIUS）
    Enter,
    /// 一对 agent 离开邻近（dist > LEAVE_RADIUS 或对方 despawn）
    Leave,
}

/// 遭遇事件——一帧内产生，供表达层消费。
#[derive(Debug, Clone, Copy)]
pub struct EncounterEvent {
    pub a: hecs::Entity,
    pub b: hecs::Entity,
    pub kind: EncounterKind,
    /// Leave 是否因对方 despawn（死亡/卸载）——真则**不冒告别**（G3）
    pub by_despawn: bool,
}

/// 有序对键——(min,max) entity bits，保证 (a,b)/(b,a) 同键。
pub(crate) fn pair_key(a: hecs::Entity, b: hecs::Entity) -> (u64, u64) {
    let (x, y) = (a.to_bits().get(), b.to_bits().get());
    if x <= y {
        (x, y)
    } else {
        (y, x)
    }
}

/// 遭遇感知层全局状态。
#[derive(Debug, Default)]
pub struct EncounterState {
    /// 当前"在范围内"的对（迟滞态·bits 键）
    pub(crate) in_range: HashSet<(u64, u64)>,
    /// 本帧事件缓冲——`encounter_system` 帧首清空、当帧填充（O5）
    pub events: Vec<EncounterEvent>,
    /// per-pair 上次问候 tick（问候冷却 G2·防闲逛重逢刷屏）
    pub(crate) last_greet: HashMap<(u64, u64), u64>,
    /// 首帧播种标记（N2·避免开局群体问候爆发）
    pub(crate) seeded: bool,
}

impl EncounterState {
    pub fn new() -> Self {
        Self::default()
    }

    /// 该对是否可再次问候（距上次问候 ≥ cooldown_ticks，或从未问候）。
    pub fn can_greet(
        &self,
        a: hecs::Entity,
        b: hecs::Entity,
        tick: u64,
        cooldown_ticks: u64,
    ) -> bool {
        match self.last_greet.get(&pair_key(a, b)) {
            Some(&last) => tick.saturating_sub(last) >= cooldown_ticks,
            None => true,
        }
    }

    /// 记录一对刚问候（写冷却时钟）。
    pub fn mark_greet(&mut self, a: hecs::Entity, b: hecs::Entity, tick: u64) {
        self.last_greet.insert(pair_key(a, b), tick);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ent(bits_seed: u64) -> hecs::Entity {
        // 用一个真实 world 生成 Entity（bits 稳定）
        let mut w = hecs::World::new();
        let mut e = w.spawn((bits_seed,));
        // spawn 多个使 seed 不同，取最后
        for _ in 0..bits_seed {
            e = w.spawn((0u64,));
        }
        e
    }

    #[test]
    fn test_pair_key_symmetric() {
        let a = ent(1);
        let b = ent(5);
        assert_eq!(pair_key(a, b), pair_key(b, a));
    }

    #[test]
    fn test_greet_cooldown() {
        let mut s = EncounterState::new();
        let a = ent(1);
        let b = ent(3);
        assert!(s.can_greet(a, b, 100, 600), "从未问候→可问候");
        s.mark_greet(a, b, 100);
        assert!(!s.can_greet(a, b, 400, 600), "冷却内→不可");
        assert!(s.can_greet(a, b, 800, 600), "冷却过→可");
    }
}
