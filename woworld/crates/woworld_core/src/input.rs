//! 输入缓冲类型 — BufferPriority + InputFeelConfig + BufferedInput
//!
//! 仅玩家实体使用。NPC 的 GOAP 产出 ActionRequest 时已考虑时机。
//!
//! 参见: `WoWorld-Design/.../角色控制器/008-手感系统.md`

use crate::action::{ActionId, ActionRequest};
use crate::player::ActionDomain;
use glam::{Mat4, Vec2};

// ── BufferPriority ──────────────────────────────────────────────

/// 输入缓冲优先级——满容量时低优先级条目被淘汰。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BufferPriority {
    /// 交互（最低优先级——可被任何条目淘汰）
    Interaction = 10,
    /// 战斗动作
    Combat = 30,
    /// 移动动作（跳跃）
    Movement = 40,
    /// 防御性动作（闪避/招架）
    Defensive = 60,
    /// 本能反应
    Instinct = 80,
    /// 系统紧急（最高优先级）
    Emergency = 100,
}

// ── InputFeelConfig ─────────────────────────────────────────────

/// 手感参数——TOML 数据驱动 `input_feel.toml`。
///
/// 所有参数值 Provisional——待实机测试调整。机制结构是最终设计。
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize))]
pub struct InputFeelConfig {
    /// 土狼时间窗口 (ms)
    pub coyote_time_ms: f32,
    /// 跳跃缓冲窗口 (ms)
    pub jump_buffer_ms: f32,
    /// 闪避缓冲窗口 (ms)
    pub dodge_buffer_ms: f32,
    /// 连招缓冲窗口 (ms)
    pub combo_buffer_ms: f32,
    /// 边缘吸附最大距离 (m)
    pub ledge_snap_distance: f32,
    /// 边缘吸附最大角度 (度)
    pub ledge_snap_angle: f32,
    /// 转身平滑时间 (s)
    pub turn_smooth_time: f32,
    /// 默认加速度 (m/s²)
    pub default_accel: f32,
    /// 默认减速度 (m/s²)
    pub default_decel: f32,
    /// 空中加速度乘数
    pub air_accel_multiplier: f32,
}

impl Default for InputFeelConfig {
    fn default() -> Self {
        Self {
            coyote_time_ms: 150.0,
            jump_buffer_ms: 200.0,
            dodge_buffer_ms: 200.0,
            combo_buffer_ms: 150.0,
            ledge_snap_distance: 0.3,
            ledge_snap_angle: 45.0,
            turn_smooth_time: 0.1,
            default_accel: 10.0,
            default_decel: 12.0,
            air_accel_multiplier: 0.3,
        }
    }
}

// ── BufferedInput ───────────────────────────────────────────────

/// 缓冲输入条目——存入 CInputBuffer 环形缓冲区。
#[derive(Debug, Clone, PartialEq)]
pub struct BufferedInput {
    /// 缓冲的动作请求
    pub action_request: ActionRequest,
    /// 按下时刻（游戏秒）
    pub pressed_at: f32,
    /// 过期时刻（游戏秒）
    pub expires_at: f32,
    /// 缓冲优先级——满容量时用于淘汰
    pub buffer_priority: BufferPriority,
}

impl BufferedInput {
    /// 新建缓冲条目。
    pub fn new(
        action_request: ActionRequest,
        pressed_at: f32,
        buffer_window_ms: f32,
        buffer_priority: BufferPriority,
    ) -> Self {
        Self {
            action_request,
            pressed_at,
            expires_at: pressed_at + buffer_window_ms / 1000.0,
            buffer_priority,
        }
    }

    /// 是否已过期。
    pub fn is_expired(&self, now: f32) -> bool {
        now >= self.expires_at
    }
}

// ── InputAction ─────────────────────────────────────────────────

