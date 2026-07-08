//! CorpseDecaySystem — 尸体腐败，从 Corpse 过渡到 DecayingRemains
//!
//! 查询 Corpse + CorpseLooted（已搜刮）→ 时间足够 → remove Corpse, insert DecayingRemains。
//! DecayingRemains 的 decay_progress 每帧递增，满 1.0 后由 CleanupSystem 清理。

use hecs::CommandBuffer;

use crate::components::vitals::{Corpse, CorpseLooted, DecayingRemains};

/// 尸体→腐败残骸的帧数阈值（~5 分钟 @ 60fps = 18000 帧）
const DECAY_THRESHOLD_FRAMES: u64 = 500; // Phase 1 加速：500 帧 ≈ 8 秒

/// 每帧执行——已搜刮尸体超过阈值 → 开始腐败
pub fn corpse_decay_system(world: &hecs::World, cmd: &mut CommandBuffer, current_tick: u64) {
    for (entity, (corpse, _looted)) in world.query::<(&Corpse, &CorpseLooted)>().iter() {
        let elapsed = current_tick.saturating_sub(corpse.death_tick);
        if elapsed < DECAY_THRESHOLD_FRAMES {
            continue;
        }

        // 过渡：Corpse → DecayingRemains
        cmd.remove_one::<Corpse>(entity);
        cmd.remove_one::<CorpseLooted>(entity);
        cmd.insert_one(
            entity,
            DecayingRemains {
                decay_progress: 0.0,
            },
        );
    }

    // 推进已有 DecayingRemains 的腐败进度
    for (_entity, remains) in world.query::<&mut DecayingRemains>().iter() {
        remains.decay_progress = (remains.decay_progress + 0.001).min(1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_corpse_decay_after_threshold() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        let e = world.spawn((
            Corpse {
                death_tick: 0,
                corpse_temperature: 20.0,
            },
            CorpseLooted,
        ));

        // tick > threshold → 应过渡
        corpse_decay_system(&world, &mut cmd, DECAY_THRESHOLD_FRAMES + 1);
        cmd.run_on(&mut world);

        assert!(world.get::<&Corpse>(e).is_err());
        assert!(world.get::<&CorpseLooted>(e).is_err());
        let remains = world
            .get::<&DecayingRemains>(e)
            .expect("should have DecayingRemains");
        assert_eq!(remains.decay_progress, 0.0);
    }

    #[test]
    fn test_corpse_decay_before_threshold_does_nothing() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        let e = world.spawn((
            Corpse {
                death_tick: 100,
                corpse_temperature: 20.0,
            },
            CorpseLooted,
        ));

        // tick 仅比 death_tick 大一点——不到阈值
        corpse_decay_system(&world, &mut cmd, 101);
        cmd.run_on(&mut world);

        assert!(world.get::<&Corpse>(e).is_ok());
        assert!(world.get::<&DecayingRemains>(e).is_err());
    }

    #[test]
    fn test_decaying_remains_progress_increments() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        let e = world.spawn((DecayingRemains {
            decay_progress: 0.0,
        },));

        corpse_decay_system(&world, &mut cmd, 0);
        cmd.run_on(&mut world);

        let remains = world
            .get::<&DecayingRemains>(e)
            .expect("still has DecayingRemains");
        assert!(remains.decay_progress > 0.0);
        assert!(remains.decay_progress < 1.0);
    }

    #[test]
    fn test_corpse_without_looted_does_not_decay() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        // Corpse 但没有 CorpseLooted——还未被搜刮
        let e = world.spawn((Corpse {
            death_tick: 0,
            corpse_temperature: 20.0,
        },));

        corpse_decay_system(&world, &mut cmd, DECAY_THRESHOLD_FRAMES + 1);
        cmd.run_on(&mut world);

        // 未搜刮的尸体不腐败——保留等待搜刮
        assert!(world.get::<&Corpse>(e).is_ok());
    }
}
