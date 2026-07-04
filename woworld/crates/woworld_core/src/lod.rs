//! LOD 协调器类型地基 — CHG-049 Phase 1
//!
//! 定义 `LodPrescription`（7 维 LOD 输出）、距离→LOD 映射函数、
//! 以及 `LodCoordinator` trait。完整 8 步算法（约束/级联/注意/VRAM/帧预算/滞后）
//! 推迟至消费者模块就位后的后续 Phase。
//!
//! 参见: `WoWorld-Design/Change/CHG-049-LOD架构全面深化-20260620.md`

// ── LodPrescription ─────────────────────

/// 7 维 LOD 处方——每实体每帧由 LODCoordinator 生成。
///
/// 各维度取值越小 = 越精细。默认全零 = 最高细节（LOD 0）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LodPrescription {
    /// 场景 LOD (0-7): 地形/建筑/海洋/云/植被统一
    pub scene_lod: u8,
    /// 骨骼 LOD (0-4): 35→33→28→15→0 bones
    pub skeleton_lod: u8,
    /// 动画 LOD (0-4): 9层→6→4→2→0 layers
    pub animation_lod: u8,
    /// 渲染 LOD (0-4): 1500→800→300→impostor→none faces
    pub render_lod: u8,
    /// 物理 LOD (0-4): full collision→capsules→AABB→AABB only→none
    pub physics_lod: u8,
    /// 音频 LOD (0-4): full propagation→simplified→event→detectable→silent
    pub audio_lod: u8,
    /// AI LOD (0-4): full GOAP→lightweight→statistical template→aggregate→exists
    pub ai_lod: u8,
}

impl LodPrescription {
    /// Phase 1 便捷构造：仅设 `scene_lod`，其余维度保持最高细节。
    pub fn new(scene_lod: u8) -> Self {
        Self { scene_lod, ..Self::default() }
    }
}

// ── 场景距离带（与 clipmap LEVELS 对齐）──

/// 场景 LOD 距离带: (min_range, max_range)。
/// 与 `woworld_worldgen::clipmap::LEVELS` 完全对齐。
/// 将相机距离（米）映射到 scene_lod (0-7)。
///
/// 距离带与 `clipmap::LEVELS` 对齐：
/// L0: 0-30m, L1: 30-80m, L2: 80-200m, L3: 200-500m,
/// L4: 500m-1.5km, L5: 1.5-4km, L6: 4-10km, L7: 10-15km。
/// 距离 < 0 clamp 到 0。距离 >= 15000m clamp 到 7。
#[inline]
pub fn distance_to_scene_lod(distance: f64) -> u8 {
    if distance < 0.0 {
        return 0;
    }
    if distance < 30.0 {
        return 0;
    }
    if distance < 80.0 {
        return 1;
    }
    if distance < 200.0 {
        return 2;
    }
    if distance < 500.0 {
        return 3;
    }
    if distance < 1500.0 {
        return 4;
    }
    if distance < 4000.0 {
        return 5;
    }
    if distance < 10000.0 {
        return 6;
    }
    7
}

// ── 角色距离带 ─────────────────────────

/// 角色 LOD 距离带: (min_range, max_range)。CHG-049 §2.2。
/// 将相机距离（米）映射到 char_lod (0-4)。
///
/// 距离带 CHG-049 §2.2：
/// L0: 0-15m, L1: 15-60m, L2: 60-200m, L3: 200-800m, L4: 800m+。
/// 距离 < 0 clamp 到 0。距离 >= 800m clamp 到 4。
#[inline]
pub fn distance_to_char_lod(distance: f64) -> u8 {
    if distance < 0.0 {
        return 0;
    }
    if distance < 15.0 {
        return 0;
    }
    if distance < 60.0 {
        return 1;
    }
    if distance < 200.0 {
        return 2;
    }
    if distance < 800.0 {
        return 3;
    }
    4
}

// ── LodCoordinator trait ────────────────