/// 玩家输入动作——平台无关枚举。
///
/// Godot InputMap → `input_bridge.gd` → `InputState` → 此枚举。
/// 定义在 `woworld_core`——Godot 桥接层和 Rust 核心都能看到。
///
/// 参见: `004-ActionResolver与输入解析.md` §一。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InputAction {
    // ── 移动（持续）──
    MoveDirection,
    Jump,
    Sprint,
    Crouch,
    Crawl,
    Walk,
    // ── 战斗（瞬时）──
    LightAttack,
    HeavyAttack,
    Block,
    Dodge,
    Parry,
    TargetLock,
    TargetSwitchLeft,
    TargetSwitchRight,
    CombatStyleSwitch,
    SpecialSkill(u8),
    // ── 交互（瞬时）──
    Interact,
    InteractWheel,
    PickUpAll,
    Talk,
    // ── 物品（瞬时）──
    HotbarSlot(u8),
    UseItem,
    DropItem,
    ThrowItem,
    QuickInventory,
    // ── 角色与视角 ──
    CameraRotate,
    CameraZoom,
    FirstPersonToggle,
    CharacterSwitch,
    ControlModeToggle,
    // ── 系统 ──
    OpenMap,
    OpenJournal,
    OpenSkills,
    OpenInventory,
    QuickSave,
    QuickLoad,
    PauseMenu,
}

impl InputAction {
    /// 该动作所属的行为域——用于 `ControlMode` 域过滤（004 §五）。
    ///
    /// `None` = 元操作（相机/角色切换/系统菜单）——不受 ControlMode 过滤，恒可用。
    ///
    /// 域映射 Provisional（Talk→Speech 等边界情形待实机调整）；仅在
    /// `DomainDelegated`（玩家系统 Phase 2）落地后才产生实际过滤效果。
    pub fn domain(self) -> Option<ActionDomain> {
        use InputAction::*;
        match self {
            MoveDirection | Jump | Sprint | Crouch | Crawl | Walk => Some(ActionDomain::Movement),
            LightAttack | HeavyAttack | Block | Dodge | Parry | TargetLock | TargetSwitchLeft
            | TargetSwitchRight | CombatStyleSwitch | SpecialSkill(_) => Some(ActionDomain::Combat),
            Interact | InteractWheel | PickUpAll => Some(ActionDomain::Interaction),
            Talk => Some(ActionDomain::Speech),
            HotbarSlot(_) | UseItem | DropItem | ThrowItem | QuickInventory => {
                Some(ActionDomain::ItemUse)
            }
            CameraRotate | CameraZoom | FirstPersonToggle | CharacterSwitch | ControlModeToggle
            | OpenMap | OpenJournal | OpenSkills | OpenInventory | QuickSave | QuickLoad
            | PauseMenu => None,
        }
    }

    /// 是否为元操作（不受 ControlMode 域过滤）。
    pub fn is_meta(self) -> bool {
        self.domain().is_none()
    }

    /// 从整数编码 + 载荷还原 `InputAction`——Godot `input_bridge.gd` 传输约定。
    ///
    /// GDScript 无法直接构造 Rust 枚举，故桥接层以 `(code, payload)` 整数对喂入
    /// `WorldDriver::input_press/input_release`，此函数还原为强类型枚举。
    /// `payload` 仅对带载荷变体（`SpecialSkill`/`HotbarSlot`）有意义，其余忽略。
    ///
    /// 编码稳定契约——改动需同步 `input_bridge.gd`。未知 code 返回 `None`（桥接层丢弃）。
    pub fn from_code(code: i64, payload: i64) -> Option<InputAction> {
        use InputAction::*;
        let slot = payload.clamp(0, u8::MAX as i64) as u8;
        Some(match code {
            // 移动 1-5
            1 => Jump,
            2 => Sprint,
            3 => Crouch,
            4 => Crawl,
            5 => Walk,
            // 战斗 10-19
            10 => LightAttack,
            11 => HeavyAttack,
            12 => Block,
            13 => Dodge,
            14 => Parry,
            15 => TargetLock,
            16 => TargetSwitchLeft,
            17 => TargetSwitchRight,
            18 => CombatStyleSwitch,
            19 => SpecialSkill(slot),
            // 交互 20-23
            20 => Interact,
            21 => InteractWheel,
            22 => PickUpAll,
            23 => Talk,
            // 物品 30-34
            30 => HotbarSlot(slot),
            31 => UseItem,
            32 => DropItem,
            33 => ThrowItem,
            34 => QuickInventory,
            // 角色与视角 40-44
            40 => CameraRotate,
            41 => CameraZoom,
            42 => FirstPersonToggle,
            43 => CharacterSwitch,
            44 => ControlModeToggle,
            // 系统 50-56
            50 => OpenMap,
            51 => OpenJournal,
            52 => OpenSkills,
            53 => OpenInventory,
            54 => QuickSave,
            55 => QuickLoad,
            56 => PauseMenu,
            _ => return None,
        })
    }
}

