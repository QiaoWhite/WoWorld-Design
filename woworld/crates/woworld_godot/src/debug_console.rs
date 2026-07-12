//! DebugConsole — Bethesda 式调试控制台
//!
//! F3 开关 → CanvasLayer 覆盖 → 命令输入 → 输出日志。
//! 采用 plain struct 模式（与 EntityRenderer 一致），不注册 GodotClass。
//!
//! 参见: `开发阶段/模型动作与物理系统/007-调试可视化与EntityRenderer架构.md` §三

use godot::classes::{Camera3D, CanvasLayer, Label, LineEdit, Node3D, RichTextLabel};
use godot::prelude::*;
use std::collections::HashMap;

use woworld_core::entity_visual::EntityDebugSnapshot;
use woworld_ecs::systems::entity_visual::entity_debug_system;

/// 每个命令的输出行数上限
const OUTPUT_MAX_LINES: usize = 500;

/// 命令函数类型
pub type CommandFn = fn(args: &[&str], state: &mut ConsoleState, world: &hecs::World) -> String;

/// 命令注册表条目
pub struct CommandEntry {
    pub func: CommandFn,
    pub help: &'static str,
}

/// 调试控制台
pub struct DebugConsole {
    // ── Godot 节点 ──
    canvas_layer: Gd<CanvasLayer>,
    bg: Gd<godot::classes::ColorRect>,
    output_label: Gd<RichTextLabel>,
    input_line: Gd<LineEdit>,
    prompt_label: Gd<Label>,
    /// 玩家镜头（raycast 选中用）
    camera: Option<Gd<Camera3D>>,

    // ── 状态 ──
    pub state: ConsoleState,
    /// 命令注册表
    commands: HashMap<String, CommandEntry>,
    /// 待处理命令队列（由 WorldDriver 消费）
    pending_commands: Vec<String>,
    /// 是否可见
    visible: bool,
    /// 上次选中（用于高亮切换检测）
    last_highlighted: Option<hecs::Entity>,
}

/// 控制台持久状态
pub struct ConsoleState {
    pub name_visible: bool,
    pub color_enhanced: bool,
    pub selected_entity: Option<hecs::Entity>,
    pub output_lines: Vec<String>,
    pub command_history: Vec<String>,
    pub history_cursor: usize,
    /// 玩家位置（用于 listnpc 距离排序，每帧由 WorldDriver 更新）
    pub player_pos: glam::Vec3,
    /// Sprint-060: possess 命令设定的目标实体（WorldDriver 下帧处理并清零）
    pub pending_possess_request: Option<hecs::Entity>,
    /// ★ V5: speed 命令设定的模拟速度倍率（WorldDriver 下帧处理并清零）
    pub pending_time_scale: Option<f32>,
}

impl Default for ConsoleState {
    fn default() -> Self {
        Self {
            name_visible: false,
            color_enhanced: false,
            selected_entity: None,
            output_lines: Vec::new(),
            command_history: Vec::new(),
            history_cursor: 0,
            player_pos: glam::Vec3::ZERO,
            pending_possess_request: None,
            pending_time_scale: None,
        }
    }
}

impl DebugConsole {
    /// 创建控制台（CanvasLayer 作为 scene_root 的子节点）
    pub fn new(scene_root: &mut Gd<Node3D>, camera: Option<Gd<Camera3D>>) -> Self {
        let mut canvas = CanvasLayer::new_alloc();
        canvas.set_layer(128);
        canvas.set_visible(false);
        canvas.set_name("DebugConsole");

        // ── 控制台 UI 节点（布局在 toggle() 中动态计算）────
        let mut bg = godot::classes::ColorRect::new_alloc();
        bg.set_color(godot::prelude::Color::from_rgba(0.0, 0.0, 0.0, 0.85));
        bg.set_mouse_filter(godot::classes::control::MouseFilter::IGNORE);

        let mut output = RichTextLabel::new_alloc();
        output.set_use_bbcode(true);
        output.set_scroll_follow(true);
        output.set_mouse_filter(godot::classes::control::MouseFilter::IGNORE);

        let mut prompt = Label::new_alloc();
        prompt.set_text("> ");
        prompt.set_mouse_filter(godot::classes::control::MouseFilter::IGNORE);

        let mut input = LineEdit::new_alloc();
        input.set_placeholder("Enter command. Type 'help' for list.");

        canvas.add_child(&bg);
        canvas.add_child(&output);
        canvas.add_child(&prompt);
        canvas.add_child(&input);
        scene_root.add_child(&canvas);

        let mut console = Self {
            canvas_layer: canvas,
            bg,
            output_label: output,
            input_line: input,
            prompt_label: prompt,
            camera,
            state: ConsoleState::default(),
            commands: HashMap::new(),
            pending_commands: Vec::new(),
            visible: false,
            last_highlighted: None,
        };

        // 注册首批命令
        console.register_commands();
        console.append_output("[color=#888888]Console ready. Type 'help' for commands.[/color]");

        console
    }

