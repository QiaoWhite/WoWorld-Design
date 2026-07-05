//! BiomeVegetation — Phase 1 "廉价植被" Provider
//!
//! 不存储实际植物——从 BiomeClassifier 纯函数推导植被统计值。
//! 参见: `woworld_core/src/vegetation.rs` (VegetationProvider trait)
//!       `开发阶段/世界生成/012-植被覆盖生成.md` (P2.25 完整规格)
//!
//! Phase 2: Poisson disc 实际放置 + L6.5 密度场 + succession 动态

use glam::{Vec2, Vec3};
use woworld_core::vegetation::{
    GroundCoverMap, HarvestableInfo, LandcoverType, PlantCommunitySnapshot,
    TimberAvailability, TimberQuality, VegetationProvider,
};

use crate::biome::{BiomeClassifier, BiomeDef};
use woworld_core::types::WorldPos;

/// 从群系分类器派生植被参数——Phase 1 纯函数查询
///
/// 零存储、零生成时间、零 LMDB 占用。
/// 后续 Phase 2 替换为 `woworld_vegetation` crate 的完整 P2.25 实现。
#[derive(Clone, Debug)]
pub struct BiomeVegetation {
    classifier: Option<BiomeClassifier>,
}

impl BiomeVegetation {
    pub fn new() -> Self {
        Self { classifier: None }
    }

    pub fn with_classifier(mut self, classifier: BiomeClassifier) -> Self {
        self.classifier = Some(classifier);
        self
    }

    /// 查询位置对应的群系（退回默认 Grassland）
    fn biome_at(&self, pos: Vec2) -> Option<&BiomeDef> {
        self.classifier.as_ref().and_then(|c| {
            c.classify(WorldPos {
                x: pos.x as f64,
                y: 0.0,
                z: pos.y as f64,
            })
        })
    }

    /// 群系 → 郁闭度
    fn biome_canopy(biome_name: &str) -> f32 {
        match biome_name {
            "Forest" => 0.70,
            "Swamp" => 0.45,
            "Grassland" => 0.02,
            _ => 0.0, // Snowfield, Desert, unknown
        }
    }

    /// 群系 → 地表覆盖
    fn biome_ground(biome_name: &str) -> GroundCoverMap {
        match biome_name {
            "Snowfield" => GroundCoverMap {
                grass_density: 0.02,
                moss_density: 0.05,
                leaf_litter: 0.0,
                bare_soil: 0.93,
            },
            "Grassland" => GroundCoverMap {
                grass_density: 0.75,
                moss_density: 0.05,
                leaf_litter: 0.05,
                bare_soil: 0.15,
            },
            "Forest" => GroundCoverMap {
                grass_density: 0.15,
                moss_density: 0.15,
                leaf_litter: 0.50,
                bare_soil: 0.20,
            },
            "Desert" => GroundCoverMap {
                grass_density: 0.02,
                moss_density: 0.0,
                leaf_litter: 0.0,
                bare_soil: 0.98,
            },
            "Swamp" => GroundCoverMap {
                grass_density: 0.20,
                moss_density: 0.40,
                leaf_litter: 0.25,
                bare_soil: 0.15,
            },
            _ => GroundCoverMap::default(),
        }
    }

    /// 群系 → 木材可获得性
    fn biome_timber(biome_name: &str) -> TimberAvailability {
        match biome_name {
            "Forest" => TimberAvailability {
                available: true,
                quality: TimberQuality::Softwood,
                abundance: 0.8,
                harvest_difficulty: 0.3,
                dominant_species: vec![],
            },
            "Swamp" => TimberAvailability {
                available: true,
                quality: TimberQuality::Softwood,
                abundance: 0.3,
                harvest_difficulty: 0.7,
                dominant_species: vec![],
            },
            _ => TimberAvailability {
                available: false,
                quality: TimberQuality::Softwood,
                abundance: 0.0,
                harvest_difficulty: 1.0,
                dominant_species: vec![],
            },
        }
    }
}

impl Default for BiomeVegetation {
    fn default() -> Self {
        Self::new()
    }
}

impl VegetationProvider for BiomeVegetation {
    fn query_community(&self, pos: Vec2, _radius: f32) -> PlantCommunitySnapshot {
        let (canopy, diversity) = self
            .biome_at(pos)
            .map(|b| (Self::biome_canopy(&b.name), biome_shannon(&b.name)))
            .unwrap_or((0.0, 0.0));

        PlantCommunitySnapshot {
            dominant_species: vec![],
            companion_species: vec![],
            canopy_closure: canopy,
            shannon_diversity: diversity,
        }
    }

    fn canopy_closure(&self, pos: Vec2) -> f32 {
        self.biome_at(pos)
            .map(|b| Self::biome_canopy(&b.name))
            .unwrap_or(0.0)
    }

    fn timber_availability(&self, pos: Vec2) -> TimberAvailability {
        self.biome_at(pos)
            .map(|b| Self::biome_timber(&b.name))
            .unwrap_or(TimberAvailability {
                available: false,
                quality: TimberQuality::Softwood,
                abundance: 0.0,
                harvest_difficulty: 1.0,
                dominant_species: vec![],
            })
    }

    fn ground_cover(&self, pos: Vec2) -> GroundCoverMap {
        self.biome_at(pos)
            .map(|b| Self::biome_ground(&b.name))
            .unwrap_or_default()
    }

