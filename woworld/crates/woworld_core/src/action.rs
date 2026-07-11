//! 动作系统类型 — ActionId/ActionParams/ActionPhase/Commitment/仲裁/事件
//!
//! 依赖: `woworld_core::kinematics` (PhysicsRequirement, MovementLock, RotationLock)
//!        `woworld_core::types` (EntityId)
//!
//! 参见: `WoWorld-Design/.../角色控制器/003-ActionController与离散动作.md`
//!       `WoWorld-Design/.../角色控制器/005-ActionOutcome与动作结果事件.md`
//!       `WoWorld-Design/.../角色控制器/006-持续动作与充能动作.md`

use glam::Vec3;

use crate::kinematics::PhysicsRequirement;
use crate::types::EntityId;

// ── ActionId ────────────────────────────────────────────────────

/// 动作标识符——TOML `action_registry.toml` 中 `[action.NAME]` 的散列或索引。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ActionId(pub u32);

impl ActionId {
    /// 哨兵值——无效/未设置的动作。
    pub const NONE: Self = Self(u32::MAX);

    /// 从 TOML key（如 `"aimed_shot"`）通过 FNV-1a 32-bit hash 生成 ActionId。
    ///
    /// ★ 单一 hash 源——`ActionRegistry` 的 TOML key 解析与 `ChargeStage`/`Trigger`
    ///   子动作引用必须走同一算法，否则 `action_id = "aimed_shot"` 解析出的 id
    ///   与 `[action.aimed_shot]` 的 map-key id 不一致，充能接续会找不到子动作。
    pub const fn from_key(key: &str) -> Self {
        let bytes = key.as_bytes();
        let mut hash: u32 = 0x811c_9dc5;
        let mut i = 0;
        while i < bytes.len() {
            hash ^= bytes[i] as u32;
            hash = hash.wrapping_mul(0x0100_0193);
            i += 1;
        }
        Self(hash)
    }
}

/// 自定义反序列化——同时接受 TOML **字符串键**（FNV hash）与**整数 id**。
///
/// 006 的 `release_behavior`/`stages` 中 `action_id = "quick_shot"` 是字符串键；
/// 手写整数 id 也保留支持（visit_i64/u64）。
#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for ActionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ActionIdVisitor;
        impl serde::de::Visitor<'_> for ActionIdVisitor {
            type Value = ActionId;
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                f.write_str("动作 key 字符串或 u32 id")
            }
            fn visit_str<E: serde::de::Error>(self, v: &str) -> Result<ActionId, E> {
                Ok(ActionId::from_key(v))
            }
            fn visit_u64<E: serde::de::Error>(self, v: u64) -> Result<ActionId, E> {
                Ok(ActionId(v as u32))
            }
            fn visit_i64<E: serde::de::Error>(self, v: i64) -> Result<ActionId, E> {
                Ok(ActionId(v as u32))
            }
        }
        deserializer.deserialize_any(ActionIdVisitor)
    }
}

// ── ActionInstanceId ────────────────────────────────────────────

/// 动作实例标识符——每次动作执行唯一，单调递增。
///
/// GOAP/Memory/Animation 通过此 ID 追溯完整因果链。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ActionInstanceId(pub u64);

// ── ActionParams ────────────────────────────────────────────────

/// 动作的紧凑参数——~36-40 bytes（Option<EntityId> + Option<Vec3> + u32），栈上分配。
///
/// `data` 由 `action_id` 解释：item_slot/spell_id/recipe_id/charge_power...
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ActionParams {
    /// 目标实体（如攻击对象、对话 NPC）
    pub target: Option<EntityId>,
    /// 目标位置（如 AoE 中心、移动目的地）
    pub position: Option<Vec3>,
    /// 由 action_id 解释的紧凑数据
    pub data: u32,
}

// ── ActionPhase ─────────────────────────────────────────────────

/// 动作的三阶段时间线。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionPhase {
    /// 前摇——已提交，碰撞体未激活，可被极高优先级中断
    Windup,
    /// 生效窗口——碰撞体存活，伤害/效果可命中，仅系统中断可通过
    Active,
    /// 后摇——碰撞体消失，取消窗口内可取消到指定动作
    Recovery,
}

