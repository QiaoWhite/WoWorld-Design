# 002-Mod 社区架构：从第一天为 Modder 设计

> 核心论点：Mod 支持不是"做完游戏再加的功能"——它必须在架构的 DNA 中。  
> 对于 WoWorld，"活的世界"的最终证明不是开发者写了什么——而是 Modder 用它创造了什么。

---

## 〇、Mod 支持的五个层级

```
Level 1: 数据替换          最容易——玩家都可以做
  └─ 替换纹理、模型、音频、文本

Level 2: 数据新增          需要理解数据格式
  └─ 新增 NPC 模板、物品、建筑、文化参数、动画

Level 3: 行为脚本          需要理解 Rhai 脚本语言
  └─ 编写 NPC 自定义行为、事件响应、对话逻辑

Level 4: 世界规则修改      需要理解 WoWorld 的系统架构
  └─ 修改世界生成参数、经济规则、文化演化参数

Level 5: 原生扩展          需要 Rust 编程能力
  └─ 编写 Rust 动态库，替换或扩展模拟核心模块
```

每个层级为上一层级提供基础。Level 1-3 是主要目标，Level 4-5 是给硬核 Modder 的额外深度。

---

## 一、数据驱动架构（Level 1-2 的基础）

### 1.1 核心原则：代码与数据分离

```
传统方式（不可 Mod）:
  NPC 模板硬编码在 Rust 中
  → let blacksmith = NpcTemplate { name: "铁匠", ... };
  → Modder 无法新增 NPC 类型

数据驱动方式（可 Mod）:
  NPC 模板定义在 TOML 文件中
  → npc_templates/blacksmith.toml
  → Modder 创建 npc_templates/clockmaker.toml → 游戏自动加载
```

### 1.2 TOML 数据格式

选择 TOML 而非 JSON：
- **人类可写**：注释、多行字符串、更少的引号
- **Rust 原生支持**：`toml` crate 成熟
- **Modder 友好**：比 JSON 更不容易出错

```toml
# npc_templates/blacksmith.toml
# 铁匠——城镇基本手工艺者

[identity]
template_id = "human.blacksmith"
species = "human"
name_pool = ["锻造者", "铁锤", "炉火"]

[personality]
# Big Five (0.0-1.0)
openness = 0.35
conscientiousness = 0.75
extraversion = 0.40
agreeableness = 0.55
neuroticism = 0.30

# 扩展特质
courage = 0.40
honesty = 0.70
diligence = 0.85
generosity = 0.35

[physiology]
base_health = 0.9
base_stamina = 0.75
strength = 0.70
endurance = 0.65

[skills]
blacksmithing = { base_level = 25.0, talent = 0.6 }
metalworking = { base_level = 20.0, talent = 0.5 }
trading = { base_level = 10.0, talent = 0.3 }

[social]
role = "village_craftsman"
typical_schedule = "workshop_day"
wealth_level = "middle"

[behavior]
daily_routine = "craftsman"
goap_goals = ["maintain_business", "feed_family", "socialize_evening"]
probability_profile = "diligent_worker"

[culture]
preferred_building_style = "workshop"
cuisine_preference = "hearty"
```

### 1.3 Mod 加载顺序与覆盖规则

```
加载顺序（优先级从低到高）:
  1. 游戏核心数据 (base/)          ← 最低优先级
  2. 官方 DLC/扩展 (official/)     ← 覆盖 base
  3. 启用的 Mod (mods/{mod_name}/)  ← 覆盖 official
  4. 用户本地覆盖 (user/)           ← 最高优先级
  
覆盖规则：
  - 同 ID 的条目：高优先级覆盖低优先级
  - 新增条目：直接添加
  - 删除条目：在更高优先级中标记为 deleted = true
```

### 1.4 可以数据驱动的系统清单

| 系统 | 数据文件 | 示例 |
|------|---------|------|
| NPC 模板 | `npc_templates/*.toml` | 所有职业、物种的性格/技能/行为预设 |
| 物品定义 | `items/*.toml` | 武器/防具/消耗品/材料/杂物 |
| 建筑模板 | `buildings/*.toml` | WFC 约束规则 + 建筑部件定义 |
| 文化种子 | `cultures/*.toml` | 初始文化参数 + 美学倾向 |
| 生物群系 | `biomes/*.toml` | 温度/降水/植被/动物 |
| 配方表 | `recipes/*.toml` | 制造配方 + 条件 + 产物 |
| 对话模板 | `dialogue/*.toml` | 预制对话文本 + 触发条件 |
| 事件模板 | `events/*.toml` | 全局事件的类型/概率/影响 |
| 动画数据库 | `animations/*.pose` | Pose Database 的姿态定义 |
| UI 主题 | `themes/*.tres` | Godot 原生 Theme 资源 |

---

## 二、Rhai 脚本引擎（Level 3）

### 2.1 为什么是 Rhai

