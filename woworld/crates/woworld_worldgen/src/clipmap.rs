//! GPU-Driven Clipmap — 8 级 LOD 环形网格 + Heightmap 纹理生成
//!
//! 网格在启动时生成一次，之后 Vertex Shader 通过 heightmap
//! 纹理采样完成 Y 轴位移——运行时零 CPU mesh 修改。

use glam::Vec3;

use crate::terrain::HeightfieldTerrain;
use crate::terrain_mesh::TerrainMeshData;
use woworld_core::prelude::WorldPos;

// ── LOD 层级定义 ──────────────────────

/// 网格生成算法类型
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MeshAlgorithm {
    /// Transvoxel 体素提取（已废弃——仅保留 spacing 定义）
    Transvoxel { voxel_size: f64 },
    /// Signed Heightfield — 2D 高度网格
    SignedHeightfield { spacing: f64 },
}

/// 一个 LOD 层级的配置
#[derive(Clone, Debug)]
pub struct LodLevel {
    pub index: u8,
    pub min_range: f64,
    pub max_range: f64,
    pub algorithm: MeshAlgorithm,
}

/// 8 级 LOD 层级配置（公开只读）
pub const LEVELS: [LodLevel; 8] = [
    LodLevel {
        index: 0,
        min_range: 0.0,
        max_range: 30.0,
        algorithm: MeshAlgorithm::Transvoxel { voxel_size: 0.5 },
    },
    LodLevel {
        index: 1,
        min_range: 30.0,
        max_range: 80.0,
        algorithm: MeshAlgorithm::Transvoxel { voxel_size: 1.0 },
    },
    LodLevel {
        index: 2,
        min_range: 80.0,
        max_range: 200.0,
        algorithm: MeshAlgorithm::Transvoxel { voxel_size: 2.0 },
    },
    LodLevel {
        index: 3,
        min_range: 200.0,
        max_range: 500.0,
        algorithm: MeshAlgorithm::Transvoxel { voxel_size: 4.0 },
    },
    LodLevel {
        index: 4,
        min_range: 500.0,
        max_range: 1500.0,
        algorithm: MeshAlgorithm::Transvoxel { voxel_size: 8.0 },
    },
    LodLevel {
        index: 5,
        min_range: 1500.0,
        max_range: 4000.0,
        algorithm: MeshAlgorithm::SignedHeightfield { spacing: 16.0 },
    },
    LodLevel {
        index: 6,
        min_range: 4000.0,
        max_range: 10000.0,
        algorithm: MeshAlgorithm::SignedHeightfield { spacing: 32.0 },
    },
    LodLevel {
        index: 7,
        min_range: 10000.0,
        max_range: 15000.0,
        algorithm: MeshAlgorithm::SignedHeightfield { spacing: 64.0 },
    },
];

// ── 按层高度图纹理配置 ──────────────────

/// 单个 LOD 层级的高度图纹理配置
#[derive(Clone, Copy, Debug)]
pub struct LayerTexConfig {
    /// 纹理分辨率（像素，正方形）
    pub hm_size: u32,
    /// 世界空间覆盖范围（米，= spacing × hm_size）
    pub hm_extent: f64,
}

/// 为指定 LOD 层级计算最优高度图纹理尺寸和覆盖范围。
///
/// 原则：覆盖网格环 + 足够漂移余量，同时最小化 Perlin 噪声采样开销。
/// 内层（L0-L3）使用 256²（节约 75% 噪声），外层（L4-L7）保持 512²。
pub fn layer_tex_config(level: &LodLevel) -> LayerTexConfig {
    let spacing = level_spacing(level);
    let (hm_size, hm_extent) = match level.index {
        0 => (256u32, 128.0),           // 0.5m/texel, 覆盖 128m
        1 => (256u32, 256.0),           // 1.0m/texel, 覆盖 256m
        2 => (256u32, 512.0),           // 2.0m/texel, 覆盖 512m
        3 => (512u32, 2048.0),          // 4.0m/texel, 覆盖 2048m, margin=520m
        _ => (512u32, 512.0 * spacing), // L4-L7: 保持 512²
    };
    LayerTexConfig { hm_size, hm_extent }
}

// ── 公开 API ──────────────────────────

/// 获取 LOD 层级的顶点间距
pub fn level_spacing(level: &LodLevel) -> f64 {
    match level.algorithm {
        MeshAlgorithm::Transvoxel { voxel_size } => voxel_size,
        MeshAlgorithm::SignedHeightfield { spacing } => spacing,
    }
}

// ── 高度图数据 ────────────────────────

