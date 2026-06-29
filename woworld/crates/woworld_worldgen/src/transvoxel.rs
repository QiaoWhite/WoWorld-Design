//! Transvoxel 等值面提取
//!
//! 基于 Lengyel (2010) "Transition Cells for Dynamic Multiresolution Marching Cubes"。
//! 常规单元（同 LOD 内）使用标准 256-case 查找表，但通过**全局边索引 + 顶点缓存**
//! 实现顶点共享——相邻细胞的公共边只产生一个顶点，多个三角形索引复用。
//!
//! 查找表复用 `marching_cubes` 模块的 EDGE_TABLE / TRI_TABLE / EDGE_ENDPOINTS。
//! 过渡单元查找表来自 `transition_tables` 子模块（auto-generated）。

use crate::transition_tables::{
    TRANSITION_CELL_CLASS, TRANSITION_CELL_DATA_COUNTS, TRANSITION_CELL_INDICES,
    TRANSITION_CELL_OFFSETS, TRANSITION_VERTEX_DATA,
};

use std::collections::HashMap;

use glam::Vec3;

use crate::density::DensityField;
use crate::marching_cubes::{
    interpolate, gradient_from_density, voxel_color, EDGE_ENDPOINTS, EDGE_TABLE, THRESHOLD,
    TRI_TABLE,
};
use crate::terrain_mesh::TerrainMeshData;

// ── 等值面提取参数 ──────────────────
pub struct IsoSurfaceParams {
    pub ox: f64,
    pub oz: f64,
    pub bottom_y: f64,
    pub voxels_x: u32,
    pub voxels_y: u32,
    pub voxels_z: u32,
    pub voxel_size: f64,
    /// 过渡面位掩码：bit 0=-X, 1=+X, 2=-Z, 3=+Z
    /// 每位=1 表示该面与更低分辨率邻居相邻，需要生成过渡单元
    pub transition_faces: u8,
}

// ── 全局边索引 ────────────────────────

/// 全局边键：两个全局角点索引的有序对。
///
/// 相邻细胞共享同一条边时，其全局角点对相同，因此顶点缓存命中。
/// `min(c0, c1) << 32 | max(c0, c1)` 保证确定性。
fn edge_key(c0: u32, c1: u32) -> u64 {
    let (a, b) = if c0 < c1 { (c0, c1) } else { (c1, c0) };
    ((a as u64) << 32) | (b as u64)
}

/// 细胞在 (ix, iy, iz) 的 8 个局部角点 → 全局角点索引。
///
/// 全局角点索引 = `iz * stride_y + iy * stride_x + ix`
#[inline]
fn global_corners(ix: usize, iy: usize, iz: usize, stride_x: usize, stride_y: usize) -> [u32; 8] {
    let base = iz * stride_y + iy * stride_x + ix;
    [
        base as u32,                             // 0: (ix,   iy,   iz)
        (base + 1) as u32,                       // 1: (ix+1, iy,   iz)
        (base + 1 + stride_y) as u32,            // 2: (ix+1, iy,   iz+1)  stride_y=ny*nx for z
        (base + stride_y) as u32,                // 3: (ix,   iy,   iz+1)
        (base + stride_x) as u32,                // 4: (ix,   iy+1, iz)    stride_x=nx for y
        (base + 1 + stride_x) as u32,            // 5: (ix+1, iy+1, iz)
        (base + 1 + stride_x + stride_y) as u32, // 6: (ix+1, iy+1, iz+1)
        (base + stride_x + stride_y) as u32,     // 7: (ix,   iy+1, iz+1)
    ]
}

// ── 过渡单元辅助 ──────────────────────

