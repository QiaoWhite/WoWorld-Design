//! 确定性 seed 派生工具
//!
//! 提供管线级 hash 原语——从世界 seed 派生各阶段/各 Chunk 的独立 RNG seed。
//! 纯算术实现，零依赖，永久跨平台一致。
//!
//! 设计依据: 001-世界生成总流程 §四 + 007-体素设计决策 §七

/// 混洗 u64 — splitmix64 finalizer 变体
///
/// 确定性、零依赖、永久跨 Rust 编译器版本一致。
/// 每个输入 bit 影响输出的所有 bit（雪崩效应）。
pub const fn mix64(mut x: u64) -> u64 {
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
    x ^ (x >> 31)
}

/// 管线阶段 seed 派生
///
/// `hash(seed, stage_name) -> u64`
/// 每个管线阶段（P0-P13）获得独立、可复现的 RNG 种子。
///
/// # 确定性
/// 同 seed + 同 stage_name → 永远同输出。
pub fn hash_stage_seed(seed: u64, stage: &str) -> u64 {
    let mut h = seed;
    for b in stage.as_bytes() {
        h = h.wrapping_mul(31).wrapping_add(*b as u64);
    }
    mix64(h)
}

/// Chunk seed 派生
///
/// `hash(stage_seed, chunk_x, chunk_y) -> u64`
/// 每个 Chunk 从阶段 seed 获得独立、可复现的 RNG 种子。
///
/// # 确定性
/// 同 stage_seed + 同 chunk 坐标 → 永远同输出。
pub fn hash_chunk_seed(stage_seed: u64, cx: i64, cy: i64) -> u64 {
    let a = stage_seed ^ (cx as u64).wrapping_mul(0x517cc1b727220a95);
    let b = a ^ (cy as u64);
    mix64(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mix64_deterministic() {
        assert_eq!(mix64(0), mix64(0));
        assert_eq!(mix64(42), mix64(42));
        assert_eq!(mix64(u64::MAX), mix64(u64::MAX));
    }

    #[test]
    fn test_mix64_avalanche() {
        // 单 bit 翻转 → 输出完全不同
        let a = mix64(0);
        let b = mix64(1);
        let diff = (a ^ b).count_ones();
        assert!(diff > 16, "avalanche too weak: {} bits differ", diff);
    }

    #[test]
    fn test_hash_chunk_seed_deterministic() {
        let s = hash_stage_seed(12345, "P2");
        assert_eq!(hash_chunk_seed(s, 0, 0), hash_chunk_seed(s, 0, 0));
        assert_eq!(hash_chunk_seed(s, -5, 10), hash_chunk_seed(s, -5, 10));
    }

    #[test]
    fn test_hash_chunk_seed_different_coords() {
        let s = hash_stage_seed(12345, "P2");
        let a = hash_chunk_seed(s, 0, 0);
        let b = hash_chunk_seed(s, 1, 0);
        let c = hash_chunk_seed(s, 0, 1);
        assert_ne!(a, b);
        assert_ne!(a, c);
    }
}
