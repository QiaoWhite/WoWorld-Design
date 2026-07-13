//! 移动状态类型 — Stance/Pace/SpecialMode + MovementState + RecoveryStack + Profile
//!
//! 角色控制器的连续移动数据模型。MovementState 是 MovementSystem 的核心输入。
//! 所有类型为纯数据——仅 glam 依赖，引擎无关。
//!
//! 参见: `WoWorld-Design/.../角色控制器/002-MovementState与连续移动.md`

use crate::kinematics::LocomotionMode;
use crate::material::Medium;
use crate::types::EntityId;

// ── Stance ──────────────────────────────────────────────────────

/// 身体姿态——持久状态，玩家按键切换或 GOAP 设定。
///
/// 影响：碰撞体高度、被感知范围、速度上限。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Stance {
    /// 全高、全速、全暴露
    #[default]
    Standing,
    /// 半高、降速 50%、感知暴露 ×0.4
    Crouching,
    /// 最低、降速 80%、感知暴露 ×0.12
    Prone,
}

// ── Pace ────────────────────────────────────────────────────────

/// 步速——每帧由输入/GOAP 重新设定，非持久。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Pace {
    /// 原地——方向为零
    #[default]
    Still,
    /// 散步——最慢，不耗体力
    Walking,
    /// 慢跑——默认移动速度
    Running,
    /// 冲刺——最快，持续消耗体力。仅 Standing 可用
    Sprinting,
}

// ── SwimPace ────────────────────────────────────────────────────

/// 游泳速度——介质自动切换不覆盖玩家选择。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwimPace {
    /// 安静、省体力、回复体力
    Slow,
    /// 耗体力、产生水花（听觉感知）
    Fast,
    /// 深度可控
    Diving,
    /// 原地维持浮力
    Treading,
}

// ── AirState ────────────────────────────────────────────────────

/// 空中状态——五态细分。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AirState {
    /// 主动跳跃——一定程度的空中控制
    Jumping {
        /// 空中可控转向比例（0.0-1.0）
        control_ratio: f32,
        /// 跳跃高度等级
        height: JumpHeight,
    },
    /// 被动被击飞——几乎不可控，指定恢复时间
    KnockedBack {
        /// 何时恢复可控（游戏秒）
        recoverable_at_secs: f32,
    },
    /// 踩空——土狼时间窗口
    Falling {
        /// 土狼时间剩余 (s)
        coyote_time_remaining: f32,
    },
    /// 完全坠落——不可控
    Terminal,
    /// 主动滑翔
    Gliding,
}

// ── JumpHeight ──────────────────────────────────────────────────

/// 跳跃高度等级。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpHeight {
    /// 小跳——快速落地
    Low,
    /// 普通跳跃
    Normal,
    /// 高跳——长滞空
    High,
}

// ── SpecialMode ─────────────────────────────────────────────────

/// 介质/物理特化模式——存在时覆盖 stance 和 pace。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpecialMode {
    /// 游泳中
    Swimming(SwimPace),
    /// 攀爬中
    Climbing,
    /// 空中
    Airborne(AirState),
    /// 骑乘/挂载中
    Mounted {
        /// 坐骑/载具实体
        anchor: EntityId,
    },
}

// ── MovementState ───────────────────────────────────────────────

/// 角色的连续移动状态——"身体在当前帧以什么姿态、什么速度、在什么介质中移动"。
///
/// MovementSystem 的核心输入之一。与 ActionController（离散动作）的关系：
/// MovementState 永远是**背景状态**，ActionController 的 MovementLock 是**前景覆盖**。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct MovementState {
    /// 身体姿态
    pub stance: Stance,
    /// 步速
    pub pace: Pace,
    /// 介质/物理特化（存在时覆盖 stance + pace）
    pub special: Option<SpecialMode>,
    /// 体力耗尽冷却剩余时间 (s)，> 0 时禁止冲刺
    pub exhaustion_cooldown: f32,
}

impl MovementState {
    /// 新建——默认站立静止。
    pub fn new() -> Self {
        Self::default()
    }

