//! LOD 等级 Component — 每实体每帧 LodCoordinatorSystem 写入
//!
//! 7 维 LOD 输出，镜像 `woworld_core::lod::LodPrescription` 的维度。
//! 消费者 System 通过查询此 Component 自行决定精度。
//!
//! 参见: `开发文档/05-全局基础设施/03-LOD协调器.md`

/// 7 维 LOD 等级——数值越小 = 精度越高
#[derive(Debug, Clone, Copy)]
pub struct LodLevel {
    /// 场景 LOD (0-7): 0.5m 体素 → 64m Billboard
    pub scene_lod: u8,
    /// 骨骼 LOD (0-4): 35 骨 → 0 骨
    pub skeleton_lod: u8,
    /// 动画 LOD (0-4): 9 层全栈 → 无动画
    pub animation_lod: u8,
    /// 渲染 LOD (0-4): 1500 面 → 不可见
    pub render_lod: u8,
    /// 物理 LOD (0-4): 全碰撞+IK → 无碰撞
    pub physics_lod: u8,
    /// 音频 LOD (0-4): 全传播 → 静默
    pub audio_lod: u8,
    /// AI LOD (0-4): 全 GOAP → 仅存在
    pub ai_lod: u8,
}

impl Default for LodLevel {
    fn default() -> Self {
        Self {
            scene_lod: 7,
            skeleton_lod: 4,
            animation_lod: 4,
            render_lod: 4,
            physics_lod: 4,
            audio_lod: 4,
            ai_lod: 4,
        }
    }
}

impl LodLevel {
    /// 从 woworld_core 的 LodPrescription 构造
    pub fn from_prescription(p: &woworld_core::lod::LodPrescription) -> Self {
        Self {
            scene_lod: p.scene_lod,
            skeleton_lod: p.skeleton_lod,
            animation_lod: p.animation_lod,
            render_lod: p.render_lod,
            physics_lod: p.physics_lod,
            audio_lod: p.audio_lod,
            ai_lod: p.ai_lod,
        }
    }

    /// 转换回 woworld_core 的 LodPrescription
    pub fn to_prescription(&self) -> woworld_core::lod::LodPrescription {
        woworld_core::lod::LodPrescription {
            scene_lod: self.scene_lod,
            skeleton_lod: self.skeleton_lod,
            animation_lod: self.animation_lod,
            render_lod: self.render_lod,
            physics_lod: self.physics_lod,
            audio_lod: self.audio_lod,
            ai_lod: self.ai_lod,
        }
    }
}