    // ── 公共接口 ────────────────────────

    /// 是否可见
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// 切换控制台可见性，自适应窗口尺寸
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        self.canvas_layer.set_visible(self.visible);
        if self.visible {
            // 读取 viewport 尺寸，按比例布局
            let vp_size = self
                .canvas_layer
                .get_viewport()
                .map(|vp| vp.get_visible_rect().size)
                .unwrap_or(Vector2::new(1920.0, 1080.0));
            let w = vp_size.x;
            let h = vp_size.y;
            let top = h * 0.55;
            let bot = h;
            let input_h = 40.0;

            // bg: 底部 45%
            self.bg.set_position(Vector2::new(0.0, top));
            self.bg.set_size(Vector2::new(w, bot - top));
            // output: bg 内部
            self.output_label.set_position(Vector2::new(8.0, top + 4.0));
            self.output_label
                .set_size(Vector2::new(w - 16.0, bot - top - input_h - 8.0));
            // prompt + input: 底行
            self.prompt_label
                .set_position(Vector2::new(8.0, bot - input_h + 4.0));
            self.prompt_label
                .set_size(Vector2::new(24.0, input_h - 8.0));
            self.input_line
                .set_position(Vector2::new(32.0, bot - input_h + 4.0));
            self.input_line
                .set_size(Vector2::new(w - 40.0, input_h - 8.0));

            self.input_line.grab_focus();
        }
    }

    /// 压入命令到队列
    pub fn push_command(&mut self, cmd: String) {
        self.pending_commands.push(cmd);
    }

    /// 取出待处理命令
    pub fn poll_command(&mut self) -> Option<String> {
        self.pending_commands.pop()
    }

    /// 向输出缓冲追加文本（BBCode 格式）
    pub fn append_output(&mut self, text: &str) {
        for line in text.lines() {
            self.state.output_lines.push(line.to_string());
        }
        // 裁剪旧行
        while self.state.output_lines.len() > OUTPUT_MAX_LINES {
            self.state.output_lines.remove(0);
        }
        // 更新 RichTextLabel
        self.output_label
            .set_text(&self.state.output_lines.join("\n"));
    }

    /// 从命令历史中回填 LineEdit
    pub fn fill_from_history(&mut self, cursor: usize) {
        if cursor < self.state.command_history.len() {
            let entry = self.state.command_history[cursor].clone();
            self.input_line.set_text(&entry);
            self.input_line.set_caret_column(entry.len() as i32);
        }
    }

    /// 获取当前输入行文本
    pub fn input_text(&self) -> String {
        self.input_line.get_text().to_string()
    }

    /// 清空输入行
    pub fn clear_input(&mut self) {
        self.input_line.clear();
    }

    /// 获取命令注册表引用（用于执行命令时查找）
    pub fn get_command(&self, name: &str) -> Option<&CommandEntry> {
        self.commands.get(name)
    }

    /// 所有命令名列表
    pub fn command_names(&self) -> Vec<&String> {
        self.commands.keys().collect()
    }

    /// 获取相机引用
    pub fn camera(&self) -> Option<&Gd<Camera3D>> {
        self.camera.as_ref()
    }

    /// 检查选中实体是否变化（用于高亮切换）
    pub fn highlight_changed(&mut self) -> Option<Option<hecs::Entity>> {
        if self.state.selected_entity != self.last_highlighted {
            let prev = self.last_highlighted;
            self.last_highlighted = self.state.selected_entity;
            Some(prev)
        } else {
            None
        }
    }

    // ── 内部 ────────────────────────────

    fn register_commands(&mut self) {
        self.commands.insert(
            "help".into(),
            CommandEntry {
                func: cmd_help,
                help: "列出所有命令及帮助",
            },
        );
        self.commands.insert(
            "nameshow".into(),
            CommandEntry {
                func: cmd_nameshow,
                help: "切换头顶名字显示（关闭控制台后保持）",
            },
        );
        self.commands.insert(
            "debugcolor".into(),
            CommandEntry {
                func: cmd_debugcolor,
                help: "切换情绪→颜色映射",
            },
        );
        self.commands.insert(
            "info".into(),
            CommandEntry {
                func: cmd_info,
                help: "打印选中实体的所有 Component 数据",
            },
        );
        self.commands.insert(
            "listnpc".into(),
            CommandEntry {
                func: cmd_listnpc,
                help: "listnpc [count] — 列出最近的 N 个 Creature",
            },
        );
        self.commands.insert(
            "select".into(),
            CommandEntry {
                func: cmd_select,
                help: "select <id> — 按 hecs entity bits 选中实体",
            },
        );
        self.commands.insert(
            "clear".into(),
            CommandEntry {
                func: cmd_clear,
                help: "清空控制台输出缓冲",
            },
        );
        self.commands.insert(
            "entitycount".into(),
            CommandEntry {
                func: cmd_entitycount,
                help: "按 EntityKind 分组统计实体数量",
            },
        );
        self.commands.insert(
            "possess".into(),
            CommandEntry {
                func: cmd_possess,
                help: "possess <entity_id> — 夺舍指定实体（按 hecs entity bits）",
            },
        );
        self.commands.insert(
            "speed".into(),
            CommandEntry {
                func: cmd_speed,
                help: "speed [value] — 设置模拟速度 (1=正常, 10=快速, 60=极快, 0=暂停)",
            },
        );
    }
}