    /// 当前姿态下的最大移动速度 (m/s)。
    pub fn max_speed(&self, profile: &MovementProfile) -> f32 {
        if self.exhaustion_cooldown > 0.0 && self.pace == Pace::Sprinting {
            return profile.walk_speed;
        }
        match self.special {
            Some(SpecialMode::Swimming(p)) => match p {
                SwimPace::Slow | SwimPace::Treading => profile.swim_slow_speed,
                SwimPace::Fast => profile.swim_fast_speed,
                SwimPace::Diving => profile.dive_speed,
            },
            Some(SpecialMode::Climbing) => profile.climb_speed,
            Some(SpecialMode::Airborne(..)) => profile.jump_horizontal_speed,
            Some(SpecialMode::Mounted { .. }) => profile.mounted_speed,
            None => match (self.stance, self.pace) {
                (Stance::Standing, Pace::Walking) => profile.walk_speed,
                (Stance::Standing, Pace::Running) => profile.run_speed,
                (Stance::Standing, Pace::Sprinting) => profile.sprint_speed,
                // 蹲行 ×0.5（002 §1.1）
                (Stance::Crouching, Pace::Walking) => profile.walk_speed * 0.5,
                (Stance::Crouching, Pace::Running) => profile.run_speed * 0.5,
                // 匍匐 ×0.2
                (Stance::Prone, Pace::Walking) => profile.walk_speed * 0.2,
                _ => 0.0,
            },
        }
    }

    /// 当前加速度 (m/s²)。
    pub fn acceleration(&self, profile: &MovementProfile) -> f32 {
        match self.special {
            Some(SpecialMode::Swimming(p)) => match p {
                SwimPace::Slow | SwimPace::Treading => profile.swim_accel_slow,
                SwimPace::Fast => profile.swim_accel_fast,
                SwimPace::Diving => profile.swim_accel_slow,
            },
            Some(SpecialMode::Climbing) => profile.climb_accel,
            Some(SpecialMode::Airborne(..)) => profile.ground_accel * 0.3,
            Some(SpecialMode::Mounted { .. }) => profile.mounted_accel,
            None => {
                if self.pace == Pace::Sprinting {
                    profile.sprint_accel
                } else {
                    profile.ground_accel
                }
            }
        }
    }

    /// 当前摩擦力 (m/s²)。
    pub fn friction(&self, profile: &MovementProfile) -> f32 {
        match self.special {
            Some(SpecialMode::Swimming(..)) => profile.swim_friction,
            Some(SpecialMode::Climbing) => profile.climb_friction,
            Some(SpecialMode::Airborne(..)) => 0.0, // 空中无摩擦
            Some(SpecialMode::Mounted { .. }) => profile.mounted_friction,
            None => {
                if self.pace == Pace::Sprinting {
                    profile.sprint_friction
                } else {
                    profile.ground_friction
                }
            }
        }
    }

    /// 当前体力消耗速率（单位/秒，正=消耗，负=回复）。
    pub fn stamina_rate(&self, profile: &MovementProfile) -> f32 {
        match self.special {
            Some(SpecialMode::Swimming(p)) => match p {
                SwimPace::Slow => profile.swim_slow_stamina_rate,
                // 踩水维持浮力→消耗体力（002 §五 treading_stamina_rate）
                SwimPace::Treading => profile.treading_stamina_rate,
                SwimPace::Fast => profile.swim_fast_stamina_rate,
                SwimPace::Diving => profile.swim_fast_stamina_rate,
            },
            Some(SpecialMode::Climbing) => profile.climb_stamina_rate,
            Some(SpecialMode::Airborne(AirState::Gliding)) => profile.glide_stamina_rate,
            Some(SpecialMode::Airborne(..)) => 0.0,
            Some(SpecialMode::Mounted { .. }) => 0.0, // 坐骑不耗骑手体力
            None => {
                if self.pace == Pace::Sprinting {
                    profile.sprint_stamina_rate
                } else {
                    0.0 // 非冲刺不耗体力
                }
            }
        }
    }

    /// 当前转身速率 (°/s)。
    pub fn turn_rate(&self, profile: &MovementProfile) -> f32 {
        match self.special {
            Some(SpecialMode::Airborne(AirState::KnockedBack { .. })) => profile.knocked_turn_rate,
            Some(SpecialMode::Airborne(..)) => profile.air_turn_rate,
            _ => {
                if self.pace == Pace::Sprinting {
                    profile.sprint_turn_rate
                } else {
                    profile.default_turn_rate
                }
            }
        }
    }

    /// 当前 MovementState 是否与给定 LocomotionMode + Medium 兼容。
    pub fn is_compatible_with(&self, _loco: LocomotionMode, _medium: Medium) -> bool {
        // Sprint 1: 简化实现——始终兼容。
        // CHG-067 + 介质系统完善后精细化。
        true
    }

