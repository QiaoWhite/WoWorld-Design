//! 地形网格生成 — 纯 Rust，从 HeightfieldTerrain 生成顶点/法线/索引
//!
//! 不依赖 Godot。输出适合传递给 Godot ArrayMesh 的原始数组。

use glam::Vec3;
use woworld_core::prelude::WorldPos;
use woworld_core::spatial::TerrainQuery;

/// 网格数据——可直接转换为 Godot PackedArray
#[derive(Debug)]
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

/// 高度→颜色映射（连续渐变，5 个色键 + lerp）
///
/// 为 Signed Heightfield 远距离着色——不依赖 SurfaceMaterial。
/// 色键锚定现有材质系统的阈值（≤0 沙 / 0-5 滨海 / 5-200 绿 / 200-500 灰褐 / >500 雪山）。
fn height_to_color(h: f32) -> Vec3 {
    if h <= 0.0 {
        Vec3::new(0.76, 0.70, 0.50) // 海床沙色
    } else if h <= 5.0 {
        let t = h / 5.0;
        Vec3::new(0.76, 0.70, 0.50).lerp(Vec3::new(0.42, 0.68, 0.30), t)
    } else if h <= 200.0 {
        let t = (h - 5.0) / 195.0;
        Vec3::new(0.42, 0.68, 0.30).lerp(Vec3::new(0.25, 0.55, 0.25), t)
    } else if h <= 500.0 {
        let t = (h - 200.0) / 300.0;
        Vec3::new(0.25, 0.55, 0.25).lerp(Vec3::new(0.40, 0.37, 0.35), t)
    } else {
        let t = ((h - 500.0) / 200.0).min(1.0); // 700m+ 饱和
        Vec3::new(0.40, 0.37, 0.35).lerp(Vec3::new(0.90, 0.90, 0.90), t)
    }
}

/// 从 TerrainQuery 生成 Signed Heightfield 网格
///
/// 用于远距离 LOD（scene_lod 5+, 1.5km+）。
/// 两遍算法：① 采样全部高度到 2D 数组 ② 从网格邻居中心差分计算法线 + 高度着色。
///
/// `terrain`: 地形查询 trait object——支持未来建筑/植被高度装饰器。
/// `grid_size`: 每边顶点数（如 33 → 33×33 顶点）。
/// `spacing`: 顶点间距（米）。
/// `overlap`: 每边向外扩展的顶点行数（0=无扩展，1=1行重叠——相邻 tile 共享边界顶点）。
pub fn generate_sh_mesh(
    terrain: &dyn TerrainQuery,
    origin_x: f64,
    origin_z: f64,
    grid_size: u32,
    spacing: f64,
    overlap: u32,
) -> TerrainMeshData {
    let n = grid_size as usize;
    let ov = overlap as usize;
    let total_n = n + 2 * ov; // 含重叠的实际网格
    let s = spacing as f32;

    // 实际采样起点（向外偏移 overlap 行）
    let sample_ox = origin_x - ov as f64 * spacing;
    let sample_oz = origin_z - ov as f64 * spacing;

    // ── 第一遍：采样全部高度（含重叠区）──
    let mut heights: Vec<f32> = Vec::with_capacity(total_n * total_n);
    for iz in 0..total_n {
        let wz = sample_oz + iz as f64 * spacing;
        for ix in 0..total_n {
            let wx = sample_ox + ix as f64 * spacing;
            let h = terrain.height_at(WorldPos {
                x: wx,
                y: 0.0,
                z: wz,
            });
            heights.push(h);
        }
    }

    // ── 第二遍：法线（中心差分）+ 颜色 + 顶点 ──
    let mut vertices = Vec::with_capacity(total_n * total_n);
    let mut normals = Vec::with_capacity(total_n * total_n);
    let mut colors = Vec::with_capacity(total_n * total_n);

    for iz in 0..total_n {
        let wz = sample_oz + iz as f64 * spacing;
        for ix in 0..total_n {
            let wx = sample_ox + ix as f64 * spacing;
            let h = heights[iz * total_n + ix];

            // 中心差分法线（扩展区也有完整邻居，边界自然消除）
            let h_left = if ix > 0 {
                heights[iz * total_n + (ix - 1)]
            } else {
                h
            };
            let h_right = if ix < total_n - 1 {
                heights[iz * total_n + (ix + 1)]
            } else {
                h
            };
            let h_back = if iz > 0 {
                heights[(iz - 1) * total_n + ix]
            } else {
                h
            };
            let h_front = if iz < total_n - 1 {
                heights[(iz + 1) * total_n + ix]
            } else {
                h
            };

            let dzdx = (h_right - h_left) / (2.0 * s);
            let dzdz = (h_front - h_back) / (2.0 * s);
            let normal = Vec3::new(-dzdx, 1.0, -dzdz).normalize();

            vertices.push(Vec3::new(wx as f32, h, wz as f32));
            normals.push(normal);
            colors.push(height_to_color(h));
        }
    }

    // ── 第三遍：索引（仅覆盖 tile 本体区域，重叠行仅供相邻 tile 共享）──
    let indices = generate_quad_indices_with_offset(grid_size, ov as u32, total_n as u32);

    TerrainMeshData {
        vertices,
        normals,
        indices,
        colors,
    }
}