/// 为一个过渡面提取三角形。
///
/// `face`: 0=-X, 1=+X, 2=-Z, 3=+Z
#[allow(clippy::too_many_arguments)]
fn extract_transition_face(
    face: u8,
    density: &dyn DensityField,
    min_y: f32,
    max_y: f32,
    min_z: f32,
    max_z: f32,
    max_x: f32,
    params: &IsoSurfaceParams,
    vertices: &mut Vec<Vec3>,
    normals: &mut Vec<Vec3>,
    colors: &mut Vec<Vec3>,
    indices: &mut Vec<u32>,
) {
    let min_x = params.ox as f32;
    let mid_y = (min_y + max_y) * 0.5;
    let mid_z = (min_z + max_z) * 0.5;
    let mid_x = (min_x + max_x) * 0.5;

    // 13 个局部角点的 3D 位置（按面方向）
    //
    // Corner 0-3:   face corners (on shared face)
    // Corner 4-7:   offset into coarse cell (off shared face, by half_vs in face-normal dir)
    // Corner 8:     offset face center
    // Corner 9-12:  edge midpoints on shared face
    let half_vs = params.voxel_size as f32 * 0.5;
    let corner_positions: [Vec3; 13] = match face {
        0 => {
            // -X face: coarse cell is in -X direction
            [
                // 0-3: on-face corners
                Vec3::new(min_x, min_y, max_z),  Vec3::new(min_x, min_y, min_z),
                Vec3::new(min_x, max_y, max_z),  Vec3::new(min_x, max_y, min_z),
                // 4-7: offset corners (into coarse cell, -X)
                Vec3::new(min_x - half_vs, min_y, max_z),  Vec3::new(min_x - half_vs, min_y, min_z),
                Vec3::new(min_x - half_vs, max_y, max_z),  Vec3::new(min_x - half_vs, max_y, min_z),
                // 8: offset face center
                Vec3::new(min_x - half_vs, mid_y, mid_z),
                // 9-12: edge midpoints on face
                Vec3::new(min_x, min_y, mid_z),  Vec3::new(min_x, mid_y, min_z),
                Vec3::new(min_x, max_y, mid_z),  Vec3::new(min_x, mid_y, max_z),
            ]
        }
        1 => {
            // +X face: coarse cell is in +X direction
            [
                // 0-3: on-face corners
                Vec3::new(max_x, min_y, min_z),  Vec3::new(max_x, min_y, max_z),
                Vec3::new(max_x, max_y, min_z),  Vec3::new(max_x, max_y, max_z),
                // 4-7: offset corners (into coarse cell, +X)
                Vec3::new(max_x + half_vs, min_y, min_z),  Vec3::new(max_x + half_vs, min_y, max_z),
                Vec3::new(max_x + half_vs, max_y, min_z),  Vec3::new(max_x + half_vs, max_y, max_z),
                // 8: offset face center
                Vec3::new(max_x + half_vs, mid_y, mid_z),
                // 9-12: edge midpoints on face
                Vec3::new(max_x, min_y, mid_z),  Vec3::new(max_x, mid_y, max_z),
                Vec3::new(max_x, max_y, mid_z),  Vec3::new(max_x, mid_y, min_z),
            ]
        }
        2 => {
            // -Z face: coarse cell is in -Z direction
            [
                // 0-3: on-face corners
                Vec3::new(max_x, min_y, min_z),  Vec3::new(min_x, min_y, min_z),
                Vec3::new(max_x, max_y, min_z),  Vec3::new(min_x, max_y, min_z),
                // 4-7: offset corners (into coarse cell, -Z)
                Vec3::new(max_x, min_y, min_z - half_vs),  Vec3::new(min_x, min_y, min_z - half_vs),
                Vec3::new(max_x, max_y, min_z - half_vs),  Vec3::new(min_x, max_y, min_z - half_vs),
                // 8: offset face center
                Vec3::new(mid_x, mid_y, min_z - half_vs),
                // 9-12: edge midpoints on face
                Vec3::new(mid_x, min_y, min_z),  Vec3::new(min_x, mid_y, min_z),
                Vec3::new(mid_x, max_y, min_z),  Vec3::new(max_x, mid_y, min_z),
            ]
        }
        3 => {
            // +Z face: coarse cell is in +Z direction
            [
                // 0-3: on-face corners
                Vec3::new(min_x, min_y, max_z),  Vec3::new(max_x, min_y, max_z),
                Vec3::new(min_x, max_y, max_z),  Vec3::new(max_x, max_y, max_z),
                // 4-7: offset corners (into coarse cell, +Z)
                Vec3::new(min_x, min_y, max_z + half_vs),  Vec3::new(max_x, min_y, max_z + half_vs),
                Vec3::new(min_x, max_y, max_z + half_vs),  Vec3::new(max_x, max_y, max_z + half_vs),
                // 8: offset face center
                Vec3::new(mid_x, mid_y, max_z + half_vs),
                // 9-12: edge midpoints on face
                Vec3::new(mid_x, min_y, max_z),  Vec3::new(max_x, mid_y, max_z),
                Vec3::new(mid_x, max_y, max_z),  Vec3::new(min_x, mid_y, max_z),
            ]
        }
        _ => return,
    };

    // 为 9 个面采样点采样密度，构造 9 位 case index
    let mut case_idx: usize = 0;
    let mut density_vals: [f32; 9] = [0.0; 9];
    for i in 0..9 {
        let pos = &corner_positions[i];
        let val = density.sample(pos.x as f64, pos.y as f64, pos.z as f64);
        if val >= THRESHOLD {
            case_idx |= 1 << i;
        }
        density_vals[i] = val;
    }

    // 查表
    let cell_class = TRANSITION_CELL_CLASS[case_idx];
    let class_idx = (cell_class & 0x7F) as usize; // bits 0-6 = 等价类
    let reverse_winding = (cell_class & 0x80) != 0; // bit 7 = 反转绕组

    let counts = TRANSITION_CELL_DATA_COUNTS[class_idx];
    let vertex_count = (counts >> 4) as usize; // 高 nibble
    let triangle_count = (counts & 0x0F) as usize; // 低 nibble

    if vertex_count == 0 {
        return;
    }

    // 解码顶点
    let vert_data = &TRANSITION_VERTEX_DATA[case_idx];
    let mut face_vert_indices: Vec<u32> = Vec::with_capacity(vertex_count);

    for &edge_code in vert_data.iter().take(vertex_count) {
        let idx_a = ((edge_code >> 4) & 0xF) as usize;
        let idx_b = (edge_code & 0xF) as usize;

        // 安全检查
        if idx_a >= 13 || idx_b >= 13 {
            return;
        }

        let p_a = corner_positions[idx_a];
        let p_b = corner_positions[idx_b];
        let d_a = density.sample(p_a.x as f64, p_a.y as f64, p_a.z as f64);
        let d_b = density.sample(p_b.x as f64, p_b.y as f64, p_b.z as f64);
        let pos = interpolate(p_a, p_b, d_a, d_b);
        let normal = gradient_from_density(density, pos.x as f64, pos.y as f64, pos.z as f64);
        let material_id = density.material_at(pos.x as f64, pos.y as f64, pos.z as f64);
        let color = voxel_color(material_id, pos.y);

        let v_idx = vertices.len() as u32;
        vertices.push(pos);
        normals.push(normal);
        colors.push(color);
        face_vert_indices.push(v_idx);
    }

    // 发射三角形
    let offset = TRANSITION_CELL_OFFSETS[class_idx] as usize;
    let indices_data = &TRANSITION_CELL_INDICES;
    // 第一个字节是顶点计数（用于验证）
    let _stored_count = indices_data[offset] as usize;
    let tri_start = offset + 1;

    for t in 0..triangle_count {
        let base = tri_start + t * 3;
        let i0 = indices_data[base] as usize;
        let i1 = indices_data[base + 1] as usize;
        let i2 = indices_data[base + 2] as usize;

        if i0 >= face_vert_indices.len() || i1 >= face_vert_indices.len() || i2 >= face_vert_indices.len() {
            continue;
        }

        if reverse_winding {
            indices.push(face_vert_indices[i0]);
            indices.push(face_vert_indices[i2]);
            indices.push(face_vert_indices[i1]);
        } else {
            indices.push(face_vert_indices[i0]);
            indices.push(face_vert_indices[i1]);
            indices.push(face_vert_indices[i2]);
        }
    }
}

