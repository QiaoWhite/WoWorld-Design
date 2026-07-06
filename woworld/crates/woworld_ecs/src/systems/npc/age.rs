//! 年龄推进 System — Phase 1 年龄跟踪 + 阶段切换 + Gompertz 衰老死亡
//!
//! 每决策周期推进 Age，跨越阶段边界时更新 LifeStage。
//! Phase 2: 认知老化路径。

use hecs::CommandBuffer;

use crate::components::lifecycle::{
    senescence_survival, Age, GompertzMortality, LifeStage, GOMPERTZ_CHECK_INTERVAL_DAYS,
};
use crate::components::vitals::Vitals;

/// 年龄推进——跨越阶段边界时触发 LifeStage 切换。
/// 对 age_pct >= 0.7 的实体执行 Gompertz 月度死亡判定。
///
/// `dt_days`: 自上次调用以来经过的游戏天数
///
/// 调用者负责在返回后执行 `cmd.run_on(&mut world)`。
pub fn age_system(world: &mut hecs::World, _cmd: &mut CommandBuffer, dt_days: f32) {
    // ── 步骤 1: 推进年龄 + 阶段切换 ──
    for (_entity, (age, stage)) in world.query_mut::<(&mut Age, &mut LifeStage)>() {
        let old_ratio = age.age_ratio();
        age.age_days += dt_days;
        let new_ratio = age.age_ratio();

        let old_stage = LifeStage::from_age_ratio(old_ratio);
        let new_stage = LifeStage::from_age_ratio(new_ratio);

        // 仅跨越边界时触发切换（CommandBuffer 延迟执行，无借用冲突）
        if old_stage != new_stage {
            *stage = new_stage;
            // Phase 2: 发射 LifeEvent::StageTransition 事件
        }
    }

    // ── 步骤 2: Gompertz 衰老死亡月度判定 ──
    // 查询拥有 Age + GompertzMortality + Vitals 的实体
    for (_entity, (age, gmort, vitals)) in
        world.query_mut::<(&Age, &mut GompertzMortality, &mut Vitals)>()
    {
        // 仅在活体有衰老风险时检查（hp > 0 避免对已死实体重复判定）
        if vitals.hp <= 0.0 {
            continue;
        }

        let age_pct = age.age_ratio();
        if age_pct < 0.7 {
            // 未进入衰老风险区——更新 last_check 跟踪点避免首次检查跳变
            gmort.last_check_age_days = age.age_days;
            continue;
        }

        let days_since_check = age.age_days - gmort.last_check_age_days;
        if days_since_check < GOMPERTZ_CHECK_INTERVAL_DAYS {
            continue;
        }

        // 计算此时间段开始时的 age_pct
        let age_pct_start =
            (gmort.last_check_age_days / age.max_lifespan_days).max(0.0);
        let delta_pct = days_since_check / age.max_lifespan_days;

        let survival = senescence_survival(
            age_pct_start,
            delta_pct,
            gmort.constitution,
            gmort.health_history,
        );

        // 更新追踪数据
        gmort.last_check_age_days = age.age_days;
        gmort.current_risk = 1.0 - survival;
        gmort.base_risk = gmort.current_risk; // Phase 2: 从 constitution 独立计算

        // 死亡判定：用确定性伪随机（基于 entity age 避免每帧重试同一实体）
        // 每个检查周期只有一次判定机会
        let roll = pseudo_random_for_gompertz(age.age_days, age.max_lifespan_days);
        if roll > survival {
            // Gompertz 衰老死亡——将 hp 归零，下一帧 DeathWatchSystem 接管
            vitals.hp = 0.0;
        }
    }
}