| | Lua (mlua) | Rhai | Python (pyo3) | 自定义语言 |
|---|---|---|---|---|
| Rust 集成 | 好 | **极好（原生）** | 一般 | 工作量巨大 |
| 学习曲线（Modder） | Lua 简单 | Rhai 类似 Rust 但更简单 | Python 最简单 | 取决于设计 |
| 沙箱安全 | 需手动 | **内置** | 需手动 | 取决于设计 |
| 性能 | 好(JIT) | 中等（解释型） | 中 | 自定义 |
| 嵌入体积 | 小 | **极小** | 大 | 极小 |

**Rhai 是 Rust 生态的原生脚本语言。** 它的语法有意设计为接近 Rust——这对 WoWorld 项目意味着：开发者在 Rust 和 Rhai 之间切换几乎没有认知负担。Rhai 内置了沙箱——可以限制 Mod 脚本能做什么。

### 2.2 Rhai 脚本示例：自定义 NPC 行为

```rust
// Rhai 脚本：mods/example/behaviors/talkative_blacksmith.rhai

// 当 NPC 空闲时触发
fn on_npc_idle(npc) {
    // 访问 NPC 状态
    let hunger = npc.get_physiology("hunger");
    let time = world.get_time_of_day();
    
    // 铁匠在午饭后喜欢去酒馆聊天
    if time > 12.0 && time < 14.0 && hunger < 0.3 {
        // 找到最近的酒馆
        let tavern = world.find_nearest_location(npc.position(), "tavern");
        if tavern != null {
            npc.set_goal("socialize", #{
                location: tavern,
                priority: 3,
                duration_minutes: 30 + rand_float() * 30,
            });
            return;
        }
    }
    
    // 默认行为交给 Rust 侧的概率决策器
    npc.use_default_behavior();
}

// 当 NPC 与玩家对话时触发
fn on_player_dialogue(npc, player, topic) {
    // 铁匠会给老顾客打折
    let familiarity = npc.get_relationship(player, "familiarity");
    if familiarity > 0.6 && topic == "trade" {
        npc.apply_trade_modifier(player, "discount", 0.15); // 15% 折扣
        npc.say("老顾客了，给你个好价钱。");
    }
}
```

### 2.3 Rhai 脚本能访问的系统

```
Rust 侧暴露给 Rhai 的 API（通过 Rhai 模块注册）:

NPC:
  npc.get_physiology(field) → f32
  npc.get_emotion(field) → f32
  npc.get_personality(trait) → f32
  npc.get_relationship(target, field) → f32
  npc.set_goal(goal_type, params)
  npc.say(text)
  npc.add_memory(event_summary, impact)
  npc.position() → (x, y, z)

World:
  world.get_time_of_day() → f32
  world.get_day() → i32
  world.find_nearest_location(pos, type) → Location | null
  world.get_npcs_in_radius(pos, radius) → [NPC]
  world.spawn_item(item_id, pos)
  world.trigger_event(event_id)

Player:
  player.get_reputation(region) → f32
  player.get_skill(skill_id) → f32
  player.add_journal_entry(text)
```

每个函数在 Rust 侧有严格的权限检查——Rhai 只能访问明确暴露的 API。

### 2.4 沙箱限制

Rhai 脚本**不能**：
- 访问文件系统（不能读写玩家的文件）
- 发起网络请求（不能联网）
- 直接修改 Rust 内存（只能通过 API 函数）
- 调用系统命令
- 在脚本内无限循环（Rhai 内置执行时间限制）

这些限制由 Rhai 引擎在 Rust 侧强制执行——Modder 无法绕过。

---

## 三、事件钩子系统

Mod 通过"钩子"注入自定义逻辑——不修改核心代码。

### 3.1 钩子类型

```rust
// Rust 侧定义的钩子注册表
enum ModHook {
    // 生命周期钩子
    OnWorldInit,           // 世界创建时
    OnNpcSpawn(u64),       // NPC 实例化时
    OnNpcDespawn(u64),     // NPC 降级消失时
    OnDayStart(u32),       // 每天开始时
    
    // 行为钩子
    OnNpcIdle(u64),        // NPC 空闲时（替代默认行为）
    OnNpcDecision(u64),    // NPC 做决策前（可修改权重）
    OnNpcSocialize(u64, u64), // 两个 NPC 社交时
    
    // 事件钩子
    OnEvent(u64),          // 全局事件发生时
    OnPlayerAction(PlayerAction), // 玩家行动时
    OnCombatStart(u64, u64), // 战斗开始时
    
    // UI 钩子
    OnDialogueRender(DialogueContext), // 对话渲染时（可插入选项）
}
```

### 3.2 Mod 注册钩子

在 Mod 的 `mod.toml` 中声明：