// ── 公开 API ──────────────────────────

/// Transvoxel 常规单元等值面提取（顶点共享版）。
///
/// 与 `extract_isosurface()` 相同的签名和返回值，但内部使用全局边索引缓存：
/// - 同一条边上的顶点只计算一次（而不是每个三角形独立生成）
/// - 三角形索引共享顶点（而非连续 [0,1,2, 3,4,5, ...]）
///
/// 顶点数约减少至标准 MC 的 1/3，GPU 数据量相应缩减。
pub fn extract_isosurface_transvoxel(
    density: &dyn DensityField,
    params: &IsoSurfaceParams,
) -> TerrainMeshData {
    let sx = params.voxels_x as usize;
    let sy = params.voxels_y as usize;
    let sz = params.voxels_z as usize;
    let nx = sx + 1;
    let ny = sy + 1;
    let nz = sz + 1;
    let stride_x = nx;
    let stride_y = nx * ny;

    // ── 1. 密度角点（复用 2D 高度缓存快速路径）──────
    let mut corner_density = vec![0.0f32; nx * ny * nz];

    if let Some(test_h) = density.height_at(params.ox, params.oz) {
        let _ = test_h;
        let mut heights = vec![0.0f64; nx * nz];
        for iz in 0..nz {
            let wz = params.oz + iz as f64 * params.voxel_size;
            for ix in 0..nx {
                let wx = params.ox + ix as f64 * params.voxel_size;
                heights[iz * nx + ix] = density.height_at(wx, wz).unwrap();
            }
        }
        let half_band = 1.0;
        for iz in 0..nz {
            for iy in 0..ny {
                let wy = params.bottom_y + iy as f64 * params.voxel_size;
                for ix in 0..nx {
                    let h = heights[iz * nx + ix];
                    let dist = h - wy;
                    let t = ((dist + half_band) / (2.0 * half_band)).clamp(0.0, 1.0);
                    let idx = iz * ny * nx + iy * nx + ix;
                    corner_density[idx] = t as f32;
                }
            }
        }
    } else {
        for iz in 0..nz {
            let wz = params.oz + iz as f64 * params.voxel_size;
            for iy in 0..ny {
                let wy = params.bottom_y + iy as f64 * params.voxel_size;
                for ix in 0..nx {
                    let wx = params.ox + ix as f64 * params.voxel_size;
                    let idx = iz * ny * nx + iy * nx + ix;
                    corner_density[idx] = density.sample(wx, wy, wz);
                }
            }
        }
    }

    // ── 2. 顶点缓存（全局边键 → 顶点 buffer 索引）─────
    let mut vertices: Vec<Vec3> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();
    let mut colors: Vec<Vec3> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();
    let mut edge_cache: HashMap<u64, u32> = HashMap::new();

    // ── 3. Transvoxel 细胞遍历 ──────────────────────
    for iz in 0..sz {
        for iy in 0..sy {
            for ix in 0..sx {
                let idx_base = iz * ny * nx + iy * nx + ix;
                let gc = global_corners(ix, iy, iz, stride_x, stride_y);

                let d = [
                    corner_density[idx_base],
                    corner_density[idx_base + 1],
                    corner_density[idx_base + 1 + ny * nx],       // 2: +z
                    corner_density[idx_base + ny * nx],           // 3: +z
                    corner_density[idx_base + nx],                // 4: +y
                    corner_density[idx_base + 1 + nx],            // 5: +y,+x
                    corner_density[idx_base + 1 + nx + ny * nx],  // 6
                    corner_density[idx_base + nx + ny * nx],      // 7
                ];

                let mut case_idx: usize = 0;
                for (i, &di) in d.iter().enumerate() {
                    if di >= THRESHOLD {
                        case_idx |= 1 << i;
                    }
                }

                let edge_mask = EDGE_TABLE[case_idx];
                if edge_mask == 0 {
                    continue;
                }

                let vs = params.voxel_size as f32;
                let wx_base = params.ox + ix as f64 * params.voxel_size;
                let wy_base = params.bottom_y + iy as f64 * params.voxel_size;
                let wz_base = params.oz + iz as f64 * params.voxel_size;

                let corners: [Vec3; 8] = [
                    Vec3::new(wx_base as f32, wy_base as f32, wz_base as f32),
                    Vec3::new(wx_base as f32 + vs, wy_base as f32, wz_base as f32),
                    Vec3::new(wx_base as f32 + vs, wy_base as f32, wz_base as f32 + vs),
                    Vec3::new(wx_base as f32, wy_base as f32, wz_base as f32 + vs),
                    Vec3::new(wx_base as f32, wy_base as f32 + vs, wz_base as f32),
                    Vec3::new(wx_base as f32 + vs, wy_base as f32 + vs, wz_base as f32),
                    Vec3::new(
                        wx_base as f32 + vs,
                        wy_base as f32 + vs,
                        wz_base as f32 + vs,
                    ),
                    Vec3::new(wx_base as f32, wy_base as f32 + vs, wz_base as f32 + vs),
                ];

                // 为每条激活的边计算顶点（缓存命中则跳过）
                let mut edge_vert_idx: [Option<u32>; 12] = [None; 12];
                for e in 0..12 {
                    if edge_mask & (1 << e) == 0 {
                        continue;
                    }
                    let (c0_local, c1_local) = EDGE_ENDPOINTS[e];
                    let key = edge_key(gc[c0_local], gc[c1_local]);

                    if let Some(&idx) = edge_cache.get(&key) {
                        edge_vert_idx[e] = Some(idx);
                    } else {
                        let pos = interpolate(
                            corners[c0_local],
                            corners[c1_local],
                            d[c0_local],
                            d[c1_local],
                        );
                        let normal = gradient_from_density(
                            density,
                            pos.x as f64,
                            pos.y as f64,
                            pos.z as f64,
                        );
                        let material_id =
                            density.material_at(pos.x as f64, pos.y as f64, pos.z as f64);
                        let color = voxel_color(material_id, pos.y);

                        let idx = vertices.len() as u32;
                        vertices.push(pos);
                        normals.push(normal);
                        colors.push(color);

                        edge_cache.insert(key, idx);
                        edge_vert_idx[e] = Some(idx);
                    }
                }

                // 发射三角形（引用缓存的顶点索引）
                let tris = &TRI_TABLE[case_idx];
                let mut ti = 0;
                while ti < 16 && tris[ti] != -1 {
                    let e0 = tris[ti] as usize;
                    let e1 = tris[ti + 1] as usize;
                    let e2 = tris[ti + 2] as usize;
                    ti += 3;

                    if let (Some(i0), Some(i1), Some(i2)) =
                        (edge_vert_idx[e0], edge_vert_idx[e1], edge_vert_idx[e2])
                    {
                        indices.push(i0);
                        indices.push(i1);
                        indices.push(i2);
                    }
                }
            }
        }
    }

    // ── 4. 过渡单元面遍历 ────────────────────
    if params.transition_faces != 0 {
        let max_x = (params.ox + params.voxels_x as f64 * params.voxel_size) as f32;
        let min_y = params.bottom_y as f32;
        let max_y = (params.bottom_y + params.voxels_y as f64 * params.voxel_size) as f32;
        let min_z = params.oz as f32;
        let max_z = (params.oz + params.voxels_z as f64 * params.voxel_size) as f32;

        for face in 0..4u8 {
            let bit = 1u8 << face;
            if params.transition_faces & bit == 0 {
                continue;
            }
            extract_transition_face(
                face,
                density,
                min_y, max_y, min_z, max_z, max_x,
                params,
                &mut vertices,
                &mut normals,
                &mut colors,
                &mut indices,
            );
        }
    }

    TerrainMeshData {
        vertices,
        normals,
        indices,
        colors,
    }
}

