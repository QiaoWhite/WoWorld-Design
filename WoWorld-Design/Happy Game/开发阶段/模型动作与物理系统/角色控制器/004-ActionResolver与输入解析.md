# 004 — ActionResolver 与输入解析

> **开发代号**: WoWorld (Wonder World)
> **模块**: 模型动作与物理系统 > 角色控制器 > 004
> **版本**: v1.0
> **日期**: 2026-07-09
> **状态**: 开发规格
> **定位**: 输入→ActionRequest 的翻译层——六层映射 + 上下文解析 + 动作轮盘 + ControlMode 域过滤
> **依赖**: [[../../玩家系统/003-双角色与托管模式|ControlMode 定义]] · Godot InputMap
> **关联**: [[001-角色控制器总纲]] · [[003-ActionController与离散动作]] · [[008-手感系统]]

---

## 〇、定位

ActionResolver 是**唯一感知游戏上下文的输入层**。ActionController 完全不感知——它只看到 `ActionRequest { action_id: OpenDoor, target: entity_123 }`。

**不做的**：不检查体力/魔力/冷却——那是 ActionController 的事。不修改 MovementState——那是 MovementModeSystem 的事。

---

## 一、InputAction——Godot ↔ Rust 契约

```rust
/// 玩家输入动作——平台无关枚举。
/// Godot InputMap → InputState Resource → 此枚举。
/// 定义在 woworld_core 中——Godot 桥接层和 Rust 核心都能看到。
pub enum InputAction {
    // 移动（持续）
    MoveDirection, Jump, Sprint, Crouch, Crawl, Walk,
    // 战斗（瞬时）
    LightAttack, HeavyAttack, Block, Dodge, Parry,
    TargetLock, TargetSwitchLeft, TargetSwitchRight,
    CombatStyleSwitch, SpecialSkill(u8),
    // 交互（瞬时）
    Interact, InteractWheel, PickUpAll, Talk,
    // 物品（瞬时）
    HotbarSlot(u8), UseItem, DropItem, ThrowItem, QuickInventory,
    // 角色与视角
    CameraRotate, CameraZoom, FirstPersonToggle,
    CharacterSwitch, ControlModeToggle,
    // 系统
    OpenMap, OpenJournal, OpenSkills, OpenInventory,
    QuickSave, QuickLoad, PauseMenu,
}
```

~40 个动作，5 组。Godot 侧 `input_bridge.gd` 映射 `InputMap` 到此枚举→通过 GDExtension 传 Rust。

> **注（整数编码契约·实现新增）**：跨 GDExtension 边界传枚举用整数对 `(code: u16, payload: u8)`，由 `InputAction::from_code(code, payload)` 路由回枚举——分段：移动 1-5 / 战斗 10-19 / 交互 20-29 / 热键 30-39 / 视角 40-49 / 系统 50-56（`SpecialSkill`/`HotbarSlot` 用 payload 携带序号）。这是 `input_bridge.gd ↔ Rust` 的**稳定契约**：改动此编码表须同步两侧。详见 `woworld_core::input`。

---

## 二、六层输入映射

```
第一层：直接动作键（不可覆盖）
  Dodge / Jump / Parry → 直接映射，不查上下文

第二层：装备相关动作
  LightAttack + 手持剑 → LightAttack
  LightAttack + 手持弓 → AimDraw
  LightAttack + 手持稿 + 面前是矿 → Mine
  LightAttack + 空手 → UnarmedStrike

第三层：热键栏
  数字键 1-9 → ActionId（玩家拖拽配置）

第四层：上下文解析（"交互"键）
  E + 面前 NPC → Speak
  E + 面前门 → OpenDoor
  E + 面前掉落物 → PickUp
  E + 面前熔炉 → UseFurnace
  有歧义 → 不选择，交给轮盘

第五层：特殊技能键
  Q/E/R/F → SpecialSkill(装备的技能)

第六层：ControlModeToggle
  切换 Auto/Manual/DomainDelegated
```

---

## 三、上下文解析——InteractTarget

