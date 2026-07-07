//! CultureRegistry — 文化数据 SoA 存储 + CultureQuery 实现

use woworld_core::culture::beauty::CulturalBeautyStandard;
use woworld_core::culture::building::BuildingStylePreferences;
use woworld_core::culture::communication::CommunicationNorms;
use woworld_core::culture::dietary::DietaryBasePreferences;
use woworld_core::culture::fertility::FertilityNorms;
use woworld_core::culture::relationship::RelationshipNorms;
use woworld_core::culture::{CultureCoreParams, CultureId, CultureQuery, CULTURE_ID_NONE};

#[derive(Debug, Default)]
pub struct CultureRegistry {
    cores: Vec<CultureCoreParams>, communication_norms: Vec<CommunicationNorms>,
    building_styles: Vec<BuildingStylePreferences>, beauty_standards: Vec<CulturalBeautyStandard>,
    fertility_norms: Vec<FertilityNorms>, dietary_prefs: Vec<DietaryBasePreferences>,
    relationship_norms: Vec<RelationshipNorms>, valid: Vec<bool>,
    culture_ids: Vec<CultureId>, next_id: u32,
}

impl CultureRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn register(&mut self, params: CultureCoreParams) -> CultureId {
        assert!(self.next_id < u32::MAX, "CultureId overflow");
        let id = CultureId(self.next_id); self.next_id += 1;
        let comm = CommunicationNorms::derive_from(&params);
        let building = BuildingStylePreferences::derive_from(&params, false, false, false, false, false, false);
        let beauty = CulturalBeautyStandard::derive_from(&params);
        let fertility = FertilityNorms::derive_from(&params);
        let dietary = DietaryBasePreferences::derive_from(&params, false, false);
        let relationship = RelationshipNorms::derive_from(&params);
        self.cores.push(params); self.communication_norms.push(comm); self.building_styles.push(building);
        self.beauty_standards.push(beauty); self.fertility_norms.push(fertility); self.dietary_prefs.push(dietary);
        self.relationship_norms.push(relationship); self.valid.push(true); self.culture_ids.push(id);
        id
    }

    #[allow(clippy::too_many_arguments)]
    pub fn register_with_biome(&mut self, params: CultureCoreParams, is_arid: bool, is_arctic: bool,
        is_forested: bool, is_desert: bool, is_cold: bool, is_warm: bool, is_grassland: bool, is_tropical: bool) -> CultureId {
        assert!(self.next_id < u32::MAX); let id = CultureId(self.next_id); self.next_id += 1;
        let comm = CommunicationNorms::derive_from(&params);
        let building = BuildingStylePreferences::derive_from(&params, is_arid, is_arctic, is_forested, is_desert, is_cold, is_warm);
        let beauty = CulturalBeautyStandard::derive_from(&params);
        let fertility = FertilityNorms::derive_from(&params);
        let dietary = DietaryBasePreferences::derive_from(&params, is_grassland, is_tropical);
        let relationship = RelationshipNorms::derive_from(&params);
        self.cores.push(params); self.communication_norms.push(comm); self.building_styles.push(building);
        self.beauty_standards.push(beauty); self.fertility_norms.push(fertility); self.dietary_prefs.push(dietary);
        self.relationship_norms.push(relationship); self.valid.push(true); self.culture_ids.push(id);
        id
    }

    pub fn get_params_mut(&mut self, id: CultureId) -> Option<&mut CultureCoreParams> {
        let idx = id.0 as usize; if idx < self.cores.len() && self.valid[idx] { Some(&mut self.cores[idx]) } else { None }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn recalculate_derived(&mut self, id: CultureId, is_arid: bool, is_arctic: bool,
        is_forested: bool, is_desert: bool, is_cold: bool, is_warm: bool, is_grassland: bool, is_tropical: bool) {
        let idx = id.0 as usize; if idx >= self.cores.len() || !self.valid[idx] { return; }
        let core = &self.cores[idx];
        self.communication_norms[idx] = CommunicationNorms::derive_from(core);
        self.building_styles[idx] = BuildingStylePreferences::derive_from(core, is_arid, is_arctic, is_forested, is_desert, is_cold, is_warm);
        self.beauty_standards[idx] = CulturalBeautyStandard::derive_from(core);
        self.fertility_norms[idx] = FertilityNorms::derive_from(core);
        self.dietary_prefs[idx] = DietaryBasePreferences::derive_from(core, is_grassland, is_tropical);
        self.relationship_norms[idx] = RelationshipNorms::derive_from(core);
    }

    pub fn len(&self) -> usize { self.culture_ids.len() }
    pub fn is_empty(&self) -> bool { self.culture_ids.is_empty() }
}

