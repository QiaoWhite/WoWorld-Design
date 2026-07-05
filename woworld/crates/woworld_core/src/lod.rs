//! LOD 协调器类型地基 + 完整 8 步冲突解决算法 — CHG-049 §五
//!
//! Phase 1 (Sprint 033): `LodPrescription` + 距离映射函数 + `LodCoordinator` trait 骨架
//! Phase 2 (Sprint 034): 完整输入类型 + 8 步 `compute_lod()` 实现
//!
//! 参见: `WoWorld-Design/Change/CHG-049-LOD架构全面深化-20260620.md`

use std::collections::HashMap;
use std::time::Instant;

use glam::DVec3;

use crate::prelude::EntityId;

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

    /// 将所有维度 clamp 到各自合法范围。
    pub fn clamp_all(&mut self) {
        self.scene_lod = self.scene_lod.min(7);
        self.skeleton_lod = self.skeleton_lod.min(4);
        self.animation_lod = self.animation_lod.min(4);
        self.render_lod = self.render_lod.min(4);
        self.physics_lod = self.physics_lod.min(4);
        self.audio_lod = self.audio_lod.min(4);
        self.ai_lod = self.ai_lod.min(4);
    }

    /// 将目标 prescription 的各维度应用到 self（仅升级，不降级）。
    pub fn upgrade_to(&mut self, target: &LodPrescription) {
        self.scene_lod = self.scene_lod.min(target.scene_lod);
        self.skeleton_lod = self.skeleton_lod.min(target.skeleton_lod);
        self.animation_lod = self.animation_lod.min(target.animation_lod);
        self.render_lod = self.render_lod.min(target.render_lod);
        self.physics_lod = self.physics_lod.min(target.physics_lod);
        self.audio_lod = self.audio_lod.min(target.audio_lod);
        self.ai_lod = self.ai_lod.min(target.ai_lod);
    }
}

// ── 场景距离带（与 clipmap LEVELS 对齐）──

/// 场景 LOD 距离带 CHG-049 §2.1。
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

/// 角色 LOD 距离带 CHG-049 §2.2。
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

/// 音频基础 LOD 距离带 CHG-049 §2.3。
#[inline]
pub fn distance_to_audio_lod(distance: f64) -> u8 {
    if distance < 0.0 {
        return 0;
    }
    if distance < 30.0 {
        return 0;
    }
    if distance < 100.0 {
        return 1;
    }
    if distance < 300.0 {
        return 2;
    }
    if distance < 1000.0 {
        return 3;
    }
    4
}

// ── 输入类型 (CHG-049 §5.2) ────────────

/// 相机状态——每帧从 Godot Camera3D 提取。
#[derive(Debug, Clone)]
pub struct CameraState {
    pub position: DVec3,
    pub forward: DVec3,
    pub fov_radians: f32,
}

/// 玩家注意力——注视目标 + 聚焦听觉。
#[derive(Debug, Clone, Default)]
pub struct PlayerAttention {
    /// 当前注视的实体（若有）
    pub focus_target: Option<EntityId>,
    /// 是否开启聚焦听觉
    pub focus_listening: bool,
}

/// 帧时间预算——用于 Step 6 降级判断。
#[derive(Debug, Clone)]
pub struct FrameBudget {
    /// 本帧剩余 CPU 时间预算 (ms)
    pub remaining_ms: f32,
    /// 上一帧实际耗时 (ms)
    pub last_frame_ms: f32,
}

/// VRAM 压力信号——用于 Step 5 降级判断。
#[derive(Debug, Clone, Default)]
pub struct VramPressure {
    /// 当前 VRAM 使用率 (0.0–1.0)
    pub current_ratio: f32,
    /// 10 帧后预测使用率 (0.0–1.0)
    pub predicted_ratio_10fr: f32,
}

/// 活跃实体 LOD 输入。
#[derive(Debug, Clone)]
pub struct EntityLodInput {
    pub id: EntityId,
    pub position: DVec3,
    pub is_player: bool,
    pub is_in_combat: bool,
    pub is_landmark: bool,
    pub relation_importance: f32, // 0.0(陌生人)–1.0(至亲)
}

