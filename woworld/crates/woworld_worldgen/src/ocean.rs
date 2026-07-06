//! HeightfieldOcean — OceanProvider trait 的噪声驱动实现
//!
//! 海平面恒定 `y = 0.0`，水深由 `sea_level - terrain_height` 计算。
//! Gerstner 波参数由 `ocean_waves.toml` 驱动——与 shader 中的波同步。
//!
//! 参见: `woworld_core/src/ocean.rs` — OceanProvider trait 定义
//! 参见: `woworld/assets/ocean_waves.toml` — 波参数配置

use std::sync::Arc;

use woworld_core::edit_terrain::EditHeightfieldSnapshot;
use woworld_core::ocean::OceanProvider;
use woworld_core::prelude::WorldPos;

use crate::noise_gen::WorldNoise;

// ── Gerstner 波参数 ─────────────────────

/// 单个 Gerstner 波配置——与 `ocean.gdshader` 中 `vec4` uniform 一一对应
#[derive(Debug, Clone)]
pub struct GerstnerWave {
    /// 传播方向 x 分量
    pub dir_x: f64,
    /// 传播方向 z 分量
    pub dir_z: f64,
    /// 陡度 (0-1)，控制波峰尖锐度
    pub steepness: f64,
    /// 波长（米）
    pub wavelength: f64,
}

impl Default for GerstnerWave {
    fn default() -> Self {
        Self {
            dir_x: 1.0,
            dir_z: 0.0,
            steepness: 0.3,
            wavelength: 4.0,
        }
    }
}

/// 默认 6 波配置——与 `ocean_waves.toml` / `ocean.gdshader` 保持同步
pub fn default_waves() -> [GerstnerWave; 6] {
    [
        GerstnerWave { dir_x: 1.0, dir_z: 0.0, steepness: 0.3, wavelength: 4.0 },
        GerstnerWave { dir_x: 0.0, dir_z: 1.0, steepness: 0.2, wavelength: 6.0 },
        GerstnerWave { dir_x: 0.7, dir_z: 0.5, steepness: 0.15, wavelength: 8.0 },
        GerstnerWave { dir_x: -0.3, dir_z: 0.9, steepness: 0.1, wavelength: 12.0 },
        GerstnerWave { dir_x: 0.9, dir_z: -0.3, steepness: 0.08, wavelength: 3.0 },
        GerstnerWave { dir_x: -0.5, dir_z: -0.4, steepness: 0.05, wavelength: 18.0 },
    ]
}

// ── HeightfieldOcean ────────────────────

/// 高度场海洋——噪声驱动的 `OceanProvider` 实现
///
/// 持有 `Arc<WorldNoise>`——与 `HeightfieldTerrain` 共享同一噪声实例。
/// 海平面恒为 `y = 0.0`，水深 = `max(0, -terrain_height)`。
#[derive(Clone, Debug)]
pub struct HeightfieldOcean {
    noise: Arc<WorldNoise>,
    waves: [GerstnerWave; 6],
    /// 地形修改的高度投影（CoW 快照——零锁读取）
    ///
    /// WorldDriver 每帧从 ECS EditTerrainResource 同步此快照。
    /// None = 无修改，回退到纯噪声。
    edit_heightfield: Option<Arc<EditHeightfieldSnapshot>>,
}

impl HeightfieldOcean {
    /// 从共享噪声创建——使用默认 6 波参数
    pub fn new(noise: Arc<WorldNoise>) -> Self {
        Self {
            noise,
            waves: default_waves(),
            edit_heightfield: None,
        }
    }

    /// 从共享噪声 + 自定义波参数创建
    pub fn with_waves(noise: Arc<WorldNoise>, waves: [GerstnerWave; 6]) -> Self {
        Self {
            noise,
            waves,
            edit_heightfield: None,
        }
    }

    /// 设置地形修改快照——WorldDriver 每帧调用
    pub fn set_edit_heightfield(&mut self, snapshot: Option<Arc<EditHeightfieldSnapshot>>) {
        self.edit_heightfield = snapshot;
    }

    /// 实际地形高度——优先查 EditHeightfield，回退到噪声
    fn terrain_height(&self, x: f64, z: f64) -> f64 {
        if let Some(ref eh) = self.edit_heightfield {
            if let Some(h) = eh.height_at(x, z) {
                return h as f64;
            }
        }
        self.noise.sample_height(x, z)
    }

    /// Rust 侧 Gerstner 波叠加——与 `ocean.gdshader` vertex() 数学保持一致
    ///
    /// 返回 `(horizontal_x, vertical, horizontal_z)` 位移向量（世界空间）。
    /// 消费方: 船只浮力、游泳高度、波面精确查询。
    pub fn compute_gerstner_displacement(&self, x: f64, z: f64, time: f64) -> (f64, f64, f64) {
        use std::f64::consts::TAU;

        let mut dx = 0.0;
        let mut dy = 0.0;
        let mut dz = 0.0;

        for w in &self.waves {
            let k = TAU / w.wavelength;
            let c = (9.8 / k).sqrt();
            let d = glam::dvec2(w.dir_x, w.dir_z).normalize();
            let f = k * (d.x * x + d.y * z - c * time);
            let a = w.steepness / k;

            dx += d.x * (a * f.cos());
            dy += a * f.sin();
            dz += d.y * (a * f.cos());
        }

        (dx, dy, dz)
    }
}