```toml
# mods/example/mod.toml
[mod]
id = "talkative_blacksmiths"
name = "健谈的铁匠们"
version = "1.0.0"
author = "SomeModder"
description = "让全世界的铁匠在午饭后都去酒馆聊天"

[dependencies]
# 此 Mod 依赖的核心游戏版本
woworld = ">=0.1.0"

[hooks]
on_npc_idle = "behaviors/talkative_blacksmith.rhai::on_npc_idle"
on_player_dialogue = "behaviors/talkative_blacksmith.rhai::on_player_dialogue"

[data]
# 此 Mod 新增的数据文件
npc_templates = ["npc_templates/"]
items = ["items/legendary_hammer.toml"]
```

---

## 四、Mod 分发与发现

### 4.1 Steam Workshop 集成（Steam 版优先）

- 游戏发布在 Steam 上时，Workshop 是最自然的 Mod 分发渠道
- 使用 Steamworks SDK 的 UGC API
- Godot 侧有社区插件处理 Steam 集成

### 4.2 独立 Mod 平台

Steam 之外的选项：
- **Nexus Mods**：最大的独立 Mod 托管平台，提供 API
- **Mod.io**：跨平台 Mod 分发服务（支持 Steam/GOG/Epic），有 Godot 插件
- Git-based：硬核 Modder 可以直接从 GitHub 安装 Mod

### 4.3 游戏内 Mod 浏览器

```
游戏主菜单
  └─ Mod 管理
      ├── 已安装的 Mod（启用/禁用/卸载）
      ├── 浏览 Mod（来自 Workshop/Mod.io）
      ├── Mod 加载顺序（拖拽排序）
      └── Mod 冲突检测（两个 Mod 修改了同一个数据文件）
```

---

## 五、Modder 文档与工具

### 5.1 文档自动生成

从 Rust 代码的文档注释自动生成 Modding API 参考：

```rust
/// 设置 NPC 的当前目标。目标会被 GOAP 规划器处理。
/// 
/// # Rhai 调用示例
/// ```rhai
/// npc.set_goal("eat", #{ priority: 5, food_type: "bread" });
/// ```
/// 
/// # 参数
/// - `goal_type`: "eat" | "sleep" | "drink" | "socialize" | "work" | "travel"
/// - `params`: 目标参数映射表（见下方说明）
/// 
/// # 优先级说明
/// - 1-3: 低优先级（休闲活动）
/// - 4-6: 中优先级（日常工作）
/// - 7-9: 高优先级（生存需求）
/// - 10: 最高优先级（紧急逃生/战斗）
#[rhai_fn]
pub fn set_goal(npc: &mut Npc, goal_type: String, params: Dynamic) {
    // ...
}
```

### 5.2 Modder 工具链

| 工具 | 用途 |
|------|------|
| **WoWorld Mod Kit** | 命令行工具：创建 Mod 骨架、验证 Mod 文件、打包为 `.wmod` 格式 |
| **VSCode 扩展** | TOML schema 验证、Rhai 语法高亮、API 自动补全 |
| **游戏内控制台** | `reload_mod <mod_id>`——热重载 Mod，无需重启游戏 |
| **Mod 沙箱测试** | 在隔离环境中加载 Mod，检测是否导致崩溃 |

### 5.3 示例 Mod 包

随游戏发布 3-5 个示例 Mod，覆盖不同难度——它们既是功能也是教程：
- `example_add_item`：新增一个物品（Level 2，入门）
- `example_npc_behavior`：自定义 NPC 行为（Level 3，核心案例）
- `example_culture_pack`：新增一个文化种子（Level 2+4）

---

## 六、Mod 系统的性能与安全

### 6.1 性能隔离

- Rhai 脚本的执行时间受严格限制（默认每帧最多 0.1ms per Mod）
- 如果 Mod 脚本超时 → 自动跳过该帧，输出警告日志
- Mod 数据文件在游戏启动时一次性加载到内存——运行时无磁盘 I/O

### 6.2 安全隔离

- Rhai 脚本运行在沙箱中——不能访问文件系统、网络、系统调用
- Mod 不能修改其他 Mod 的数据
- 如果 Mod 导致崩溃 → 自动禁用该 Mod，下次启动时提示
- Mod 数字签名（可选）——官方 Mod 平台可要求签名以验证来源

### 6.3 存档兼容性

这是 Mod 系统最大的技术挑战：

- 存档中包含"创建此存档时启用的 Mod 列表 + 版本号"
- 加载存档时检测 Mod 变化：
  - Mod 新增 → 自动应用（新物品/NPC 模板可用）
  - Mod 删除 → 该 Mod 添加的物品标记为"残留物品"（保留但不再生成新的）
  - Mod 更新 → 提示玩家确认兼容性
- 不兼容的 Mod 组合 → 警告但允许尝试

---

> **Mod 不是"做完游戏再加的功能"——它是 WoWorld 长期生命力的来源。当玩家开始用你的工具创造你自己的游戏里没有的东西时，你创造的不再是一个游戏——你创造了一个平台。**