// ── CommitmentLevel ─────────────────────────────────────────────

/// 四级承诺——决定动作被打断的难易程度。
///
/// `None` 是 Rust 合法标识符（非关键字），serde 反序列化时匹配 TOML 字符串 "None"。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub enum CommitmentLevel {
    /// 空闲——任意动作可立即开始
    None = 0,
    /// 软承诺——前摇或后摇中，可被闪避/跳跃/招架取消
    Soft = 1,
    /// 硬承诺——生效窗口中，只能被物理命中/死亡/系统中断打断
    Hard = 2,
    /// 锁定——仪式/读条中，物理命中也不能打断（极少用）
    Locked = 3,
}

// ── ActionPriority ──────────────────────────────────────────────

/// 动作优先级常量——数值越大优先级越高。
pub mod action_priority {
    pub const RELEASE: u8 = 0;
    pub const CHARGE_TRIGGER: u8 = 5;
    pub const INTERACT: u8 = 10;
    pub const HOTBAR: u8 = 12;
    pub const ATTACK: u8 = 15;
    pub const SPECIAL_SKILL: u8 = 18;
    pub const JUMP: u8 = 20;
    pub const BLOCK: u8 = 20;
    pub const CLIMB_ENTRY: u8 = 25;
    pub const CLIMB_TRANSITION: u8 = 28;
    pub const DODGE: u8 = 25;
    pub const PARRY: u8 = 30;
    /// 硬直打断阈值——≥此值的 System 来源请求始终通过
    pub const STAGGER_THRESHOLD: u8 = 35;
    pub const INSTINCT: u8 = 80;
    pub const EMERGENCY: u8 = 100;
}

// ── ActionSource ────────────────────────────────────────────────

/// 动作请求的来源——决定仲裁优先级。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionSource {
    /// 玩家按键
    Player,
    /// NPC GOAP 决策
    GOAP,
    /// 本能层——不受 ControlMode 影响
    Instinct,
    /// 系统中断——Stagger/Knockback/Death
    System,
    /// 充能动作触发的子动作
    ChargedAction,
}

// ── ActionKind ─────────────────────────────────────────────────

/// 动作类型——离散/持续/充能。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub enum ActionKind {
    /// 离散动作——按键瞬间触发，计时器到期结束（light_attack, dodge, jump）
    Discrete,
    /// 持续动作——按住持续，松键结束（block, hold_breath, meditate）
    Continuous,
    /// 充能动作——按住充能，松键触发子动作或到顶自动释放（aim_bow, charge_heavy）
    Charge,
}

// ── ResourceType ────────────────────────────────────────────────

/// 实体可消耗的资源类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub enum ResourceType {
    /// 体力——冲刺/攀爬/防御消耗
    Stamina,
    /// 生命——濒死/死亡门控
    Health,
    /// 魔力——法术充能消耗
    Mana,
}

// ── SustainPhase / SustainDrain ─────────────────────────────────

/// 持续/充能动作的体力消耗阶段。
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub enum SustainPhase {
    /// 正常——资源按基础速率消耗
    Normal,
    /// 过久——消耗加速
    Overextended {
        /// 额外消耗倍率（1.0 = 正常，2.0 = 双倍）
        penalty: f32,
    },
    /// 即将强制结束
    Critical {
        /// 距离强制结束剩余时间 (s)
        forced_release_in: f32,
    },
}

/// 持续动作的资源消耗配置。
///
/// 参见: `006-持续动作与充能动作.md` §二
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct SustainDrain {
    /// 消耗的资源类型
    pub resource: ResourceType,
    /// 基础消耗速率（单位/秒）
    pub rate_per_sec: f32,
    /// Overextended 时的消耗乘数
    pub overextend_multiplier: f32,
}

// ── ReleaseBehavior / ChargeStage / OverchargeBehavior ──────────