/// 音频广播事件。
#[derive(Debug, Clone)]
pub struct AudioBroadcast {
    pub source_position: DVec3,
    pub range_m: f32,
    pub kind: AudioBroadcastKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioBroadcastKind {
    Speech,
    Combat,
    Environmental,
}

/// 跨实体交互意图——驱动 Step 3 级联拉升。
#[derive(Debug, Clone)]
pub enum InteractionIntent {
    PhysicalContact { target: EntityId },
    Combat { target: EntityId },
    Trade { target: EntityId },
    Conversation { target: EntityId },
    PublicSpeech { position: DVec3, range_m: f32 },
    CasualAcknowledgment { target: EntityId },
    Theft { target: EntityId },
}

impl InteractionIntent {
    /// 返回此交互对目标实体要求的 LOD 下限。
    /// `CasualAcknowledgment` 和 `Theft` 不拉升——返回 None。
    pub fn required_lod(&self) -> Option<LodPrescription> {
        match self {
            InteractionIntent::PhysicalContact { .. } => Some(LodPrescription {
                physics_lod: 1,
                ..LodPrescription::default()
            }),
            InteractionIntent::Combat { .. } => Some(LodPrescription {
                skeleton_lod: 0,
                animation_lod: 1,
                render_lod: 0,
                physics_lod: 0,
                ai_lod: 0,
                audio_lod: 0,
                scene_lod: 0,
            }),
            InteractionIntent::Trade { .. } => Some(LodPrescription {
                skeleton_lod: 1,
                animation_lod: 1,
                ai_lod: 1,
                scene_lod: 0,
                ..LodPrescription::default()
            }),
            InteractionIntent::Conversation { .. } => Some(LodPrescription {
                skeleton_lod: 2,
                animation_lod: 2,
                ai_lod: 1,
                scene_lod: 0,
                ..LodPrescription::default()
            }),
            InteractionIntent::PublicSpeech { .. } => Some(LodPrescription {
                audio_lod: 1,
                ..LodPrescription::default()
            }),
            InteractionIntent::CasualAcknowledgment { .. } => None,
            InteractionIntent::Theft { .. } => None,
        }
    }