/// 生成 tile 本体区域的四边形索引（跳过重叠边框）。
///
/// 顶点网格为 `total_n × total_n`，其中 tile 本体从 `(ov, ov)` 开始，
/// 覆盖 `grid_size × grid_size` 顶点。仅为本体区域生成 `(grid_size-1)²` 个四边形。
fn generate_quad_indices_with_offset(grid_size: u32, offset: u32, total_n: u32) -> Vec<u32> {
    let n = grid_size as usize;
    let off = offset as usize;
    let tn = total_n as usize;
    let mut indices = Vec::with_capacity((n - 1) * (n - 1) * 6);
    for iz in 0..(n - 1) {
        for ix in 0..(n - 1) {
            let tl = ((off + iz) * tn + (off + ix)) as u32;
            let tr = tl + 1;
            let bl = ((off + iz + 1) * tn + (off + ix)) as u32;
            let br = bl + 1;
            indices.extend_from_slice(&[tl, bl, tr, tr, bl, br]);
        }
    }
    indices
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::HeightfieldTerrain;

    // ── Signed Heightfield 测试 ─────────

    #[test]
    fn test_sh_mesh_generation() {
        let terrain = HeightfieldTerrain::new(42);
        let mesh = generate_sh_mesh(&terrain, 0.0, 0.0, 33, 16.0, 0);

        // 33² = 1089 顶点
        assert_eq!(mesh.vertices.len(), 1089);
        assert_eq!(mesh.normals.len(), 1089);
        assert_eq!(mesh.colors.len(), 1089);

        // (33-1)² × 6 = 6144 索引
        assert_eq!(mesh.indices.len(), 6144);

        // 所有法线都是单位向量
        for n in &mesh.normals {
            let len = n.length();
            assert!((len - 1.0).abs() < 0.01, "normal not normalized: {}", len);
        }
    }

    #[test]
    fn test_sh_mesh_deterministic() {
        let t1 = HeightfieldTerrain::new(42);
        let t2 = HeightfieldTerrain::new(42);
        let m1 = generate_sh_mesh(&t1, 0.0, 0.0, 16, 8.0, 0);
        let m2 = generate_sh_mesh(&t2, 0.0, 0.0, 16, 8.0, 0);

        for (a, b) in m1.vertices.iter().zip(m2.vertices.iter()) {
            assert_eq!(a.x, b.x);
            assert_eq!(a.y, b.y);
            assert_eq!(a.z, b.z);
        }
        for (a, b) in m1.colors.iter().zip(m2.colors.iter()) {
            assert_eq!(a.x, b.x);
            assert_eq!(a.y, b.y);
            assert_eq!(a.z, b.z);
        }
    }

    #[test]
    fn test_sh_mesh_normals_normalized() {
        let terrain = HeightfieldTerrain::new(42);
        let mesh = generate_sh_mesh(&terrain, 500.0, 500.0, 33, 16.0, 0);

        for n in &mesh.normals {
            let len = n.length();
            assert!((len - 1.0).abs() < 0.01, "normal len={} not normalized", len);
        }
    }

    #[test]
    fn test_sh_mesh_colors_from_height() {
        let terrain = HeightfieldTerrain::new(42);
        let mesh = generate_sh_mesh(&terrain, 0.0, 0.0, 33, 16.0, 0);

        // 找到最低和最高顶点
        let mut min_h = f32::MAX;
        let mut max_h = f32::MIN;
        for v in &mesh.vertices {
            min_h = min_h.min(v.y);
            max_h = max_h.max(v.y);
        }

        // 低海拔应偏绿/沙色（非白非灰）
        let low_verts: Vec<_> = mesh
            .vertices
            .iter()
            .zip(mesh.colors.iter())
            .filter(|(v, _)| v.y < 5.0)
            .collect();
        if !low_verts.is_empty() {
            // 低地绿色通道 > 蓝色通道
            for (_, c) in &low_verts {
                assert!(
                    c.y > 0.2,
                    "lowland color should have green component, got {:?}",
                    c
                );
            }
        }

        // 高海拔应偏白（高 RGB 值）
        let high_verts: Vec<_> = mesh
            .vertices
            .iter()
            .zip(mesh.colors.iter())
            .filter(|(v, _)| v.y > 400.0)
            .collect();
        if !high_verts.is_empty() {
            for (_, c) in &high_verts {
                let brightness = (c.x + c.y + c.z) / 3.0;
                assert!(
                    brightness > 0.35,
                    "highland color should be bright, got {:?}",
                    c
                );
            }
        }
    }

    #[test]
    fn test_sh_mesh_vertices_in_range() {
        let terrain = HeightfieldTerrain::new(99);
        let spacing = 16.0;
        let grid_size = 33u32;
        let ox = 100.0;
        let oz = 200.0;
        let mesh = generate_sh_mesh(&terrain, ox, oz, grid_size, spacing, 0);

        let max_coord = (grid_size - 1) as f64 * spacing;
        for v in &mesh.vertices {
            assert!(
                v.x as f64 >= ox && v.x as f64 <= ox + max_coord + 0.01,
                "x={} out of range [{}, {}]",
                v.x,
                ox,
                ox + max_coord
            );
            assert!(
                v.z as f64 >= oz && v.z as f64 <= oz + max_coord + 0.01,
                "z={} out of range [{}, {}]",
                v.z,
                oz,
                oz + max_coord
            );
            // 高度在合理范围
            assert!(
                v.y >= -250.0 && v.y <= 800.0,
                "height {} out of range",
                v.y
            );
        }
    }
}