// ── 命令实现 ────────────────────────────

fn cmd_help(_args: &[&str], _state: &mut ConsoleState, _world: &hecs::World) -> String {
    // 注意：help 命令需要访问 registry，特殊处理在 WorldDriver 中
    String::new() // 实际输出由 WorldDriver 拼接
}

fn cmd_nameshow(_args: &[&str], state: &mut ConsoleState, _world: &hecs::World) -> String {
    state.name_visible = !state.name_visible;
    format!(
        "[color=#88ff88][Console] Name display: {}[/color]",
        if state.name_visible { "ON" } else { "OFF" }
    )
}

fn cmd_debugcolor(_args: &[&str], state: &mut ConsoleState, _world: &hecs::World) -> String {
    state.color_enhanced = !state.color_enhanced;
    format!(
        "[color=#88ff88][Console] Emotion color mapping: {}[/color]",
        if state.color_enhanced {
            "ON"
        } else {
            "OFF (hash colors)"
        }
    )
}

/// ★ V5: `speed [value]` — 设置或查看模拟速度倍率
///
/// 速度档位常量（集中管理，非硬编码分散）:
///   1.0 = 正常速度, 10.0 = 快速, 60.0 = 极快（~1 秒 = 1 分钟游戏时间）, 0.0 = 暂停
fn cmd_speed(args: &[&str], state: &mut ConsoleState, _world: &hecs::World) -> String {
    const SPEED_PRESETS: &[f32] = &[1.0, 10.0, 60.0];

    match args.first() {
        None => {
            // 无参数——显示当前速度（WorldDriver 会填入当前值，此占位在注册时替换）
            "[color=#88ffff]Usage: speed <value> — e.g. speed 10 (fast), speed 1 (normal), speed 60 (very fast), speed 0 (pause)[/color]".into()
        }
        Some(&"presets") | Some(&"list") => {
            format!(
                "[color=#88ffff]Speed presets: {:?} — Usage: speed <value>[/color]",
                SPEED_PRESETS
            )
        }
        Some(arg) => match arg.parse::<f32>() {
            Ok(val) if (0.0..=100.0).contains(&val) => {
                state.pending_time_scale = Some(val);
                if val == 0.0 {
                    "[color=#ffaa00][speed] Simulation paused (0x)[/color]".into()
                } else {
                    format!("[color=#88ff88][speed] {val}x[/color]")
                }
            }
            Ok(_) => "[color=#ff8888][speed] Value out of range [0.0, 100.0][/color]".into(),
            Err(_) => format!(
                "[color=#ff8888][speed] Invalid value: '{arg}'. Use a number, e.g. speed 10[/color]"
            ),
        },
    }
}

fn cmd_info(_args: &[&str], state: &mut ConsoleState, world: &hecs::World) -> String {
    let Some(entity) = state.selected_entity else {
        return "[color=#ff8888]No entity selected. Click an entity or use 'select <id>'.[/color]"
            .into();
    };

    match entity_debug_system(world, entity, None) {
        Some(snap) => format_snapshot(&snap),
        None => format!(
            "[color=#ff8888]Entity {:?} not found in ECS world (may have despawned).[/color]",
            entity.to_bits().get()
        ),
    }
}

