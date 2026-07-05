//! 确定性伪随机数生成器 — splitmix64 变体
//!
//! 无 `rand` 依赖。所有函数给定相同输入始终产生相同输出。
//!
//! 原本在 4 个文件中各自复制——Sprint 056 提取到此共享模块。

/// 确定性伪随机 f32 ∈ [0, 1)
///
/// splitmix64 变体——与 salted(seed, 0) 等价。
pub fn pseudo_random_f32(seed: u64) -> f32 {
    pseudo_random_f32_salted(seed, 0)
}

/// 确定性伪随机 f32 ∈ [0, 1)，含 salt 参数用于同一 seed 下派生多值
///
/// `salt=0` 退化为 `pseudo_random_f32(seed)`。
/// 用于 BigFive::from_seed 等需要从同一 seed 派生 5 个独立值的场景。
pub fn pseudo_random_f32_salted(seed: u64, salt: u64) -> f32 {
    let mut x = seed
        .wrapping_add(salt.wrapping_mul(0x9E37_79B9_7F4A_7C15))
        .wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^= x >> 31;
    // 取高 24 位作为尾数 → f32 ∈ [0, 1)
    (x >> 40) as f32 / (1u64 << 24) as f32
}

/// 确定性 [min, max] 伪随机，从 seed + salt 派生
pub fn pseudo_random_f32_range(seed: u64, salt: u64, min: f32, max: f32) -> f32 {
    let t = pseudo_random_f32(seed.wrapping_add(salt));
    min + t * (max - min)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic() {
        let a = pseudo_random_f32(42);
        let b = pseudo_random_f32(42);
        assert!((a - b).abs() < f32::EPSILON);
    }

    #[test]
    fn test_salted_zero_equals_unsalted() {
        let a = pseudo_random_f32(99);
        let b = pseudo_random_f32_salted(99, 0);
        assert!((a - b).abs() < f32::EPSILON);
    }

    #[test]
    fn test_salted_different_salt_different_output() {
        let a = pseudo_random_f32_salted(42, 0);
        let b = pseudo_random_f32_salted(42, 1);
        assert!((a - b).abs() > 0.001, "different salts should yield different values");
    }

    #[test]
    fn test_output_in_range() {
        for seed in 0..100 {
            let v = pseudo_random_f32(seed);
            assert!((0.0..1.0).contains(&v), "seed {seed}: {v} not in [0,1)");
        }
    }

    #[test]
    fn test_salted_output_in_range() {
        for seed in 0..50 {
            for salt in 0..10 {
                let v = pseudo_random_f32_salted(seed, salt);
                assert!((0.0..1.0).contains(&v), "seed {seed} salt {salt}: {v} not in [0,1)");
            }
        }
    }

    #[test]
    fn test_range_in_bounds() {
        for seed in 0..30 {
            let v = pseudo_random_f32_range(seed, 0, -0.12, 0.12);
            assert!(v >= -0.12 && v <= 0.12, "seed {seed}: {v} out of [-0.12, 0.12]");
        }
    }
}
