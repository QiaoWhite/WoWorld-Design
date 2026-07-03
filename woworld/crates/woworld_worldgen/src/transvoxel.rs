//! Transvoxel 等值面提取
//!
//! 基于 Lengyel (2010) "Transition Cells for Dynamic Multiresolution Marching Cubes"。
//! 消费 `DensityStack`（新 P2a 体系），输出 `TerrainMeshData`。
//!
//! ★ 等值面阈值 = 0.0（`DensityProvider` 体系：正值=实体，负值=空）。
//!
//! 查找表：
//! - MC EDGE_TABLE / TRI_TABLE_DATA / EDGE_ENDPOINTS → `tri_table_data` 模块（Paul Bourke 标准值）
//! - 过渡单元 TRANSITION_* → `transition_tables` 模块（Lengyel 2010）

use std::collections::HashMap;

use glam::Vec3;
use woworld_core::density::{DensityProvider, DensityStack};
use woworld_core::prelude::WorldPos;

use crate::terrain_mesh::TerrainMeshData;
use crate::transition_tables::{
    TRANSITION_CELL_CLASS, TRANSITION_CELL_DATA_COUNTS, TRANSITION_CELL_INDICES,
    TRANSITION_CELL_OFFSETS, TRANSITION_VERTEX_DATA,
};
use crate::tri_table_data::{EDGE_ENDPOINTS, EDGE_TABLE, TRI_TABLE_DATA};

// ── 常量 ─────────────────────────────

/// ★ 等值面阈值——正值=实体，负值=空
const ISOVALUE: f32 = 0.0;
/// 梯度有限差分步长（米）
const GRADIENT_EPS: f64 = 0.25;

// MC 查找表（EDGE_TABLE / TRI_TABLE_DATA / EDGE_ENDPOINTS）
// 来自 `tri_table_data` 模块 — Paul Bourke 标准常量，纯数学数据。
// 角点约定（与 global_corners 的 stride 排序一致）：
//   0=(0,0,0) 1=(1,0,0) 2=(1,0,1) 3=(0,0,1)
//   4=(0,1,0) 5=(1,1,0) 6=(1,1,1) 7=(0,1,1)

#[inline]
fn tri_table_entry(case: usize) -> &'static [i8; 16] {
    &TRI_TABLE_DATA[case]
}

// ── 线性插值 ─────────────────────────
fn interpolate(p1: Vec3, p2: Vec3, d1: f32, d2: f32) -> Vec3 {
    if (d1 - ISOVALUE).abs() < f32::EPSILON { return p1; }
    if (d2 - ISOVALUE).abs() < f32::EPSILON { return p2; }
    if d1 == d2 { return (p1 + p2) * 0.5; }
    let t = (ISOVALUE - d1) / (d2 - d1);
    p1 + (p2 - p1) * t
}

// ── 3D 有限差分梯度 ──────────────────
fn gradient_from_stack(stack: &DensityStack, x: f64, y: f64, z: f64) -> Vec3 {
    let eps = GRADIENT_EPS;
    let dx = stack.density_at(WorldPos { x: x + eps, y, z })
           - stack.density_at(WorldPos { x: x - eps, y, z });
    let dy = stack.density_at(WorldPos { x, y: y + eps, z })
           - stack.density_at(WorldPos { x, y: y - eps, z });
    let dz = stack.density_at(WorldPos { x, y, z: z + eps })
           - stack.density_at(WorldPos { x, y, z: z - eps });
    let v = Vec3::new(dx, dy, dz);
    if v.length_squared() < 1e-12 { Vec3::Y } else { v.normalize() }
}

// ── 全局边索引（顶点共享）────────────
#[inline]
fn edge_key(c0: u32, c1: u32) -> u64 {
    let (a, b) = if c0 < c1 { (c0, c1) } else { (c1, c0) };
    ((a as u64) << 32) | (b as u64)
}

#[inline]
fn global_corners(ix: usize, iy: usize, iz: usize, stride_x: usize, stride_y: usize) -> [u32; 8] {
    let base = iz * stride_y + iy * stride_x + ix;
    [base as u32, (base+1) as u32, (base+1+stride_y) as u32, (base+stride_y) as u32,
     (base+stride_x) as u32, (base+1+stride_x) as u32, (base+1+stride_x+stride_y) as u32, (base+stride_x+stride_y) as u32]
}