fn cmd_listnpc(args: &[&str], state: &mut ConsoleState, world: &hecs::World) -> String {
    use woworld_ecs::components::entity_kind::EntityKind;
    use woworld_ecs::components::goal::Goal;
    use woworld_ecs::components::transform::Position;

    let count = args
        .first()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(10);
    let player_pos = state.player_pos;

    let mut npcs: Vec<(u64, String, f32, String)> = Vec::new();
    for (entity, (pos, kind, goal)) in world.query::<(&Position, &EntityKind, &Goal)>().iter() {
        if !matches!(*kind, EntityKind::Creature) {
            continue;
        }
        let dist = pos.0.distance(player_pos);
        let goal_str = format!("{:?} ({:.2})", goal.goal_type, goal.urgency);
        npcs.push((
            entity.to_bits().get(),
            format!("NPC_{}", entity.to_bits().get()),
            dist,
            goal_str,
        ));
    }

    npcs.sort_by(|a, b| a.2.partial_cmp(&b.2).unwrap_or(std::cmp::Ordering::Equal));
    let shown = npcs.iter().take(count).collect::<Vec<_>>();
    if shown.is_empty() {
        return "[color=#888888]No Creature entities found.[/color]".into();
    }

    let mut out = format!("[color=#88ffff]NPCs (nearest {}):[/color]\n", shown.len());
    for (id, _name, dist, goal) in shown {
        out.push_str(&format!(
            "  [color=#ffffff]{id}[/color] — {:.1}m — {goal}\n",
            dist
        ));
    }
    out
}

fn cmd_select(args: &[&str], state: &mut ConsoleState, _world: &hecs::World) -> String {
    let Some(id_str) = args.first() else {
        return "[color=#ff8888]Usage: select <entity_bits_id>[/color]".into();
    };

    match id_str.parse::<u64>() {
        Ok(bits) => match hecs::Entity::from_bits(bits) {
            Some(entity) => {
                state.selected_entity = Some(entity);
                format!("[color=#88ff88]Selected entity: {bits}[/color]")
            }
            None => format!(
                "[color=#ff8888]Invalid entity bits: {bits} (no entity with that ID)[/color]"
            ),
        },
        Err(_) => format!("[color=#ff8888]Invalid ID: '{id_str}'. Must be an integer.[/color]"),
    }
}

fn cmd_clear(_args: &[&str], state: &mut ConsoleState, _world: &hecs::World) -> String {
    state.output_lines.clear();
    String::from("[color=#888888](cleared)[/color]")
}

fn cmd_entitycount(_args: &[&str], _state: &mut ConsoleState, world: &hecs::World) -> String {
    use woworld_ecs::components::entity_kind::EntityKind;
    let mut counts: HashMap<String, u32> = HashMap::new();
    for (_entity, kind) in world.query::<&EntityKind>().iter() {
        let name = format!("{:?}", *kind);
        *counts.entry(name).or_default() += 1;
    }

    let mut out = "[color=#88ffff]Entity counts:[/color]\n".to_string();
    let mut sorted: Vec<_> = counts.iter().collect();
    sorted.sort_by_key(|(name, _)| *name);
    for (kind, count) in &sorted {
        out.push_str(&format!("  {kind}: {count}\n"));
    }
    if sorted.is_empty() {
        out.push_str("  (no entities)\n");
    }
    out
}

/// Sprint-060: `possess <entity_id>` — 夺舍指定实体
fn cmd_possess(args: &[&str], state: &mut ConsoleState, _world: &hecs::World) -> String {
    if args.is_empty() {
        return "[color=#ff8888]Usage: possess <entity_id>[/color]".into();
    }
    let Ok(bits) = args[0].parse::<u64>() else {
        return format!(
            "[color=#ff8888]Invalid entity_id: '{}'. Must be a number.[/color]",
            args[0]
        );
    };

    // hecs::Entity::from_bits returns Option<Entity>
    let Some(entity) = hecs::Entity::from_bits(bits) else {
        return format!(
            "[color=#ff8888]Invalid entity_id: {bits}. Could not construct Entity.[/color]"
        );
    };

    // 验证实体存在
    if _world
        .get::<&woworld_ecs::components::transform::Position>(entity)
        .is_err()
    {
        return format!("[color=#ff8888]Entity {bits} not found in ECS.[/color]");
    }
    // 设置请求——WorldDriver 下帧处理
    state.pending_possess_request = Some(entity);
    format!("[color=#88ff88]Possess request queued for entity {bits}. Teleporting next frame...[/color]")
}

// ── 格式化 ──────────────────────────────

/// 将 EntityDebugSnapshot 格式化为控制台输出
fn format_snapshot(snap: &EntityDebugSnapshot) -> String {
    let mut out = format!(
        "[color=#ffcc00]=== {} (ID: {}) ===[/color]\n",
        snap.display_name, snap.entity_bits
    );
    out.push_str(&format!(
        "[color=#888888]  Kind: {:?}  Pos: ({:.1}, {:.1}, {:.1})[/color]\n",
        snap.kind, snap.position.x, snap.position.y, snap.position.z
    ));

    for section in &snap.sections {
        out.push_str(&format!("[color=#88ccff]  [{}][/color]\n", section.title));
        for field in &section.fields {
            let colored = if let Some(ref color) = field.color_hint {
                format!("[color={}]{}[/color]", color, field.value)
            } else {
                field.value.clone()
            };
            out.push_str(&format!("    {}: {}\n", field.label, colored));
        }
    }

    out
}
