//! EntityRenderer — EntityVisual → Godot Node3D 视觉桥接
//!
//! 架构: Rust ECS 权威 → EntityVisual 中间数据 → Godot 纯表现。
//! 只依赖 `woworld_core::entity_visual::EntityVisual`，不直接查询 hecs。
//!
//! 参见: `开发阶段/模型动作与物理系统/007-调试可视化与EntityRenderer架构.md`

use glam::Vec3;
use godot::classes::{CapsuleMesh, Label3D, MeshInstance3D, Node3D, StandardMaterial3D};
use godot::prelude::*;
use std::collections::HashMap;

use woworld_core::entity_visual::EntityVisual;

/// Label3D 距离裁剪阈值（米）
const LABEL_MAX_DISTANCE: f32 = 50.0;

/// 管理所有实体在 Godot 侧的可视化节点
pub struct EntityRenderer {
    /// hecs Entity → Godot Node3D（含 CapsuleMesh 子节点 + 可选 Label3D）
    nodes: HashMap<hecs::Entity, EntityNode>,
    /// 所有实体可视化的父节点容器
    parent: Gd<Node3D>,
    /// 首次 sync 标记
    first_sync_done: bool,
    /// 头顶名字可见（由 nameshow 命令切换）
    name_visible: bool,
    /// 情绪颜色增强（由 debugcolor 命令切换，true=PAD颜色, false=hash颜色）
    color_enhanced: bool,
    /// 玩家位置缓存（用于距离裁剪）
    player_pos: Vec3,
}

/// 单个实体的 Godot 节点集
struct EntityNode {
    root: Gd<Node3D>,
    mesh_instance: Gd<MeshInstance3D>,
    label: Option<Gd<Label3D>>,
    /// 对话气泡标签（头顶,在名字之上,由 speech_bubble_system 驱动）
    bubble_label: Option<Gd<Label3D>>,
}

impl EntityRenderer {
    /// 创建渲染器，在 `world_root` 下建 "Entities" 容器
    pub fn new(world_root: &mut Gd<Node3D>) -> Self {
        let mut entities_node = Node3D::new_alloc();
        entities_node.set_name("Entities");
        world_root.add_child(&entities_node);
        Self {
            nodes: HashMap::new(),
            parent: entities_node,
            first_sync_done: false,
            name_visible: false,
            color_enhanced: false,
            player_pos: Vec3::ZERO,
        }
    }

    /// 每帧调用：消费 EntityVisual 切片，同步 Godot 节点树
    ///
    /// - render_lod >= 4 → 跳过（无 Node3D）
    /// - render_lod >= 2 → CapsuleMesh 无 Label3D
    /// - 距玩家 > LABEL_MAX_DISTANCE → 额外裁剪 Label3D
    pub fn sync(&mut self, visuals: &[(hecs::Entity, &EntityVisual)]) {
        // 诊断
        if !self.first_sync_done {
            godot_print!("EntityRenderer first sync: {} entities", visuals.len());
            self.first_sync_done = true;
        }

        // 收集存活实体
        let alive: HashMap<hecs::Entity, &EntityVisual> =
            visuals.iter().map(|(e, v)| (*e, *v)).collect();

        // 移除已 despawn 的节点
        let to_remove: Vec<hecs::Entity> = self
            .nodes
            .keys()
            .filter(|e| !alive.contains_key(e))
            .copied()
            .collect();
        for entity in to_remove {
            if let Some(mut node) = self.nodes.remove(&entity) {
                node.root.queue_free();
            }
        }

        // 创建/更新节点（分两阶段以避免 borrow 冲突）
        let player = self.player_pos;
        let name_vis = self.name_visible;

        for (entity, visual) in visuals {
            if !visual.is_visible() {
                if let Some(mut node) = self.nodes.remove(entity) {
                    node.root.queue_free();
                }
                continue;
            }

            let exists = self.nodes.contains_key(entity);
            if exists {
                // 更新已有节点 — Gd<T> 是 Copy，通过值操作
                if let Some(node) = self.nodes.get(entity) {
                    let mut root = node.root.clone();
                    root.set_global_position(Vector3::new(
                        visual.position.x,
                        visual.position.y,
                        visual.position.z,
                    ));
                    let rot = visual.rotation;
                    root.set_quaternion(Quaternion::new(rot.x, rot.y, rot.z, rot.w));
                    if let Some(ref label) = node.label {
                        let mut lbl = label.clone();
                        let show = name_vis
                            && visual.show_label()
                            && visual.position.distance(player) <= LABEL_MAX_DISTANCE;
                        lbl.set_visible(show);
                        lbl.set_text(&visual.display_name);
                    }
                    // 对话气泡：有 bubble_text 且在距离/LOD 范围内则显示
                    if let Some(ref bubble) = node.bubble_label {
                        let mut b = bubble.clone();
                        match &visual.bubble_text {
                            Some(text)
                                if visual.show_label()
                                    && visual.position.distance(player) <= LABEL_MAX_DISTANCE =>
                            {
                                b.set_text(text);
                                if let Some(c) = visual.bubble_color {
                                    b.set_modulate(Color::from_rgb(c[0], c[1], c[2]));
                                }
                                b.set_visible(true);
                            }
                            _ => b.set_visible(false),
                        }
                    }
                }
            } else {
                // 创建新节点 — 需要 &mut self.nodes
                let node = self.create_node(*entity, visual);
                self.nodes.insert(*entity, node);
            }
        }
    }

