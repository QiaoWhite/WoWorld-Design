//! OceanPlane — 海洋水面 GodotClass
//!
//! 大面积 PlaneMesh + Gerstner 波顶点 shader。
//! 跟随玩家移动，保持 y=0（海平面）。

use godot::classes::{MeshInstance3D, Node3D, PlaneMesh, ResourceLoader, Shader, ShaderMaterial};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base = Node3D)]
pub struct OceanPlane {
    ocean_mesh: Option<Gd<MeshInstance3D>>,
    player_node: Option<Gd<Node3D>>,

    #[base]
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for OceanPlane {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            ocean_mesh: None,
            player_node: None,
            base,
        }
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

        // 覆盖 LOD 7 最远距离 (10000m) 的整个可渲染区域
        let plane_size = 22000.0;
        let mut plane = PlaneMesh::new_gd();
        plane.set_size(Vector2::new(plane_size, plane_size));
        plane.set_subdivide_width(200);
        plane.set_subdivide_depth(200);

        let mut mi = MeshInstance3D::new_alloc();
        mi.set_name("OceanPlane");
        mi.set_mesh(&plane);
        mi.set_surface_override_material(0, &mat);
        mi.set_position(Vector3::ZERO);

        let node = mi.clone().upcast::<Node>();
        self.base_mut().add_child(&node);
        self.ocean_mesh = Some(mi);

        // 缓存 Player 引用
        self.player_node = self
            .base()
            .get_parent()
            .and_then(|p| p.get_node_or_null("Player"))
            .and_then(|n| n.try_cast::<Node3D>().ok());

        godot_print!(
            "OceanPlane: {}x{}m ocean surface (200×200)",
            plane_size,
            plane_size
        );
    }

    fn process(&mut self, _delta: f64) {
        if let (Some(ref mut mesh), Some(ref player)) = (&mut self.ocean_mesh, &self.player_node) {
            let p = player.get_position();
            mesh.set_position(Vector3::new(p.x, 0.0, p.z));
        }
    }
}