    /// 是否为自愿地面状态（无 SpecialMode，可入恢复栈）。
    pub fn is_voluntary_ground(&self) -> bool {
        self.special.is_none()
    }

    /// 生成步态参数——供动画系统消费。
    ///
    /// Sprint 1: 返回默认值（全 0.5）。GaitProfile 类型尚未定义（设计留白）。
    /// 动画系统 sprint 中实现完整调制逻辑。
    pub fn gait_params(&self) -> GaitParams {
        GaitParams::default()
    }
}

// ── MovementRecoveryStack ───────────────────────────────────────

/// 介质变迁恢复栈——嵌套物理过渡的恢复机制。
///
/// 最大深度 3（地面/空中/水各一层）。
/// 使用固定数组 + u8 计数，零堆分配。
#[derive(Debug, Clone, Copy, Default)]
pub struct MovementRecoveryStack {
    stack: [MovementState; 3],
    len: u8,
}

impl MovementRecoveryStack {
    /// 若当前状态为自愿地面状态，推入栈顶。
    /// 栈满时静默丢弃最早条目（FIFO 降级——保护最近的）。
    pub fn push_if_voluntary(&mut self, state: MovementState) {
        if !state.is_voluntary_ground() {
            return;
        }
        if self.len >= 3 {
            // 栈满——移位降级：丢弃 stack[0]，保留 stack[1],stack[2]
            self.stack[0] = self.stack[1];
            self.stack[1] = self.stack[2];
            self.stack[2] = state;
            // len 保持 3
        } else {
            self.stack[self.len as usize] = state;
            self.len += 1;
        }
    }

    /// 弹出栈顶兼容条目——跳过与给定 loco/medium 不兼容的。
    ///
    /// 返回恢复后的 MovementState。栈空时返回默认状态。
    pub fn pop_compatible(&mut self, loco: LocomotionMode, medium: Medium) -> MovementState {
        while self.len > 0 {
            let idx = (self.len - 1) as usize;
            let candidate = self.stack[idx];
            self.len -= 1;

            if candidate.is_compatible_with(loco, medium) {
                return candidate;
            }
            // 不兼容——跳过，继续向下找
        }
        // 栈空——返回默认地面状态
        MovementState::default()
    }

    /// 清空恢复栈（死亡/传送时调用）。
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// 栈中条目数。
    pub fn len(&self) -> u8 {
        self.len
    }

    /// 栈是否为空。
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
}

// ── StanceTransition ───────────────────────────────────────────

/// 姿态切换时长（毫秒）。
pub struct StanceTransition;

impl StanceTransition {
    pub const STAND_CROUCH_MS: u32 = 250;
    pub const CROUCH_PRONE_MS: u32 = 400;
    pub const STAND_PRONE_MS: u32 = 350;

    /// 查表获取从 from 到 to 的切换时长。
    pub const fn duration(from: Stance, to: Stance) -> u32 {
        use Stance::*;
        match (from, to) {
            (Standing, Crouching) | (Crouching, Standing) => Self::STAND_CROUCH_MS,
            (Crouching, Prone) | (Prone, Crouching) => Self::CROUCH_PRONE_MS,
            (Standing, Prone) | (Prone, Standing) => Self::STAND_PRONE_MS,
            _ => 0, // 同姿态无切换
        }
    }
}

// ── GaitParams ──────────────────────────────────────────────────

/// 步态参数——9 个连续值（0.0-1.0），供动画系统 9 层动画栈 Layer 0-2 消费。
///
/// 从 MovementState + GaitProfile + BigFive + EmotionState + fatigue 涌现。
/// Sprint 1: 返回默认值。完整调制在动画系统 sprint 中实现。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GaitParams {
    pub hip_sway: f32,
    pub stride_length: f32,
    pub arm_swing: f32,
    pub bounce: f32,
    pub forward_lean: f32,
    pub rhythm_regularity: f32,
    pub foot_drag: f32,
    pub shoulder_stability: f32,
    pub gaze_level: f32,
}

impl Default for GaitParams {
    fn default() -> Self {
        Self {
            hip_sway: 0.5,
            stride_length: 0.5,
            arm_swing: 0.5,
            bounce: 0.5,
            forward_lean: 0.5,
            rhythm_regularity: 0.5,
            foot_drag: 0.5,
            shoulder_stability: 0.5,
            gaze_level: 0.5,
        }
    }
}

// ── MovementProfile ─────────────────────────────────────────────