/// 持续/充能动作的释放行为。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub enum ReleaseBehavior {
    /// 纯结束——释放即完成
    Complete,
    /// 释放时触发固定子动作
    Trigger {
        /// 触发的动作 ID
        action_id: ActionId,
    },
    /// 根据充能时长选择子动作
    Charged {
        /// 充能阶梯
        stages: Vec<ChargeStage>,
        /// 充能到顶后的行为
        on_overcharge: OverchargeBehavior,
    },
}

/// 充能阶梯——充能达到阈值时触发的子动作。
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct ChargeStage {
    /// 充能阈值 (ms)
    pub threshold_ms: u32,
    /// 触发的子动作 ID
    pub action_id: ActionId,
    /// 伤害/效果乘数
    pub power_multiplier: f32,
}

/// 充能到顶后的行为。
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub enum OverchargeBehavior {
    /// 自动释放
    AutoRelease,
    /// 允许继续充能但精度惩罚
    Penalize {
        /// 每秒精度损失
        accuracy_loss_per_sec: f32,
    },
    /// 超时动作失败
    ForceCancel,
}

// ── ActiveAction ────────────────────────────────────────────────

/// 运行时动作——ActionController 的状态存于 CActiveAction Component 中。
#[derive(Debug, Clone, PartialEq)]
pub struct ActiveAction {
    /// 唯一追溯 ID
    pub instance: ActionInstanceId,
    /// 动作定义 ID
    pub action_id: ActionId,
    /// 当前时间线阶段
    pub phase: ActionPhase,
    /// 承诺级别
    pub commitment: CommitmentLevel,
    /// 动作已执行的总时长 (s)
    pub elapsed: f32,
    /// 当前是否在可取消的时间窗口内
    pub cancel_window_open: bool,
    /// 当前资源消耗速率（持续动作）
    pub resource_drain_rate: f32,
    /// 持续动作的维持阶段
    pub sustain_phase: SustainPhase,
}

// ── ActionRequest ───────────────────────────────────────────────

/// 动作请求——多来源写入 CActionRequestBuf，ActionController 统一消费。
#[derive(Debug, Clone, PartialEq)]
pub struct ActionRequest {
    /// 请求的动作 ID
    pub action_id: ActionId,
    /// 优先级（越高越优先）
    pub priority: u8,
    /// 请求来源
    pub source: ActionSource,
    /// 动作参数（目标/位置/data）
    pub params: ActionParams,
}

// ── ActionDef ───────────────────────────────────────────────────

/// 动作定义——TOML 数据驱动，`action_registry.toml` 中一行。
///
/// Sprint 1: 仅处理 Discrete 动作。Continuous/Charge 字段为 Option，后续 sprint 激活。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct ActionDef {
    /// 动作名（显示用）
    pub name: String,
    /// 动作分类（Combat/Interaction/Movement）
    pub category: String,
    /// 动作类型（Discrete/Continuous/Charge）
    pub kind: ActionKind,
    /// 优先级
    pub priority: u8,
    /// 承诺级别
    pub commitment: CommitmentLevel,
    /// 前摇时长 (ms)
    pub windup_ms: u32,
    /// 生效窗口时长 (ms)，0 = 持续动作
    pub active_ms: u32,
    /// 后摇时长 (ms)
    pub recovery_ms: u32,
    /// 可被哪些动作取消（ActionId 名称列表）
    pub cancel_set: Vec<String>,
    /// 取消窗口时长 (ms)
    pub cancel_window_ms: u32,
    /// 是否可缓冲
    pub bufferable: bool,
    /// 缓冲窗口时长 (ms)
    #[cfg_attr(feature = "serde", serde(default))]
    pub buffer_window_ms: u32,
    /// 物理约束
    pub physics_req: PhysicsRequirement,
    /// 移动锁类型
    #[cfg_attr(feature = "serde", serde(default))]
    pub movement_lock: MovementLockDef,
    /// 朝向锁类型
    #[cfg_attr(feature = "serde", serde(default))]
    pub rotation_lock: RotationLockDef,
    /// 移动输入是否打断此动作
    #[cfg_attr(feature = "serde", serde(default))]
    pub interrupt_on_move: bool,
    /// 持续动作的资源消耗（Optional——仅 Continuous/Charge 动作使用）
    #[cfg_attr(feature = "serde", serde(default))]
    pub sustain_drain: Option<SustainDrain>,
    /// 释放行为（Optional——仅 Continuous/Charge 动作使用）
    #[cfg_attr(feature = "serde", serde(default))]
    pub release_behavior: Option<ReleaseBehavior>,
    /// 过久阈值 (s)——仅 Continuous/Charge
    #[cfg_attr(feature = "serde", serde(default))]
    pub overextend_threshold_secs: Option<f32>,
    /// 强制结束阈值 (s)——仅 Continuous/Charge
    #[cfg_attr(feature = "serde", serde(default))]
    pub critical_threshold_secs: Option<f32>,
}