    /// 设置头顶名字可见性（由 nameshow 命令调用）
    pub fn set_name_visible(&mut self, visible: bool) {
        self.name_visible = visible;
        // 批量更新所有 label 的可见性
        let show = visible;
        for node in self.nodes.values_mut() {
            if let Some(ref mut label) = node.label {
                label.set_visible(show);
            }
        }
    }

    /// 设置情绪颜色增强（由 debugcolor 命令调用）
    pub fn set_color_enhanced(&mut self, enhanced: bool) {
        self.color_enhanced = enhanced;
    }

    /// 设置玩家位置（用于距离裁剪）
    pub fn set_player_pos(&mut self, pos: Vec3) {
        self.player_pos = pos;
    }

    /// 为 raycast 选中提供实体 AABB 列表
    pub fn entity_aabbs(&self) -> Vec<(hecs::Entity, Vec3, Vec3)> {
        self.nodes
            .iter()
            .map(|(entity, node)| {
                let pos = node.root.get_global_position();
                let p = Vec3::new(pos.x, pos.y, pos.z);
                let aabb_min = p - Vec3::new(0.4, 0.0, 0.4);
                let aabb_max = p + Vec3::new(0.4, 1.8, 0.4);
                (*entity, aabb_min, aabb_max)
            })
            .collect()
    }

    /// raycast 选中实体——AABB slab 交测，返回最近命中
    pub fn raycast_select(&self, ray_origin: Vec3, ray_dir: Vec3) -> Option<hecs::Entity> {
        let aabbs = self.entity_aabbs();
        let mut closest: Option<(hecs::Entity, f32)> = None;

        for (entity, aabb_min, aabb_max) in &aabbs {
            if let Some(t) = ray_aabb_intersect(ray_origin, ray_dir, *aabb_min, *aabb_max) {
                if closest.map_or(true, |(_, ct)| t < ct) {
                    closest = Some((*entity, t));
                }
            }
        }

        closest.map(|(e, _)| e)
    }

    /// 高亮选中实体（金色），取消时恢复 hash 颜色
    pub fn highlight_entity(&mut self, entity: Option<hecs::Entity>) {
        for (e, node) in self.nodes.iter_mut() {
            let color = if entity == Some(*e) {
                Color::from_rgb(1.0, 0.85, 0.2) // 金色高亮
            } else {
                entity_hash_color(*e)
            };
            let mut mi = node.mesh_instance.clone();
            let mut mat = StandardMaterial3D::new_gd();
            mat.set_albedo(color);
            mat.set_shading_mode(godot::classes::base_material_3d::ShadingMode::UNSHADED);
            mi.set_surface_override_material(0, &mat);
        }
    }

    // ── 内部方法 ────────────────────────

