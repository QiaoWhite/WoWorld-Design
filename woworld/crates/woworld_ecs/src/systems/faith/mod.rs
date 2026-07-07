//! Faith ECS Systems
//!
//! Phase 1: faith_seed_system — 从种子 + Culture.religiosity 确定性生成初始信仰
//!
//! Phase 2+: FaithPracticeSystem, FaithPropagationSystem（需 NPC 交互系统）

use hecs::{CommandBuffer, World};
use woworld_core::faith::{FaithTheology, ReligiousMotivation, ReligiousPracticeProfile};
use woworld_core::faith::FaithQuery;
use woworld_core::culture::CultureQuery;
use woworld_core::types::EntityId;

use crate::components::bigfive::BigFive;
use crate::components::faith::Faith;
use crate::resources::culture_registry::CultureRegistry;
use crate::resources::faith_registry::FaithRegistry;

/// 信仰种子系统：从种子 + Culture.religiosity 为 NPC 分配初始信仰
///
/// - religiosity < 0.1 → 跳过（世俗文化，无信仰分配）
/// - 否则 fan_out_count = floor(religiosity * 2.5) + probabilistic_extra, max 3
/// - 每个信仰从种子独立派生 FaithTheology
///
/// Phase 1: 信仰总数为从 seed 派生的 1-3 个，NPC 随机分配到其中之一。
pub fn faith_seed_system(
    world: &World,
    cmd: &mut CommandBuffer,
    faith_registry: &mut FaithRegistry,
    culture_registry: &CultureRegistry,
) {
    // 确定该世界有信仰需要创建
    for (entity, _bigfive) in world.query::<&BigFive>().iter().filter(|(e, _)| {
        world.get::<&Faith>(*e).is_err()
    }) {
        let seed = entity.to_bits().get();

        // 从 NPC 的文化归属获取 religiosity
        let religiosity = if let Ok(culture) = world.get::<&crate::components::culture::Culture>(entity) {
            culture_registry.core_params(culture.culture_id)
                .map(|p| p.religiosity)
                .unwrap_or(0.5)
        } else {
            0.5 // 无文化 → 默认中等 religiosity
        };

        // religiosity < 0.1 → 世俗，不分配信仰
        if religiosity < 0.1 {
            // 仍然插入 Faith Component 标记为无信仰
            cmd.insert_one(entity, Faith::default());
            continue;
        }

        // 为此 NPC 分配 1-3 个信仰
        let base_count = (religiosity * 2.5).floor() as usize; // 0, 1, or 2
        let extra = if culture_hash_ext(seed, 100) < (religiosity - 0.5).max(0.0) * 2.0 { 1 } else { 0 };
        let total_faiths = (base_count + extra).clamp(1, 3);

        let mut participation = Vec::with_capacity(total_faiths);
        for i in 0..total_faiths {
            let faith_seed = seed.wrapping_add((i as u64 + 1) * 7919);
            let theology = FaithTheology::from_seed(faith_seed, religiosity);
            let faith_id = faith_registry.register(theology);
            let weight = (0.9 - i as f32 * 0.3).max(0.2); // 递减权重: 0.9, 0.6, 0.3
            participation.push((faith_id, weight));
        }

        // 实践档案存入 Registry
        let profile = ReligiousPracticeProfile {
            participation,
            theological_depth: 0.05 + culture_hash_ext(seed, 200) * 0.10,
            motivation: ReligiousMotivation::Habitual,
        };
        faith_registry.set_profile(EntityId(seed), profile);

        // 主要信仰 → Faith Component
        let primary_id = faith_registry.all_faiths()[faith_registry.faith_count() - total_faiths];
        cmd.insert_one(entity, Faith { faith_id: primary_id });
    }
}

/// 确定性哈希（与 woworld_core::faith 内部的一致）
pub(crate) fn culture_hash_ext(seed: u64, salt: u64) -> f32 {
    let mut x = seed
        .wrapping_add(salt.wrapping_mul(0x9E37_79B9_7F4A_7C15))
        .wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    (x >> 40) as f32 / (1u64 << 24) as f32
}