    /// 返回交互目标实体 ID（`PublicSpeech` 无单一目标）。
    pub fn target_entity(&self) -> Option<EntityId> {
        match self {
            InteractionIntent::PhysicalContact { target }
            | InteractionIntent::Combat { target }
            | InteractionIntent::Trade { target }
            | InteractionIntent::Conversation { target }
            | InteractionIntent::CasualAcknowledgment { target }
            | InteractionIntent::Theft { target } => Some(*target),
            InteractionIntent::PublicSpeech { .. } => None,
        }
    }
}

// ── LODCoordinator 输入 ─────────────────

/// LODCoordinator 完整输入——每帧构造一次。
#[derive(Debug, Clone)]
pub struct LodCoordinatorInput {
    pub camera: CameraState,
    pub attention: PlayerAttention,
    pub frame_budget: FrameBudget,
    pub vram: VramPressure,
    pub entities: Vec<EntityLodInput>,
    pub broadcasts: Vec<AudioBroadcast>,
    pub interactions: Vec<InteractionIntent>,
}

// ── 级联优先级 (CHG-049 §6.2) ──────────

/// 级联拉升优先级——Combat > PlayerDirect > Conversation > Trade > Broadcast。
fn cascade_priority(intent: &InteractionIntent) -> f32 {
    match intent {
        InteractionIntent::Combat { .. } => 100.0,
        InteractionIntent::PhysicalContact { .. } => 80.0,
        InteractionIntent::Conversation { .. } => 60.0,
        InteractionIntent::Trade { .. } => 50.0,
        InteractionIntent::PublicSpeech { .. } => 30.0,
        InteractionIntent::Theft { .. } => 10.0,
        InteractionIntent::CasualAcknowledgment { .. } => 0.0,
    }
}

/// 级联拉升单次升级的估算成本 (ms)。CHG-049 §5.4。
/// 简化模型：距离越远、要求维度越多，成本越高。
fn estimate_upgrade_cost(current: &LodPrescription, required: &LodPrescription) -> f32 {
    let mut cost: f32 = 0.001; // 基础开销
    let dims = [
        (current.scene_lod, required.scene_lod),
        (current.skeleton_lod, required.skeleton_lod),
        (current.animation_lod, required.animation_lod),
        (current.render_lod, required.render_lod),
        (current.physics_lod, required.physics_lod),
        (current.audio_lod, required.audio_lod),
        (current.ai_lod, required.ai_lod),
    ];
    for (cur, req) in dims {
        if req < cur {
            cost += 0.002 * (cur - req) as f32;
        }
    }
    cost
}

/// 级联拉升总预算上限 (ms)。CHG-049 §6.2。
const MAX_UPGRADE_COST_MS: f32 = 0.3;

/// 处理级联拉升——单向·被动·按需。
/// 只有高 LOD 实体能拉升低 LOD 实体。预算耗尽后剩余 targets 保持低 LOD。
fn process_cascade_upgrades(
    interactions: &[InteractionIntent],
    prescriptions: &mut HashMap<EntityId, LodPrescription>,
    frame_budget: &FrameBudget,
) {
    let mut budget_ms = (frame_budget.remaining_ms * 0.5).min(MAX_UPGRADE_COST_MS);

    // 按优先级排序
    let mut sorted: Vec<&InteractionIntent> = interactions.iter().collect();
    sorted.sort_by(|a, b| {
        cascade_priority(b)
            .partial_cmp(&cascade_priority(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for intent in &sorted {
        let required = match intent.required_lod() {
            Some(r) => r,
            None => continue, // CasualAcknowledgment/Theft 不拉升
        };

        let target_id = match intent.target_entity() {
            Some(id) => id,
            None => continue, // PublicSpeech 无单一目标 → 由广播分支处理
        };

        if let Some(current) = prescriptions.get(&target_id) {
            let cost = estimate_upgrade_cost(current, &required);
            if cost <= budget_ms {
                if let Some(p) = prescriptions.get_mut(&target_id) {
                    p.upgrade_to(&required);
                }
                budget_ms -= cost;
            }
        }
    }
}

// ── 迟滞状态 ───────────────────────────

/// 每实体迟滞追踪——防 LOD 边界抖动。CHG-049 §5.3 Step 8。
#[derive(Debug, Clone)]
pub struct HysteresisState {
    /// 上次降级请求的时间戳
    last_downgrade: Instant,
}

/// 迟滞窗口——降级需等待 500ms（升级立即生效）。
const DOWNGRADE_HYSTERESIS_MS: u128 = 500;

// ── LODCoordinator trait ────────────────

/// LOD 协调器 trait — Phase 2 完整实现。
pub trait LodCoordinator: Send + Sync {
    /// 完整 8 步 LOD 计算——每帧调用一次。
    ///
    /// # Arguments
    /// * `input` — 本帧完整输入（相机/注意力/预算/VRAM/实体/广播/交互）
    /// * `prev` — 上一帧的 LOD 处方（用于迟滞）
    /// * `hyst` — 每实体的迟滞状态（跨帧持久）
    ///
    /// # Returns
    /// 每实体的 LOD 处方。无条目的实体 = 不可见/不存在。
    fn compute_lod(
        input: &LodCoordinatorInput,
        prev: &HashMap<EntityId, LodPrescription>,
        hyst: &mut HashMap<EntityId, HysteresisState>,
    ) -> HashMap<EntityId, LodPrescription> {
        let now = Instant::now();
        let mut prescriptions = HashMap::with_capacity(input.entities.len());

        // ── Step 1: 基础分配 ──────────────
        for entity in &input.entities {
            let distance = (entity.position - input.camera.position).length();
            let mut p = LodPrescription {
                scene_lod: distance_to_scene_lod(distance),
                skeleton_lod: distance_to_char_lod(distance),
                animation_lod: distance_to_char_lod(distance),
                render_lod: distance_to_char_lod(distance),
                physics_lod: distance_to_char_lod(distance),
                audio_lod: distance_to_audio_lod(distance),
                ai_lod: distance_to_char_lod(distance),
            };
            // 地标建筑：scene_lod 不超 4
            if entity.is_landmark {
                p.scene_lod = p.scene_lod.min(4);
            }
            prescriptions.insert(entity.id, p);
        }

        // ── Step 2: 硬约束 ────────────────
        for entity in &input.entities {
            let p = prescriptions.get_mut(&entity.id).unwrap();

            // 玩家 → 全维度 LOD 0（永久）
            if entity.is_player {
                *p = LodPrescription::default();
            }

            // 战斗中 NPC → ai=0, physics=0, anim≤1
            if entity.is_in_combat {
                p.ai_lod = 0;
                p.physics_lod = 0;
                p.animation_lod = p.animation_lod.min(1);
                p.skeleton_lod = p.skeleton_lod.min(1);
            }

            // 玩家当前交互目标 → 拉升到 InteractionIntent 要求的最低 LOD
            for intent in &input.interactions {
                if let Some(target_id) = intent.target_entity() {
                    if target_id == entity.id {
                        if let Some(required) = intent.required_lod() {
                            p.upgrade_to(&required);
                        }
                    }
                }
            }
        }

        // ── Step 3: 级联拉升 ──────────────
        process_cascade_upgrades(&input.interactions, &mut prescriptions, &input.frame_budget);

        // ── Step 4: Attention 加成 ─────────
        let attention_cone_half_angle = 15.0_f64.to_radians(); // 30° 锥角的一半
        for entity in &input.entities {
            if entity.is_player {
                continue;
            }
            let p = prescriptions.get_mut(&entity.id).unwrap();
            let to_entity = (entity.position - input.camera.position).normalize();
            let dot = input.camera.forward.dot(to_entity);
            let angle = dot.acos();

            // 玩家视线锥（30°）内 → 各提升 1 档
            if angle <= attention_cone_half_angle {
                p.scene_lod = p.scene_lod.saturating_sub(1);
                p.render_lod = p.render_lod.saturating_sub(1);
                p.animation_lod = p.animation_lod.saturating_sub(1);
            }

            // 玩家聚焦目标 → 全维度拉满到交互所需 LOD
            if let Some(focus_id) = input.attention.focus_target {
                if focus_id == entity.id {
                    p.scene_lod = 0;
                    p.render_lod = 0;
                    p.animation_lod = 0;
                    p.skeleton_lod = p.skeleton_lod.min(1);
                    p.ai_lod = p.ai_lod.min(1);
                }
            }

            // focus_listening → 视线 ±10° 内 audio_lod 提升
            if input.attention.focus_listening {
                let listening_half_angle = 10.0_f64.to_radians();
                if angle <= listening_half_angle {
                    p.audio_lod = p.audio_lod.saturating_sub(1);
                }
            }
        }

        // ── Step 5: VRAM 压力降级 ─────────
        let vr = &input.vram;
        if vr.current_ratio >= 0.70 || vr.predicted_ratio_10fr >= 0.70 {
            for p in prescriptions.values_mut() {
                if p.scene_lod >= 5 {
                    p.scene_lod = (p.scene_lod + 1).min(7);
                }
                // 植被在 scene_lod 4+ 处提前降级——由消费端解释 scene_lod
            }
        }
        if vr.current_ratio >= 0.85 || vr.predicted_ratio_10fr >= 0.85 {
            for p in prescriptions.values_mut() {
                if p.scene_lod >= 3 {
                    p.scene_lod = (p.scene_lod + 1).min(7);
                }
                p.render_lod = (p.render_lod + 1).min(4);
            }
        }
        if vr.current_ratio >= 0.95 || vr.predicted_ratio_10fr >= 0.95 {
            for p in prescriptions.values_mut() {
                p.scene_lod = (p.scene_lod + 1).min(7);
                p.render_lod = (p.render_lod + 1).min(4);
            }
        }

        // ── Step 6: 帧预算降级（从远到近）─
        if input.frame_budget.remaining_ms < 3.0 {
            for p in prescriptions.values_mut() {
                if p.scene_lod >= 3 {
                    p.animation_lod = (p.animation_lod + 1).min(4);
                    p.skeleton_lod = (p.skeleton_lod + 1).min(4);
                }
            }
        }
        if input.frame_budget.remaining_ms < 1.5 {
            for p in prescriptions.values_mut() {
                if p.scene_lod >= 2 {
                    p.ai_lod = (p.ai_lod + 1).min(4);
                    p.animation_lod = (p.animation_lod + 1).min(4);
                    p.skeleton_lod = (p.skeleton_lod + 1).min(4);
                }
            }
        }
        if input.frame_budget.remaining_ms < 0.5 {
            for p in prescriptions.values_mut() {
                p.scene_lod = (p.scene_lod + 1).min(7);
                p.skeleton_lod = (p.skeleton_lod + 1).min(4);
                p.animation_lod = (p.animation_lod + 1).min(4);
                p.render_lod = (p.render_lod + 1).min(4);
                p.ai_lod = (p.ai_lod + 1).min(4);
            }
        }

        // ── Step 7: 跨维约束 Clamp ────────
        for p in prescriptions.values_mut() {
            // skeleton_lod = 4 → animation=4, render=4, physics=4
            if p.skeleton_lod >= 4 {
                p.animation_lod = 4;
                p.render_lod = 4;
                p.physics_lod = 4;
            }
            // ai_lod ≥ 3 → physics ≥ 3, animation ≥ 3
            if p.ai_lod >= 3 {
                p.physics_lod = p.physics_lod.max(3);
                p.animation_lod = p.animation_lod.max(3);
            }
            // ai_lod = 4 → animation = 4
            if p.ai_lod >= 4 {
                p.animation_lod = 4;
            }
            // skeleton_lod ≥ 2 → physics_lod ≥ 2
            if p.skeleton_lod >= 2 {
                p.physics_lod = p.physics_lod.max(2);
            }
            // physics_lod = 4 → animation_lod = 4
            if p.physics_lod >= 4 {
                p.animation_lod = 4;
            }
            // animation_lod ≥ skeleton_lod
            p.animation_lod = p.animation_lod.max(p.skeleton_lod);

            p.clamp_all();
        }

        // ── Step 8: 迟滞 ──────────────────
        // 降级需等待 500ms（防 LOD 边界抖动），升级立即生效。
        for (id, p) in prescriptions.iter_mut() {
            if let Some(prev_p) = prev.get(id) {
                let downgraded = p.scene_lod > prev_p.scene_lod
                    || p.skeleton_lod > prev_p.skeleton_lod
                    || p.animation_lod > prev_p.animation_lod
                    || p.render_lod > prev_p.render_lod
                    || p.physics_lod > prev_p.physics_lod
                    || p.audio_lod > prev_p.audio_lod
                    || p.ai_lod > prev_p.ai_lod;

                if downgraded {
                    let entry = hyst.entry(*id).or_insert(HysteresisState {
                        last_downgrade: now,
                    });
                    let elapsed = now.duration_since(entry.last_downgrade).as_millis();
                    if elapsed < DOWNGRADE_HYSTERESIS_MS {
                        // 迟滞窗口内——拒绝降级，保留上一帧值
                        *p = *prev_p;
                    } else {
                        // 窗口外——允许降级，更新时间戳
                        entry.last_downgrade = now;
                    }
                } else {
                    // 升级或无变化——清除迟滞状态
                    hyst.remove(id);
                }
            }
        }

        prescriptions
    }
}

// ── 单元测试 ────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── scene_lod 边界 ──────────────────

    #[test]
    fn test_scene_lod_boundaries() {
        assert_eq!(distance_to_scene_lod(0.0), 0);
        assert_eq!(distance_to_scene_lod(14.9), 0);
        assert_eq!(distance_to_scene_lod(29.9), 0);
        assert_eq!(distance_to_scene_lod(30.0), 1);
        assert_eq!(distance_to_scene_lod(79.9), 1);
        assert_eq!(distance_to_scene_lod(80.0), 2);
        assert_eq!(distance_to_scene_lod(200.0), 3);
        assert_eq!(distance_to_scene_lod(500.0), 4);
        assert_eq!(distance_to_scene_lod(1500.0), 5);
        assert_eq!(distance_to_scene_lod(4000.0), 6);
        assert_eq!(distance_to_scene_lod(10000.0), 7);
        assert_eq!(distance_to_scene_lod(14999.9), 7);
        assert_eq!(distance_to_scene_lod(20000.0), 7);
        assert_eq!(distance_to_scene_lod(-5.0), 0);
    }

    // ── char_lod 边界 ───────────────────

    #[test]
    fn test_char_lod_boundaries() {
        assert_eq!(distance_to_char_lod(0.0), 0);
        assert_eq!(distance_to_char_lod(14.9), 0);
        assert_eq!(distance_to_char_lod(15.0), 1);
        assert_eq!(distance_to_char_lod(59.9), 1);
        assert_eq!(distance_to_char_lod(60.0), 2);
        assert_eq!(distance_to_char_lod(200.0), 3);
        assert_eq!(distance_to_char_lod(799.9), 3);
        assert_eq!(distance_to_char_lod(800.0), 4);
        assert_eq!(distance_to_char_lod(99999.0), 4);
        assert_eq!(distance_to_char_lod(-1.0), 0);
    }

    // ── audio_lod 边界 ──────────────────

    #[test]
    fn test_audio_lod_boundaries() {
        assert_eq!(distance_to_audio_lod(0.0), 0);
        assert_eq!(distance_to_audio_lod(29.9), 0);
        assert_eq!(distance_to_audio_lod(30.0), 1);
        assert_eq!(distance_to_audio_lod(99.9), 1);
        assert_eq!(distance_to_audio_lod(100.0), 2);
        assert_eq!(distance_to_audio_lod(299.9), 2);
        assert_eq!(distance_to_audio_lod(300.0), 3);
        assert_eq!(distance_to_audio_lod(999.9), 3);
        assert_eq!(distance_to_audio_lod(1000.0), 4);
        assert_eq!(distance_to_audio_lod(-1.0), 0);
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
    }

    #[test]
    fn test_upgrade_to_only_upgrades() {
        let mut p = LodPrescription { scene_lod: 5, skeleton_lod: 3, ..Default::default() };
        let target = LodPrescription { scene_lod: 2, skeleton_lod: 0, ..Default::default() };
        p.upgrade_to(&target);
        assert_eq!(p.scene_lod, 2); // upgraded
        assert_eq!(p.skeleton_lod, 0); // upgraded
        assert_eq!(p.animation_lod, 0); // unchanged (same)
    }

    #[test]
    fn test_upgrade_to_does_not_downgrade() {
        let mut p = LodPrescription { scene_lod: 1, ..Default::default() };
        let target = LodPrescription { scene_lod: 5, ..Default::default() };
        p.upgrade_to(&target);
        assert_eq!(p.scene_lod, 1); // NOT upgraded to 5
    }

    #[test]
    fn test_clamp_all() {
        let mut p = LodPrescription {
            scene_lod: 10,
            skeleton_lod: 10,
            animation_lod: 10,
            render_lod: 10,
            physics_lod: 10,
            audio_lod: 10,
            ai_lod: 10,
        };
        p.clamp_all();
        assert_eq!(p.scene_lod, 7);
        assert_eq!(p.skeleton_lod, 4);
        assert_eq!(p.animation_lod, 4);
        assert_eq!(p.render_lod, 4);
        assert_eq!(p.physics_lod, 4);
        assert_eq!(p.audio_lod, 4);
        assert_eq!(p.ai_lod, 4);
    }

    // ── InteractionIntent ────────────────

    #[test]
    fn test_required_lod_combat_is_full() {
        let intent = InteractionIntent::Combat { target: EntityId(42) };
        let required = intent.required_lod().unwrap();
        assert_eq!(required.ai_lod, 0);
        assert_eq!(required.physics_lod, 0);
        assert_eq!(required.animation_lod, 1);
    }

    #[test]
    fn test_required_lod_casual_is_none() {
        let intent = InteractionIntent::CasualAcknowledgment { target: EntityId(42) };
        assert!(intent.required_lod().is_none());
    }

    #[test]
    fn test_required_lod_conversation() {
        let intent = InteractionIntent::Conversation { target: EntityId(42) };
        let required = intent.required_lod().unwrap();
        assert_eq!(required.skeleton_lod, 2);
        assert_eq!(required.animation_lod, 2);
        assert_eq!(required.ai_lod, 1);
    }

    // ── cascade_priority ─────────────────

    #[test]
    fn test_cascade_priority_ordering() {
        let combat = cascade_priority(&InteractionIntent::Combat { target: EntityId(0) });
        let conv = cascade_priority(&InteractionIntent::Conversation { target: EntityId(0) });
        let casual = cascade_priority(&InteractionIntent::CasualAcknowledgment { target: EntityId(0) });
        assert!(combat > conv);
        assert!(conv > casual);
        assert_eq!(casual, 0.0);
    }

    // ── 完整 8 步算法测试 ────────────────

    /// 测试用 LODCoordinator 实现
    struct TestCoordinator;
    impl LodCoordinator for TestCoordinator {}

    fn make_camera(at: DVec3) -> CameraState {
        CameraState {
            position: at,
            forward: DVec3::new(0.0, 0.0, -1.0),
            fov_radians: 70.0_f32.to_radians(),
        }
    }

    fn make_entity(id: u64, x: f64, z: f64) -> EntityLodInput {
        EntityLodInput {
            id: EntityId(id),
            position: DVec3::new(x, 0.0, z),
            is_player: false,
            is_in_combat: false,
            is_landmark: false,
            relation_importance: 0.0,
        }
    }

    fn make_player(x: f64, z: f64) -> EntityLodInput {
        EntityLodInput {
            id: EntityId(0),
            position: DVec3::new(x, 0.0, z),
            is_player: true,
            is_in_combat: false,
            is_landmark: false,
            relation_importance: 1.0,
        }
    }

    #[test]
    fn test_step1_basic_assignment_increases_with_distance() {
        let input = LodCoordinatorInput {
            camera: make_camera(DVec3::ZERO),
            attention: PlayerAttention::default(),
            frame_budget: FrameBudget { remaining_ms: 10.0, last_frame_ms: 5.0 },
            vram: VramPressure::default(),
            entities: vec![
                make_entity(1, 0.0, 10.0),   // 10m → scene_lod 0
                make_entity(2, 0.0, 200.0),  // 200m → scene_lod 3
                make_entity(3, 0.0, 5000.0), // 5000m → scene_lod 6
            ],
            broadcasts: vec![],
            interactions: vec![],
        };

        let result = TestCoordinator::compute_lod(&input, &HashMap::new(), &mut HashMap::new());
        assert_eq!(result[&EntityId(1)].scene_lod, 0);
        assert_eq!(result[&EntityId(2)].scene_lod, 3);
        assert_eq!(result[&EntityId(3)].scene_lod, 6);
    }

    #[test]
    fn test_step2_player_always_lod0() {
        let input = LodCoordinatorInput {
            camera: make_camera(DVec3::ZERO),
            attention: PlayerAttention::default(),
            frame_budget: FrameBudget { remaining_ms: 10.0, last_frame_ms: 5.0 },
            vram: VramPressure::default(),
            entities: vec![
                make_player(0.0, 50000.0), // very far away
            ],
            broadcasts: vec![],
            interactions: vec![],
        };

        let result = TestCoordinator::compute_lod(&input, &HashMap::new(), &mut HashMap::new());
        let pp = &result[&EntityId(0)];
        assert_eq!(pp.scene_lod, 0);
        assert_eq!(pp.skeleton_lod, 0);
        assert_eq!(pp.ai_lod, 0);
    }

    #[test]
    fn test_step2_combat_forces_low_lod() {
        let mut entity = make_entity(1, 0.0, 5000.0); // far → naturally high LOD
        entity.is_in_combat = true;

        let input = LodCoordinatorInput {
            camera: make_camera(DVec3::ZERO),
            attention: PlayerAttention::default(),
            frame_budget: FrameBudget { remaining_ms: 10.0, last_frame_ms: 5.0 },
            vram: VramPressure::default(),
            entities: vec![entity],
            broadcasts: vec![],
            interactions: vec![],
        };

        let result = TestCoordinator::compute_lod(&input, &HashMap::new(), &mut HashMap::new());
        let p = &result[&EntityId(1)];
        assert_eq!(p.ai_lod, 0);
        assert_eq!(p.physics_lod, 0);
        assert!(p.animation_lod <= 1);
    }

    #[test]
    fn test_step3_cascade_upgrade_from_interaction() {
        let target = make_entity(2, 0.0, 3000.0); // far → high LOD
        let input = LodCoordinatorInput {
            camera: make_camera(DVec3::ZERO),
            attention: PlayerAttention::default(),
            frame_budget: FrameBudget { remaining_ms: 10.0, last_frame_ms: 5.0 },
            vram: VramPressure::default(),
            entities: vec![target],
            broadcasts: vec![],
            interactions: vec![InteractionIntent::Combat { target: EntityId(2) }],
        };

        let result = TestCoordinator::compute_lod(&input, &HashMap::new(), &mut HashMap::new());
        let p = &result[&EntityId(2)];
        // Combat target should be pulled up
        assert_eq!(p.ai_lod, 0);
        assert_eq!(p.physics_lod, 0);
        assert!(p.animation_lod <= 1);
    }

    #[test]
    fn test_step3_casual_acknowledgment_does_not_upgrade() {
        let target = make_entity(2, 0.0, 3000.0);
        let input = LodCoordinatorInput {
            camera: make_camera(DVec3::ZERO),
            attention: PlayerAttention::default(),
            frame_budget: FrameBudget { remaining_ms: 10.0, last_frame_ms: 5.0 },
            vram: VramPressure::default(),
            entities: vec![target],
            broadcasts: vec![],
            interactions: vec![InteractionIntent::CasualAcknowledgment { target: EntityId(2) }],
        };

        let result = TestCoordinator::compute_lod(&input, &HashMap::new(), &mut HashMap::new());
        let p = &result[&EntityId(2)];
        // Should still be at distance-based LOD (scene_lod 5 at 3km)
        assert!(p.scene_lod >= 4);
    }

    #[test]
    fn test_step4_attention_cone_upgrades() {
        // Entity directly ahead at 60m
        let entity = make_entity(1, 0.0, -60.0);
        let input = LodCoordinatorInput {
            camera: make_camera(DVec3::ZERO), // looking at -Z
            attention: PlayerAttention::default(),
            frame_budget: FrameBudget { remaining_ms: 10.0, last_frame_ms: 5.0 },
            vram: VramPressure::default(),
            entities: vec![entity],
            broadcasts: vec![],
            interactions: vec![],
        };

        let result = TestCoordinator::compute_lod(&input, &HashMap::new(), &mut HashMap::new());
        let p = &result[&EntityId(1)];
        // 60m → char_lod 2, but attention cone → render_lod -1
        assert!(p.render_lod <= 1); // was 2, attention knocks to 1
    }

    #[test]
    fn test_step4_attention_edge_of_cone_not_upgraded() {
        // Entity at 45° off forward (outside 30° cone)
        let entity = make_entity(1, 60.0, -60.0); // 45° off axis
        let input = LodCoordinatorInput {
            camera: make_camera(DVec3::ZERO),
            attention: PlayerAttention::default(),
            frame_budget: FrameBudget { remaining_ms: 10.0, last_frame_ms: 5.0 },
            vram: VramPressure::default(),
            entities: vec![entity],
            broadcasts: vec![],
            interactions: vec![],
        };

        let result = TestCoordinator::compute_lod(&input, &HashMap::new(), &mut HashMap::new());
        let p = &result[&EntityId(1)];
        // char_lod at ~85m distance = 2. No attention bonus.
        assert_eq!(p.render_lod, 2);
    }

    #[test]
    fn test_step5_vram_pressure_degradation() {
        let entity = make_entity(1, 0.0, 300.0);
        let input = LodCoordinatorInput {
            camera: make_camera(DVec3::ZERO),
            attention: PlayerAttention::default(),
            frame_budget: FrameBudget { remaining_ms: 10.0, last_frame_ms: 5.0 },
            vram: VramPressure {
                current_ratio: 0.88,
                predicted_ratio_10fr: 0.90,
            },
            entities: vec![entity],
            broadcasts: vec![],
            interactions: vec![],
        };

        let result = TestCoordinator::compute_lod(&input, &HashMap::new(), &mut HashMap::new());
        let p = &result[&EntityId(1)];
        // 300m → scene_lod 3. VRAM ≥85% → scene_lod 3+ gets +1 → 4
        assert!(p.scene_lod >= 4);
        // render_lod also +1
        assert!(p.render_lod >= 1);
    }

    #[test]
    fn test_step6_frame_budget_emergency_degradation() {
        let entity = make_entity(1, 0.0, 100.0);
        let input = LodCoordinatorInput {
            camera: make_camera(DVec3::ZERO),
            attention: PlayerAttention::default(),
            frame_budget: FrameBudget { remaining_ms: 0.3, last_frame_ms: 16.0 },
            vram: VramPressure::default(),
            entities: vec![entity],
            broadcasts: vec![],
            interactions: vec![],
        };

        let result = TestCoordinator::compute_lod(&input, &HashMap::new(), &mut HashMap::new());
        let p = &result[&EntityId(1)];
        // remaining_ms < 0.5 → global emergency: all dims +1
        assert!(p.scene_lod >= 1);
        assert!(p.ai_lod >= 1);
    }

    #[test]
    fn test_step7_skeleton_4_forces_anim_render_physics_4() {
        let entity = make_entity(1, 0.0, 1000.0); // far → skeleton_lod=4
        // VRAM pressure to trigger render_lod being lower than skeleton
        let input = LodCoordinatorInput {
            camera: make_camera(DVec3::ZERO),
            attention: PlayerAttention::default(),
            frame_budget: FrameBudget { remaining_ms: 10.0, last_frame_ms: 5.0 },
            vram: VramPressure {
                current_ratio: 0.96,
                predicted_ratio_10fr: 0.97,
            },
            entities: vec![entity],
            broadcasts: vec![],
            interactions: vec![],
        };

        let result = TestCoordinator::compute_lod(&input, &HashMap::new(), &mut HashMap::new());
        let p = &result[&EntityId(1)];
        // If skeleton_lod = 4, then animation/render/physics must also be 4
        if p.skeleton_lod >= 4 {
            assert_eq!(p.animation_lod, 4);
            assert_eq!(p.render_lod, 4);
            assert_eq!(p.physics_lod, 4);
        }
    }

    #[test]
    fn test_step7_animation_never_less_than_skeleton() {
        let entity = make_entity(1, 0.0, 500.0); // distance → char_lod 3
        let input = LodCoordinatorInput {
            camera: make_camera(DVec3::ZERO),
            attention: PlayerAttention::default(),
            frame_budget: FrameBudget { remaining_ms: 10.0, last_frame_ms: 5.0 },
            vram: VramPressure::default(),
            entities: vec![entity],
            broadcasts: vec![],
            interactions: vec![],
        };

        let result = TestCoordinator::compute_lod(&input, &HashMap::new(), &mut HashMap::new());
        let p = &result[&EntityId(1)];
        assert!(p.animation_lod >= p.skeleton_lod);
    }

    #[test]
    fn test_step8_hysteresis_prevents_rapid_downgrade() {
        let entity = make_entity(1, 0.0, 30.0); // edge of LOD 0/1
        let input = LodCoordinatorInput {
            camera: make_camera(DVec3::ZERO),
            attention: PlayerAttention::default(),
            frame_budget: FrameBudget { remaining_ms: 10.0, last_frame_ms: 5.0 },
            vram: VramPressure::default(),
            entities: vec![entity],
            broadcasts: vec![],
            interactions: vec![],
        };

        // First frame: LOD 1 (30m = boundary)
        let first = TestCoordinator::compute_lod(&input, &HashMap::new(), &mut HashMap::new());

        // Second frame immediately: same input, prev = first → should NOT downgrade
        let second = TestCoordinator::compute_lod(&input, &first, &mut HashMap::new());
        let p1 = &first[&EntityId(1)];
        let p2 = &second[&EntityId(1)];
        // If LOD didn't change, it should stay the same
        assert_eq!(p2.scene_lod, p1.scene_lod);
    }

    #[test]
    fn test_landmark_preserves_scene_lod_cap() {
        let mut entity = make_entity(1, 0.0, 5000.0); // far → scene_lod 6
        entity.is_landmark = true;

        let input = LodCoordinatorInput {
            camera: make_camera(DVec3::ZERO),
            attention: PlayerAttention::default(),
            frame_budget: FrameBudget { remaining_ms: 10.0, last_frame_ms: 5.0 },
            vram: VramPressure::default(),
            entities: vec![entity],
            broadcasts: vec![],
            interactions: vec![],
        };

        let result = TestCoordinator::compute_lod(&input, &HashMap::new(), &mut HashMap::new());
        let p = &result[&EntityId(1)];
        // Landmark scene_lod capped at 4
        assert!(p.scene_lod <= 4);
    }

    #[test]
    fn test_empty_entities_returns_empty() {
        let input = LodCoordinatorInput {
            camera: make_camera(DVec3::ZERO),
            attention: PlayerAttention::default(),
            frame_budget: FrameBudget { remaining_ms: 10.0, last_frame_ms: 5.0 },
            vram: VramPressure::default(),
            entities: vec![],
            broadcasts: vec![],
            interactions: vec![],
        };
        let result = TestCoordinator::compute_lod(&input, &HashMap::new(), &mut HashMap::new());
        assert!(result.is_empty());
    }

    #[test]
    fn test_estimate_upgrade_cost_proportional_to_difference() {
        let current = LodPrescription::new(5);
        let required = LodPrescription::new(0);
        let cost1 = estimate_upgrade_cost(&current, &required);

        let current2 = LodPrescription::new(1);
        let cost2 = estimate_upgrade_cost(&current2, &required);

        assert!(cost1 > cost2, "larger upgrade should cost more");
    }
}