/// 移动参数配置——TOML 数据驱动，每种生物类型一个 profile。
///
/// 所有字段为 f32 数值。TOML 反序列化通过 serde feature gate 启用。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MovementProfile {
    // 地面速度
    pub walk_speed: f32,
    pub run_speed: f32,
    pub sprint_speed: f32,

    // 地面加速度/摩擦
    pub ground_accel: f32,
    pub sprint_accel: f32,
    pub ground_friction: f32,
    pub sprint_friction: f32,

    // 转身速率 (°/s)
    pub default_turn_rate: f32,
    pub sprint_turn_rate: f32,
    pub air_turn_rate: f32,
    pub knocked_turn_rate: f32,

    // 体力
    pub sprint_stamina_rate: f32,
    pub sprint_min_stamina_to_start: f32,

    // 攀爬
    pub climb_speed: f32,
    pub climb_accel: f32,
    pub climb_friction: f32,
    pub climb_stamina_rate: f32,

    // 游泳
    pub swim_slow_speed: f32,
    pub swim_fast_speed: f32,
    pub dive_speed: f32,
    pub swim_accel_slow: f32,
    pub swim_accel_fast: f32,
    pub swim_friction: f32,
    pub swim_fast_stamina_rate: f32,
    pub swim_slow_stamina_rate: f32,
    pub treading_stamina_rate: f32,

    // 滑翔
    pub glide_horizontal_speed: f32,
    pub glide_vertical_speed: f32,
    pub glide_accel: f32,
    pub glide_stamina_rate: f32,

    // 跳跃
    pub jump_horizontal_speed: f32,
    /// 重力加速度 (m/s²)——腾空态垂直积分。Provisional（游戏化，非真实 9.8）。
    pub gravity: f32,
    /// 跳跃起跳垂直速度 (m/s)。Provisional。跳高 = jump_speed²/(2·gravity)。
    pub jump_speed: f32,

    // 挂载
    pub mounted_speed: f32,
    pub mounted_accel: f32,
    pub mounted_friction: f32,

    // 其他
    pub knockback_recover_secs: f32,
}

impl Default for MovementProfile {
    fn default() -> Self {
        Self {
            walk_speed: 1.4,
            run_speed: 3.5,
            sprint_speed: 5.5,
            ground_accel: 10.0,
            sprint_accel: 14.0,
            ground_friction: 12.0,
            sprint_friction: 8.0,
            default_turn_rate: 720.0,
            sprint_turn_rate: 360.0,
            air_turn_rate: 180.0,
            knocked_turn_rate: 30.0,
            sprint_stamina_rate: 8.0,
            sprint_min_stamina_to_start: 8.0,
            climb_speed: 0.6,
            climb_accel: 3.0,
            climb_friction: 8.0,
            climb_stamina_rate: 6.0,
            swim_slow_speed: 1.0,
            swim_fast_speed: 2.5,
            dive_speed: 1.5,
            swim_accel_slow: 3.0,
            swim_accel_fast: 5.0,
            swim_friction: 6.0,
            swim_fast_stamina_rate: 10.0,
            swim_slow_stamina_rate: -2.0,
            treading_stamina_rate: 2.0,
            glide_horizontal_speed: 12.0,
            glide_vertical_speed: -1.5,
            glide_accel: 4.0,
            glide_stamina_rate: 3.0,
            jump_horizontal_speed: 3.0,
            gravity: 20.0,
            jump_speed: 7.0,
            mounted_speed: 7.0,
            mounted_accel: 5.0,
            mounted_friction: 4.0,
            knockback_recover_secs: 0.4,
        }
    }
}

// ── 组合合法性 ──────────────────────────────────────────────────