// ── 测试 ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::bigfive::BigFive;
    use crate::components::culture::Culture;
    use crate::components::faith::Faith;
    use woworld_core::culture::CultureCoreParams;
    use woworld_core::faith::{FaithId, FaithQuery, FaithTheology, FAITH_ID_NONE};

    #[test]
    fn test_secular_no_faith() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut faith_reg = FaithRegistry::new();
        let mut culture_reg = CultureRegistry::new();

        // 创建世俗文化
        let culture_id = culture_reg.register(CultureCoreParams {
            religiosity: 0.05,
            ..CultureCoreParams::default()
        });

        world.spawn((
            BigFive::from_seed(42),
            Culture { culture_id },
        ));

        faith_seed_system(&world, &mut cmd, &mut faith_reg, &culture_reg);
        cmd.run_on(&mut world);

        // 世俗文化 → 无信仰
        let faith_count = faith_reg.faith_count();
        // 没有信仰被注册
        for (_e, faith) in world.query::<&Faith>().iter() {
            assert_eq!(faith.faith_id, FAITH_ID_NONE);
        }
    }

    #[test]
    fn test_religious_has_faith() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut faith_reg = FaithRegistry::new();
        let mut culture_reg = CultureRegistry::new();

        let culture_id = culture_reg.register(CultureCoreParams {
            religiosity: 0.9,
            ..CultureCoreParams::default()
        });

        world.spawn((
            BigFive::from_seed(42),
            Culture { culture_id },
        ));

        faith_seed_system(&world, &mut cmd, &mut faith_reg, &culture_reg);
        cmd.run_on(&mut world);

        // 高 religiosity → 应有信仰注册
        assert!(faith_reg.faith_count() > 0);
        for (_e, faith) in world.query::<&Faith>().iter() {
            assert_ne!(faith.faith_id, FAITH_ID_NONE);
        }
    }

    #[test]
    fn test_skips_existing_faith() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut faith_reg = FaithRegistry::new();
        let mut culture_reg = CultureRegistry::new();

        let culture_id = culture_reg.register(CultureCoreParams {
            religiosity: 0.9,
            ..CultureCoreParams::default()
        });

        let existing_faith = faith_reg.register(FaithTheology::from_seed(99, 0.9));
        world.spawn((
            BigFive::from_seed(42),
            Culture { culture_id },
            Faith { faith_id: existing_faith },
        ));

        let initial_count = faith_reg.faith_count();
        faith_seed_system(&world, &mut cmd, &mut faith_reg, &culture_reg);
        cmd.run_on(&mut world);

        // 不应新增
        assert_eq!(faith_reg.faith_count(), initial_count);
    }

    #[test]
    fn test_empty_world_no_panic() {
        let mut world = World::new();
        let mut cmd = CommandBuffer::new();
        let mut faith_reg = FaithRegistry::new();
        let culture_reg = CultureRegistry::new();

        faith_seed_system(&world, &mut cmd, &mut faith_reg, &culture_reg);
        cmd.run_on(&mut world);
    }

    #[test]
    fn test_deterministic() {
        let mut w1 = World::new();
        let mut c1 = CommandBuffer::new();
        let mut f1 = FaithRegistry::new();
        let mut cr1 = CultureRegistry::new();

        let mut w2 = World::new();
        let mut c2 = CommandBuffer::new();
        let mut f2 = FaithRegistry::new();
        let mut cr2 = CultureRegistry::new();

        let culture_id1 = cr1.register(CultureCoreParams { religiosity: 0.8, ..Default::default() });
        let culture_id2 = cr2.register(CultureCoreParams { religiosity: 0.8, ..Default::default() });

        w1.spawn((BigFive::from_seed(42), Culture { culture_id: culture_id1 }));
        w2.spawn((BigFive::from_seed(42), Culture { culture_id: culture_id2 }));

        faith_seed_system(&w1, &mut c1, &mut f1, &cr1);
        c1.run_on(&mut w1);
        faith_seed_system(&w2, &mut c2, &mut f2, &cr2);
        c2.run_on(&mut w2);

        assert_eq!(f1.faith_count(), f2.faith_count());
        for i in 0..f1.faith_count() {
            let t1 = f1.theology(FaithId(i as u32)).unwrap();
            let t2 = f2.theology(FaithId(i as u32)).unwrap();
            assert!((t1.deity_count - t2.deity_count).abs() < 0.001);
        }
    }
}