// ── 过渡单元提取 ─────────────────────
// 从旧 transvoxel.rs 移植，改 DensityStack 签名 + ISOVALUE=0.0
#[allow(clippy::too_many_arguments)]
fn extract_transition_cell(
    face: u8,
    stack: &DensityStack,
    min_y: f32, max_y: f32, min_z: f32, max_z: f32, min_x: f32, max_x: f32,
    half_coarse_vs: f32,
    vertices: &mut Vec<Vec3>, normals: &mut Vec<Vec3>, colors: &mut Vec<Vec3>,
    indices: &mut Vec<u32>,
    base_layer: &dyn DensityProvider,
    material_colors: &HashMap<woworld_core::material::SurfaceMaterial, [f32; 4]>,
) {
    let mid_y = (min_y + max_y) * 0.5;
    let mid_z = (min_z + max_z) * 0.5;
    let mid_x = (min_x + max_x) * 0.5;
    let half_vs = half_coarse_vs;

    let corner_positions: [Vec3; 13] = match face {
        0 => [ // -X
            Vec3::new(min_x, min_y, max_z), Vec3::new(min_x, min_y, min_z),
            Vec3::new(min_x, max_y, max_z), Vec3::new(min_x, max_y, min_z),
            Vec3::new(min_x - half_vs, min_y, max_z), Vec3::new(min_x - half_vs, min_y, min_z),
            Vec3::new(min_x - half_vs, max_y, max_z), Vec3::new(min_x - half_vs, max_y, min_z),
            Vec3::new(min_x - half_vs, mid_y, mid_z),
            Vec3::new(min_x, min_y, mid_z), Vec3::new(min_x, mid_y, min_z),
            Vec3::new(min_x, max_y, mid_z), Vec3::new(min_x, mid_y, max_z),
        ],
        1 => [ // +X
            Vec3::new(max_x, min_y, min_z), Vec3::new(max_x, min_y, max_z),
            Vec3::new(max_x, max_y, min_z), Vec3::new(max_x, max_y, max_z),
            Vec3::new(max_x + half_vs, min_y, min_z), Vec3::new(max_x + half_vs, min_y, max_z),
            Vec3::new(max_x + half_vs, max_y, min_z), Vec3::new(max_x + half_vs, max_y, max_z),
            Vec3::new(max_x + half_vs, mid_y, mid_z),
            Vec3::new(max_x, min_y, mid_z), Vec3::new(max_x, mid_y, max_z),
            Vec3::new(max_x, max_y, mid_z), Vec3::new(max_x, mid_y, min_z),
        ],
        2 => [ // -Z
            Vec3::new(max_x, min_y, min_z), Vec3::new(min_x, min_y, min_z),
            Vec3::new(max_x, max_y, min_z), Vec3::new(min_x, max_y, min_z),
            Vec3::new(max_x, min_y, min_z - half_vs), Vec3::new(min_x, min_y, min_z - half_vs),
            Vec3::new(max_x, max_y, min_z - half_vs), Vec3::new(min_x, max_y, min_z - half_vs),
            Vec3::new(mid_x, mid_y, min_z - half_vs),
            Vec3::new(mid_x, min_y, min_z), Vec3::new(min_x, mid_y, min_z),
            Vec3::new(mid_x, max_y, min_z), Vec3::new(max_x, mid_y, min_z),
        ],
        3 => [ // +Z
            Vec3::new(min_x, min_y, max_z), Vec3::new(max_x, min_y, max_z),
            Vec3::new(min_x, max_y, max_z), Vec3::new(max_x, max_y, max_z),
            Vec3::new(min_x, min_y, max_z + half_vs), Vec3::new(max_x, min_y, max_z + half_vs),
            Vec3::new(min_x, max_y, max_z + half_vs), Vec3::new(max_x, max_y, max_z + half_vs),
            Vec3::new(mid_x, mid_y, max_z + half_vs),
            Vec3::new(mid_x, min_y, max_z), Vec3::new(max_x, mid_y, max_z),
            Vec3::new(mid_x, max_y, max_z), Vec3::new(min_x, mid_y, max_z),
        ],
        _ => return,
    };

    // 9 个面采样点 → 9-bit case index
    let mut case_idx: usize = 0;
    for (i, p) in corner_positions.iter().enumerate().take(9) {
        let d = stack.density_at(WorldPos { x: p.x as f64, y: p.y as f64, z: p.z as f64 });
        if d >= ISOVALUE { case_idx |= 1 << i; }
    }

    let cell_class = TRANSITION_CELL_CLASS[case_idx];
    let class_idx = (cell_class & 0x7F) as usize;
    let reverse_winding = (cell_class & 0x80) != 0;
    let counts = TRANSITION_CELL_DATA_COUNTS[class_idx];
    let vertex_count = (counts >> 4) as usize;
    let triangle_count = (counts & 0x0F) as usize;
    if vertex_count == 0 { return; }

    let vert_data = &TRANSITION_VERTEX_DATA[case_idx];
    let mut face_vert_indices: Vec<u32> = Vec::with_capacity(vertex_count);

    for &edge_code in vert_data.iter().take(vertex_count) {
        let idx_a = ((edge_code >> 4) & 0xF) as usize;
        let idx_b = (edge_code & 0xF) as usize;
        if idx_a >= 13 || idx_b >= 13 { return; }

        let p_a = corner_positions[idx_a];
        let p_b = corner_positions[idx_b];
        let d_a = stack.density_at(WorldPos { x: p_a.x as f64, y: p_a.y as f64, z: p_a.z as f64 });
        let d_b = stack.density_at(WorldPos { x: p_b.x as f64, y: p_b.y as f64, z: p_b.z as f64 });
        let mut pos = interpolate(p_a, p_b, d_a, d_b);

        // Half-Thickness Offset (Lengyel 2010 §4.3)
        {
            let on_face_a = idx_a <= 8;
            let on_face_b = idx_b <= 8;
            let atten: f32 = match (on_face_a, on_face_b) {
                (true, true) => 1.0, (true, false) | (false, true) => 0.5, (false, false) => 0.0,
            };
            if atten > 0.0 {
                let offset_dir = match face {
                    0 => Vec3::new(-1.0, 0.0, 0.0), 1 => Vec3::new(1.0, 0.0, 0.0),
                    2 => Vec3::new(0.0, 0.0, -1.0), 3 => Vec3::new(0.0, 0.0, 1.0),
                    _ => Vec3::ZERO,
                };
                pos += offset_dir * half_vs * atten;
            }
        }

        let normal = gradient_from_stack(stack, pos.x as f64, pos.y as f64, pos.z as f64);
        let mat_id = base_layer.material_at(WorldPos { x: pos.x as f64, y: pos.y as f64, z: pos.z as f64 });
        let color = material_to_color(mat_id, pos.y, material_colors);

        let v_idx = vertices.len() as u32;
        vertices.push(pos); normals.push(normal); colors.push(color);
        face_vert_indices.push(v_idx);
    }

    let offset = TRANSITION_CELL_OFFSETS[class_idx] as usize;
    let tri_start = offset + 1;
    for t in 0..triangle_count {
        let base = tri_start + t * 3;
        let i0 = TRANSITION_CELL_INDICES[base] as usize;
        let i1 = TRANSITION_CELL_INDICES[base + 1] as usize;
        let i2 = TRANSITION_CELL_INDICES[base + 2] as usize;
        if i0 >= face_vert_indices.len() || i1 >= face_vert_indices.len() || i2 >= face_vert_indices.len() { continue; }
        if reverse_winding {
            indices.push(face_vert_indices[i0]); indices.push(face_vert_indices[i2]); indices.push(face_vert_indices[i1]);
        } else {
            indices.push(face_vert_indices[i0]); indices.push(face_vert_indices[i1]); indices.push(face_vert_indices[i2]);
        }
    }
}