// ── HeldItemKind ────────────────────────────────────────────────

/// 手持物类别——ActionResolver 第二层装备映射的**输入**（004 §二）。
///
/// ⚠️ **Stub 数据源**：装备系统未接线前，实体默认视为 `Empty`（空手）。
/// 装备系统建好后填充对应组件即接线，resolver 逻辑零改动。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HeldItemKind {
    /// 空手——攻击键映射到徒手/通用近战
    #[default]
    Empty,
    /// 近战武器——攻击键映射到轻/重攻击
    Melee,
    /// 远程武器（弓）——攻击键映射到拉弓瞄准
    Ranged,
    /// 工具（稿/斧）——攻击键结合上下文映射到采集
    Tool,
}

// ── HotbarConfig ────────────────────────────────────────────────

/// 热键栏配置——数字键槽位 → `ActionId`（玩家拖拽配置，004 §二 第三层）。
///
/// 槽位 0-9：键 `1`-`9` 映射到 idx 1-9，idx 0 预留。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HotbarConfig {
    slots: [Option<ActionId>; 10],
}

impl HotbarConfig {
    /// 空热键栏。
    pub fn new() -> Self {
        Self { slots: [None; 10] }
    }

    /// 绑定槽位 → 动作。越界槽位忽略。
    pub fn set(&mut self, slot: u8, action: ActionId) {
        if (slot as usize) < self.slots.len() {
            self.slots[slot as usize] = Some(action);
        }
    }

    /// 查询槽位绑定的动作。
    pub fn get(&self, slot: u8) -> Option<ActionId> {
        self.slots.get(slot as usize).copied().flatten()
    }
}

impl Default for HotbarConfig {
    fn default() -> Self {
        Self::new()
    }
}

// ── InputState ──────────────────────────────────────────────────

/// 单帧输入快照——`input_bridge.gd` 每帧填充，ActionResolver/PlayerInput 消费。
///
/// 非 ECS Component——作为 `WorldDriver` 字段按引用传入系统（与 `ActionRegistry`
/// 等资源同模式）。仅玩家输入源。
///
/// `pressed`/`released` 为**本帧边沿**（begin_frame 清空），`held` 跨帧持续。
///
/// 参见: `004-ActionResolver与输入解析.md` §一/§五/§六。
#[derive(Debug, Clone)]
pub struct InputState {
    /// 本帧新按下的动作（边沿）。
    pressed: Vec<InputAction>,
    /// 本帧释放的动作（边沿）。
    released: Vec<InputAction>,
    /// 当前按住的动作（跨帧持续）。
    held: Vec<InputAction>,
    /// 原始移动输入（WASD/摇杆，相机相对——由 PlayerInputSystem 转世界空间）。
    pub move_direction: Vec2,
    /// 相机变换——用于将 move_direction 转到世界空间。
    pub camera_transform: Mat4,
    /// 相机环顾增量（鼠标/右摇杆）。
    pub camera_look_delta: Vec2,
    /// 相机缩放增量（滚轮）。
    pub camera_zoom_delta: f32,
}

impl Default for InputState {
    fn default() -> Self {
        Self {
            pressed: Vec::new(),
            released: Vec::new(),
            held: Vec::new(),
            move_direction: Vec2::ZERO,
            camera_transform: Mat4::IDENTITY,
            camera_look_delta: Vec2::ZERO,
            camera_zoom_delta: 0.0,
        }
    }
}

impl InputState {
    /// 本帧是否新按下该动作。
    pub fn was_pressed(&self, action: InputAction) -> bool {
        self.pressed.contains(&action)
    }

    /// 本帧是否释放该动作。
    pub fn was_released(&self, action: InputAction) -> bool {
        self.released.contains(&action)
    }

    /// 该动作当前是否按住。
    pub fn is_held(&self, action: InputAction) -> bool {
        self.held.contains(&action)
    }

    /// 本帧新按下的动作切片。
    pub fn pressed_actions(&self) -> &[InputAction] {
        &self.pressed
    }

    /// 当前按住的动作切片。
    pub fn held_actions(&self) -> &[InputAction] {
        &self.held
    }

