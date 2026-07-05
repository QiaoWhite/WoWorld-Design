//! EntityRenderer — ECS Entity → Godot Node3D 视觉桥接
//!
//! 每帧 sync: ECS `Position` → Godot `Node3D.global_position`
//! 最小化: 内置 BoxMesh 原语 + 程序化颜色，零资产管线。
//!
//! 架构: Rust ECS 权威 → Godot 纯表现

use godot::classes::{BoxMesh, MeshInstance3D, Node3D, StandardMaterial3D};
use godot::prelude::*;
use glam::Vec3;
use hecs::World as EcsWorld;
use std::collections::HashMap;

use woworld_ecs::components::entity_kind::EntityKind;
use woworld_ecs::components::transform::Position;

/// 管理所有实体在 Godot 侧的可视化节点
pub struct EntityRenderer {
    /// hecs Entity → Godot Node3D（含 BoxMesh 子节点）
    nodes: HashMap<hecs::Entity, Gd<Node3D>>,
    /// 所有实体可视化的父节点容器
    parent: Gd<Node3D>,
    /// 首帧已报告
    first_sync_done: bool,
}

impl EntityRenderer {
    /// 创建 renderer，在 `world_root` 下建 "Entities" 容器
    pub fn new(world_root: &mut Gd<Node3D>) -> Self {
        let mut entities_node = Node3D::new_alloc();
        entities_node.set_name("Entities");
        world_root.add_child(&entities_node);
        Self {
            nodes: HashMap::new(),
            parent: entities_node,
            first_sync_done: false,
        }
    }

    /// 每帧调用：sync ECS 状态到 Godot 节点
    pub fn sync(&mut self, world: &EcsWorld) {
        let mut alive: HashMap<hecs::Entity, Vec3> = HashMap::new();
        for (entity, (pos, kind)) in world.query::<(&Position, &EntityKind)>().iter() {
            if matches!(*kind, EntityKind::Creature) {
                alive.insert(entity, pos.0);
            }
        }

        // 诊断：首帧打印
        if !self.first_sync_done {
            godot_print!("EntityRenderer first sync: {} Creature entities in ECS", alive.len());
            if let Some((e, pos)) = alive.iter().next() {
                godot_print!("  example entity {:?} pos=({:.1}, {:.1}, {:.1})",
                    e.to_bits().get(), pos.x, pos.y, pos.z);
            }
            self.first_sync_done = true;
        }

        // 移除已 despawn 的节点
        self.nodes.retain(|entity, node| {
            if alive.contains_key(entity) {
                true
            } else {
                node.queue_free();
                false
            }
        });

        // 创建/更新节点
        for (entity, pos) in &alive {
            if let Some(node) = self.nodes.get_mut(entity) {
                node.set_global_position(Vector3::new(pos.x, pos.y, pos.z));
            } else {
                let mut parent = self.parent.clone();
                let node = Self::create_entity_node(*entity, *pos, &mut parent);
                self.nodes.insert(*entity, node);
            }
        }
    }

    /// 为一个实体创建 Node3D + BoxMesh 子节点
    ///
    /// root 放在地表 (y=terrain)，2m 立方体放在 root 上方 1m 处。
    fn create_entity_node(entity: hecs::Entity, pos: Vec3, parent: &mut Gd<Node3D>) -> Gd<Node3D> {
        let mut root = Node3D::new_alloc();
        let name_str = format!("NPC_{}", entity.to_bits().get());
        root.set_name(&name_str);

        // 2m BoxMesh —— 大而明显，方便诊断
        let mut box_mesh = BoxMesh::new_gd();
        box_mesh.set_size(Vector3::new(1.0, 2.0, 1.0)); // 1×2×1 m

        let mut mesh_instance = MeshInstance3D::new_alloc();
        mesh_instance.set_mesh(&box_mesh);
        // BoxMesh 原点是几何中心 → 向上偏移 1m 让底部贴 root
        mesh_instance.set_position(Vector3::new(0.0, 1.0, 0.0));

        // 程序化颜色
        let bits = entity.to_bits().get();
        let r = ((bits >> 16) & 0xFF) as f32 / 255.0;
        let g = ((bits >> 8) & 0xFF) as f32 / 255.0;
        let b = (bits & 0xFF) as f32 / 255.0;
        let color = Color::from_rgb(0.3 + r * 0.7, 0.3 + g * 0.7, 0.3 + b * 0.7);

        let mut mat = StandardMaterial3D::new_gd();
        mat.set_albedo(color);
        mat.set_shading_mode(godot::classes::base_material_3d::ShadingMode::UNSHADED);
        mesh_instance.set_surface_override_material(0, &mat);

        root.add_child(&mesh_instance);
        parent.add_child(&root);
        root.set_global_position(Vector3::new(pos.x, pos.y, pos.z));

        root
    }
}
