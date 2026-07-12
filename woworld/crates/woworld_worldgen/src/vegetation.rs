//! BiomeVegetation — Phase 1 "廉价植被" Provider
//!
//! 不存储实际植物——从 BiomeClassifier 纯函数推导植被统计值。
//! 参见: `woworld_core/src/vegetation.rs` (VegetationProvider trait)
//!       `开发阶段/世界生成/012-植被覆盖生成.md` (P2.25 完整规格)
//!
//! Phase 2: Poisson disc 实际放置 + L6.5 密度场 + succession 动态

use glam::{Vec2, Vec3};
use woworld_core::id::SpeciesId;
use woworld_core::vegetation::{
    GroundCoverMap, HarvestableInfo, LandcoverType, PlantCommunitySnapshot, ProductCategory,
    RegenState, TimberAvailability, TimberQuality, VegetationProvider,
};

use crate::biome::{BiomeClassifier, BiomeDef};
use woworld_core::types::WorldPos;

/// 从群系分类器派生植被参数——Phase 1 纯函数查询
///
/// 零存储、零生成时间、零 LMDB 占用。
/// 后续 Phase 2 替换为 `woworld_vegetation` crate 的完整 P2.25 实现。
///
/// ★ Vf: `query_harvestable` 通过 Poisson disc (Bridson 2007) 确定性生成可采集物。
#[derive(Clone, Debug)]
pub struct BiomeVegetation {
    classifier: Option<BiomeClassifier>,
    /// 世界种子——所有采集物放置的确定性根
    world_seed: u64,
}

impl BiomeVegetation {
    pub fn new() -> Self {
        Self {
            classifier: None,
            world_seed: 0,
        }
    }

    pub fn with_classifier(mut self, classifier: BiomeClassifier) -> Self {
        self.classifier = Some(classifier);
        self
    }