// ── 材质 → RGB ──────────────────────
fn material_to_color(
    mat_id: u8, _height: f32,
    material_colors: &HashMap<woworld_core::material::SurfaceMaterial, [f32; 4]>,
) -> Vec3 {
    use woworld_core::material::SurfaceMaterial;
    // u8 → SurfaceMaterial via simple index mapping (matches enum order)
    let mat = match mat_id {
        0 => SurfaceMaterial::Grass, 1 => SurfaceMaterial::Sand, 2 => SurfaceMaterial::Rock,
        3 => SurfaceMaterial::Stone, 4 => SurfaceMaterial::Wood, 5 => SurfaceMaterial::Metal,
        6 => SurfaceMaterial::Water, 7 => SurfaceMaterial::Ice, 8 => SurfaceMaterial::Mud,
        9 => SurfaceMaterial::Snow, 10 => SurfaceMaterial::Gravel, 11 => SurfaceMaterial::Clay,
        12 => SurfaceMaterial::Moss, 13 => SurfaceMaterial::LeafLitter, 14 => SurfaceMaterial::Cobblestone,
        15 => SurfaceMaterial::Marble, 16 => SurfaceMaterial::Glass, 17 => SurfaceMaterial::Fabric,
        18 => SurfaceMaterial::Thatch, 19 => SurfaceMaterial::Bone, 20 => SurfaceMaterial::Flesh,
        _ => SurfaceMaterial::Stone,
    };
    let c = material_colors.get(&mat).copied().unwrap_or([0.4, 0.5, 0.3, 1.0]);
    Vec3::new(c[0], c[1], c[2])
}