```rust
fn resolve_interact_target(
    interactables: &NearbyInteractables,  // 感官系统填充
) -> Option<Interactable> {
    let candidates = interactables
        .in_range(2.0)                    // 交互范围 2m
        .in_facing_cone(120.0)            // 前方 120° 锥体
        .collect();

    candidates.sort_by(|a, b|
        a.interact_priority().cmp(&b.interact_priority())
            .then(a.distance().total_cmp(&b.distance()))
    );

    let best = candidates[0];
    // 歧义检测：第二候选距离 < 第一 × 1.5 且同优先级→不自动选择
    if candidates.len() > 1 {
        let second = candidates[1];
        if second.interact_priority() == best.interact_priority()
            && second.distance() < best.distance() * 1.5 {
            return None;  // → 轮盘
        }
    }
    Some(best)
}
```

**交互优先级**（TOML 数据）：
| 优先级 | 类型 |
|--------|------|
| 100 | 救起倒地同伴 |
| 80 | 战斗中拉杆 |
| 60 | 拾取稀有物品 |
| 55 | 战斗中搜尸 |
| 50 | 门/容器/机关 |
| 40 | 对话 |
| 35 | 制作站 |
| 30 | 非战斗搜尸 |
| 25 | 采集节点 |
| 20 | 普通物品 |
| 10 | 阅读标识 |
| 5 | 坐下/休息 |

---

## 四、动作轮盘——Elin 中键方案的 WoWorld 版本

中键按住→弹出动作轮盘→右摇杆选择→释放执行。

```rust
// Phase 0 填充此 Resource → Godot 读取渲染
pub struct ActionWheelData {
    pub available_actions: Vec<WheelActionEntry>,
    pub has_ambiguity: bool,
}

pub struct WheelActionEntry {
    pub action_id: ActionId,
    pub label: String,
    pub icon: Option<IconId>,
    pub target: Option<EntityId>,
    pub disabled_reason: Option<String>,  // "体力不足"/"需要稿子"
}
```

Elin 的关键教训：中键轮盘是兜底——新玩家通过轮盘发现可用动作，老玩家把常用动作拖到热键栏。不强制"一个 E 键解决所有问题"。

---

## 五、与 ControlMode 的域过滤

```rust
fn action_resolver_system(
    input: &InputState, mode: &ControlMode,
    buf: &mut ActionRequestBuf, /* ... */
) {
    let domains = mode.manual_domains();

    // 只对玩家控制的域发出 ActionRequest
    if domains.contains(ActionDomain::Combat) {
        // 解析战斗输入
    }
    if domains.contains(ActionDomain::Interaction) {
        // 解析交互输入
    }
    // Movement 域不在这里处理——MoveIntent 由 PlayerInputSystem 直接写
}
```

Movement 域特殊——玩家方向输入→PlayerInputSystem 写 `CMoveIntent.direction`，不经过 ActionResolver。

---

## 六、持续动作的释放检测

```rust
// 释放右键 → 发出 Block 的 RELEASE 信号
if input.was_released(InputAction::Block) {
    buf.push(ActionRequest { action_id: Block, priority: 0, source: Player });
}
// ActionResolver 不需要知道当前是否在防御——ActionController 判断
```

---

## 七、NPC 路径

NPC 不经过 ActionResolver。GOAP 通过 `GoapIntentDispatchSystem` 直接写 `ActionRequestBuf`。玩家和 NPC 的 ActionRequest 在同一队列中——ActionController 按优先级仲裁，不区分来源。

---

## 八、输入反馈

ActionController 拒绝请求时→`ActionRejectedEvent`→UI System:
- 视觉：HUD 动作图标闪红（~200ms）
- 音频：低音量短促拒绝音效
- 原因提示：若明确（"体力不足"），屏幕左下角短暂显示

---

> **下一篇**: [[005-ActionOutcome与动作结果事件]]
> **上一篇**: [[003-ActionController与离散动作]]
> **父文档**: [[001-角色控制器总纲]]