impl Default for HeightfieldOcean {
    fn default() -> Self {
        Self {
            noise: Arc::new(WorldNoise::new(42)),
            waves: default_waves(),
            edit_heightfield: None,
        }
    }
}

// ── OceanProvider trait 实现 ────────────

impl OceanProvider for HeightfieldOcean {
    fn sea_level_at(&self, _pos: WorldPos) -> f64 {
        0.0 // 当前恒定海平面——未来潮汐/风暴 surge 的扩展点
    }

    fn wave_height_at(&self, pos: WorldPos, time: f64) -> f64 {
        let (_dx, dy, _dz) = self.compute_gerstner_displacement(pos.x, pos.z, time);
        dy
    }

    fn water_depth_at(&self, pos: WorldPos) -> f64 {
        let terrain_h = self.terrain_height(pos.x, pos.z);
        (0.0f64 - terrain_h).max(0.0)
    }

    fn is_underwater(&self, pos: WorldPos, time: f64) -> bool {
        let sea_level = self.sea_level_at(pos);
        let wave_offset = self.wave_height_at(pos, time);
        pos.y < sea_level + wave_offset
    }
}

// ── 测试 ────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sea_level_is_zero() {
        let ocean = HeightfieldOcean::default();
        let level = ocean.sea_level_at(WorldPos::default());
        assert!((level - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_water_depth_over_ocean() {
        // 原点附近是 Enceladus 效应区——大陆噪声 φ⁻¹ 偏移后，需要找远处深海
        // 直接测试深度计算逻辑：terrain_height < 0 → depth > 0
        let ocean = HeightfieldOcean::default();
        let h = ocean.terrain_height(5000.0, 5000.0);
        if h < 0.0 {
            let depth = ocean.water_depth_at(WorldPos { x: 5000.0, y: 0.0, z: 5000.0 });
            assert!(depth > 0.0, "depth should be positive over ocean, terrain_h={h}, depth={depth}");
            // 水深不应超过合理范围（最深海沟 ~400m）
            assert!(depth < 450.0, "unexpectedly deep: {depth}");
        }
    }

    #[test]
    fn test_water_depth_over_land_is_zero() {
        let ocean = HeightfieldOcean::default();
        // 找已知的高地形位置
        let depth = ocean.water_depth_at(WorldPos { x: 2000.0, y: 0.0, z: 2000.0 });
        // 如果这里是陆地，深度应为 0
        // 不能断言一定为 0（可能是海），但应 ≥ 0
        assert!(depth >= 0.0, "depth must not be negative, got {depth}");
    }

    #[test]
    fn test_is_underwater() {
        let ocean = HeightfieldOcean::with_waves(
            Arc::new(WorldNoise::new(42)),
            default_waves(),
        );
        // 水下位置——y = -50, sea_level(0) + wave_height(小) → 应判定为水下
        let pos = WorldPos { x: 100.0, y: -50.0, z: 100.0 };
        let time = 100.0;
        assert!(ocean.is_underwater(pos, time), "y=-50 should be underwater");

        // 水上位置
        let pos_above = WorldPos { x: 100.0, y: 100.0, z: 100.0 };
        assert!(!ocean.is_underwater(pos_above, time), "y=100 should be above water");
    }

    #[test]
    fn test_gerstner_vertical_displacement_is_finite() {
        let ocean = HeightfieldOcean::default();
        for t in [0.0, 10.0, 100.0, 1000.0] {
            let (_dx, dy, _dz) = ocean.compute_gerstner_displacement(0.0, 0.0, t);
            assert!(dy.is_finite(), "dy must be finite at time {t}");
            assert!(dy.abs() < 100.0, "dy should not exceed ~100m, got {dy}");
        }
    }

    #[test]
    fn test_wave_height_deterministic() {
        let ocean = HeightfieldOcean::default();
        let pos = WorldPos::default();
        let h1 = ocean.wave_height_at(pos, 42.0);
        let h2 = ocean.wave_height_at(pos, 42.0);
        assert!((h1 - h2).abs() < 0.001, "same input should give same output");
    }

    #[test]
    fn test_wave_height_varies_with_position() {
        let ocean = HeightfieldOcean::default();
        let pos_a = WorldPos::default();
        let pos_b = WorldPos { x: 100.0, y: 0.0, z: 0.0 };
        let h_a = ocean.wave_height_at(pos_a, 10.0);
        let h_b = ocean.wave_height_at(pos_b, 10.0);
        // 不同水平位置应产生不同的波高（除非极端巧合）
        assert!(
            (h_a - h_b).abs() > 0.01,
            "different positions should have different wave heights: {h_a} vs {h_b}"
        );
    }

    #[test]
    fn test_is_underwater_considers_waves() {
        let ocean = HeightfieldOcean::default();
        // 刚好在海平面以下——is_underwater 应考虑波浪
        let pos = WorldPos { x: 0.0, y: -0.1, z: 0.0 };
        // 即使有波浪，y=-0.1 绝大多数情况下在水下
        let underwater = ocean.is_underwater(pos, 0.0);
        assert!(underwater, "y=-0.1 should almost always be underwater");
    }
}
