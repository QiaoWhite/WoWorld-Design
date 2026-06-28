//! 物种表 — MVP 硬编码物种集
//!
//! 从 `assets/species.toml` 加载 8 个物种。
//! 后续将 `SpeciesDef` 和适应度计算迁移到 `woworld_life` crate。
//!
//! 参见: `WoWorld-Design/Happy Game/开发阶段/生命/009-植物.md`

use serde::Deserialize;
use woworld_core::id::SpeciesId;

// ── TOML 物种定义 ──────────────────────────────────────

/// 内嵌物种表（MVP 硬编码，后续由 Life crate 的 PlantSpeciesRegistry 替代）
const EMBEDDED_SPECIES_TOML: &str = include_str!("../assets/species.toml");

#[derive(Debug, Clone, Deserialize)]
struct SpeciesToml {
    species: Vec<SpeciesDef>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)] // MVP 字段后续由完整 P2.25 管线消费
pub(crate) struct SpeciesDef {
    pub id: u64,
    pub name: String,
    pub temp_optimum: f32,
    pub temp_tolerance: f32,
    pub water_optimum: f32,
    pub water_tolerance: f32,
    pub soil_types: Vec<String>,
    pub light_optimum: f32,
    pub max_height: f32,
    pub wood_type: String,
}

// ── 适应度计算 ─────────────────────────────────────────

/// 高斯适应度：距最优值越近适应度越高
#[allow(dead_code)]
fn gaussian(x: f32, optimum: f32, tolerance: f32) -> f32 {
    let z = (x - optimum) / tolerance;
    (-0.5 * z * z).exp()
}

/// 计算某物种对给定环境参数的适应度 [0, 1]
///
/// 公式（来自 009-植物 设计文档）:
///   fitness = temp_fit × 0.35 + water_fit × 0.35 + soil_fit × 0.20 + light_fit × 0.10
#[allow(dead_code)]
pub(crate) fn fitness(def: &SpeciesDef, temperature: f32, precipitation: f32) -> f32 {
    let temp_fit = gaussian(temperature, def.temp_optimum, def.temp_tolerance);
    let water_fit = gaussian(precipitation, def.water_optimum, def.water_tolerance);
    // MVP: soil_types 匹配简化为纯气候适应的一部分（后续 Life crate 处理）
    let soil_fit = 0.8; // 默认匹配（气候主导）
    let light_fit = def.light_optimum; // 简化——后续由垂直光照竞争处理
    temp_fit * 0.35 + water_fit * 0.35 + soil_fit * 0.20 + light_fit * 0.10
}

// ── 物种表加载 ─────────────────────────────────────────

/// 物种表（MVP 硬编码 8 种）
#[derive(Debug, Clone)]
pub(crate) struct SpeciesTable {
    species: Vec<SpeciesDef>,
}

impl SpeciesTable {
    /// 从嵌入 TOML 加载
    pub fn load() -> Self {
        let table: SpeciesToml =
            toml::from_str(EMBEDDED_SPECIES_TOML).expect("embedded species.toml must be valid");
        Self {
            species: table.species,
        }
    }

    /// 查询候选物种: (SpeciesId, fitness) 列表
    #[allow(dead_code)]
    pub fn query(&self, temperature: f32, precipitation: f32) -> Vec<(SpeciesId, f32)> {
        self.species
            .iter()
            .map(|def| (SpeciesId(def.id), fitness(def, temperature, precipitation)))
            .collect()
    }

    /// 物种总数（调试用）
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.species.len()
    }
}

// ── 测试 ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_species_table_loads() {
        let table = SpeciesTable::load();
        assert!(table.len() >= 5, "expected at least 5 species");
    }

    #[test]
    fn test_fitness_range() {
        let table = SpeciesTable::load();
        let candidates = table.query(0.5, 0.5);
        for (_, f) in &candidates {
            assert!(*f >= 0.0 && *f <= 1.0, "fitness {} out of range", f);
        }
    }

    #[test]
    fn test_oak_favors_temperate() {
        let table = SpeciesTable::load();
        let temperate = table.query(0.55, 0.5); // oak's sweet spot
        let desert = table.query(0.9, 0.05); // hot, dry
                                             // oak (id=0) should be more fit in temperate
        let oak_temp = temperate.iter().find(|(id, _)| id.0 == 0).map(|(_, f)| *f);
        let oak_desert = desert.iter().find(|(id, _)| id.0 == 0).map(|(_, f)| *f);
        assert!(oak_temp.unwrap() > oak_desert.unwrap());
    }

    #[test]
    fn test_cactus_favors_desert() {
        let table = SpeciesTable::load();
        let desert = table.query(0.9, 0.05);
        let temperate = table.query(0.55, 0.5);
        // cactus (id=3) should be more fit in desert
        let cactus_desert = desert.iter().find(|(id, _)| id.0 == 3).map(|(_, f)| *f);
        let cactus_temp = temperate.iter().find(|(id, _)| id.0 == 3).map(|(_, f)| *f);
        assert!(cactus_desert.unwrap() > cactus_temp.unwrap());
    }
}