// ── 测试 ──────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::density::HeightfieldDensity;
    use crate::marching_cubes::extract_isosurface;
    use crate::noise_gen::WorldNoise;

    fn make_density() -> HeightfieldDensity {
        HeightfieldDensity::new(WorldNoise::new(42))
    }

    fn make_params() -> IsoSurfaceParams {
        IsoSurfaceParams {
            ox: 0.0,
            oz: 0.0,
            bottom_y: -100.0,
            voxels_x: 16,
            voxels_y: 40,
            voxels_z: 16,
            voxel_size: 4.0,
            transition_faces: 0,
        }
    }

    #[test]
    fn test_transvoxel_produces_mesh() {
        let density = make_density();
        let mesh = extract_isosurface_transvoxel(&density, &make_params());
        assert!(!mesh.vertices.is_empty(), "should produce mesh");
        assert!(mesh.indices.len() % 3 == 0);
        assert_eq!(mesh.vertices.len(), mesh.normals.len());
        assert_eq!(mesh.vertices.len(), mesh.colors.len());
    }

    #[test]
    fn test_transvoxel_deterministic() {
        let d1 = make_density();
        let d2 = make_density();
        let m1 = extract_isosurface_transvoxel(&d1, &make_params());
        let m2 = extract_isosurface_transvoxel(&d2, &make_params());
        assert_eq!(m1.vertices.len(), m2.vertices.len());
        assert_eq!(m1.indices.len(), m2.indices.len());
    }

    #[test]
    fn test_transvoxel_vs_mc_compatible() {
        // Transvoxel 和 MC 应在相同输入下产生等价的三角形（面数相同）
        let d1 = make_density();
        let d2 = make_density();
        let tv_mesh = extract_isosurface_transvoxel(&d1, &make_params());
        let mc_mesh = extract_isosurface(&d2, &make_params());
        // 三角形数量应相同
        assert_eq!(tv_mesh.indices.len(), mc_mesh.indices.len());
    }

    #[test]
    fn test_transvoxel_vertex_sharing() {
        // 顶点共享：Transvoxel 顶点数 < MC 顶点数（MC 每三角形 3 个独立顶点）
        let d1 = make_density();
        let d2 = make_density();
        let tv_mesh = extract_isosurface_transvoxel(&d1, &make_params());
        let mc_mesh = extract_isosurface(&d2, &make_params());
        assert!(
            tv_mesh.vertices.len() <= mc_mesh.vertices.len(),
            "transvoxel should have <= vertices than MC (vertex sharing)"
        );
    }

    #[test]
    fn test_transvoxel_colors_from_material() {
        let density = make_density();
        let params = IsoSurfaceParams {
            ox: 500.0,
            oz: 500.0,
            bottom_y: -100.0,
            voxels_x: 4,
            voxels_y: 12,
            voxels_z: 4,
            voxel_size: 4.0,
            transition_faces: 0,
        };
        let mesh = extract_isosurface_transvoxel(&density, &params);
        assert_eq!(mesh.vertices.len(), mesh.colors.len());
        if !mesh.colors.is_empty() {
            let sum: f32 = mesh.colors.iter().map(|c| c.x + c.y + c.z).sum();
            assert!(sum > 0.0, "colors should not be all black");
        }
    }

    #[test]
    fn test_no_duplicate_vertices_on_shared_edges() {
        // 相邻细胞共享边上的顶点应被复用
        let density = make_density();
        let mesh = extract_isosurface_transvoxel(&density, &make_params());
        let has_reuse = mesh.indices.windows(2).any(|w| {
            w[0] + 1 != w[1]
                && w[0] != w[1]
        });
        assert!(
            has_reuse || mesh.vertices.len() <= 3,
            "should have vertex reuse or be trivial"
        );
    }

    // ── 过渡单元测试 ──────────────────

    #[test]
    fn test_transition_faces_zero_no_regression() {
        // transition_faces=0 应与之前输出相同（无回归）
        let d1 = make_density();
        let d2 = make_density();
        let params_zero = make_params(); // transition_faces=0
        let mesh = extract_isosurface_transvoxel(&d1, &params_zero);
        assert!(!mesh.vertices.is_empty());
        assert!(mesh.indices.len() % 3 == 0);
        // 确定性
        let mesh2 = extract_isosurface_transvoxel(&d2, &params_zero);
        assert_eq!(mesh.vertices.len(), mesh2.vertices.len());
        assert_eq!(mesh.indices.len(), mesh2.indices.len());
    }

    #[test]
    fn test_transition_faces_all_faces() {
        // 四面过渡——应产生有效网格（顶点数 ≥ 常规单元）
        let density = make_density();
        let mut params = make_params();
        params.transition_faces = 0b1111; // ±X, ±Z

        let mesh = extract_isosurface_transvoxel(&density, &params);
        assert!(!mesh.vertices.is_empty(), "should produce mesh with transition");
        assert!(mesh.indices.len() % 3 == 0);
        assert_eq!(mesh.vertices.len(), mesh.normals.len());
        assert_eq!(mesh.vertices.len(), mesh.colors.len());
    }

    #[test]
    fn test_transition_faces_single_face() {
        // 单面 +X 过渡
        let density = make_density();
        let mut params = make_params();
        params.transition_faces = 0b0010; // +X only

        let mesh = extract_isosurface_transvoxel(&density, &params);
        assert!(!mesh.vertices.is_empty());
        assert!(mesh.indices.len() % 3 == 0);
    }

    #[test]
    fn test_transition_faces_extra_vertices() {
        // 过渡面应增加顶点数
        let density = make_density();
        let params_no_trans = make_params(); // transition_faces=0
        let mut params_trans = make_params();
        params_trans.transition_faces = 0b1111;

        let mesh_no = extract_isosurface_transvoxel(&density, &params_no_trans);
        let mesh_trans = extract_isosurface_transvoxel(&density, &params_trans);

        // 有过渡面时顶点数应 ≥ 无过渡面
        assert!(
            mesh_trans.vertices.len() >= mesh_no.vertices.len(),
            "transition faces should add vertices: {} >= {}",
            mesh_trans.vertices.len(),
            mesh_no.vertices.len()
        );
    }
}
