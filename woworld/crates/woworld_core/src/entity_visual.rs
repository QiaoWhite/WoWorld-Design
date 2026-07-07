//! ECS → Godot 可视化管线数据合同
//!
//! 纯数据结构（引擎无关）——GPU 为 EntityRenderer 和 DebugConsole 提供统一的
//! 数据中间层。所有类型仅依赖 glam，可被 woworld_ecs 和 woworld_godot 平等消费。
//!
//! 参见: `开发阶段/模型动作与物理系统/007-调试可视化与EntityRenderer架构.md`

use glam::{Quat, Vec3};

use crate::types::EntityKind;

// ── EntityVisual ────────────────────────

/// 实体头顶渲染的最小数据集（每实体每帧由 entity_visual_system 生成）
#[derive(Debug, Clone)]
pub struct EntityVisual {
    /// 世界空间位置
    pub position: Vec3,
    /// 世界空间朝向（从 Rotation Component 或 Movement.direction 派生）
    pub rotation: Quat,
    /// 显示名（调试用，种子生成；游戏模式下为空字符串）
    pub display_name: String,
    /// RGB 颜色暗示（从情绪/状态派生，各通道 0.0-1.0）
    pub color_hint: [f32; 3],
    /// 实体种类
    pub kind: EntityKind,
    /// 渲染 LOD 等级: 0=全细节, 4=不可见
    pub render_lod: u8,
}

impl EntityVisual {
    /// 当 render_lod >= 4 时，实体应完全跳过渲染
    pub fn is_visible(&self) -> bool {
        self.render_lod < 4
    }

    /// 当 render_lod >= 2 时，应隐藏 Label3D 标签
    pub fn show_label(&self) -> bool {
        self.render_lod < 2
    }
}

// ── EntityDebugSnapshot ─────────────────

/// 单个实体的完整调试快照（按需生成，仅对选中实体调用 entity_debug_system）
#[derive(Debug, Clone)]
pub struct EntityDebugSnapshot {
    /// hecs Entity 的 bits 表示（跨 FFI 安全）
    pub entity_bits: u64,
    /// 实体种类
    pub kind: EntityKind,
    /// 显示名
    pub display_name: String,
    /// 世界空间位置
    pub position: Vec3,
    /// 分组调试字段
    pub sections: Vec<DebugSection>,
}

// ── DebugSection / DebugField ───────────

/// 一组相关的调试字段（如 "Vitals", "Emotion", "Goal"）
#[derive(Debug, Clone)]
pub struct DebugSection {
    pub title: String,
    pub fields: Vec<DebugField>,
}

/// 单个调试字段
#[derive(Debug, Clone)]
pub struct DebugField {
    /// 字段标签（如 "HP", "Pleasure"）
    pub label: String,
    /// 格式化后的值（如 "85.3 / 100.0"）
    pub value: String,
    /// 可选的 BBCode 颜色标签（如 "#ff6666"），用于控制台 RichTextLabel 着色
    pub color_hint: Option<String>,
}

impl DebugSection {
    /// 便捷构造：单字段 section
    pub fn single(title: impl Into<String>, label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            fields: vec![DebugField {
                label: label.into(),
                value: value.into(),
                color_hint: None,
            }],
        }
    }

    /// 带颜色的字段
    pub fn field_colored(
        label: impl Into<String>,
        value: impl Into<String>,
        color: impl Into<String>,
    ) -> DebugField {
        DebugField {
            label: label.into(),
            value: value.into(),
            color_hint: Some(color.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_visual_is_visible_lod3() {
        let v = EntityVisual {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            display_name: String::new(),
            color_hint: [1.0, 1.0, 1.0],
            kind: EntityKind::Creature,
            render_lod: 3,
        };
        assert!(v.is_visible());
        assert!(!v.show_label());
    }

    #[test]
    fn test_entity_visual_not_visible_lod4() {
        let v = EntityVisual {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            display_name: String::new(),
            color_hint: [1.0, 1.0, 1.0],
            kind: EntityKind::Creature,
            render_lod: 4,
        };
        assert!(!v.is_visible());
    }

    #[test]
    fn test_entity_visual_show_label_lod0() {
        let v = EntityVisual {
            position: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            display_name: String::new(),
            color_hint: [1.0, 1.0, 1.0],
            kind: EntityKind::Creature,
            render_lod: 0,
        };
        assert!(v.show_label());
    }

    #[test]
    fn test_debug_section_single() {
        let s = DebugSection::single("Status", "HP", "100 / 100");
        assert_eq!(s.title, "Status");
        assert_eq!(s.fields.len(), 1);
        assert_eq!(s.fields[0].label, "HP");
        assert_eq!(s.fields[0].value, "100 / 100");
    }

    #[test]
    fn test_debug_field_colored() {
        let f = DebugSection::field_colored("HP", "25", "#ff0000");
        assert_eq!(f.label, "HP");
        assert_eq!(f.value, "25");
        assert_eq!(f.color_hint, Some("#ff0000".into()));
    }
}