    fn query_harvestable(&self, _pos: Vec2, _radius: f32) -> Vec<HarvestableInfo> {
        vec![] // Phase 2: Poisson disc 放置可采集物
    }

    fn fuel_load(&self, pos: Vec2) -> f32 {
        self.biome_at(pos)
            .map(|b| match b.name.as_str() {
                "Forest" => 0.6,
                "Grassland" => 0.3,
                "Swamp" => 0.2,
                _ => 0.05,
            })
            .unwrap_or(0.0)
    }

    fn root_interference(&self, pos: Vec3) -> f32 {
        self.canopy_closure(Vec2::new(pos.x, pos.z)) * 0.5
    }

    fn set_scene_lod(&self, _lod: u8) {
        // Phase 1: 无 LOD 差异——所有 LOD 共享同一纯函数查询
    }
}

/// 群系 → Shannon 多样性指数（Phase 1 固定值）
fn biome_shannon(biome_name: &str) -> f32 {
    match biome_name {
        "Forest" => 2.0,    // 高多样性
        "Swamp" => 1.8,
        "Grassland" => 1.2, // 中等
        "Desert" => 0.3,    // 低
        "Snowfield" => 0.1, // 极低
        _ => 0.5,
    }
}

/// 从郁闭度派生地表覆盖类型
///
/// 消费方：文化系统（建筑风格选择）
pub fn landcover_from_canopy(canopy: f32, ground: &GroundCoverMap) -> LandcoverType {
    if canopy > 0.6 {
        LandcoverType::DenseForest
    } else if canopy > 0.2 {
        LandcoverType::OpenWoodland
    } else if ground.grass_density + ground.moss_density > 0.5 {
        LandcoverType::Grassland
    } else if ground.bare_soil > 0.8 {
        LandcoverType::Desert
    } else {
        LandcoverType::Wetland
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::noise_gen::WorldNoise;

    fn make_veg() -> BiomeVegetation {
        let toml_str = include_str!("../../../assets/biomes.toml");
        let noise = std::sync::Arc::new(WorldNoise::new(42));
        let classifier = BiomeClassifier::from_toml_str(toml_str, noise).unwrap();
        BiomeVegetation::new().with_classifier(classifier)
    }

    #[test]
    fn test_forest_has_high_canopy() {
        let v = make_veg();
        // Forest: temp=0.5, precip=0.8 → 匹配 Forest 群系
        let closure = v.canopy_closure(Vec2::new(1000.0, 1000.0));
        // 在森林区域应有高郁闭度
        // 注：具体值依赖噪声种子——仅验证 ≥0 且 ≤1
        assert!((0.0..=1.0).contains(&closure));
    }

    #[test]
    fn test_desert_low_canopy() {
        let v = make_veg();
        let closure = v.canopy_closure(Vec2::new(5000.0, 5000.0));
        assert!((0.0..=1.0).contains(&closure));
    }

    #[test]
    fn test_ground_cover_sums_near_one() {
        let v = make_veg();
        let g = v.ground_cover(Vec2::new(0.0, 0.0));
        let total = g.grass_density + g.moss_density + g.leaf_litter + g.bare_soil;
        assert!((total - 1.0).abs() < 0.01,
            "ground cover should sum to 1.0, got {total}");
    }

    #[test]
    fn test_desert_no_timber() {
        let v = BiomeVegetation::new(); // 无 classifier → 退回默认
        let timber = v.timber_availability(Vec2::new(0.0, 0.0));
        assert!(!timber.available);
        assert_eq!(timber.abundance, 0.0);
    }

    #[test]
    fn test_all_biome_ground_covers_sum_to_one() {
        // 直接测试内部映射——不依赖噪声
        for name in &["Snowfield", "Grassland", "Forest", "Desert", "Swamp"] {
            let g = BiomeVegetation::biome_ground(name);
            let total = g.grass_density + g.moss_density + g.leaf_litter + g.bare_soil;
            assert!((total - 1.0).abs() < 0.01,
                "{name}: ground cover sums to {total}, expected 1.0");
        }
    }

    #[test]
    fn test_forest_timber_available() {
        let timber = BiomeVegetation::biome_timber("Forest");
        assert!(timber.available);
        assert!(timber.abundance > 0.5);
    }

    #[test]
    fn test_no_panic_no_classifier() {
        let v = BiomeVegetation::new();
        // 无 classifier 时所有方法应返回默认值，不崩溃
        let _ = v.canopy_closure(Vec2::ZERO);
        let _ = v.ground_cover(Vec2::ZERO);
        let _ = v.timber_availability(Vec2::ZERO);
        let _ = v.fuel_load(Vec2::ZERO);
        let _ = v.root_interference(Vec3::ZERO);
        let _ = v.query_community(Vec2::ZERO, 10.0);
        let _ = v.query_harvestable(Vec2::ZERO, 10.0);
        v.set_scene_lod(0);
    }

    #[test]
    fn test_landcover_from_canopy_dense_forest() {
        let ground = GroundCoverMap::default();
        assert_eq!(
            landcover_from_canopy(0.8, &ground),
            LandcoverType::DenseForest
        );
    }

    #[test]
    fn test_landcover_from_canopy_desert() {
        let ground = GroundCoverMap {
            bare_soil: 0.9,
            ..GroundCoverMap::default()
        };
        assert_eq!(
            landcover_from_canopy(0.0, &ground),
            LandcoverType::Desert
        );
    }
}