/// Pace × Stance 组合是否合法。
pub fn is_valid_stance_pace(stance: Stance, pace: Pace) -> bool {
    match (stance, pace) {
        (_, Pace::Still) => true,
        (_, Pace::Walking) => true,
        (Stance::Standing, Pace::Running) => true,
        (Stance::Crouching, Pace::Running) => true,
        (Stance::Prone, Pace::Running) => false,
        (Stance::Standing, Pace::Sprinting) => true,
        (Stance::Crouching, Pace::Sprinting) => false,
        (Stance::Prone, Pace::Sprinting) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Stance / Pace ──

    #[test]
    fn test_stance_default_is_standing() {
        assert_eq!(Stance::default(), Stance::Standing);
    }

    #[test]
    fn test_pace_default_is_still() {
        assert_eq!(Pace::default(), Pace::Still);
    }

    // ── 组合合法性矩阵 ──

    #[test]
    fn test_standing_sprinting_valid() {
        assert!(is_valid_stance_pace(Stance::Standing, Pace::Sprinting));
    }

    #[test]
    fn test_crouching_sprinting_invalid() {
        assert!(!is_valid_stance_pace(Stance::Crouching, Pace::Sprinting));
    }

    #[test]
    fn test_prone_sprinting_invalid() {
        assert!(!is_valid_stance_pace(Stance::Prone, Pace::Sprinting));
    }

    #[test]
    fn test_prone_running_invalid() {
        assert!(!is_valid_stance_pace(Stance::Prone, Pace::Running));
    }

    #[test]
    fn test_all_stances_valid_for_still() {
        assert!(is_valid_stance_pace(Stance::Standing, Pace::Still));
        assert!(is_valid_stance_pace(Stance::Crouching, Pace::Still));
        assert!(is_valid_stance_pace(Stance::Prone, Pace::Still));
    }

    #[test]
    fn test_all_stances_valid_for_walking() {
        assert!(is_valid_stance_pace(Stance::Standing, Pace::Walking));
        assert!(is_valid_stance_pace(Stance::Crouching, Pace::Walking));
        assert!(is_valid_stance_pace(Stance::Prone, Pace::Walking));
    }

    // ── MovementState ──

    #[test]
    fn test_movement_state_default() {
        let ms = MovementState::default();
        assert_eq!(ms.stance, Stance::Standing);
        assert_eq!(ms.pace, Pace::Still);
        assert!(ms.special.is_none());
        assert_eq!(ms.exhaustion_cooldown, 0.0);
    }

    #[test]
    fn test_max_speed_walking() {
        let ms = MovementState {
            stance: Stance::Standing,
            pace: Pace::Walking,
            ..Default::default()
        };
        let profile = MovementProfile::default();
        let speed = ms.max_speed(&profile);
        assert!((speed - 1.4).abs() < 0.01);
    }

    #[test]
    fn test_max_speed_sprinting() {
        let ms = MovementState {
            stance: Stance::Standing,
            pace: Pace::Sprinting,
            ..Default::default()
        };
        let profile = MovementProfile::default();
        let speed = ms.max_speed(&profile);
        assert!((speed - 5.5).abs() < 0.01);
    }

    #[test]
    fn test_prone_walking_speed_reduced() {
        let ms = MovementState {
            stance: Stance::Prone,
            pace: Pace::Walking,
            ..Default::default()
        };
        let profile = MovementProfile::default();
        let speed = ms.max_speed(&profile);
        assert!(speed < 1.0); // 20% of walk_speed (1.4 * 0.2 = 0.28)
    }

    #[test]
    fn test_exhaustion_cooldown_blocks_sprint_speed() {
        let ms = MovementState {
            stance: Stance::Standing,
            pace: Pace::Sprinting,
            exhaustion_cooldown: 0.5,
            ..Default::default()
        };
        let profile = MovementProfile::default();
        let speed = ms.max_speed(&profile);
        assert!((speed - profile.walk_speed).abs() < 0.01);
    }

    #[test]
    fn test_is_voluntary_ground_no_special() {
        let ms = MovementState::default();
        assert!(ms.is_voluntary_ground());
    }

    #[test]
    fn test_is_voluntary_ground_with_special() {
        let ms = MovementState {
            special: Some(SpecialMode::Swimming(SwimPace::Slow)),
            ..Default::default()
        };
        assert!(!ms.is_voluntary_ground());
    }

    #[test]
    fn test_acceleration_sprinting() {
        let ms = MovementState {
            stance: Stance::Standing,
            pace: Pace::Sprinting,
            ..Default::default()
        };
        let profile = MovementProfile::default();
        let a = ms.acceleration(&profile);
        assert!((a - 14.0).abs() < 0.01);
    }

    #[test]
    fn test_stamina_rate_sprinting() {
        let ms = MovementState {
            stance: Stance::Standing,
            pace: Pace::Sprinting,
            ..Default::default()
        };
        let profile = MovementProfile::default();
        let rate = ms.stamina_rate(&profile);
        assert!(rate > 0.0); // 消耗体力
    }

    #[test]
    fn test_stamina_rate_walking_zero() {
        let ms = MovementState {
            stance: Stance::Standing,
            pace: Pace::Walking,
            ..Default::default()
        };
        let profile = MovementProfile::default();
        let rate = ms.stamina_rate(&profile);
        assert!((rate).abs() < 0.01); // 不耗体力
    }

    #[test]
    fn test_gait_params_default_all_half() {
        let ms = MovementState::default();
        let gp = ms.gait_params();
        assert!((gp.hip_sway - 0.5).abs() < 0.01);
        assert!((gp.stride_length - 0.5).abs() < 0.01);
    }

    // ── MovementRecoveryStack ──

    #[test]
    fn test_recovery_stack_push_pop() {
        let mut stack = MovementRecoveryStack::default();
        assert!(stack.is_empty());

        let s1 = MovementState {
            stance: Stance::Standing,
            pace: Pace::Walking,
            ..Default::default()
        };
        stack.push_if_voluntary(s1);
        assert_eq!(stack.len(), 1);

        let recovered = stack.pop_compatible(LocomotionMode::Grounded, Medium::Air);
        assert_eq!(recovered.stance, Stance::Standing);
        assert_eq!(recovered.pace, Pace::Walking);
        assert!(stack.is_empty());
    }

    #[test]
    fn test_recovery_stack_push_non_voluntary_ignored() {
        let mut stack = MovementRecoveryStack::default();
        let s = MovementState {
            special: Some(SpecialMode::Swimming(SwimPace::Slow)),
            ..Default::default()
        };
        stack.push_if_voluntary(s);
        assert_eq!(stack.len(), 0);
    }

    #[test]
    fn test_recovery_stack_clear() {
        let mut stack = MovementRecoveryStack::default();
        stack.push_if_voluntary(MovementState::default());
        stack.push_if_voluntary(MovementState::default());
        assert_eq!(stack.len(), 2);
        stack.clear();
        assert!(stack.is_empty());
    }

    #[test]
    fn test_recovery_stack_over_capacity_shifts() {
        let mut stack = MovementRecoveryStack::default();
        let s1 = MovementState {
            pace: Pace::Walking,
            ..Default::default()
        };
        let s2 = MovementState {
            pace: Pace::Running,
            ..Default::default()
        };
        let s3 = MovementState {
            pace: Pace::Sprinting,
            ..Default::default()
        };
        let s4 = MovementState {
            pace: Pace::Still,
            ..Default::default()
        };
        stack.push_if_voluntary(s1);
        stack.push_if_voluntary(s2);
        stack.push_if_voluntary(s3);
        stack.push_if_voluntary(s4); // 超容量——s1 被挤出
        assert_eq!(stack.len(), 3);

        // 最后入栈的 s4 在栈顶，最先弹出
        let r = stack.pop_compatible(LocomotionMode::Grounded, Medium::Air);
        assert_eq!(r.pace, Pace::Still);
    }

    #[test]
    fn test_recovery_stack_pop_empty_returns_default() {
        let mut stack = MovementRecoveryStack::default();
        let r = stack.pop_compatible(LocomotionMode::Grounded, Medium::Air);
        assert_eq!(r.stance, Stance::Standing);
        assert_eq!(r.pace, Pace::Still);
    }

    // ── StanceTransition ──

    #[test]
    fn test_stance_transition_stand_to_crouch() {
        let d = StanceTransition::duration(Stance::Standing, Stance::Crouching);
        assert_eq!(d, StanceTransition::STAND_CROUCH_MS);
    }

    #[test]
    fn test_stance_transition_same_zero() {
        let d = StanceTransition::duration(Stance::Standing, Stance::Standing);
        assert_eq!(d, 0);
    }

    // ── MovementProfile ──

    #[test]
    fn test_movement_profile_default_values() {
        let p = MovementProfile::default();
        assert!(p.walk_speed > 0.0);
        assert!(p.run_speed > p.walk_speed);
        assert!(p.sprint_speed > p.run_speed);
    }

    #[test]
    fn test_movement_profile_swim_slow_stamina_negative() {
        let p = MovementProfile::default();
        // 慢泳回复体力——值为负
        assert!(p.swim_slow_stamina_rate < 0.0);
    }

    // ── GaitParams ──

    #[test]
    fn test_gait_params_default_range() {
        let gp = GaitParams::default();
        let vals = [
            gp.hip_sway,
            gp.stride_length,
            gp.arm_swing,
            gp.bounce,
            gp.forward_lean,
            gp.rhythm_regularity,
            gp.foot_drag,
            gp.shoulder_stability,
            gp.gaze_level,
        ];
        for v in vals {
            assert!((0.0..=1.0).contains(&v));
        }
    }
}
