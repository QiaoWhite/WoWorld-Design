//! 植被群落 — Shannon 熵优势种筛选
//!
//! 核心算法：从候选物种列表中用 Shannon 熵加权筛选 1-5 个优势种。
//! 丰富度由熵 × 土壤肥力自然决定，而非硬编码"每群系 N 种树"。
//!
//! 参见: `WoWorld-Design/Happy Game/开发阶段/生命/010-植被群落与覆盖.md` §三.2

use woworld_core::id::SpeciesId;

/// Shannon 熵优势种筛选
///
/// # 参数
/// - `candidates`: (物种ID, 适应度) 列表（已由 `SpeciesTable::query()` 产生）
/// - `soil_fertility`: 土壤肥力 [0, 1]（来自群系参数场，MVP 通过 temperature + precipitation 合成）
///
/// # 返回
/// 1-5 个优势种及其归一化权重，按适应度降序排列。
///
/// # 涌现效果
/// - 热带雨林（高温 + 高湿 + 中高肥力）→ 多候选中高适应度 → 高熵 → 5 种共存
/// - 针叶林（低温 + 中湿）→ 仅针叶候选高适应度 → 低熵 → 1-2 种
/// - 过渡带（临界参数值）→ 不同群系的候选适应度相近 → 高熵 → 斑块马赛克
pub fn select_dominant_species(
    candidates: &[(SpeciesId, f32)],
    soil_fertility: f32,
) -> Vec<(SpeciesId, f32)> {
    // Step 1: 过滤低于阈值 + 按适应度降序
    let mut ranked: Vec<(SpeciesId, f32)> = candidates
        .iter()
        .filter(|(_, fitness)| *fitness > 0.3)
        .copied()
        .collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    if ranked.is_empty() {
        return Vec::new();
    }

    // Step 2: Shannon 熵
    // H = -Σ(p_i × ln(p_i))   where p_i = fitness_i / Σfitness
    let total_fitness: f32 = ranked.iter().map(|(_, f)| f).sum();
    let entropy: f32 = -ranked
        .iter()
        .map(|(_, f)| {
            let p = f / total_fitness;
            if p > 0.0 {
                p * p.ln()
            } else {
                0.0_f32
            }
        })
        .sum::<f32>();

    // Step 3: 丰富度 = f(entropy, soil_fertility) — 连续函数，非硬编码整数
    // 高熵 + 高肥力 → 多物种共存
    // 低熵 + 低肥力 → 单优势种
    let richness = (entropy * soil_fertility * 8.0).ceil() as usize;
    let n = richness.clamp(1, 5);

    ranked.truncate(n);

    // Step 4: 归一化权重
    let weight_sum: f32 = ranked.iter().map(|(_, f)| f).sum();
    if weight_sum > 0.0 {
        ranked.iter().map(|(id, f)| (*id, f / weight_sum)).collect()
    } else {
        ranked
    }
}

// ── 测试 ─────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// 构造测试用候选集：多个相似适应度的物种
    fn rainforest_candidates() -> Vec<(SpeciesId, f32)> {
        vec![
            (SpeciesId(0), 0.85), // oak — varies by biome
            (SpeciesId(1), 0.80), // pine
            (SpeciesId(5), 0.75), // birch
            (SpeciesId(4), 0.72), // mangrove
            (SpeciesId(6), 0.68), // fern
        ]
    }

    /// 构造针叶林候选集：仅 1-2 个高适应度
    fn taiga_candidates() -> Vec<(SpeciesId, f32)> {
        vec![
            (SpeciesId(1), 0.90), // pine — dominant
            (SpeciesId(2), 0.85), // spruce — secondary
            (SpeciesId(0), 0.20), // oak — poor fit
            (SpeciesId(6), 0.15), // fern — poor fit
        ]
    }

    #[test]
    fn test_rainforest_high_diversity() {
        // 热带雨林：高肥力 → 应选出较多物种
        let result = select_dominant_species(&rainforest_candidates(), 0.9);
        assert!(
            result.len() >= 3,
            "rainforest should have >=3 species, got {}",
            result.len()
        );
        assert!(result.len() <= 5);
    }

    #[test]
    fn test_taiga_low_diversity() {
        // 针叶林：中肥力 → 应选出较少物种
        let result = select_dominant_species(&taiga_candidates(), 0.5);
        assert!(
            result.len() <= 2,
            "taiga should have <=2 species, got {}",
            result.len()
        );
        assert!(!result.is_empty());
        // pine (id=1) 应该排在第一位
        assert_eq!(result[0].0, SpeciesId(1));
    }

    #[test]
    fn test_deterministic() {
        let a = select_dominant_species(&rainforest_candidates(), 0.7);
        let b = select_dominant_species(&rainforest_candidates(), 0.7);
        assert_eq!(a.len(), b.len());
        for (i, (id_a, w_a)) in a.iter().enumerate() {
            assert_eq!(id_a, &b[i].0);
            assert!((w_a - b[i].1).abs() < 1e-9);
        }
    }

    #[test]
    fn test_empty_input() {
        let empty: Vec<(SpeciesId, f32)> = Vec::new();
        let result = select_dominant_species(&empty, 0.5);
        assert!(result.is_empty());
    }

    #[test]
    fn test_all_below_threshold() {
        // 所有候选适应度都 < 0.3 → 应返回空
        let poor: Vec<(SpeciesId, f32)> = vec![(SpeciesId(0), 0.25), (SpeciesId(1), 0.10)];
        let result = select_dominant_species(&poor, 0.5);
        assert!(result.is_empty());
    }

    #[test]
    fn test_weights_normalized() {
        let result = select_dominant_species(&rainforest_candidates(), 0.8);
        let sum: f32 = result.iter().map(|(_, w)| w).sum();
        assert!(
            (sum - 1.0).abs() < 0.01,
            "weights should sum to 1.0, got {}",
            sum
        );
    }
}
