//! 地形网格生成 — 纯 Rust，从 HeightfieldTerrain 生成顶点/法线/索引
//!
//! 不依赖 Godot。输出适合传递给 Godot ArrayMesh 的原始数组。

use glam::Vec3;
use woworld_core::prelude::WorldPos;
use woworld_core::spatial::TerrainQuery;
use woworld_worldgen::HeightfieldTerrain;

/// 网格数据——可直接转换为 Godot PackedArray
pub struct TerrainMeshData {
    /// 顶点位置 (每个顶点一个 Vec3)
    pub vertices: Vec<Vec3>,
    /// 顶点法线
    pub normals: Vec<Vec3>,
    /// 三角形索引 (每 3 个 u32 = 一个三角形)
    pub indices: Vec<u32>,
    /// 顶点色 (根据高度/材质映射的颜色)
    pub colors: Vec<Vec3>,
}

/// 从高度场地形生成规则网格
///
/// - `terrain`: 地形查询器
/// - `origin_x`, `origin_z`: 网格左下角世界坐标
/// - `grid_size`: 每边顶点数 (如 128 → 128×128 顶点)
/// - `spacing`: 顶点间距 (米), 如 2.0 → 256m×256m 覆盖
pub fn generate_terrain_mesh(
    terrain: &HeightfieldTerrain,
    origin_x: f64,
    origin_z: f64,
    grid_size: u32,
    spacing: f64,
) -> TerrainMeshData {
    let n = grid_size as usize;
    let total_verts = n * n;

    let mut vertices = Vec::with_capacity(total_verts);
    let mut normals = Vec::with_capacity(total_verts);
    let mut colors = Vec::with_capacity(total_verts);

    // 生成顶点
    for iz in 0..n {
        let wz = origin_z + iz as f64 * spacing;
        for ix in 0..n {
            let wx = origin_x + ix as f64 * spacing;
            let wp = WorldPos {
                x: wx,
                y: 0.0,
                z: wz,
            };

            let h = terrain.height_at(wp);
            let n = terrain.normal_at(wp);
            let mat = terrain.surface_material_at(wp);

            vertices.push(Vec3::new(wx as f32, h, wz as f32));
            normals.push(n);

            // 颜色映射
            let color = material_color(mat, h);
            colors.push(color);
        }
    }

    // 生成索引 (每 quad 两个三角形)
    let total_quads = (n - 1) * (n - 1);
    let mut indices = Vec::with_capacity(total_quads * 6);

    for iz in 0..(n - 1) {
        for ix in 0..(n - 1) {
            let tl = (iz * n + ix) as u32;
            let tr = (iz * n + ix + 1) as u32;
            let bl = ((iz + 1) * n + ix) as u32;
            let br = ((iz + 1) * n + ix + 1) as u32;

            // Triangle 1: top-left, bottom-left, top-right
            indices.extend_from_slice(&[tl, bl, tr]);
            // Triangle 2: top-right, bottom-left, bottom-right
            indices.extend_from_slice(&[tr, bl, br]);
        }
    }

    TerrainMeshData {
        vertices,
        normals,
        indices,
        colors,
    }
}

/// 根据地表材质和高度映射顶点色
fn material_color(material: woworld_core::material::SurfaceMaterial, height: f32) -> Vec3 {
    use woworld_core::material::SurfaceMaterial::*;
    match material {
        Water => Vec3::new(0.1, 0.3, 0.8),
        Sand => Vec3::new(0.76, 0.7, 0.5),
        Grass => {
            if height > 100.0 {
                Vec3::new(0.2, 0.55, 0.2) // 高山草甸深绿
            } else {
                Vec3::new(0.3, 0.65, 0.25) // 平原浅绿
            }
        }
        Rock => Vec3::new(0.45, 0.42, 0.38),
        Stone => Vec3::new(0.35, 0.35, 0.35),
        Gravel => Vec3::new(0.5, 0.45, 0.4),
        Snow => Vec3::new(0.95, 0.95, 0.95),
        Mud => Vec3::new(0.4, 0.3, 0.2),
        _ => Vec3::new(0.4, 0.5, 0.3), // 默认绿
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use woworld_worldgen::HeightfieldTerrain;

    #[test]
    fn test_mesh_generation() {
        let terrain = HeightfieldTerrain::new(42);
        let mesh = generate_terrain_mesh(&terrain, 0.0, 0.0, 32, 4.0);

        // 32×32 = 1024 顶点
        assert_eq!(mesh.vertices.len(), 1024);
        assert_eq!(mesh.normals.len(), 1024);
        assert_eq!(mesh.colors.len(), 1024);

        // (32-1)×(32-1) = 961 quads × 6 = 5766 索引
        assert_eq!(mesh.indices.len(), 961 * 6);

        // 每个法线都是单位向量
        for n in &mesh.normals {
            let len = n.length();
            assert!((len - 1.0).abs() < 0.01, "normal not normalized: {}", len);
        }
    }

    #[test]
    fn test_mesh_vertices_in_range() {
        let terrain = HeightfieldTerrain::new(99);
        let mesh = generate_terrain_mesh(&terrain, 100.0, 200.0, 16, 3.0);

        // 检查顶点范围: x ∈ [100, 145], z ∈ [200, 245]
        for v in &mesh.vertices {
            assert!(v.x >= 100.0 && v.x <= 145.5);
            assert!(v.z >= 200.0 && v.z <= 245.5);
            // 高度在合理范围
            assert!(v.y >= -250.0 && v.y <= 800.0);
        }
    }

    #[test]
    fn test_deterministic_mesh() {
        let t1 = HeightfieldTerrain::new(42);
        let t2 = HeightfieldTerrain::new(42);
        let m1 = generate_terrain_mesh(&t1, 0.0, 0.0, 16, 4.0);
        let m2 = generate_terrain_mesh(&t2, 0.0, 0.0, 16, 4.0);

        for (a, b) in m1.vertices.iter().zip(m2.vertices.iter()) {
            assert_eq!(a.x, b.x);
            assert_eq!(a.y, b.y);
            assert_eq!(a.z, b.z);
        }
    }
}