// ── TOML 辅助枚举 ──────────────────────────────────────────────

/// MovementLock 的 TOML 字符串表示。
///
/// MovementLock::Partial{speed_cap} 暂时不支持 TOML 反序列化（Sprint 1 不需要）。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub enum MovementLockDef {
    #[default]
    Free,
    /// 减速上限——防御/瞄准中可慢走。speed_cap 暂不可从 TOML 配置（Sprint 1 限制）
    Partial,
    Full,
    /// 动作接管——闪避/跳跃的强制位移。displacement 暂不可从 TOML 配置（Sprint 1 限制）
    Override,
}

/// RotationLock 的 TOML 字符串表示。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub enum RotationLockDef {
    #[default]
    Free,
    InputDirection,
    CameraForward,
    TargetDirection,
    Locked,
}

// ── ActionLifecycleEvent ────────────────────────────────────────

/// 动作生命周期事件——双层事件体系的 Layer A。
///
/// 所有动作统一发出，GOAP/Memory/Animation/UI 通过 ActionInstanceId 追溯因果链。
/// 参见: `005-ActionOutcome与动作结果事件.md` §二
#[derive(Debug, Clone, PartialEq)]
pub enum ActionLifecycleEvent {
    /// 动作开始
    Started {
        instance: ActionInstanceId,
        entity: EntityId,
        action_id: ActionId,
        params: ActionParams,
        /// 开始时间（游戏秒）
        started_at: f32,
    },
    /// 阶段切换
    PhaseChanged {
        instance: ActionInstanceId,
        entity: EntityId,
        from: ActionPhase,
        to: ActionPhase,
    },
    /// 动作正常完成
    Completed {
        instance: ActionInstanceId,
        entity: EntityId,
        action_id: ActionId,
        /// 总耗时 (ms)
        total_duration_ms: u32,
    },
    /// 动作被打断
    Interrupted {
        instance: ActionInstanceId,
        entity: EntityId,
        action_id: ActionId,
        /// 打断来源
        by: InterruptSource,
        /// 完成进度 (0.0-1.0)
        progress: f32,
    },
    /// 动作失败（不满足条件）
    Failed {
        instance: ActionInstanceId,
        entity: EntityId,
        action_id: ActionId,
        /// 失败原因
        reason: ActionFailureReason,
    },
    /// 充能触发——充能达到阶梯，触发子动作
    ChargeTrigger {
        instance: ActionInstanceId,
        entity: EntityId,
        /// 充能时长 (ms)
        charge_ms: u32,
        /// 触发的阶梯
        stage: ChargeStage,
    },
}

// ── InterruptSource ─────────────────────────────────────────────

/// 动作被打断的来源。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptSource {
    /// 被更高优先级的动作打断
    HigherPriorityAction(ActionId),
    /// ImpulseQueue 切 PhysicsBody
    PhysicsKnockback,
    /// 被轻击硬直
    Staggered,
    /// 致命伤害
    Death,
    /// 濒死——比 Death 先到
    Dying,
    /// 玩家夺舍
    PossessionTakeover,
    /// interrupt_on_move 触发
    MoveInput,
    /// 持续动作松键（非正常）
    InputReleased,
    /// 资源耗尽
    VitalDepleted(ResourceType),
    /// 闪避取消
    DodgeCancel,
    /// 招架取消
    ParryCancel,
}

// ── ActionFailureReason ─────────────────────────────────────────