/// LOD 协调器 trait — Phase 1 最小签名。
///
/// 后续 Phase 将扩展 `compute_lod(input: &LodCoordinatorInput, prev: &HashMap<…>)`
/// 以支持完整 8 步算法。当前仅提供距离映射的默认实现。
pub trait LodCoordinator: Send + Sync {
    /// 场景 LOD 映射（地形/植被/建筑/海洋/云统一）。
    fn compute_scene_lod(distance: f64) -> u8 {
        distance_to_scene_lod(distance)
    }

    /// 角色 LOD 映射（骨骼/动画/渲染/物理/音频/AI）。
    fn compute_char_lod(distance: f64) -> u8 {
        distance_to_char_lod(distance)
    }
}

// ── 单元测试 ────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── scene_lod 边界 ──────────────────

    #[test]
    fn test_scene_lod_boundaries() {
        // L0: [0, 30)
        assert_eq!(distance_to_scene_lod(0.0), 0);
        assert_eq!(distance_to_scene_lod(14.9), 0);
        assert_eq!(distance_to_scene_lod(29.9), 0);
        // L1: [30, 80)
        assert_eq!(distance_to_scene_lod(30.0), 1);
        assert_eq!(distance_to_scene_lod(79.9), 1);
        // L2: [80, 200)
        assert_eq!(distance_to_scene_lod(80.0), 2);
        // L3: [200, 500)
        assert_eq!(distance_to_scene_lod(200.0), 3);
        // L4: [500, 1500)
        assert_eq!(distance_to_scene_lod(500.0), 4);
        // L5: [1500, 4000)
        assert_eq!(distance_to_scene_lod(1500.0), 5);
        // L6: [4000, 10000)
        assert_eq!(distance_to_scene_lod(4000.0), 6);
        // L7: [10000, 15000)
        assert_eq!(distance_to_scene_lod(10000.0), 7);
        assert_eq!(distance_to_scene_lod(14999.9), 7);
        // Clamp
        assert_eq!(distance_to_scene_lod(20000.0), 7);
        assert_eq!(distance_to_scene_lod(-5.0), 0);
    }

    // ── char_lod 边界 ───────────────────

    #[test]
    fn test_char_lod_boundaries() {
        // L0: [0, 15)
        assert_eq!(distance_to_char_lod(0.0), 0);
        assert_eq!(distance_to_char_lod(14.9), 0);
        // L1: [15, 60)
        assert_eq!(distance_to_char_lod(15.0), 1);
        assert_eq!(distance_to_char_lod(59.9), 1);
        // L2: [60, 200)
        assert_eq!(distance_to_char_lod(60.0), 2);
        // L3: [200, 800)
        assert_eq!(distance_to_char_lod(200.0), 3);
        assert_eq!(distance_to_char_lod(799.9), 3);
        // L4: [800, +inf)
        assert_eq!(distance_to_char_lod(800.0), 4);
        assert_eq!(distance_to_char_lod(99999.0), 4);
        // Clamp
        assert_eq!(distance_to_char_lod(-1.0), 0);
    }

    // ── LodPrescription ──────────────────

    #[test]
    fn test_lod_prescription_default_all_zero() {
        let p = LodPrescription::default();
        assert_eq!(p.scene_lod, 0);
        assert_eq!(p.skeleton_lod, 0);
        assert_eq!(p.animation_lod, 0);
        assert_eq!(p.render_lod, 0);
        assert_eq!(p.physics_lod, 0);
        assert_eq!(p.audio_lod, 0);
        assert_eq!(p.ai_lod, 0);
    }

    #[test]
    fn test_lod_prescription_new_scene_only() {
        let p = LodPrescription::new(3);
        assert_eq!(p.scene_lod, 3);
        assert_eq!(p.skeleton_lod, 0);
        assert_eq!(p.animation_lod, 0);
        assert_eq!(p.render_lod, 0);
        assert_eq!(p.physics_lod, 0);
        assert_eq!(p.audio_lod, 0);
        assert_eq!(p.ai_lod, 0);
    }
}