/// 一次高度图生成迭代的完整输出
#[derive(Debug)]
pub struct HeightmapData {
    /// 高度值——hm_size² 像素，R32F
    pub heights: Vec<f32>,
    /// 材质颜色——hm_size² 像素，RGBA（每分量 f32 ∈ [0,1]）
    pub material_colors: Vec<[f32; 4]>,
}

// ── 公开 API ──────────────────────────

/// 为指定 LOD 层级生成 GPU-driven clipmap 环形网格。
///
/// 生成中心在原点的环形正方形网格 (XZ plane, Y=0)。
/// 仅包含 min_range 到 max_range 之间的顶点——内部空洞由更细层覆盖。
///
/// 此网格在启动时创建一次，之后 Vertex Shader 通过
/// heightmap 纹理采样完成 Y 轴位移——运行时零 CPU mesh 修改。
pub fn generate_clipmap_grid(level: &LodLevel) -> TerrainMeshData {
    let spacing = level_spacing(level);
    let half = level.max_range;
    let inner = level.min_range;
    let n = (2.0 * half / spacing).ceil() as u32 + 1;
    let n_usize = n as usize;

    let mut vertices = Vec::new();
    let mut normals = Vec::new();
    let mut colors = Vec::new();

    // 生成顶点：仅包含环形区域
    let mut vertex_map: Vec<Option<u32>> = vec![None; n_usize * n_usize];

    for iz in 0..n {
        let z = -half + iz as f64 * spacing;
        for ix in 0..n {
            let x = -half + ix as f64 * spacing;
            // Chebyshev 距离——方形环与方形网格天然匹配，零部分四边形
            let dist = x.abs().max(z.abs());
            if dist >= inner - spacing && dist <= half + spacing {
                let idx = vertices.len() as u32;
                vertices.push(Vec3::new(x as f32, 0.0, z as f32));
                normals.push(Vec3::Y);
                colors.push(Vec3::ONE);
                vertex_map[iz as usize * n_usize + ix as usize] = Some(idx);
            }
        }
    }

    // 生成索引：完整 quad → 2 个三角；边界 quad（3 顶点）→ 1 个三角
    let mut indices = Vec::new();
    for iz in 0..(n_usize - 1) {
        for ix in 0..(n_usize - 1) {
            let tl = vertex_map[iz * n_usize + ix];
            let tr = vertex_map[iz * n_usize + (ix + 1)];
            let bl = vertex_map[(iz + 1) * n_usize + ix];
            let br = vertex_map[(iz + 1) * n_usize + (ix + 1)];
            match (tl, tr, bl, br) {
                (Some(tl), Some(tr), Some(bl), Some(br)) => {
                    indices.extend_from_slice(&[tl, tr, bl, bl, tr, br]);
                }
                // 缺失 1 个顶点 → 用剩余的 3 个生成 1 个三角填充边界空洞
                (Some(tl), Some(tr), Some(bl), None) => {
                    indices.extend_from_slice(&[tl, tr, bl]);
                }
                (Some(tl), Some(tr), None, Some(br)) => {
                    indices.extend_from_slice(&[tl, tr, br]);
                }
                (Some(tl), None, Some(bl), Some(br)) => {
                    indices.extend_from_slice(&[tl, br, bl]);
                }
                (None, Some(tr), Some(bl), Some(br)) => {
                    indices.extend_from_slice(&[bl, tr, br]);
                }
                _ => { /* 2 或更少顶点——不生成 */ }
            }
        }
    }

    TerrainMeshData {
        vertices,
        normals,
        indices,
        colors,
    }
}

/// 为指定 LOD 层级生成 heightmap 数据（R32F 像素数组）。
pub fn generate_heightmap(
    terrain: &HeightfieldTerrain,
    center_x: f64,
    center_z: f64,
    spacing: f64,
    texture_size: u32,
) -> Vec<f32> {
    use woworld_core::spatial::TerrainQuery;
    let half = texture_size as f64 * spacing * 0.5;
    let ox = center_x - half;
    let oz = center_z - half;
    let pixel_count = (texture_size * texture_size) as usize;
    let mut data = Vec::with_capacity(pixel_count);
    for iz in 0..texture_size {
        let wz = oz + iz as f64 * spacing;
        for ix in 0..texture_size {
            let wx = ox + ix as f64 * spacing;
            let h = terrain.height_at(WorldPos {
                x: wx,
                y: 0.0,
                z: wz,
            });
            data.push(h);
        }
    }
    data
}

