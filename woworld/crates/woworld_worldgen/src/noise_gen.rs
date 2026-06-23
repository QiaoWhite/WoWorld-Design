//! 双层 Perlin 噪声地形生成器
//!
//! 三层噪声叠加：
//! - continent (~100km 波长): 海陆分布
//! - detail (~100m 波长): 地形起伏
//! - mountain (~500m 波长): 山脊

use noise::{NoiseFn, Perlin};

/// 噪声参数——可调
#[derive(Clone, Debug)]
pub struct NoiseParams {
    pub continent_scale: f64,  // 默认 0.00001 (100km 波长)
    pub detail_scale: f64,     // 默认 0.01 (100m 波长)
    pub mountain_scale: f64,   // 默认 0.002 (500m 波长)
    pub sea_threshold: f64,    // 默认 0.3 (海:陆≈7:3)
    pub height_amplitude: f64, // 默认 350.0 (最高~700m)
    pub sea_depth: f64,        // 默认 200.0 (最深-200m)
}

impl Default for NoiseParams {
    fn default() -> Self {
        Self {
            continent_scale: 0.00001,
            detail_scale: 0.01,
            mountain_scale: 0.002,
            sea_threshold: 0.3,
            height_amplitude: 350.0,
            sea_depth: 200.0,
        }
    }
}

/// 世界噪声生成器
#[derive(Clone, Debug)]
pub struct WorldNoise {
    continent: Perlin,
    detail: Perlin,
    mountain: Perlin,
    pub params: NoiseParams,
}

impl WorldNoise {
    /// 使用种子创建——不同种子 → 不同世界
    pub fn new(seed: u32) -> Self {
        Self {
            continent: Perlin::new(seed),
            detail: Perlin::new(seed.wrapping_add(1)),
            mountain: Perlin::new(seed.wrapping_add(2)),
            params: NoiseParams::default(),
        }
    }

    pub fn with_params(seed: u32, params: NoiseParams) -> Self {
        Self {
            continent: Perlin::new(seed),
            detail: Perlin::new(seed.wrapping_add(1)),
            mountain: Perlin::new(seed.wrapping_add(2)),
            params,
        }
    }

    /// 采样 (x, z) 处的地形高度（米）
    ///
    /// 算法:
    /// 1. 大陆噪声 → continent_value ∈ [-1, 1]
    /// 2. continent_value > sea_threshold → 陆地，否则 → 海洋
    /// 3. 陆地: 叠加 detail + mountain 噪声 → 高度
    /// 4. 海洋: 负高度（海床）
    pub fn sample_height(&self, x: f64, z: f64) -> f64 {
        let p = &self.params;
        let continent_val = self
            .continent
            .get([x * p.continent_scale, z * p.continent_scale]);

        if continent_val > p.sea_threshold {
            // 陆地——叠层
            let land_factor = (continent_val - p.sea_threshold) / (1.0 - p.sea_threshold); // 0→1 越接近大陆中心越高

            let detail_val = self.detail.get([x * p.detail_scale, z * p.detail_scale]);

            let mountain_val = self
                .mountain
                .get([x * p.mountain_scale, z * p.mountain_scale]);

            // mountain 在山脊区域（detail > 0.3）更突出
            let mountain_factor = if detail_val > 0.3 {
                (detail_val - 0.3) / 0.7
            } else {
                0.0
            };

            let base_height = land_factor * 100.0; // 海岸→内陆基准上升
            let detail_height = detail_val * p.height_amplitude * 0.6;
            let mountain_height = mountain_val * mountain_factor * p.height_amplitude * 0.4;

            base_height + detail_height + mountain_height
        } else {
            // 海洋——海床深度
            let sea_factor = (p.sea_threshold - continent_val) / (p.sea_threshold + 1.0); // 0→1 越远离海岸越深
            -sea_factor * p.sea_depth
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deterministic() {
        let n1 = WorldNoise::new(42);
        let n2 = WorldNoise::new(42);
        for x in (0..100).step_by(10) {
            for z in (0..100).step_by(10) {
                assert_eq!(
                    n1.sample_height(x as f64, z as f64),
                    n2.sample_height(x as f64, z as f64)
                );
            }
        }
    }

    #[test]
    fn test_different_seeds_different_worlds() {
        let n1 = WorldNoise::new(42);
        let n2 = WorldNoise::new(99);
        // 至少有一半的点不同（统计概率极高）
        let mut diff = 0;
        for x in 0..50 {
            for z in 0..50 {
                let h1 = n1.sample_height(x as f64 * 10.0, z as f64 * 10.0);
                let h2 = n2.sample_height(x as f64 * 10.0, z as f64 * 10.0);
                if (h1 - h2).abs() > 0.01 {
                    diff += 1;
                }
            }
        }
        assert!(diff > 1250); // >50%
    }

    #[test]
    fn test_height_range() {
        let n = WorldNoise::new(123);
        let mut min_h = f64::MAX;
        let mut max_h = f64::MIN;
        for x in 0..100 {
            for z in 0..100 {
                let h = n.sample_height(x as f64 * 50.0, z as f64 * 50.0);
                min_h = min_h.min(h);
                max_h = max_h.max(h);
            }
        }
        // 高度范围合理
        assert!(min_h >= -250.0, "min too low: {}", min_h);
        assert!(max_h <= 800.0, "max too high: {}", max_h);
    }
}
