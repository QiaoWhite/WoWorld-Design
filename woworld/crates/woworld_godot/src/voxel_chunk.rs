//! VoxelChunk — Transvoxel 等值面块 GodotClass
//!
//! 封装 `transvoxel_extract()` 的 Godot 侧表现。
//! - Goal 1 (MVP): 被动容器——WorldDriver 调用 `set_mesh()` 设置已提取的 ArrayMesh，
//!   `set_world_position()` 移动块位置。
//! - Goal 2 (rayon 后台): 扩展为自驱动——持有 DensityStack 引用，rayon 后台自提取。

use godot::classes::{ArrayMesh, MeshInstance3D, INode3D};
use godot::prelude::*;

/// Transvoxel 等值面块——LOD 0 3D 体素地形的 Godot 表现节点。
///
/// 当前为被动容器模式：WorldDriver 负责提取和上传网格，
/// VoxelChunk 只负责持有 MeshInstance3D 和世界位置。
#[derive(GodotClass)]
#[class(base=Node3D, init)]
pub struct VoxelChunk {
    #[base]
    base: Base<Node3D>,

    /// Child MeshInstance3D holding the terrain ArrayMesh
    mesh_instance: Option<Gd<MeshInstance3D>>,

    /// Chunk world origin (southwest-bottom corner, meters)
    origin_x: f64,
    origin_y: f64,
    origin_z: f64,

    /// Chunk active flag — if false, mesh is stale/discarded
    active: bool,

    /// Material pending application (set before ready() fires)
    pending_material: Option<Gd<godot::classes::Material>>,
}

#[godot_api]
impl INode3D for VoxelChunk {
    fn ready(&mut self) {
        let mut mi = MeshInstance3D::new_alloc();
        mi.set_name("VoxelTerrain");
        mi.set_visible(false);
        // Apply pending material if set before ready() fired
        if let Some(ref mat) = self.pending_material {
            mi.set_material_override(mat);
        }
        self.base_mut().add_child(&mi);
        self.mesh_instance = Some(mi);
    }
}

#[godot_api]
impl VoxelChunk {
    /// Set the material on the terrain MeshInstance3D. Called once from WorldDriver.
    pub fn set_terrain_material(&mut self, material: Gd<godot::classes::Material>) {
        if let Some(ref mut mi) = self.mesh_instance {
            mi.set_material_override(&material);
        } else {
            // ready() hasn't fired yet — store for later application
            self.pending_material = Some(material);
        }
    }

    /// Set the extracted terrain mesh. Called from Rust (WorldDriver).
    /// Passing `None` hides the chunk (e.g. all-air chunk).
    #[func]
    pub fn set_terrain_mesh(&mut self, mesh: Option<Gd<ArrayMesh>>) {
        if let Some(ref mut mi) = self.mesh_instance {
            if let Some(m) = mesh {
                mi.set_mesh(&m);
                mi.set_visible(true);
                self.active = true;
            } else {
                mi.set_visible(false);
                self.active = false;
            }
        }
    }

    /// Get the chunk's world-space origin (Rust-only, not exposed to GDScript).
    pub fn origin_tuple(&self) -> (f64, f64, f64) {
        (self.origin_x, self.origin_y, self.origin_z)
    }

    /// Move this chunk to a new world origin. Does NOT trigger re-extraction.
    #[func]
    pub fn set_world_origin(&mut self, x: f64, y: f64, z: f64) {
        self.origin_x = x;
        self.origin_y = y;
        self.origin_z = z;
        self.base_mut().set_position(Vector3::new(x as f32, y as f32, z as f32));
    }

    /// Get current world origin
    #[func]
    pub fn get_world_origin(&self) -> Vector3 {
        Vector3::new(self.origin_x as f32, self.origin_y as f32, self.origin_z as f32)
    }

    /// Whether this chunk currently has a visible mesh
    #[func]
    pub fn is_active(&self) -> bool {
        self.active
    }
}
