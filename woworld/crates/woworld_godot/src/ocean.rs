//! OceanPlane — 海洋水面 GodotClass
//!
//! 大面积 PlaneMesh + Gerstner 波顶点 shader。
//! 放在 y=0（海平面），覆盖玩家视野。

use godot::classes::{MeshInstance3D, PlaneMesh, ResourceLoader, Shader, ShaderMaterial};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base = Node3D)]
pub struct OceanPlane {
    #[base]
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for OceanPlane {
    fn init(base: Base<Node3D>) -> Self {
        Self { base }
    }

    fn ready(&mut self) {
        // 加载 Gerstner 波 shader
        let mut loader = ResourceLoader::singleton();
        let shader = match loader.load("res://shaders/ocean.gdshader") {
            Some(s) => s.cast::<Shader>(),
            None => {
                godot_error!("OceanPlane: failed to load ocean.gdshader");
                return;
            }
        };

        let mut mat = ShaderMaterial::new_gd();
        mat.set_shader(&shader);

        // 800m × 800m 海平面——覆盖 LOD 视野
        let mut plane = PlaneMesh::new_gd();
        plane.set_size(Vector2::new(800.0, 800.0));
        plane.set_subdivide_width(200);
        plane.set_subdivide_depth(200);

        let mut mi = MeshInstance3D::new_alloc();
        mi.set_name("OceanPlane");
        mi.set_mesh(&plane);
        mi.set_surface_override_material(0, &mat);
        // 放在场景原点，y=0 = 海平面
        mi.set_position(Vector3::ZERO);

        let node = mi.upcast::<Node>();
        self.base_mut().add_child(&node);

        godot_print!("OceanPlane: 800x800m Gerstner wave surface initialized");
    }
}
