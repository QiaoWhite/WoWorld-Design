//! CleanupSystem — Phase 1 最后执行，清理标记 Entity
//!
//! 查询：
//! - DecayingRemains{decay_progress >= 1.0} → despawn
//! - PendingDespawn → despawn
//!
//! 铁律 4：标记 + 延迟清理——不在其他 System 的迭代中 despawn。

use hecs::CommandBuffer;

use crate::components::vitals::{DecayingRemains, PendingDespawn};

/// 每帧最后执行——清理完成腐败的残骸和标记删除的 Entity。
///
/// ⚠️ PendingDespawn 当前无生产者——Phase 2 手动标记删除（如 GM 命令、剧情脚本）。
/// DecayingRemains{>=1.0} 是当前唯一的自动清理路径。
pub fn cleanup_system(world: &hecs::World, cmd: &mut CommandBuffer) {
    // 腐败完成 → 消失
    for (entity, remains) in world.query::<&DecayingRemains>().iter() {
        if remains.decay_progress >= 1.0 {
            cmd.despawn(entity);
        }
    }

    // 标记删除（Phase 2: GM 命令、脚本触发）
    for (entity, _) in world.query::<&PendingDespawn>().iter() {
        cmd.despawn(entity);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cleanup_fully_decayed() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        let e = world.spawn((DecayingRemains { decay_progress: 1.0 },));

        cleanup_system(&world, &mut cmd);
        cmd.run_on(&mut world);

        // Entity 被 despawn
        assert!(world.get::<&DecayingRemains>(e).is_err());
    }

    #[test]
    fn test_cleanup_partial_decay_kept() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        let e = world.spawn((DecayingRemains { decay_progress: 0.5 },));

        cleanup_system(&world, &mut cmd);
        cmd.run_on(&mut world);

        // 未腐败完成——保留
        assert!(world.get::<&DecayingRemains>(e).is_ok());
    }

    #[test]
    fn test_cleanup_pending_despawn() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        let e = world.spawn((PendingDespawn,));

        cleanup_system(&world, &mut cmd);
        cmd.run_on(&mut world);

        assert!(world.get::<&PendingDespawn>(e).is_err());
    }

    #[test]
    fn test_cleanup_empty_world_does_not_panic() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();

        cleanup_system(&world, &mut cmd);
        cmd.run_on(&mut world);
    }
}