/// 动作失败的原因。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionFailureReason {
    /// 目标超出范围
    TargetOutOfRange,
    /// 目标已死亡
    TargetDied,
    /// 资源不足
    InsufficientResource(ResourceType),
    /// 上下文失效（门被他人打开 / 矿被采完）
    ContextInvalidated,
    /// 环境打断（被水冲走 / 被风吹落）
    EnvironmentalInterrupt,
    /// 自身状态不满足（受伤 / 中毒中断读条）
    SelfStateInvalidated,
    /// 物理状态不兼容（PhysicsRequirement 检查失败）
    PhysicsIncompatible,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── ActionId ──

    #[test]
    fn test_action_id_none_sentinel() {
        assert_eq!(ActionId::NONE.0, u32::MAX);
    }

    #[test]
    fn test_action_id_eq() {
        assert_eq!(ActionId(1), ActionId(1));
        assert_ne!(ActionId(1), ActionId(2));
    }

    #[test]
    fn test_action_id_from_key_deterministic() {
        // ★ Sprint-065: from_key 确定性——同键同 id，异键异 id（FNV-1a）
        assert_eq!(
            ActionId::from_key("aimed_shot"),
            ActionId::from_key("aimed_shot")
        );
        assert_ne!(
            ActionId::from_key("aimed_shot"),
            ActionId::from_key("quick_shot")
        );
        // 空键 = FNV-1a offset basis（字符串反序列化路径由 woworld_ecs registry 测试覆盖）
        assert_eq!(ActionId::from_key("").0, 0x811c_9dc5);
    }

    // ── ActionInstanceId ──

    #[test]
    fn test_action_instance_id_ordering() {
        let a = ActionInstanceId(1);
        let b = ActionInstanceId(2);
        assert!(a < b);
    }

    // ── ActionParams ──

    #[test]
    fn test_action_params_size() {
        // Option<EntityId> = 16 bytes (u64 + discriminant)
        // Option<Vec3> = 16 bytes (3×f32 + discriminant)
        // u32 = 4 bytes
        // Total with alignment: ~36-40 bytes. 核心要求: < 64 bytes (cache line friendly)
        let size = std::mem::size_of::<ActionParams>();
        assert!(size <= 64, "ActionParams should be ≤64 bytes, got {size}");
    }

    #[test]
    fn test_action_params_default() {
        let p = ActionParams::default();
        assert!(p.target.is_none());
        assert!(p.position.is_none());
        assert_eq!(p.data, 0);
    }

    // ── CommitmentLevel ──

    #[test]
    fn test_commitment_level_ordering() {
        assert!(CommitmentLevel::None < CommitmentLevel::Soft);
        assert!(CommitmentLevel::Soft < CommitmentLevel::Hard);
        assert!(CommitmentLevel::Hard < CommitmentLevel::Locked);
    }

    #[test]
    fn test_commitment_level_discriminant() {
        assert_eq!(CommitmentLevel::None as u8, 0);
        assert_eq!(CommitmentLevel::Locked as u8, 3);
    }

    // ── ActionPriority ──

    #[test]
    fn test_action_priority_ordering() {
        use action_priority::*;
        assert!(RELEASE < INTERACT);
        assert!(INTERACT < ATTACK);
        assert!(ATTACK < JUMP);
        assert!(JUMP < DODGE);
        assert!(DODGE < STAGGER_THRESHOLD);
        assert!(STAGGER_THRESHOLD < INSTINCT);
        assert!(INSTINCT < EMERGENCY);
    }

    #[test]
    fn test_stagger_threshold_above_most_actions() {
        use action_priority::*;
        assert!(STAGGER_THRESHOLD > ATTACK);
        assert!(STAGGER_THRESHOLD > DODGE);
        assert!(STAGGER_THRESHOLD > PARRY);
    }

    // ── ActionSource ──

    #[test]
    fn test_action_source_variants_distinct() {
        use ActionSource::*;
        let variants = [Player, GOAP, Instinct, System, ChargedAction];
        for i in 0..variants.len() {
            for j in (i + 1)..variants.len() {
                assert_ne!(variants[i], variants[j]);
            }
        }
    }

    // ── ResourceType ──

    #[test]
    fn test_resource_type_variants_distinct() {
        use ResourceType::*;
        assert_ne!(Stamina, Health);
        assert_ne!(Health, Mana);
        assert_ne!(Stamina, Mana);
    }

    // ── ActionLifecycleEvent ──

    #[test]
    fn test_lifecycle_event_started_fields() {
        let event = ActionLifecycleEvent::Started {
            instance: ActionInstanceId(1),
            entity: EntityId(100),
            action_id: ActionId(5),
            params: ActionParams::default(),
            started_at: 0.0,
        };
        match event {
            ActionLifecycleEvent::Started {
                instance,
                entity,
                action_id,
                ..
            } => {
                assert_eq!(instance, ActionInstanceId(1));
                assert_eq!(entity, EntityId(100));
                assert_eq!(action_id, ActionId(5));
            }
            _ => panic!("expected Started"),
        }
    }

    #[test]
    fn test_lifecycle_event_interrupted_carries_progress() {
        let event = ActionLifecycleEvent::Interrupted {
            instance: ActionInstanceId(2),
            entity: EntityId(200),
            action_id: ActionId(3),
            by: InterruptSource::Staggered,
            progress: 0.45,
        };
        match event {
            ActionLifecycleEvent::Interrupted { progress, .. } => {
                assert!((progress - 0.45).abs() < 0.01);
            }
            _ => panic!("expected Interrupted"),
        }
    }

    // ── InterruptSource ──

    #[test]
    fn test_interrupt_source_higher_priority_carries_action_id() {
        let src = InterruptSource::HigherPriorityAction(ActionId(10));
        assert_eq!(src, InterruptSource::HigherPriorityAction(ActionId(10)));
    }

    #[test]
    fn test_interrupt_source_vital_depleted_carries_resource() {
        let src = InterruptSource::VitalDepleted(ResourceType::Stamina);
        assert_eq!(src, InterruptSource::VitalDepleted(ResourceType::Stamina));
    }

    // ── ActionFailureReason ──

    #[test]
    fn test_failure_reason_insufficient_resource() {
        let reason = ActionFailureReason::InsufficientResource(ResourceType::Mana);
        assert_eq!(
            reason,
            ActionFailureReason::InsufficientResource(ResourceType::Mana)
        );
    }

    // ── ActionRequest ──

    #[test]
    fn test_action_request_creation() {
        let req = ActionRequest {
            action_id: ActionId(1),
            priority: action_priority::JUMP,
            source: ActionSource::Player,
            params: ActionParams::default(),
        };
        assert_eq!(req.action_id, ActionId(1));
        assert_eq!(req.priority, action_priority::JUMP);
        assert_eq!(req.source, ActionSource::Player);
    }

    // ── ActionDef ──

    #[test]
    fn test_action_def_default_fields() {
        let def = ActionDef {
            name: "test".into(),
            category: "Combat".into(),
            kind: ActionKind::Discrete,
            priority: 15,
            commitment: CommitmentLevel::Hard,
            windup_ms: 120,
            active_ms: 100,
            recovery_ms: 250,
            cancel_set: vec![],
            cancel_window_ms: 120,
            bufferable: false,
            buffer_window_ms: 0,
            physics_req: PhysicsRequirement::Grounded,
            movement_lock: MovementLockDef::Full,
            rotation_lock: RotationLockDef::TargetDirection,
            interrupt_on_move: false,
            sustain_drain: None,
            release_behavior: None,
            overextend_threshold_secs: None,
            critical_threshold_secs: None,
        };
        assert_eq!(def.name, "test");
        assert_eq!(def.kind, ActionKind::Discrete);
        assert!(def.sustain_drain.is_none());
    }

    // ── SustainDrain ──

    #[test]
    fn test_sustain_drain_fields() {
        let sd = SustainDrain {
            resource: ResourceType::Stamina,
            rate_per_sec: 3.0,
            overextend_multiplier: 2.0,
        };
        assert_eq!(sd.resource, ResourceType::Stamina);
        assert!((sd.rate_per_sec - 3.0).abs() < 0.01);
        assert!((sd.overextend_multiplier - 2.0).abs() < 0.01);
    }
}
