//! 群系分类系统
//!
//! 温度×降水 2D 噪声 → 5 群系硬盒分类。
//! TOML 数据驱动，`include_str!` 编译期嵌入——零运行时 I/O。
//!
//! 参见: `WoWorld-Design/开发路线图/002-轨A-正式开发.md` A.4c

use std::sync::Arc;

use serde::Deserialize;
use woworld_core::material::SurfaceMaterial;
use woworld_core::prelude::WorldPos;

use crate::noise_gen::WorldNoise;

// ── TOML 反序列化类型 ──────────────────────────

/// TOML 中的群系定义（serde 反序列化）
#[derive(Debug, Clone, Deserialize)]
pub struct BiomeDef {
    pub name: String,
    pub temp_min: f64,
    pub temp_max: f64,
    pub precip_min: f64,
    pub precip_max: f64,
    pub surface_material: SurfaceMaterial,
    #[serde(default)]
    pub sub_materials: Vec<SubMaterialWeight>,
}

/// 群系内子材质权重
#[derive(Debug, Clone, Deserialize)]
pub struct SubMaterialWeight {
    pub material: SurfaceMaterial,
    /// 相对权重 (0.0-1.0)
    pub weight: f64,
    /// 触发条件: "steep" | "flat" | "low" | "high" | "any"
    pub condition: String,
}

/// TOML 文件顶层结构
#[derive(Debug, Clone, Deserialize)]
struct BiomesToml {
    #[serde(default)]
    biome: Vec<BiomeDef>,
}

// ── 运行时群系分类器 ───────────────────────────

/// 群系分类器——从 WorldNoise 采样温度/降水，匹配最近群系
#[derive(Clone, Debug)]
pub struct BiomeClassifier {
    biomes: Vec<BiomeDef>,
    noise: Arc<WorldNoise>,
}

impl BiomeClassifier {
    /// 从 TOML 字符串构建（编译期嵌入）
    ///
    /// `toml_str`: `include_str!("../../../assets/biomes.toml")` 的内容
    /// `noise`: 已构建的 WorldNoise（含温度/降水层）——与 HeightfieldTerrain 共享 Arc
    pub fn from_toml_str(toml_str: &str, noise: Arc<WorldNoise>) -> Result<Self, String> {
        let config: BiomesToml =
            toml::from_str(toml_str).map_err(|e| format!("Failed to parse biomes.toml: {}", e))?;

        if config.biome.is_empty() {
            return Err("biomes.toml must contain at least one [[biome]]".into());
        }

        Ok(Self {
            biomes: config.biome,
            noise,
        })
    }

    /// 分类世界坐标 → 匹配的群系定义
    ///
    /// 返回 `None` 表示 T/P 空间存在空档——调用方应回退到高度法
    pub fn classify(&self, pos: WorldPos) -> Option<&BiomeDef> {
        let temp = self.noise.sample_temperature(pos.x, pos.z);
        let precip = self.noise.sample_precipitation(pos.x, pos.z);

        self.classify_tp(temp, precip)
    }

    /// 按温度+降水值分类（测试用）
    fn classify_tp(&self, temp: f64, precip: f64) -> Option<&BiomeDef> {
        self.biomes.iter().find(|b| {
            temp >= b.temp_min
                && temp < b.temp_max
                && precip >= b.precip_min
                && precip < b.precip_max
        })
    }

    /// 查询位置的温度 (0.0-1.0)
    pub fn temperature_at(&self, pos: WorldPos) -> f64 {
        self.noise.sample_temperature(pos.x, pos.z)
    }

    /// 查询位置的降水 (0.0-1.0)
    pub fn precipitation_at(&self, pos: WorldPos) -> f64 {
        self.noise.sample_precipitation(pos.x, pos.z)
    }

    /// 采样地形高度（委托 WorldNoise）——供植被/采集物放置使用
    ///
    /// 返回该 (x, z) 处的程序化地形高度（米）。不含运行时地形修改。
    pub fn sample_height(&self, x: f64, z: f64) -> f64 {
        self.noise.sample_height(x, z)
    }
}

// ── 测试 ────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::noise_gen::WorldNoise;

    // Test-only: moved from main impl (P2c 群系细化将正式实现)
    impl BiomeClassifier {
        pub fn sub_material_matches(condition: &str, _height: f64, steepness: f32) -> bool {
            match condition {
                "steep" => steepness > 0.6,
                "flat" => steepness < 0.15,
                "low" => true,
                "high" => true,
                "any" => true,
                _ => false,
            }
        }
    }

    fn test_toml() -> &'static str {
        r#"
climate_scale = 0.005

[[biome]]
name = "Frost"
temp_min = 0.0
temp_max = 0.3
precip_min = 0.0
precip_max = 0.5
surface_material = "Snow"
sub_materials = [
    { material = "Ice", weight = 0.2, condition = "steep" },
]

[[biome]]
name = "Green"
temp_min = 0.3
temp_max = 0.7
precip_min = 0.2
precip_max = 0.8
surface_material = "Grass"

[[biome]]
name = "Arid"
temp_min = 0.7
temp_max = 1.0
precip_min = 0.0
precip_max = 0.3
surface_material = "Sand"
"#
    }

    fn make_classifier() -> BiomeClassifier {
        let noise = WorldNoise::new(42);
        BiomeClassifier::from_toml_str(test_toml(), Arc::new(noise)).unwrap()
    }

    #[test]
    fn test_parse_and_classify() {
        let bc = make_classifier();

        // Cold + Dry = Frost
        let frost = bc.classify_tp(0.15, 0.25);
        assert!(frost.is_some());
        assert_eq!(frost.unwrap().name, "Frost");

        // Mild + Wet = Green
        let green = bc.classify_tp(0.5, 0.5);
        assert!(green.is_some());
        assert_eq!(green.unwrap().name, "Green");

        // Hot + Dry = Arid
        let arid = bc.classify_tp(0.85, 0.1);
        assert!(arid.is_some());
        assert_eq!(arid.unwrap().name, "Arid");
    }

    #[test]
    fn test_gap_returns_none() {
        let bc = make_classifier();
        // T/P 空间的空档（不在任何群系范围内）
        let gap = bc.classify_tp(0.15, 0.75); // cold + wet — not defined
        assert!(gap.is_none());
    }

    #[test]
    fn test_surface_material_deserialize() {
        let bc = make_classifier();
        let frost = bc.classify_tp(0.1, 0.1).unwrap();
        assert_eq!(frost.surface_material, SurfaceMaterial::Snow);

        let green = bc.classify_tp(0.5, 0.5).unwrap();
        assert_eq!(green.surface_material, SurfaceMaterial::Grass);

        let arid = bc.classify_tp(0.85, 0.1).unwrap();
        assert_eq!(arid.surface_material, SurfaceMaterial::Sand);
    }

    #[test]
    fn test_sub_material_conditions() {
        assert!(BiomeClassifier::sub_material_matches("steep", 0.0, 0.8));
        assert!(!BiomeClassifier::sub_material_matches("steep", 0.0, 0.3));
        assert!(BiomeClassifier::sub_material_matches("flat", 0.0, 0.05));
        assert!(!BiomeClassifier::sub_material_matches("flat", 0.0, 0.4));
        assert!(BiomeClassifier::sub_material_matches("any", 0.0, 0.5));
        assert!(!BiomeClassifier::sub_material_matches("bogus", 0.0, 0.5));
    }

    #[test]
    fn test_empty_biomes_error() {
        let result = BiomeClassifier::from_toml_str("", Arc::new(WorldNoise::new(1)));
        assert!(result.is_err());
    }
}