// ── 公开 API ──────────────────────

/// Transvoxel 等值面提取（顶点共享版）
///
/// 从 `DensityStack` 提取 3D 等值面，包括：
/// - 常规单元：MC 256-case table + 全局边索引顶点共享
/// - 过渡单元：Lengyel transition cells（消除 LOD 接缝）
#[allow(clippy::too_many_arguments)]
pub fn transvoxel_extract(
    stack: &DensityStack,
    base_layer: &dyn DensityProvider,
    ox: f64, oy: f64, oz: f64,
    voxels_x: u32, voxels_y: u32, voxels_z: u32,
    voxel_size: f64,
    transition_faces: u8,
    material_colors: &HashMap<woworld_core::material::SurfaceMaterial, [f32; 4]>,
) -> TerrainMeshData {
    let sx = voxels_x as usize;
    let sy = voxels_y as usize;
    let sz = voxels_z as usize;
    let nx = sx + 1; let ny = sy + 1; let nz = sz + 1;
    let stride_x = nx;
    let stride_y = nx * ny;
    let total_corners = nx * ny * nz;

    // 1. 密度角点
    let mut densities: Vec<f32> = Vec::with_capacity(total_corners);
    for iz in 0..nz {
        let wz = oz + iz as f64 * voxel_size;
        for iy in 0..ny {
            let wy = oy + iy as f64 * voxel_size;
            for ix in 0..nx {
                let wx = ox + ix as f64 * voxel_size;
                let d = stack.density_at(WorldPos { x: wx, y: wy, z: wz });
                densities.push(d);
            }
        }
    }

    let mut vertices: Vec<Vec3> = Vec::new();
    let mut normals: Vec<Vec3> = Vec::new();
    let mut colors: Vec<Vec3> = Vec::new();
    let mut indices: Vec<u32> = Vec::new();

    // 2. 常规单元遍历 + 顶点共享
    let mut edge_cache: HashMap<u64, u32> = HashMap::new();
    for iz in 0..sz {
        for iy in 0..sy {
            for ix in 0..sx {
                let corners = global_corners(ix, iy, iz, stride_x, stride_y);
                let mut d: [f32; 8] = [0.0; 8];
                let mut case_idx: usize = 0;
                for (ci, &gc) in corners.iter().enumerate() {
                    d[ci] = densities[gc as usize];
                    if d[ci] >= ISOVALUE { case_idx |= 1 << ci; }
                }
                if case_idx == 0 || case_idx == 255 { continue; }

                let edge_mask = EDGE_TABLE[case_idx];
                let mut edge_vert_idx: [u32; 12] = [0; 12];
                for e in 0..12 {
                    if (edge_mask & (1 << e)) == 0 { continue; }
                    let (c0, c1) = EDGE_ENDPOINTS[e];
                    let key = edge_key(corners[c0], corners[c1]);
                    if let Some(&vi) = edge_cache.get(&key) {
                        edge_vert_idx[e] = vi;
                    } else {
                        let p0 = corner_pos(ox, oy, oz, voxel_size, ix, iy, iz, c0);
                        let p1 = corner_pos(ox, oy, oz, voxel_size, ix, iy, iz, c1);
                        let pos = interpolate(p0, p1, d[c0], d[c1]);
                        let n = gradient_from_stack(stack, pos.x as f64, pos.y as f64, pos.z as f64);
                        let mat_id = base_layer.material_at(WorldPos { x: pos.x as f64, y: pos.y as f64, z: pos.z as f64 });
                        let c = material_to_color(mat_id, pos.y, material_colors);
                        let vi = vertices.len() as u32;
                        vertices.push(pos); normals.push(n); colors.push(c);
                        edge_cache.insert(key, vi);
                        edge_vert_idx[e] = vi;
                    }
                }
                let tris = tri_table_entry(case_idx);
                let mut ti = 0;
                while ti + 2 < 16 && tris[ti] != -1 {
                    indices.push(edge_vert_idx[tris[ti] as usize]);
                    indices.push(edge_vert_idx[tris[ti+1] as usize]);
                    indices.push(edge_vert_idx[tris[ti+2] as usize]);
                    ti += 3;
                }
            }
        }
    }

    // 3. 过渡单元
    for face_bit in 0..4u8 {
        if transition_faces & (1 << face_bit) == 0 { continue; }
        let vs2 = voxel_size * 2.0; // coarse cell = 2x fine voxel
        match face_bit {
            0 => { // -X
                for iz in 0..sz { for iy in 0..sy {
                    let cx = ox; let cy = oy + iy as f64 * voxel_size; let cz = oz + iz as f64 * voxel_size;
                    extract_transition_cell(0, stack,
                        cy as f32, (cy + vs2) as f32, cz as f32, (cz + vs2) as f32, cx as f32, (cx + vs2) as f32,
                        voxel_size as f32, &mut vertices, &mut normals, &mut colors, &mut indices, base_layer, material_colors);
                }}
            }
            1 => { // +X
                for iz in 0..sz { for iy in 0..sy {
                    let cx = ox + (sx as f64 - 2.0) * voxel_size; let cy = oy + iy as f64 * voxel_size; let cz = oz + iz as f64 * voxel_size;
                    if sx < 2 { continue; }
                    extract_transition_cell(1, stack,
                        cy as f32, (cy + vs2) as f32, cz as f32, (cz + vs2) as f32, cx as f32, (cx + vs2) as f32,
                        voxel_size as f32, &mut vertices, &mut normals, &mut colors, &mut indices, base_layer, material_colors);
                }}
            }
            2 => { // -Z
                for ix in 0..sx { for iy in 0..sy {
                    let cx = ox + ix as f64 * voxel_size; let cy = oy + iy as f64 * voxel_size; let cz = oz;
                    extract_transition_cell(2, stack,
                        cy as f32, (cy + vs2) as f32, cz as f32, (cz + vs2) as f32, cx as f32, (cx + vs2) as f32,
                        voxel_size as f32, &mut vertices, &mut normals, &mut colors, &mut indices, base_layer, material_colors);
                }}
            }
            3 => { // +Z
                for ix in 0..sx { for iy in 0..sy {
                    let cx = ox + ix as f64 * voxel_size; let cy = oy + iy as f64 * voxel_size; let cz = oz + (sz as f64 - 2.0) * voxel_size;
                    if sz < 2 { continue; }
                    extract_transition_cell(3, stack,
                        cy as f32, (cy + vs2) as f32, cz as f32, (cz + vs2) as f32, cx as f32, (cx + vs2) as f32,
                        voxel_size as f32, &mut vertices, &mut normals, &mut colors, &mut indices, base_layer, material_colors);
                }}
            }
            _ => {}
        }
    }

    TerrainMeshData { vertices, normals, indices, colors }
}