/// 为指定 LOD 层级生成高度图 + 材质色图双重数据
pub fn generate_heightmap_data(
    terrain: &HeightfieldTerrain,
    center_x: f64,
    center_z: f64,
    spacing: f64,
    texture_size: u32,
    material_colors: &std::collections::HashMap<woworld_core::material::SurfaceMaterial, [f32; 4]>,
) -> HeightmapData {
    let half = texture_size as f64 * spacing * 0.5;
    let ox = center_x - half;
    let oz = center_z - half;
    let pixel_count = (texture_size * texture_size) as usize;
    let mut heights = Vec::with_capacity(pixel_count);
    let mut material_colors_out = Vec::with_capacity(pixel_count);
    for iz in 0..texture_size {
        let wz = oz + iz as f64 * spacing;
        for ix in 0..texture_size {
            let wx = ox + ix as f64 * spacing;
            let pos = WorldPos {
                x: wx,
                y: 0.0,
                z: wz,
            };
            let (h, _normal, material) = terrain.sample_vertex(pos.x, pos.z);
            heights.push(h);
            let color = material_colors
                .get(&material)
                .copied()
                .unwrap_or([0.4, 0.5, 0.3, 1.0]); // 与 transvoxel 回退色统一 (Sprint 045)
            material_colors_out.push(color);
        }
    }
    HeightmapData {
        heights,
        material_colors: material_colors_out,
    }
}

/// 从 TOML 字符串加载 SurfaceMaterial → RGBA 色表
pub fn load_material_colors(
    toml_str: &str,
) -> Result<std::collections::HashMap<woworld_core::material::SurfaceMaterial, [f32; 4]>, String> {
    use serde::Deserialize;
    use std::collections::HashMap;
    use woworld_core::material::SurfaceMaterial;
    #[derive(Deserialize)]
    struct MaterialColorEntry {
        color: [f32; 4],
    }
    #[derive(Deserialize)]
    struct MaterialColorsToml {
        materials: HashMap<String, MaterialColorEntry>,
    }
    let parsed: MaterialColorsToml =
        toml::from_str(toml_str).map_err(|e| format!("Failed to parse: {}", e))?;
    let mut map = HashMap::new();
    for (k, v) in parsed.materials {
        let mat = match k.as_str() {
            "Grass" => SurfaceMaterial::Grass,
            "Sand" => SurfaceMaterial::Sand,
            "Rock" => SurfaceMaterial::Rock,
            "Stone" => SurfaceMaterial::Stone,
            "Water" => SurfaceMaterial::Water,
            "Ice" => SurfaceMaterial::Ice,
            "Mud" => SurfaceMaterial::Mud,
            "Snow" => SurfaceMaterial::Snow,
            "Gravel" => SurfaceMaterial::Gravel,
            "Clay" => SurfaceMaterial::Clay,
            "Moss" => SurfaceMaterial::Moss,
            "LeafLitter" => SurfaceMaterial::LeafLitter,
            "Cobblestone" => SurfaceMaterial::Cobblestone,
            "Marble" => SurfaceMaterial::Marble,
            "Wood" => SurfaceMaterial::Wood,
            "Metal" => SurfaceMaterial::Metal,
            "Glass" => SurfaceMaterial::Glass,
            "Fabric" => SurfaceMaterial::Fabric,
            "Thatch" => SurfaceMaterial::Thatch,
            "Bone" => SurfaceMaterial::Bone,
            "Flesh" => SurfaceMaterial::Flesh,
            _ => return Err(format!("Unknown material: {}", k)),
        };
        map.insert(mat, v.color);
    }
    Ok(map)
}

/// Downsample heightmap: 2x spacing (box filter 2x2 -> 1)
/// 从源高度图双线性采样生成目标层高度图
/// 保证粗层数据 = 细层数据的精确插值——跨 LOD 边界高度完全一致
fn bilinear_sample(src: &[f32], src_size: u32, fx: f64, fz: f64) -> f32 {
    let x = fx.clamp(0.0, src_size as f64 - 1.001);
    let z = fz.clamp(0.0, src_size as f64 - 1.001);
    let ix = x as u32;
    let iz = z as u32;
    let tx = (x - ix as f64) as f32;
    let tz = (z - iz as f64) as f32;
    let s00 = src[(iz * src_size + ix) as usize];
    let s10 = src[(iz * src_size + ix + 1) as usize];
    let s01 = src[((iz + 1) * src_size + ix) as usize];
    let s11 = src[((iz + 1) * src_size + ix + 1) as usize];
    (1.0 - tx) * (1.0 - tz) * s00 + tx * (1.0 - tz) * s10 + (1.0 - tx) * tz * s01 + tx * tz * s11
}