impl CultureQuery for CultureRegistry {
    fn core_params(&self, id: CultureId) -> Option<&CultureCoreParams> {
        if id == CULTURE_ID_NONE { return None; } let idx = id.0 as usize;
        if idx < self.cores.len() && self.valid[idx] { Some(&self.cores[idx]) } else { None }
    }
    fn communication_norms(&self, id: CultureId) -> Option<&CommunicationNorms> {
        if id == CULTURE_ID_NONE { return None; } let idx = id.0 as usize;
        if idx < self.communication_norms.len() && self.valid[idx] { Some(&self.communication_norms[idx]) } else { None }
    }
    fn building_style(&self, id: CultureId) -> Option<&BuildingStylePreferences> {
        if id == CULTURE_ID_NONE { return None; } let idx = id.0 as usize;
        if idx < self.building_styles.len() && self.valid[idx] { Some(&self.building_styles[idx]) } else { None }
    }
    fn beauty_standard(&self, id: CultureId) -> Option<&CulturalBeautyStandard> {
        if id == CULTURE_ID_NONE { return None; } let idx = id.0 as usize;
        if idx < self.beauty_standards.len() && self.valid[idx] { Some(&self.beauty_standards[idx]) } else { None }
    }
    fn fertility_norms(&self, id: CultureId) -> Option<&FertilityNorms> {
        if id == CULTURE_ID_NONE { return None; } let idx = id.0 as usize;
        if idx < self.fertility_norms.len() && self.valid[idx] { Some(&self.fertility_norms[idx]) } else { None }
    }
    fn dietary_preferences(&self, id: CultureId) -> Option<&DietaryBasePreferences> {
        if id == CULTURE_ID_NONE { return None; } let idx = id.0 as usize;
        if idx < self.dietary_prefs.len() && self.valid[idx] { Some(&self.dietary_prefs[idx]) } else { None }
    }
    fn relationship_norms(&self, id: CultureId) -> Option<&RelationshipNorms> {
        if id == CULTURE_ID_NONE { return None; } let idx = id.0 as usize;
        if idx < self.relationship_norms.len() && self.valid[idx] { Some(&self.relationship_norms[idx]) } else { None }
    }
    fn all_cultures(&self) -> &[CultureId] { &self.culture_ids }
    fn culture_count(&self) -> usize { self.culture_ids.len() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use woworld_core::culture::CultureQuery;

    #[test] fn test_new_empty() { let r = CultureRegistry::new(); assert_eq!(r.len(), 0); }
    #[test] fn test_register_ids() { let mut r = CultureRegistry::new(); assert_eq!(r.register(CultureCoreParams::from_seed(0)), CultureId(0)); }
    #[test] fn test_query() { let mut r = CultureRegistry::new(); let id = r.register(CultureCoreParams::from_seed(42)); assert!(r.core_params(id).is_some()); }
    #[test] fn test_register_with_biome() { let mut r = CultureRegistry::new(); r.register_with_biome(CultureCoreParams::from_seed(0), true, false, true, false, false, true, true, true); }
    #[test] fn test_recalculate() { let mut r = CultureRegistry::new(); let id = r.register(CultureCoreParams::from_seed(0)); r.recalculate_derived(id, false, false, false, false, false, false, false, false); }
    #[test] fn test_none_id() { let mut r = CultureRegistry::new(); r.register(CultureCoreParams::from_seed(0)); assert!(r.core_params(CULTURE_ID_NONE).is_none()); }
}