    /// 记录一次按下（桥接层/测试用）——同时进入 pressed 与 held。
    pub fn press(&mut self, action: InputAction) {
        if !self.pressed.contains(&action) {
            self.pressed.push(action);
        }
        if !self.held.contains(&action) {
            self.held.push(action);
        }
    }

    /// 记录一次释放（桥接层/测试用）——进入 released，移出 held。
    pub fn release(&mut self, action: InputAction) {
        if !self.released.contains(&action) {
            self.released.push(action);
        }
        self.held.retain(|a| *a != action);
    }

    /// 帧初清理——清空按下/释放边沿，held 保持。
    pub fn begin_frame(&mut self) {
        self.pressed.clear();
        self.released.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::{ActionId, ActionParams, ActionSource};

    // ── BufferPriority ──

    #[test]
    fn test_buffer_priority_ordering() {
        assert!(BufferPriority::Interaction < BufferPriority::Combat);
        assert!(BufferPriority::Combat < BufferPriority::Movement);
        assert!(BufferPriority::Movement < BufferPriority::Defensive);
        assert!(BufferPriority::Defensive < BufferPriority::Instinct);
        assert!(BufferPriority::Instinct < BufferPriority::Emergency);
    }

    #[test]
    fn test_buffer_priority_discriminant() {
        assert_eq!(BufferPriority::Interaction as u8, 10);
        assert_eq!(BufferPriority::Emergency as u8, 100);
    }

    // ── InputFeelConfig ──

    #[test]
    fn test_input_feel_config_default() {
        let c = InputFeelConfig::default();
        assert!(c.coyote_time_ms > 0.0);
        assert!(c.jump_buffer_ms > 0.0);
        assert!(c.default_accel > 0.0);
        assert!(c.default_decel > 0.0);
        assert!(c.air_accel_multiplier > 0.0);
        assert!(c.air_accel_multiplier < 1.0);
    }

    #[test]
    fn test_input_feel_config_ledge_snap() {
        let c = InputFeelConfig::default();
        assert!(c.ledge_snap_distance > 0.0);
        assert!(c.ledge_snap_angle > 0.0);
        assert!(c.ledge_snap_angle < 90.0);
    }

    // ── BufferedInput ──

    #[test]
    fn test_buffered_input_expiration() {
        let req = ActionRequest {
            action_id: ActionId(1),
            priority: 20,
            source: ActionSource::Player,
            params: ActionParams::default(),
        };
        let bi = BufferedInput::new(req, 1.0, 200.0, BufferPriority::Movement);
        assert!(!bi.is_expired(1.0));
        assert!(!bi.is_expired(1.19));
        assert!(bi.is_expired(1.2)); // 200ms = 0.2s
        assert!(bi.is_expired(2.0));
    }

    #[test]
    fn test_buffered_input_fields() {
        let req = ActionRequest {
            action_id: ActionId(2),
            priority: 15,
            source: ActionSource::Player,
            params: ActionParams::default(),
        };
        let bi = BufferedInput::new(req.clone(), 5.0, 150.0, BufferPriority::Combat);
        assert_eq!(bi.action_request.action_id, ActionId(2));
        assert!((bi.pressed_at - 5.0).abs() < 0.001);
        assert!((bi.expires_at - 5.15).abs() < 0.001);
        assert_eq!(bi.buffer_priority, BufferPriority::Combat);
    }

    // ── InputAction ──

    #[test]
    fn test_input_action_domain_mapping() {
        assert_eq!(InputAction::Jump.domain(), Some(ActionDomain::Movement));
        assert_eq!(
            InputAction::LightAttack.domain(),
            Some(ActionDomain::Combat)
        );
        assert_eq!(
            InputAction::SpecialSkill(3).domain(),
            Some(ActionDomain::Combat)
        );
        assert_eq!(
            InputAction::Interact.domain(),
            Some(ActionDomain::Interaction)
        );
        assert_eq!(InputAction::Talk.domain(), Some(ActionDomain::Speech));
        assert_eq!(InputAction::UseItem.domain(), Some(ActionDomain::ItemUse));
        assert_eq!(
            InputAction::HotbarSlot(1).domain(),
            Some(ActionDomain::ItemUse)
        );
    }

    #[test]
    fn test_input_action_meta_has_no_domain() {
        assert!(InputAction::ControlModeToggle.is_meta());
        assert!(InputAction::CameraRotate.is_meta());
        assert!(InputAction::PauseMenu.is_meta());
        assert!(InputAction::CharacterSwitch.is_meta());
        assert_eq!(InputAction::PauseMenu.domain(), None);
        assert!(!InputAction::Jump.is_meta());
    }

    #[test]
    fn test_input_action_payload_variants_distinct() {
        assert_ne!(InputAction::SpecialSkill(0), InputAction::SpecialSkill(1));
        assert_ne!(InputAction::HotbarSlot(1), InputAction::HotbarSlot(2));
        assert_eq!(InputAction::HotbarSlot(3), InputAction::HotbarSlot(3));
    }

    #[test]
    fn test_input_action_from_code_basic() {
        assert_eq!(InputAction::from_code(1, 0), Some(InputAction::Jump));
        assert_eq!(InputAction::from_code(13, 0), Some(InputAction::Dodge));
        assert_eq!(InputAction::from_code(20, 0), Some(InputAction::Interact));
        assert_eq!(InputAction::from_code(56, 0), Some(InputAction::PauseMenu));
    }

    #[test]
    fn test_input_action_from_code_payload() {
        assert_eq!(
            InputAction::from_code(19, 3),
            Some(InputAction::SpecialSkill(3))
        );
        assert_eq!(
            InputAction::from_code(30, 7),
            Some(InputAction::HotbarSlot(7))
        );
        // payload 对无载荷变体无影响
        assert_eq!(InputAction::from_code(1, 99), Some(InputAction::Jump));
        // payload 越界钳制到 u8
        assert_eq!(
            InputAction::from_code(30, 9999),
            Some(InputAction::HotbarSlot(255))
        );
    }

    #[test]
    fn test_input_action_from_code_unknown_is_none() {
        assert_eq!(InputAction::from_code(0, 0), None); // MoveDirection 不经 press 路由
        assert_eq!(InputAction::from_code(999, 0), None);
        assert_eq!(InputAction::from_code(-1, 0), None);
    }

    // ── HotbarConfig ──

    #[test]
    fn test_hotbar_config_set_get() {
        let mut hb = HotbarConfig::new();
        assert_eq!(hb.get(1), None);
        hb.set(1, ActionId(42));
        hb.set(9, ActionId(7));
        assert_eq!(hb.get(1), Some(ActionId(42)));
        assert_eq!(hb.get(9), Some(ActionId(7)));
        assert_eq!(hb.get(5), None);
    }

    #[test]
    fn test_hotbar_config_out_of_range_ignored() {
        let mut hb = HotbarConfig::new();
        hb.set(10, ActionId(1)); // 越界——忽略，不 panic
        hb.set(200, ActionId(2));
        assert_eq!(hb.get(10), None);
        assert_eq!(hb.get(200), None);
    }

    // ── InputState ──

    #[test]
    fn test_input_state_press_hold_release() {
        let mut s = InputState::default();
        assert!(!s.is_held(InputAction::Jump));

        s.press(InputAction::Jump);
        assert!(s.was_pressed(InputAction::Jump));
        assert!(s.is_held(InputAction::Jump));
        assert!(!s.was_released(InputAction::Jump));

        // 下一帧：边沿清空，held 保持
        s.begin_frame();
        assert!(!s.was_pressed(InputAction::Jump));
        assert!(s.is_held(InputAction::Jump));

        s.release(InputAction::Jump);
        assert!(s.was_released(InputAction::Jump));
        assert!(!s.is_held(InputAction::Jump));
    }

    #[test]
    fn test_input_state_press_idempotent() {
        let mut s = InputState::default();
        s.press(InputAction::Sprint);
        s.press(InputAction::Sprint);
        assert_eq!(s.pressed_actions().len(), 1);
        assert_eq!(s.held_actions().len(), 1);
    }

    #[test]
    fn test_input_state_default_fields() {
        let s = InputState::default();
        assert_eq!(s.move_direction, Vec2::ZERO);
        assert_eq!(s.camera_transform, Mat4::IDENTITY);
        assert_eq!(s.camera_zoom_delta, 0.0);
        assert!(s.pressed_actions().is_empty());
        assert!(s.held_actions().is_empty());
    }
}