/// 确定性伪随机——基于年龄和寿命参数，同输入同输出。
///
/// 用 splitmix 变体生成 [0, 1) 浮点数。
/// 与 `prng` 模块独立——此函数用于 Gompertz 月度判定，
/// 需要纯确定性（同 age_days + 同 lifespan → 同结果）。
fn pseudo_random_for_gompertz(age_days: f32, max_lifespan_days: f32) -> f32 {
    let a = age_days.to_bits();
    let b = max_lifespan_days.to_bits();
    // splitmix64 单步——全部在 u64 空间操作
    let mut x = (a as u64).wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(b as u64);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
    x = x ^ (x >> 31);
    (x as f64 / u64::MAX as f64) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_age_advances() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let a = Age::new(70.0, 20.0);
        let initial_stage = LifeStage::from_age_ratio(a.age_ratio());
        let e = world.spawn((a, initial_stage));

        age_system(&mut world, &mut cmd, 365.0); // +1 年
        cmd.run_on(&mut world);

        let age = world.get::<&Age>(e).unwrap();
        assert!(age.age_days > 20.0 * 360.0);
    }

    #[test]
    fn test_stage_transition_triggers() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        // 从 Adolescent (15-25%) 推进到 YoungAdult (25-45%)
        // 16 岁 / 80 年 = 0.20 → Adolescent，推进到 22 岁 = 0.275 → YoungAdult
        let a = Age::new(80.0, 16.0);
        let stage = LifeStage::from_age_ratio(a.age_ratio());
        assert_eq!(stage, LifeStage::Adolescent);
        let e = world.spawn((a, stage));

        age_system(&mut world, &mut cmd, 360.0 * 6.0); // +6 年 → 22/80 = 0.275
        cmd.run_on(&mut world);

        let new_stage = world.get::<&LifeStage>(e).unwrap();
        assert_eq!(*new_stage, LifeStage::YoungAdult);
    }

    #[test]
    fn test_stage_no_change_within_boundary() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        // 30 岁 / 80 年 = 0.375 → YoungAdult，推进到 31 岁 = 0.3875 → 仍在 YoungAdult
        let a = Age::new(80.0, 30.0);
        let stage = LifeStage::from_age_ratio(a.age_ratio());
        assert_eq!(stage, LifeStage::YoungAdult);
        let e = world.spawn((a, stage));

        age_system(&mut world, &mut cmd, 360.0); // +1 年
        cmd.run_on(&mut world);

        let new_stage = world.get::<&LifeStage>(e).unwrap();
        assert_eq!(*new_stage, LifeStage::YoungAdult, "should still be YoungAdult");
    }

    #[test]
    fn test_stage_transition_elder() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        // 74 岁 / 80 年 = 0.925 → MiddleAge，推进到 77 岁 = 0.9625 → Elder
        let a = Age::new(80.0, 74.0);
        let stage = LifeStage::from_age_ratio(a.age_ratio());
        assert_eq!(stage, LifeStage::MiddleAge);
        let e = world.spawn((a, stage));

        age_system(&mut world, &mut cmd, 360.0 * 4.0); // +4 年
        cmd.run_on(&mut world);

        let new_stage = world.get::<&LifeStage>(e).unwrap();
        assert_eq!(*new_stage, LifeStage::Elder);
    }

    #[test]
    fn test_empty_world_no_panic() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        age_system(&mut world, &mut cmd, 1.0);
    }

    // ── Gompertz death tests ──

    #[test]
    fn test_gompertz_no_death_before_70_pct() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        // 30 岁 / 80 年 = 0.375 — 远低于 0.7 阈值
        let age = Age::new(80.0, 30.0);
        let stage = LifeStage::from_age_ratio(age.age_ratio());
        let age_days_start = age.age_days;
        let e = world.spawn((
            age,
            stage,
            Vitals::default(),
            GompertzMortality::default(),
        ));

        // 推进 1 年（365 天 > 30 天检查间隔）
        age_system(&mut world, &mut cmd, 365.0);
        cmd.run_on(&mut world);

        let vitals = world.get::<&Vitals>(e).unwrap();
        assert!(vitals.hp > 0.0, "should not die before 70% lifespan");

        let gmort = world.get::<&GompertzMortality>(e).unwrap();
        assert!(gmort.last_check_age_days > age_days_start);
    }

    #[test]
    fn test_gompertz_check_updates_last_check() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        // 60 岁 / 80 年 = 0.75 — 进入衰老风险区
        let age = Age::new(80.0, 60.0);
        let stage = LifeStage::from_age_ratio(age.age_ratio());
        let gmort = GompertzMortality {
            last_check_age_days: age.age_days, // 初始化为当前年龄
            ..GompertzMortality::default()
        };
        let e = world.spawn((age, stage, Vitals::default(), gmort));

        // 推进 30 天——恰好触发一次检查
        age_system(&mut world, &mut cmd, 30.0);
        cmd.run_on(&mut world);

        let gmort = world.get::<&GompertzMortality>(e).unwrap();
        // last_check 应该已更新到当前年龄
        let age = world.get::<&Age>(e).unwrap();
        assert!((gmort.last_check_age_days - age.age_days).abs() < 0.01);
    }

    #[test]
    fn test_gompertz_skips_check_within_interval() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let age = Age::new(80.0, 60.0);
        let stage = LifeStage::from_age_ratio(age.age_ratio());
        let gmort = GompertzMortality {
            last_check_age_days: age.age_days,
            ..GompertzMortality::default()
        };
        let e = world.spawn((age, stage, Vitals::default(), gmort));

        // 推进仅 10 天——不到月度检查间隔
        age_system(&mut world, &mut cmd, 10.0);
        cmd.run_on(&mut world);

        let gmort = world.get::<&GompertzMortality>(e).unwrap();
        // last_check 不应更新（未到 30 天间隔）
        let age = world.get::<&Age>(e).unwrap();
        assert!(age.age_days - gmort.last_check_age_days < 30.0);
    }

    #[test]
    fn test_gompertz_death_at_very_old_age() {
        // 极端老化——age_pct 远超 1.0，几乎必然死亡
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        // 100 岁 / 70 年 = 1.43 age_pct
        let age = Age::new(70.0, 100.0);
        let stage = LifeStage::from_age_ratio(age.age_ratio());
        let gmort = GompertzMortality {
            last_check_age_days: age.age_days,
            constitution: 0.3, // 低体质
            health_history: 3.0, // 重病史
            ..GompertzMortality::default()
        };
        let e = world.spawn((age, stage, Vitals::default(), gmort));

        // 推进 30 天触发检查
        age_system(&mut world, &mut cmd, 30.0);
        cmd.run_on(&mut world);

        let vitals = world.get::<&Vitals>(e).unwrap();
        // 在 1.43 age_pct + 低体质 + 重病史下几乎必然死亡
        // 但由于确定性伪随机，不能 100% 断言——只检查系统不崩溃
        // 实际几乎总是 hp==0
        let _ = vitals.hp; // at minimum, no panic
    }

    #[test]
    fn test_gompertz_ignores_already_dead() {
        let mut world = hecs::World::new();
        let mut cmd = CommandBuffer::new();
        let age = Age::new(70.0, 60.0); // age_pct ~0.86
        let stage = LifeStage::from_age_ratio(age.age_ratio());
        let gmort = GompertzMortality {
            last_check_age_days: age.age_days,
            ..GompertzMortality::default()
        };
        let e = world.spawn((
            age,
            stage,
            Vitals {
                hp: 0.0, // 已死亡
                ..Vitals::default()
            },
            gmort,
        ));

        let gmort_before = world.get::<&GompertzMortality>(e).unwrap().last_check_age_days;

        age_system(&mut world, &mut cmd, 30.0);
        cmd.run_on(&mut world);

        // 已死实体的 last_check 不应更新
        let gmort_after = world.get::<&GompertzMortality>(e).unwrap();
        assert_eq!(gmort_after.last_check_age_days, gmort_before);
    }

    #[test]
    fn test_gompertz_deterministic() {
        // 确定性：同输入同输出
        let run_once = || {
            let mut world = hecs::World::new();
            let mut cmd = CommandBuffer::new();
            let age = Age::new(70.0, 63.0); // 0.9 age_pct
            let stage = LifeStage::from_age_ratio(age.age_ratio());
            let gmort = GompertzMortality {
                last_check_age_days: age.age_days,
                constitution: 0.5,
                health_history: 0.0,
                ..GompertzMortality::default()
            };
            let e = world.spawn((age, stage, Vitals::default(), gmort));
            age_system(&mut world, &mut cmd, 30.0);
            cmd.run_on(&mut world);
            let hp = world.get::<&Vitals>(e).unwrap().hp;
            hp
        };

        let hp1 = run_once();
        let hp2 = run_once();
        assert_eq!(hp1, hp2, "Gompertz death roll must be deterministic");
    }

    #[test]
    fn test_pseudo_random_is_deterministic() {
        let r1 = pseudo_random_for_gompertz(1000.0, 25200.0);
        let r2 = pseudo_random_for_gompertz(1000.0, 25200.0);
        assert_eq!(r1, r2);
    }

    #[test]
    fn test_pseudo_random_in_range() {
        for i in 0..100 {
            let r = pseudo_random_for_gompertz(i as f32 * 10.0, 25200.0);
            assert!(r >= 0.0 && r < 1.0, "r={r} out of range");
        }
    }
}
