//! 建筑风格偏好 — 从 CultureCoreParams × 环境的第一层派生 (严格对齐设计文档 004 §1)
//!
//! 消费者: 世界生成 (P5-P6 建筑 WFC)、家具与放置物品

use super::CultureCoreParams;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoofStyle {
    Flat,
    #[default]
    Sloped,
    Dome,
    Spire,
    Curved,
    Longhouse,
    Thatched,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WallMaterial {
    LocalStone,
    Timber,
    Brick,
    Adobe,
    WattleDaub,
    ImportedStone,
    Ice,
    Hide,
    #[default]
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WallMaterialPreference {
    pub primary: WallMaterial,
    pub fallback: WallMaterial,
    pub imported_stone_ratio: f32,
}
impl Default for WallMaterialPreference {
    fn default() -> Self {
        Self {
            primary: WallMaterial::default(),
            fallback: WallMaterial::WattleDaub,
            imported_stone_ratio: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum DecorationLevel {
    #[default]
    Bare,
    Minimal,
    Moderate,
    Ornate,
    Extravagant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HueFamily {
    #[default]
    Warm,
    Cool,
    Earth,
    Neutral,
    Vibrant,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorPalette {
    pub primary_hue: HueFamily,
    pub accent_hue: HueFamily,
    pub saturation: f32,
    pub contrast: f32,
}
impl Default for ColorPalette {
    fn default() -> Self {
        Self {
            primary_hue: HueFamily::default(),
            accent_hue: HueFamily::Cool,
            saturation: 0.5,
            contrast: 0.5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct AdjacencyModifiers {
    pub kitchen_mainhall_required: bool,
    pub entry_buffer_preferred: bool,
    pub courtyard_preference: f32,
    pub zoning_strictness: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BuildingStylePreferences {
    pub roof: RoofStyle,
    pub wall_material: WallMaterialPreference,
    pub decoration: DecorationLevel,
    pub palette: ColorPalette,
    pub symmetry: f32,
    pub scale: f32,
    pub adjacency_modifiers: AdjacencyModifiers,
}
impl Default for BuildingStylePreferences {
    fn default() -> Self {
        Self {
            roof: RoofStyle::default(),
            wall_material: WallMaterialPreference::default(),
            decoration: DecorationLevel::default(),
            palette: ColorPalette::default(),
            symmetry: 0.5,
            scale: 0.5,
            adjacency_modifiers: AdjacencyModifiers::default(),
        }
    }
}

impl BuildingStylePreferences {
    /// 从 CultureCoreParams 和环境标志派生建筑风格（严格对齐 004 §1）
    pub fn derive_from(
        core: &CultureCoreParams,
        is_arid: bool,
        is_arctic: bool,
        is_forested: bool,
        is_desert: bool,
        is_cold: bool,
        is_warm: bool,
    ) -> Self {
        // roof: climate-dominant, then cultural
        let roof = if is_arctic {
            RoofStyle::Sloped
        } else if is_arid || is_desert {
            RoofStyle::Flat
        } else if core.artistry > 0.7 && core.religiosity > 0.6 {
            RoofStyle::Dome
        } else if core.competition_orientation > 0.7 && core.power_distance > 0.6 {
            RoofStyle::Spire
        } else if core.individualism < 0.3 && core.long_term_orientation > 0.6 {
            RoofStyle::Longhouse
        } else if core.artistry > 0.65 && core.competition_orientation < 0.4 {
            RoofStyle::Curved
        } else if is_forested {
            RoofStyle::Thatched
        } else {
            RoofStyle::Sloped
        };

        // wall material
        let primary = if is_arctic {
            WallMaterial::Ice
        } else if is_desert {
            WallMaterial::Adobe
        } else if is_forested {
            WallMaterial::Timber
        } else {
            WallMaterial::LocalStone
        };
        let imported_stone_ratio = if core.artistry > 0.7 && core.long_term_orientation > 0.5 {
            0.15
        } else {
            0.03
        };
        let wall_material = WallMaterialPreference {
            primary,
            fallback: WallMaterial::WattleDaub,
            imported_stone_ratio,
        };

        // decoration: 设计文档阈值 >0.8 Extravagant, >0.6 Ornate, >0.35 Moderate, >0.15 Minimal, else Bare
        let deco_raw = core.artistry * 0.55
            + core.power_distance * 0.15
            + core.religiosity * 0.15
            + core.individualism * 0.15;
        let decoration = if deco_raw > 0.8 {
            DecorationLevel::Extravagant
        } else if deco_raw > 0.6 {
            DecorationLevel::Ornate
        } else if deco_raw > 0.35 {
            DecorationLevel::Moderate
        } else if deco_raw > 0.15 {
            DecorationLevel::Minimal
        } else {
            DecorationLevel::Bare
        };

        // color palette
        let saturation =
            (core.artistry * 0.5 + core.indulgence * 0.3 + (1.0 - core.power_distance) * 0.2)
                .clamp(0.0, 1.0);
        let contrast = (core.competition_orientation * 0.5 + core.artistry * 0.5).clamp(0.0, 1.0);
        let primary_hue = HueFamily::Earth; // 设计: 大多数文化以大地色为主
        let accent_hue = if core.artistry > 0.6 && core.indulgence > 0.5 {
            HueFamily::Vibrant
        } else if core.artistry > 0.6 {
            HueFamily::Warm
        } else {
            HueFamily::Cool
        };
        let palette = ColorPalette {
            primary_hue,
            accent_hue,
            saturation,
            contrast,
        };

        // symmetry
        let symmetry = (core.uncertainty_avoidance * 0.45
            + core.power_distance * 0.35
            + (1.0 - core.artistry) * 0.20)
            .clamp(0.0, 1.0);
        // scale
        let scale = (core.power_distance * 0.45
            + core.competition_orientation * 0.30
            + (1.0 - core.individualism) * 0.25)
            .clamp(0.0, 1.0);

        // adjacency: 设计文档公式
        let adjacency_modifiers = AdjacencyModifiers {
            kitchen_mainhall_required: is_cold || core.long_term_orientation > 0.7,
            entry_buffer_preferred: core.individualism > 0.6,
            courtyard_preference: ((if is_warm { 0.6 } else { 0.2 })
                + (1.0 - core.individualism) * 0.4)
                .clamp(0.0, 1.0),
            zoning_strictness: (core.uncertainty_avoidance * 0.7 + core.power_distance * 0.3)
                .clamp(0.0, 1.0),
        };

        Self {
            roof,
            wall_material,
            decoration,
            palette,
            symmetry,
            scale,
            adjacency_modifiers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_fields_in_range() {
        for seed in 0..50 {
            let core = CultureCoreParams::from_seed(seed);
            for (arid, arctic, forested, desert, cold, warm) in &[
                (false, false, false, false, false, false),
                (true, false, false, false, false, true),
                (false, true, false, false, true, false),
                (false, false, true, false, false, false),
            ] {
                let s = BuildingStylePreferences::derive_from(
                    &core, *arid, *arctic, *forested, *desert, *cold, *warm,
                );
                assert!((0.0..=1.0).contains(&s.symmetry));
                assert!((0.0..=1.0).contains(&s.scale));
                assert!((0.0..=1.0).contains(&s.palette.saturation));
                assert!((0.0..=1.0).contains(&s.palette.contrast));
            }
        }
    }

    #[test]
    fn test_desert_flat_roof() {
        let s = BuildingStylePreferences::derive_from(
            &CultureCoreParams::default(),
            false,
            false,
            false,
            true,
            false,
            true,
        );
        assert_eq!(s.roof, RoofStyle::Flat);
    }

    #[test]
    fn test_dome() {
        let core = CultureCoreParams {
            artistry: 0.9,
            religiosity: 0.8,
            ..Default::default()
        };
        assert_eq!(
            BuildingStylePreferences::derive_from(&core, false, false, false, false, false, false)
                .roof,
            RoofStyle::Dome
        );
    }

    #[test]
    fn test_extravagant_decoration() {
        let core = CultureCoreParams {
            artistry: 1.0,
            power_distance: 1.0,
            religiosity: 1.0,
            individualism: 1.0,
            ..Default::default()
        };
        assert_eq!(
            BuildingStylePreferences::derive_from(&core, false, false, false, false, false, false)
                .decoration,
            DecorationLevel::Extravagant
        );
    }

    #[test]
    fn test_earth_primary_hue() {
        let core = CultureCoreParams::default();
        assert_eq!(
            BuildingStylePreferences::derive_from(&core, false, false, false, false, false, false)
                .palette
                .primary_hue,
            HueFamily::Earth
        );
    }

    #[test]
    fn test_imported_stone_ratio() {
        let low = BuildingStylePreferences::derive_from(
            &CultureCoreParams::default(),
            false,
            false,
            false,
            false,
            false,
            false,
        );
        assert!(low.wall_material.imported_stone_ratio < 0.1);
    }

    #[test]
    fn test_deterministic() {
        let core = CultureCoreParams::from_seed(42);
        let a =
            BuildingStylePreferences::derive_from(&core, false, false, false, false, false, false);
        let b =
            BuildingStylePreferences::derive_from(&core, false, false, false, false, false, false);
        assert_eq!(a.roof, b.roof);
    }
}