pub fn sample_heightmap_from(
    src: &[f32],
    src_size: u32,
    src_extent: f64,
    dst_size: u32,
    dst_spacing: f64,
    grid_origin_x: f64,
    grid_origin_z: f64,
) -> Vec<f32> {
    let mut dst = Vec::with_capacity((dst_size * dst_size) as usize);
    for iz in 0..dst_size {
        let wz = grid_origin_z + iz as f64 * dst_spacing;
        let fz = (wz - grid_origin_z) / src_extent * src_size as f64;
        for ix in 0..dst_size {
            let wx = grid_origin_x + ix as f64 * dst_spacing;
            let fx = (wx - grid_origin_x) / src_extent * src_size as f64;
            dst.push(bilinear_sample(src, src_size, fx, fz));
        }
    }
    dst
}

pub fn sample_colors_from(
    src: &[[f32; 4]],
    src_size: u32,
    src_extent: f64,
    dst_size: u32,
    dst_spacing: f64,
    grid_origin_x: f64,
    grid_origin_z: f64,
) -> Vec<[f32; 4]> {
    let mut dst = Vec::with_capacity((dst_size * dst_size) as usize);
    for iz in 0..dst_size {
        let wz = grid_origin_z + iz as f64 * dst_spacing;
        let fz = (wz - grid_origin_z) / src_extent * src_size as f64;
        for ix in 0..dst_size {
            let wx = grid_origin_x + ix as f64 * dst_spacing;
            let fx = (wx - grid_origin_x) / src_extent * src_size as f64;
            let x = fx.clamp(0.0, src_size as f64 - 1.001) as u32;
            let z = fz.clamp(0.0, src_size as f64 - 1.001) as u32;
            let tx = (fx - x as f64) as f32;
            let tz = (fz - z as f64) as f32;
            let s00 = src[(z * src_size + x) as usize];
            let s10 = src[(z * src_size + x + 1) as usize];
            let s01 = src[((z + 1) * src_size + x) as usize];
            let s11 = src[((z + 1) * src_size + x + 1) as usize];
            dst.push([
                (1.0 - tx) * (1.0 - tz) * s00[0]
                    + tx * (1.0 - tz) * s10[0]
                    + (1.0 - tx) * tz * s01[0]
                    + tx * tz * s11[0],
                (1.0 - tx) * (1.0 - tz) * s00[1]
                    + tx * (1.0 - tz) * s10[1]
                    + (1.0 - tx) * tz * s01[1]
                    + tx * tz * s11[1],
                (1.0 - tx) * (1.0 - tz) * s00[2]
                    + tx * (1.0 - tz) * s10[2]
                    + (1.0 - tx) * tz * s01[2]
                    + tx * tz * s11[2],
                (1.0 - tx) * (1.0 - tz) * s00[3]
                    + tx * (1.0 - tz) * s10[3]
                    + (1.0 - tx) * tz * s01[3]
                    + tx * tz * s11[3],
            ]);
        }
    }
    dst
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipmap_grid_l0_full_disc() {
        let grid = generate_clipmap_grid(&LEVELS[0]);
        assert!(!grid.vertices.is_empty());
        // 地表顶点 y≈0；裙边底点 y≈-400
        for v in &grid.vertices {
            let is_surface = v.y.abs() < 0.001;
            let is_skirt = (v.y + 400.0).abs() < 0.001;
            assert!(
                is_surface || is_skirt,
                "vertex y={} is neither surface nor skirt",
                v.y
            );
            assert!(v.x >= -30.1 && v.x <= 30.1);
        }
        assert!(grid.indices.len() % 3 == 0);
    }

    #[test]
    fn test_clipmap_grid_ring() {
        let grid = generate_clipmap_grid(&LEVELS[3]);
        assert!(!grid.vertices.is_empty());
        let full_n = (1000.0f64 / 4.0).ceil() as usize + 1;
        assert!(grid.vertices.len() < full_n * full_n);
        // Chebyshev 方形环：L3 min=200, max=500, sp=4, 扩展 2sp → [192, 508]
        for v in &grid.vertices {
            assert!(v.y.abs() < 0.001, "vertex y={} not at surface", v.y);
            let cheb = v.x.abs().max(v.z.abs());
            assert!(
                cheb >= 192.0 && cheb <= 508.0,
                "vertex ({},{}) cheb={} out of ring range",
                v.x,
                v.z,
                cheb
            );
        }
    }

    #[test]
    fn test_level_spacing() {
        assert!((level_spacing(&LEVELS[0]) - 0.5).abs() < 0.001);
        assert!((level_spacing(&LEVELS[5]) - 16.0).abs() < 0.001);
        assert!((level_spacing(&LEVELS[7]) - 64.0).abs() < 0.001);
    }
}