    fn create_node(&mut self, entity: hecs::Entity, visual: &EntityVisual) -> EntityNode {
        let mut root = Node3D::new_alloc();
        let name_str = format!("NPC_{}", entity.to_bits().get());
        root.set_name(&name_str);

        // CapsuleMesh — 人形占位体
        let mut capsule = CapsuleMesh::new_gd();
        capsule.set_radius(0.4);
        capsule.set_height(1.8);

        let mut mesh_instance = MeshInstance3D::new_alloc();
        mesh_instance.set_mesh(&capsule);
        // CapsuleMesh 原点在几何中心 → 向上偏移一半高度让底部贴 root
        mesh_instance.set_position(Vector3::new(0.0, 0.9, 0.0));

        // 颜色
        let color = if self.color_enhanced {
            let c = visual.color_hint;
            Color::from_rgb(c[0], c[1], c[2])
        } else {
            entity_hash_color(entity)
        };

        let mut mat = StandardMaterial3D::new_gd();
        mat.set_albedo(color);
        mat.set_shading_mode(godot::classes::base_material_3d::ShadingMode::UNSHADED);
        mesh_instance.set_surface_override_material(0, &mat);

        // ★ 先入树再设位置（避免 get_global_transform 报错）
        let mut parent = self.parent.clone();
        parent.add_child(&root);

        root.set_global_position(Vector3::new(
            visual.position.x,
            visual.position.y,
            visual.position.z,
        ));

        root.add_child(&mesh_instance);

        // Label3D — 名字标签（初始隐藏，由 nameshow 控制）
        let label = {
            let mut lbl = Label3D::new_alloc();
            lbl.set_text(&visual.display_name);
            lbl.set_billboard_mode(godot::classes::base_material_3d::BillboardMode::ENABLED);
            lbl.set_position(Vector3::new(0.0, 2.2, 0.0)); // 头顶上方
            lbl.set_visible(self.name_visible);
            lbl.set_modulate(Color::from_rgb(1.0, 1.0, 1.0));
            lbl.set_font_size(32);
            root.add_child(&lbl);
            Some(lbl)
        };

        // Label3D — 对话气泡（初始隐藏，由 speech_bubble_system 驱动）
        // 位于名字标签之上；黑色描边保证任意地形背景下可读（对齐 UI 002 §Bark 气泡）。
        let bubble_label = {
            let mut lbl = Label3D::new_alloc();
            lbl.set_billboard_mode(godot::classes::base_material_3d::BillboardMode::ENABLED);
            lbl.set_position(Vector3::new(0.0, 2.7, 0.0)); // 名字(2.2)之上
            lbl.set_visible(false);
            lbl.set_font_size(28);
            lbl.set_outline_size(10); // ≈ font_size/3，防糊
            lbl.set_outline_modulate(Color::from_rgb(0.0, 0.0, 0.0)); // 黑边
            lbl.set_width(300.0); // 最大宽度 300px（UI 002）
            lbl.set_autowrap_mode(godot::classes::text_server::AutowrapMode::WORD_SMART);
            root.add_child(&lbl);
            Some(lbl)
        };

        EntityNode {
            root,
            mesh_instance,
            label,
            bubble_label,
        }
    }

    #[allow(dead_code)]
    fn update_node(
        &mut self,
        node: &mut EntityNode,
        visual: &EntityVisual,
        _entity: &hecs::Entity,
    ) {
        // 位置
        node.root.set_global_position(Vector3::new(
            visual.position.x,
            visual.position.y,
            visual.position.z,
        ));

        // 朝向 — glam Quat → Godot Quaternion
        let rot = visual.rotation;
        node.root
            .set_quaternion(Quaternion::new(rot.x, rot.y, rot.z, rot.w));

        // Label3D 可见性：name_visible + LOD + 距离
        let show_label = self.name_visible
            && visual.show_label()
            && visual.position.distance(self.player_pos) <= LABEL_MAX_DISTANCE;
        if let Some(ref mut label) = node.label {
            label.set_visible(show_label);
            label.set_text(&visual.display_name);
        }

        // 颜色（如果 color_enhanced 模式）
        if self.color_enhanced {
            let c = visual.color_hint;
            if let Some(mat) = node.mesh_instance.get_surface_override_material(0) {
                let mut std_mat = mat.cast::<StandardMaterial3D>();
                std_mat.set_albedo(Color::from_rgb(c[0], c[1], c[2]));
            }
        } else {
            // 确保使用 hash 颜色（只在首次创建时，后续不重复设置）
        }
    }
}

/// 实体→确定性颜色（splitmix hash）
fn entity_hash_color(entity: hecs::Entity) -> Color {
    let id = entity.to_bits().get();
    let hash = |salt: u64| -> f32 {
        let mut x = id.wrapping_add(salt.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        x ^= x >> 31;
        (x >> 40) as f32 / (1u64 << 24) as f32
    };
    Color::from_rgb(
        0.2 + hash(1) * 0.8,
        0.2 + hash(2) * 0.8,
        0.2 + hash(3) * 0.8,
    )
}

/// AABB-ray 交测（slab 方法）——用于控制台鼠标选中
///
/// 返回命中距离 t。零方向分量以 f32::EPSILON 保护避免除零。
fn ray_aabb_intersect(origin: Vec3, dir: Vec3, aabb_min: Vec3, aabb_max: Vec3) -> Option<f32> {
    let eps = f32::EPSILON;
    let inv_x = 1.0
        / if dir.x.abs() < eps {
            eps * dir.x.signum().max(1.0)
        } else {
            dir.x
        };
    let inv_y = 1.0
        / if dir.y.abs() < eps {
            eps * dir.y.signum().max(1.0)
        } else {
            dir.y
        };
    let inv_z = 1.0
        / if dir.z.abs() < eps {
            eps * dir.z.signum().max(1.0)
        } else {
            dir.z
        };

    let inv = Vec3::new(inv_x, inv_y, inv_z);
    let t1 = (aabb_min - origin) * inv;
    let t2 = (aabb_max - origin) * inv;
    let tmin = t1.min(t2);
    let tmax = t1.max(t2);
    let tnear = tmin.x.max(tmin.y).max(tmin.z);
    let tfar = tmax.x.min(tmax.y).min(tmax.z);

    if tnear <= tfar && tfar > 0.0 {
        Some(tnear.max(0.0))
    } else {
        None
    }
}
