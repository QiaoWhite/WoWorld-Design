//! VegetationProvider 存根实现
//!
//! MVP 阶段所有查询方法返回默认值。
//! 后续完整 P2.25 管线实现后替换为真实 LMDB-backed 实现。
//!
//! 参见: `WoWorld-Design/Happy Game/开发阶段/生命/010-植被群落与覆盖.md`

use std::sync::RwLock;

use glam::{Vec2, Vec3};

use woworld_core::vegetation::{
    GroundCoverMap, HarvestableInfo, LandcoverType, PlantCommunitySnapshot, TimberAvailability,
    TimberQuality, VegetationProvider,
};

use crate::species::SpeciesTable;

/// MVP 存根实现——所有方法返回默认/空值。
///
/// Shannon 熵筛选独立为纯函数（`community::select_dominant_species`），
/// 不与此存根绑定。
pub struct VegetationStub {
    species_table: SpeciesTable,
    scene_lod: RwLock<u8>,
}

impl VegetationStub {
    pub fn new() -> Self {
        Self {
            species_table: SpeciesTable::load(),
            scene_lod: RwLock::new(0),
        }
    }

    /// 获取物种表（供外部调用 Shannon 熵筛选）
    #[allow(dead_code)]
    pub(crate) fn species_table(&self) -> &SpeciesTable {
        &self.species_table
    }
}

impl Default for VegetationStub {
    fn default() -> Self {
        Self::new()
    }
}

impl VegetationProvider for VegetationStub {
    fn query_community(&self, _pos: Vec2, _radius: f32) -> PlantCommunitySnapshot {
        PlantCommunitySnapshot {
            dominant_species: Vec::new(),
            companion_species: Vec::new(),
            canopy_closure: 0.0,
            shannon_diversity: 0.0,
        }
    }

    fn canopy_closure(&self, _pos: Vec2) -> f32 {
        0.0
    }

    fn timber_availability(&self, _pos: Vec2) -> TimberAvailability {
        TimberAvailability {
            available: false,
            quality: TimberQuality::Softwood,
            abundance: 0.0,
            harvest_difficulty: 1.0,
            dominant_species: Vec::new(),
        }
    }

    fn ground_cover(&self, _pos: Vec2) -> GroundCoverMap {
        GroundCoverMap::default()
    }

    fn query_harvestable(&self, _pos: Vec2, _radius: f32) -> Vec<HarvestableInfo> {
        Vec::new()
    }

    fn fuel_load(&self, _pos: Vec2) -> f32 {
        0.0
    }

    fn root_interference(&self, _pos: Vec3) -> f32 {
        0.0
    }

    fn set_scene_lod(&self, lod: u8) {
        if let Ok(mut sl) = self.scene_lod.write() {
            *sl = lod;
        }
    }
}

// ── 辅助函数 ────────────────────────────────────────────

/// 从 canopy_closure 推导地表覆盖类型（供文化系统使用）
///
/// 参见: `WoWorld-Design/Happy Game/开发阶段/生命/010-植被群落与覆盖.md`
#[allow(dead_code)]
pub fn classify_landcover(canopy_closure: f32, grass_density: f32) -> LandcoverType {
    if canopy_closure > 0.6 {
        LandcoverType::DenseForest
    } else if canopy_closure > 0.2 {
        LandcoverType::OpenWoodland
    } else if grass_density > 0.4 {
        LandcoverType::Grassland
    } else {
        LandcoverType::Desert
    }
}

// ── 测试 ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub_returns_defaults() {
        let provider = VegetationStub::new();
        assert_eq!(provider.canopy_closure(Vec2::ZERO), 0.0);
        assert_eq!(provider.fuel_load(Vec2::ZERO), 0.0);
        assert_eq!(provider.root_interference(Vec3::ZERO), 0.0);
    }

    #[test]
    fn test_stub_loads_species_table() {
        let provider = VegetationStub::new();
        let table = provider.species_table();
        assert!(table.len() >= 5);
    }

    #[test]
    fn test_landcover_classification() {
        assert_eq!(classify_landcover(0.8, 0.3), LandcoverType::DenseForest);
        assert_eq!(classify_landcover(0.4, 0.3), LandcoverType::OpenWoodland);
        assert_eq!(classify_landcover(0.1, 0.6), LandcoverType::Grassland);
        assert_eq!(classify_landcover(0.1, 0.1), LandcoverType::Desert);
    }
}