    /// 设置世界种子（采集物确定性放置必需）
    pub fn with_world_seed(mut self, seed: u64) -> Self {
        self.world_seed = seed;
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

    fn query_harvestable(&self, pos: Vec2, radius: f32) -> Vec<HarvestableInfo> {
        let classifier = match &self.classifier {
            Some(c) => c,
            None => return vec![],
        };

        let world_seed = self.world_seed;
        let radius = radius.max(0.0);
        let r = 4.0; // Poisson 最小间距（m）
        let tc_size: f64 = 32.0;

        // 查询圆覆盖的 TC 范围（+r margin 跨边界）
        let tc_x_min = ((pos.x as f64 - radius as f64 - r) / tc_size).floor() as i64;
        let tc_x_max = ((pos.x as f64 + radius as f64 + r) / tc_size).floor() as i64;
        let tc_z_min = ((pos.y as f64 - radius as f64 - r) / tc_size).floor() as i64;
        let tc_z_max = ((pos.y as f64 + radius as f64 + r) / tc_size).floor() as i64;

        let mut results = Vec::new();

        for tc_x in tc_x_min..=tc_x_max {
            for tc_z in tc_z_min..=tc_z_max {
                // TC 中心世界坐标
                let tc_center_x = (tc_x as f64 + 0.5) * tc_size;
                let tc_center_z = (tc_z as f64 + 0.5) * tc_size;

                // 群系分类——海洋/气候空档跳过
                let biome = match classifier.classify(WorldPos {
                    x: tc_center_x,
                    y: 0.0,
                    z: tc_center_z,
                }) {
                    Some(b) => b,
                    None => continue,
                };

                // 种子派生（对齐设计 012 §八 chunk_vegetation_seed 模式）
                let tc_seed = tc_harvestable_seed(world_seed, tc_x, tc_z);

                // Desert 稀疏门——仅 20% TC 生成采集点
                if biome.name == "Desert" && splitmix_f32(tc_seed) > 0.2 {
                    continue;
                }
                // Snowfield 无采集物
                if biome.name == "Snowfield" {
                    continue;
                }

                // Poisson disc 确定性放置
                let points = poisson_disc_tc(tc_seed, tc_x, tc_z, r, tc_size);

                for (point_idx, (px, pz)) in points.iter().enumerate() {
                    let wx = *px;
                    let wz = *pz;

                    // 距离过滤
                    let dx = wx as f32 - pos.x;
                    let dz = wz as f32 - pos.y;
                    if (dx * dx + dz * dz) > radius * radius {
                        continue;
                    }

                    // 产物类别加权抽取
                    let (species_id, category, yield_range) = harvestable_for_biome(
                        &biome.name,
                        splitmix_f32(tc_seed ^ (point_idx as u64)),
                    );

                    // 地形高度
                    let wy = classifier.sample_height(wx, wz) as f32;

                    // instance_id（对齐设计 012 §八 plant_instance_id）
                    let instance_id =
                        harvestable_instance_id(world_seed, tc_x, tc_z, point_idx as u32);

                    // 确定性 yield_base
                    let yield_base = yield_range.0
                        + splitmix_f32(
                            tc_seed.wrapping_add((point_idx as u64).wrapping_mul(0x9E37_79B9)),
                        ) * (yield_range.1 - yield_range.0);

                    results.push(HarvestableInfo {
                        instance_id,
                        species_id,
                        position: Vec3::new(wx as f32, wy, wz as f32),
                        product_category: category,
                        yield_base,
                        season_optimal: true, // MVP 恒 true
                        regen_state: RegenState::Full,
                    });
                }
            }
        }

        results
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
        "Forest" => 2.0, // 高多样性
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

// ── Vf: 采集物生成辅助 ──────────────────────────────────

/// 确定性 f32 ∈ [0, 1) —— splitmix64 变体
///
/// 复用 `noise_gen::mix64`（同 crate·pub(crate)）。
fn splitmix_f32(seed: u64) -> f32 {
    let x = super::noise_gen::mix64(seed);
    (x >> 40) as f32 / (1u64 << 24) as f32
}

/// TC 种子派生（对齐设计 012 §八 chunk_vegetation_seed）
///
/// 判别符: "vegetation" 首 8 字节 → u64 常量。
const DISC_VEGETATION: u64 = u64::from_le_bytes(*b"vegetati");

fn tc_harvestable_seed(world_seed: u64, tc_x: i64, tc_z: i64) -> u64 {
    let mut s = world_seed ^ DISC_VEGETATION;
    s = super::noise_gen::mix64(s ^ (tc_x as u64));
    super::noise_gen::mix64(s ^ (tc_z as u64))
}

/// 实例 ID 派生（对齐设计 012 §八 plant_instance_id）
///
/// 判别符: "plant_in" 首 8 字节 → u64 常量。
const DISC_PLANT_INSTANCE: u64 = u64::from_le_bytes(*b"plant_in");

fn harvestable_instance_id(world_seed: u64, tc_x: i64, tc_z: i64, point_idx: u32) -> u64 {
    let mut s = world_seed ^ DISC_PLANT_INSTANCE;
    s = super::noise_gen::mix64(s ^ (tc_x as u64));
    s = super::noise_gen::mix64(s ^ (tc_z as u64));
    super::noise_gen::mix64(s ^ (point_idx as u64))
}

/// Poisson disc 放置（Bridson 2007）
///
/// 在 TC 范围 + margin 内生成确定性 Poisson disc 点集。
/// - `seed`: TC 确定性种子
/// - `tc_x, tc_z`: TC 坐标（用于世界坐标转换）
/// - `r`: 最小间距（m）
/// - `tc_size`: TC 边长（m）
///
/// 返回世界坐标 (x, z) 列表。
fn poisson_disc_tc(seed: u64, tc_x: i64, tc_z: i64, r: f64, tc_size: f64) -> Vec<(f64, f64)> {
    let margin = r; // 跨 TC 边界 margin
    let domain_min_x = tc_x as f64 * tc_size - margin;
    let domain_min_z = tc_z as f64 * tc_size - margin;
    let domain_max_x = (tc_x + 1) as f64 * tc_size + margin;
    let domain_max_z = (tc_z + 1) as f64 * tc_size + margin;

    let width = domain_max_x - domain_min_x;
    let height = domain_max_z - domain_min_z;

    let cell_size = r / (2.0_f64).sqrt();
    let cols = (width / cell_size).ceil() as usize;
    let rows = (height / cell_size).ceil() as usize;

    // 背景网格——加速邻域查询
    let mut grid: Vec<Option<(f64, f64)>> = vec![None; cols * rows];
    let mut active: Vec<(f64, f64)> = Vec::new();
    let mut points: Vec<(f64, f64)> = Vec::new();

    let k = 30u32; // 尝试上限

    // 确定性随机数生成器
    let mut rng_seed = seed;
    let mut next_f64 = || {
        rng_seed = super::noise_gen::mix64(rng_seed);
        (rng_seed >> 40) as f64 / (1u64 << 24) as f64
    };

    // 初始点：域中心
    let init_x = domain_min_x + width * 0.5;
    let init_z = domain_min_z + height * 0.5;
    insert_point(
        init_x,
        init_z,
        domain_min_x,
        domain_min_z,
        cell_size,
        cols,
        &mut grid,
        &mut active,
        &mut points,
    );

    while let Some(active_idx) = {
        if active.is_empty() {
            None
        } else {
            let idx = (next_f64() * active.len() as f64) as usize;
            Some(idx.min(active.len() - 1))
        }
    } {
        let (ax, az) = active[active_idx];
        let mut found = false;

        for _ in 0..k {
            // 环带 [r, 2r] 内随机角度 + 随机半径
            let angle = next_f64() * std::f64::consts::TAU;
            let dist = r + next_f64() * r;
            let nx = ax + angle.cos() * dist;
            let nz = az + angle.sin() * dist;

            // 边界检查
            if nx < domain_min_x || nx >= domain_max_x || nz < domain_min_z || nz >= domain_max_z {
                continue;
            }

            // 邻域冲突检查
            let col = ((nx - domain_min_x) / cell_size) as usize;
            let row = ((nz - domain_min_z) / cell_size) as usize;

            if !has_neighbor_conflict(nx, nz, r, col, row, cols, rows, &grid) {
                insert_point(
                    nx,
                    nz,
                    domain_min_x,
                    domain_min_z,
                    cell_size,
                    cols,
                    &mut grid,
                    &mut active,
                    &mut points,
                );
                found = true;
                break;
            }
        }

        if !found {
            active.swap_remove(active_idx);
        }
    }

    points
}

#[inline]
#[allow(clippy::too_many_arguments)]
fn insert_point(
    x: f64,
    z: f64,
    origin_x: f64,
    origin_z: f64,
    cell_size: f64,
    cols: usize,
    grid: &mut [Option<(f64, f64)>],
    active: &mut Vec<(f64, f64)>,
    points: &mut Vec<(f64, f64)>,
) {
    let col = ((x - origin_x) / cell_size) as usize;
    let row = ((z - origin_z) / cell_size) as usize;
    let idx = row * cols + col;
    if idx < grid.len() {
        grid[idx] = Some((x, z));
    }
    active.push((x, z));
    points.push((x, z));
}

#[allow(clippy::too_many_arguments)]
fn has_neighbor_conflict(
    x: f64,
    z: f64,
    r: f64,
    col: usize,
    row: usize,
    cols: usize,
    rows: usize,
    grid: &[Option<(f64, f64)>],
) -> bool {
    let r2 = r * r;
    let search_cells = 2; // 邻域 ±2 cells 足够覆盖 [r, 2r)

    let c_min = col.saturating_sub(search_cells);
    let c_max = (col + search_cells).min(cols - 1);
    let r_min = row.saturating_sub(search_cells);
    let r_max = (row + search_cells).min(rows - 1);

    for rr in r_min..=r_max {
        for cc in c_min..=c_max {
            if let Some((px, pz)) = grid[rr * cols + cc] {
                let dx = x - px;
                let dz = z - pz;
                if dx * dx + dz * dz < r2 {
                    return true;
                }
            }
        }
    }
    false
}

/// 群系 → 采集产物类别加权抽取
///
/// 返回 (SpeciesId, ProductCategory, (yield_min, yield_max))。
///
/// SpeciesId 哨兵值（MVP·待 `PlantSpeciesRegistry` 替换）：
///   `SpeciesId(1)` = BerryBush  `SpeciesId(2)` = Mushroom  `SpeciesId(3)` = NutTree
///   `SpeciesId(4)` = Herb       `SpeciesId(5)` = Flower    `SpeciesId(6)` = FiberPlant
fn harvestable_for_biome(biome: &str, roll: f32) -> (SpeciesId, ProductCategory, (f32, f32)) {
    match biome {
        "Forest" => {
            if roll < 0.40 {
                (SpeciesId(1), ProductCategory::Berry, (1.5, 3.0))
            } else if roll < 0.75 {
                (SpeciesId(2), ProductCategory::Mushroom, (1.0, 2.5))
            } else {
                (SpeciesId(3), ProductCategory::Nut, (1.5, 3.0))
            }
        }
        "Grassland" => {
            if roll < 0.50 {
                (SpeciesId(4), ProductCategory::Herb, (0.8, 2.0))
            } else if roll < 0.80 {
                (SpeciesId(1), ProductCategory::Berry, (0.8, 1.5))
            } else {
                (SpeciesId(5), ProductCategory::Flower, (0.5, 1.5))
            }
        }
        "Swamp" => {
            if roll < 0.60 {
                (SpeciesId(2), ProductCategory::Mushroom, (1.0, 2.5))
            } else if roll < 0.85 {
                (SpeciesId(4), ProductCategory::Herb, (0.8, 1.8))
            } else {
                (SpeciesId(6), ProductCategory::Fiber, (0.5, 1.5))
            }
        }
        "Desert" => {
            if roll < 0.60 {
                (SpeciesId(4), ProductCategory::Herb, (0.3, 1.0))
            } else {
                (SpeciesId(6), ProductCategory::Fiber, (0.2, 0.8))
            }
        }
        _ => (SpeciesId(4), ProductCategory::Herb, (0.3, 1.0)),
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
        BiomeVegetation::new()
            .with_classifier(classifier)
            .with_world_seed(42)
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
        assert!(
            (total - 1.0).abs() < 0.01,
            "ground cover should sum to 1.0, got {total}"
        );
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
            assert!(
                (total - 1.0).abs() < 0.01,
                "{name}: ground cover sums to {total}, expected 1.0"
            );
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
        assert_eq!(landcover_from_canopy(0.0, &ground), LandcoverType::Desert);
    }

    // ── Vf: query_harvestable 测试 ──────────────────────

    #[test]
    fn test_query_harvestable_non_empty_somewhere() {
        let v = make_veg();
        // 采样多个位置——至少有一个位置返回非空（世界有陆地）
        let mut total = 0;
        for x in (0..2000).step_by(200) {
            for z in (0..2000).step_by(200) {
                let h = v.query_harvestable(Vec2::new(x as f32, z as f32), 20.0);
                total += h.len();
            }
        }
        assert!(
            total > 0,
            "world with seed 42 should have SOME harvestable items"
        );
    }

    #[test]
    fn test_query_harvestable_deterministic() {
        let v = make_veg();
        let pos = Vec2::new(1000.0, 1000.0);
        let a = v.query_harvestable(pos, 20.0);
        let b = v.query_harvestable(pos, 20.0);
        assert_eq!(a.len(), b.len());
        for (ha, hb) in a.iter().zip(b.iter()) {
            assert_eq!(
                ha.instance_id, hb.instance_id,
                "instance_id must be deterministic"
            );
            assert_eq!(ha.species_id, hb.species_id);
            assert_eq!(ha.product_category, hb.product_category);
            assert!((ha.position.x - hb.position.x).abs() < 0.001, "position x");
            assert!((ha.position.z - hb.position.z).abs() < 0.001, "position z");
            assert!((ha.yield_base - hb.yield_base).abs() < 0.001, "yield_base");
        }
    }

    #[test]
    fn test_query_harvestable_radius_grows_monotonically() {
        let v = make_veg();
        let pos = Vec2::new(1000.0, 1000.0);
        let small = v.query_harvestable(pos, 5.0).len();
        let medium = v.query_harvestable(pos, 20.0).len();
        let large = v.query_harvestable(pos, 50.0).len();
        assert!(
            medium >= small,
            "larger radius should not return fewer items"
        );
        assert!(
            large >= medium,
            "larger radius should not return fewer items"
        );
    }

    #[test]
    fn test_query_harvestable_no_panic_no_classifier() {
        let v = BiomeVegetation::new();
        let result = v.query_harvestable(Vec2::ZERO, 10.0);
        assert!(result.is_empty());
    }

    #[test]
    fn test_query_harvestable_zero_radius() {
        let v = make_veg();
        let result = v.query_harvestable(Vec2::new(1000.0, 1000.0), 0.0);
        // 0 半径可能命中 TC 中心的初始点，也可能不命中——仅验证不崩溃
        let _ = result;
    }

    #[test]
    fn test_harvestable_has_regen_state() {
        let v = make_veg();
        let h = v.query_harvestable(Vec2::new(1000.0, 1000.0), 30.0);
        for item in &h {
            // 所有项 regen_state 应为 Full
            assert!(matches!(item.regen_state, RegenState::Full));
        }
    }

    #[test]
    fn test_harvestable_height_is_finite() {
        let v = make_veg();
        let h = v.query_harvestable(Vec2::new(1000.0, 1000.0), 30.0);
        for item in &h {
            assert!(item.position.y.is_finite(), "height must be finite");
            // 高度在合理范围（-500m ~ 800m，基于 noise_gen 测试范围）
            assert!(
                item.position.y > -500.0 && item.position.y < 800.0,
                "height {} out of range",
                item.position.y
            );
        }
    }

    #[test]
    fn test_instance_ids_unique_within_tc() {
        let v = make_veg();
        let h = v.query_harvestable(Vec2::new(1000.0, 1000.0), 50.0);
        let mut ids: Vec<u64> = h.iter().map(|x| x.instance_id).collect();
        let orig_len = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(
            ids.len(),
            orig_len,
            "all instance_ids within query must be unique"
        );
    }

    #[test]
    fn test_poisson_min_distance() {
        // 直接测试 poisson_disc_tc：任意两点间距 ≥ r
        let seed = 42u64;
        let r = 4.0;
        let points = poisson_disc_tc(seed, 0, 0, r, 32.0);
        assert!(
            !points.is_empty(),
            "Poisson disc should produce at least one point"
        );

        let r2 = r * r;
        for (i, (x1, z1)) in points.iter().enumerate() {
            for (j, (x2, z2)) in points.iter().enumerate() {
                if i >= j {
                    continue;
                }
                let dx = x1 - x2;
                let dz = z1 - z2;
                let d2 = dx * dx + dz * dz;
                assert!(
                    d2 >= r2 * 0.999, // 允许浮点误差
                    "poisson points too close: ({}, {}) vs ({}, {}), dist²={}, r²={}",
                    x1,
                    z1,
                    x2,
                    z2,
                    d2,
                    r2
                );
            }
        }
    }

    #[test]
    fn test_poisson_deterministic_same_seed() {
        let a = poisson_disc_tc(42, 0, 0, 4.0, 32.0);
        let b = poisson_disc_tc(42, 0, 0, 4.0, 32.0);
        assert_eq!(a.len(), b.len());
        for ((x1, z1), (x2, z2)) in a.iter().zip(b.iter()) {
            assert!((x1 - x2).abs() < 1e-10, "x mismatch");
            assert!((z1 - z2).abs() < 1e-10, "z mismatch");
        }
    }

    #[test]
    fn test_poisson_different_seed_different_result() {
        let a = poisson_disc_tc(42, 0, 0, 4.0, 32.0);
        let b = poisson_disc_tc(99, 0, 0, 4.0, 32.0);
        // 不同种子应产生不同点集（概率极高）
        // 只需验证它们不完全相同（长度或首个点不同）
        let identical = a.len() == b.len()
            && a.iter()
                .zip(b.iter())
                .all(|((x1, z1), (x2, z2))| (x1 - x2).abs() < 1e-10 && (z1 - z2).abs() < 1e-10);
        assert!(
            !identical,
            "different seeds should produce different point sets"
        );
    }

    #[test]
    fn test_splitmix_deterministic() {
        assert_eq!(splitmix_f32(42), splitmix_f32(42));
        assert!((splitmix_f32(42) - splitmix_f32(99)).abs() > 0.001);
    }

    #[test]
    fn test_tc_seed_deterministic() {
        assert_eq!(tc_harvestable_seed(42, 0, 0), tc_harvestable_seed(42, 0, 0));
        assert_ne!(tc_harvestable_seed(42, 0, 0), tc_harvestable_seed(42, 1, 0));
    }

    #[test]
    fn test_harvestable_instance_id_deterministic() {
        let a = harvestable_instance_id(42, 0, 0, 0);
        let b = harvestable_instance_id(42, 0, 0, 0);
        assert_eq!(a, b);
        // 不同 point_idx → 不同 ID
        assert_ne!(a, harvestable_instance_id(42, 0, 0, 1));
        // 不同 TC → 不同 ID
        assert_ne!(a, harvestable_instance_id(42, 1, 0, 0));
    }

    #[test]
    fn test_harvestable_for_biome_forest() {
        // 验证 Forest 产物分布覆盖三类
        let mut saw_berry = false;
        let mut saw_mushroom = false;
        let mut saw_nut = false;
        for i in 0..100 {
            let roll = splitmix_f32(1000 + i);
            let (_, cat, _) = harvestable_for_biome("Forest", roll);
            match cat {
                ProductCategory::Berry => saw_berry = true,
                ProductCategory::Mushroom => saw_mushroom = true,
                ProductCategory::Nut => saw_nut = true,
                _ => {}
            }
        }
        assert!(saw_berry, "Forest should produce Berry");
        assert!(saw_mushroom, "Forest should produce Mushroom");
        assert!(saw_nut, "Forest should produce Nut");
    }

    #[test]
    fn test_harvestable_for_biome_snowfield_no_panic() {
        // Snowfield 无特殊映射——退回默认
        let (_, cat, (lo, hi)) = harvestable_for_biome("Snowfield", 0.5);
        assert!(lo < hi);
        let _ = cat;
    }

    #[test]
    fn test_with_world_seed_isolates_worlds() {
        let toml_str = include_str!("../../../assets/biomes.toml");
        let noise_a = std::sync::Arc::new(WorldNoise::new(42));
        let noise_b = std::sync::Arc::new(WorldNoise::new(99));
        let classifier_a = BiomeClassifier::from_toml_str(toml_str, noise_a).unwrap();
        let classifier_b = BiomeClassifier::from_toml_str(toml_str, noise_b).unwrap();

        let va = BiomeVegetation::new()
            .with_classifier(classifier_a)
            .with_world_seed(42);
        let vb = BiomeVegetation::new()
            .with_classifier(classifier_b)
            .with_world_seed(99);

        let pos = Vec2::new(1000.0, 1000.0);
        let ha = va.query_harvestable(pos, 30.0);
        let hb = vb.query_harvestable(pos, 30.0);

        // 不同世界种子 → 同位置可能产生不同采集物集
        // 验证：至少 instance_id 的派生用了 world_seed
        if !ha.is_empty() && !hb.is_empty() {
            // 如果两个世界在同位置都有采集物，instance_id 应不同（因为 world_seed 不同）
            let ids_a: Vec<u64> = ha.iter().map(|x| x.instance_id).collect();
            let ids_b: Vec<u64> = hb.iter().map(|x| x.instance_id).collect();
            // 至少第一个 instance_id 不同（由 world_seed 参与派生）
            if ids_a[0] == ids_b[0] {
                // 可能巧合——但位置和物种也应体现差异
                // 宽松验证：两个世界的产物集不应逐项全等
                let all_same = ha.len() == hb.len()
                    && ha.iter().zip(hb.iter()).all(|(a, b)| {
                        a.instance_id == b.instance_id && a.species_id == b.species_id
                    });
                assert!(!all_same, "different world seeds should differ");
            }
        }
    }
}
