//! EntityRenderer — ECS Entity → Godot Node3D 视觉桥接
//!
//! 每帧 sync: ECS `Position` → Godot `Node3D.global_position`
//! 最小化: 内置 CapsuleMesh 原语 + 程序化颜色，零资产管线。
//!
//! 架构: Rust ECS 权威 → Godot 纯表现

use godot::classes::{MeshInstance3D, Node3D, StandardMaterial3D};
use godot::prelude::*;
use glam::Vec3;
use hecs::World as EcsWorld;
use std::collections::HashMap;

use woworld_ecs::components::entity_kind::EntityKind;
use woworld_ecs::components::transform::Position;

/// 管理所有实体在 Godot 侧的可视化节点
pub struct EntityRenderer {
    /// hecs Entity → Godot Node3D（含 CapsuleMesh 子节点）
    nodes: HashMap<hecs::Entity, Gd<Node3D>>,
    /// 所有实体可视化的父节点容器
    parent: Gd<Node3D>,
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
        }
    }

    /// 每帧调用：sync ECS 状态到 Godot 节点
    ///
    /// - 为无节点的 Creature entity 创建可视化节点
    /// - 更新已有节点的 world position
    /// - 移除已 despawn entity 的节点
    pub fn sync(&mut self, world: &EcsWorld) {
        // 收集所有 Creature 的当前位置
        let mut alive: HashMap<hecs::Entity, Vec3> = HashMap::new();
        for (entity, (pos, kind)) in world.query::<(&Position, &EntityKind)>().iter() {
            if matches!(*kind, EntityKind::Creature) {
                alive.insert(entity, pos.0);
            }
        }

        // 诊断：首次发现实体时打印
        let new_entities: Vec<_> = alive.keys()
            .filter(|e| !self.nodes.contains_key(e))
            .collect();
        if !new_entities.is_empty() {
            godot_print!("EntityRenderer: creating {} new NPC node(s) (total alive: {})",
                new_entities.len(), alive.len());
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
                // 更新位置
                node.set_global_position(Vector3::new(pos.x, pos.y, pos.z));
            } else {
                // 新实体 → 创建可视化
                let mut parent = self.parent.clone();
                let node = Self::create_npc_node(*entity, *pos, &mut parent);
                self.nodes.insert(*entity, node);
            }
        }
    }

    /// 为一个 NPC 创建 Node3D + CapsuleMesh 子节点
    ///
    /// root Node3D 放在地表（y = terrain），CapsuleMesh 子节点向上偏移
    /// 半个高度，使胶囊体底部刚好贴地。
    fn create_npc_node(entity: hecs::Entity, pos: Vec3, parent: &mut Gd<Node3D>) -> Gd<Node3D> {
        const CAPSULE_HEIGHT: f32 = 1.8;

        // Root Node3D 定位到地表
        let mut root = Node3D::new_alloc();
        let name_str = format!("NPC_{}", entity.to_bits().get());
        root.set_name(&name_str);

        // CapsuleMesh 占位（0.4m 半径 × 1.8m 高，近似人体）
        let mut capsule = godot::classes::CapsuleMesh::new_gd();
        capsule.set_radius(0.4);
        capsule.set_height(CAPSULE_HEIGHT);

        let mut mesh_instance = MeshInstance3D::new_alloc();
        mesh_instance.set_mesh(&capsule);
        // Mesh 原点在几何中心 → 向上偏移半个高度让底部贴地
        mesh_instance.set_position(Vector3::new(0.0, CAPSULE_HEIGHT * 0.5, 0.0));

        // 程序化颜色——从 entity bits 确定性派生
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
        // 先入树再设 global_position——否则 get_global_transform 报 !is_inside_tree()
        parent.add_child(&root);
        root.set_global_position(Vector3::new(pos.x, pos.y, pos.z));

        root
    }

    /// 活跃可视化实体数
    #[allow(dead_code)]
    pub fn entity_count(&self) -> usize {
        self.nodes.len()
    }
}