// ── 角点偏移（与 global_corners stride 排序一致）──
const CORNER_DX: [i32; 8] = [0, 1, 1, 0, 0, 1, 1, 0];
const CORNER_DY: [i32; 8] = [0, 0, 0, 0, 1, 1, 1, 1];
const CORNER_DZ: [i32; 8] = [0, 0, 1, 1, 0, 0, 1, 1];

#[allow(clippy::too_many_arguments)]
#[inline]
fn corner_pos(ox: f64, oy: f64, oz: f64, vs: f64, ix: usize, iy: usize, iz: usize, corner: usize) -> Vec3 {
    let dx = CORNER_DX[corner];
    let dy = CORNER_DY[corner];
    let dz = CORNER_DZ[corner];
    Vec3::new(
        (ox + (ix as i32 + dx) as f64 * vs) as f32,
        (oy + (iy as i32 + dy) as f64 * vs) as f32,
        (oz + (iz as i32 + dz) as f64 * vs) as f32,
    )
}

// ── 测试 ────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use woworld_core::density::{DensityProvider, DensityStack};
    use woworld_core::prelude::WorldPos;

    #[derive(Debug)]
    struct SphereDensity { cx: f64, cy: f64, cz: f64, r: f64 }
    impl DensityProvider for SphereDensity {
        fn density_at(&self, pos: WorldPos) -> f32 {
            let dx = pos.x - self.cx; let dy = pos.y - self.cy; let dz = pos.z - self.cz;
            (self.r * self.r - (dx*dx + dy*dy + dz*dz)) as f32
        }
        fn material_at(&self, _pos: WorldPos) -> u8 { 3 } // Stone
        fn priority(&self) -> u8 { 0 }
    }

    #[derive(Debug)]
    struct EmptyDensity;
    impl DensityProvider for EmptyDensity {
        fn density_at(&self, _pos: WorldPos) -> f32 { -1.0 }
        fn material_at(&self, _pos: WorldPos) -> u8 { 0 }
        fn priority(&self) -> u8 { 0 }
    }

    #[test]
    fn test_empty_stack_returns_empty_mesh() {
        let mut stack = DensityStack::new();
        let empty = Arc::new(EmptyDensity);
        stack.push(empty.clone());
        let colors = HashMap::new();
        let mesh = transvoxel_extract(&stack, &*empty, 0.0, 0.0, 0.0, 8, 8, 8, 1.0, 0, &colors);
        assert!(mesh.vertices.is_empty(), "all-negative density should produce no vertices");
        assert!(mesh.indices.is_empty());
    }

    #[test]
    fn test_solid_sphere_produces_surface() {
        let mut stack = DensityStack::new();
        let sphere = Arc::new(SphereDensity { cx: 4.0, cy: 4.0, cz: 4.0, r: 3.0 });
        stack.push(sphere.clone());
        let colors = HashMap::new();
        let mesh = transvoxel_extract(&stack, &*sphere, 0.0, 0.0, 0.0, 8, 8, 8, 1.0, 0, &colors);
        assert!(mesh.vertices.len() > 50, "sphere should produce at least 50 vertices, got {}", mesh.vertices.len());
        assert!(mesh.indices.len() >= 3);
        // 所有顶点应在球面上: |p - c| ≈ r
        for v in &mesh.vertices {
            let d = ((v.x as f64 - 4.0).powi(2) + (v.y as f64 - 4.0).powi(2) + (v.z as f64 - 4.0).powi(2)).sqrt();
            assert!((d - 3.0).abs() < 0.7, "vertex distance from center should be ~3.0, got {d} at {:?}", v);
        }
    }

    #[test]
    fn test_deterministic_output() {
        let mut stack_a = DensityStack::new();
        let sphere_a = Arc::new(SphereDensity { cx: 4.0, cy: 4.0, cz: 4.0, r: 2.0 });
        stack_a.push(sphere_a.clone());
        let colors = HashMap::new();
        let mesh_a = transvoxel_extract(&stack_a, &*sphere_a, 0.0, 0.0, 0.0, 8, 8, 8, 1.0, 0, &colors);

        let mut stack_b = DensityStack::new();
        let sphere_b = Arc::new(SphereDensity { cx: 4.0, cy: 4.0, cz: 4.0, r: 2.0 });
        stack_b.push(sphere_b.clone());
        let mesh_b = transvoxel_extract(&stack_b, &*sphere_b, 0.0, 0.0, 0.0, 8, 8, 8, 1.0, 0, &colors);

        assert_eq!(mesh_a.vertices.len(), mesh_b.vertices.len());
        assert_eq!(mesh_a.indices.len(), mesh_b.indices.len());
        for i in 0..mesh_a.vertices.len() {
            let va = mesh_a.vertices[i]; let vb = mesh_b.vertices[i];
            assert!((va.x - vb.x).abs() < 0.001 && (va.y - vb.y).abs() < 0.001 && (va.z - vb.z).abs() < 0.001,
                "deterministic output mismatch at vertex {i}");
        }
    }

    #[test]
    fn test_edge_cache_vertex_sharing() {
        let mut stack = DensityStack::new();
        let sphere = Arc::new(SphereDensity { cx: 4.0, cy: 4.0, cz: 4.0, r: 3.0 });
        stack.push(sphere.clone());
        let colors = HashMap::new();
        let mesh = transvoxel_extract(&stack, &*sphere, 0.0, 0.0, 0.0, 8, 8, 8, 1.0, 0, &colors);
        // 标准 MC 无共享时顶点数 ≈ 三角形数 × 3；顶点共享后约 1/3
        let unique_vertices = mesh.vertices.len();
        let max_if_unshared = mesh.indices.len();
        assert!(unique_vertices < max_if_unshared,
            "vertex sharing should reduce vertex count: {unique_vertices} < {max_if_unshared}");
    }
}
