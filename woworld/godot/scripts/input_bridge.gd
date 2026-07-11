extends Node
## Sprint-063: input_bridge — Godot 输入采集 → WorldDriver.InputState → Block A0
##
## GDScript 铁律 §14.1：仅采集 + 转发，无游戏逻辑。所有映射/解析/域过滤在 Rust 侧。
## 每帧：begin_frame（清边沿）→ 填相机变换 + WASD 移动 → 按键边沿 press/release。
## process_priority = -100 保证在 WorldDriver 消费 InputState 之前喂入（同帧无延迟）。
##
## code 编码见 woworld_core::input::InputAction::from_code（改动需两侧同步）。

# (godot_key, code, payload)。离散与保持动作统一走 press/release 边沿——
# 使 Rust 侧 is_held（Sprint/Crouch 修饰键）与 was_pressed（Jump/Dodge）都生效。
const KEY_BINDINGS := [
	{ "key": KEY_SPACE, "code": 1,  "payload": 0 },  # Jump
	{ "key": KEY_SHIFT, "code": 2,  "payload": 0 },  # Sprint（保持）
	{ "key": KEY_CTRL,  "code": 3,  "payload": 0 },  # Crouch（保持）
	{ "key": KEY_Q,     "code": 13, "payload": 0 },  # Dodge
	{ "key": KEY_E,     "code": 20, "payload": 0 },  # Interact
]
const MOUSE_BINDINGS := [
	{ "button": MOUSE_BUTTON_LEFT, "code": 10, "payload": 0 },  # LightAttack
]

var _prev: Dictionary = {}

func _ready() -> void:
	# 先于 WorldDriver 处理，保证 InputState 在同帧被消费前已填充
	process_priority = -100

func _process(_delta: float) -> void:
	var driver = get_node_or_null("../WorldDriver")
	if driver == null or not driver.has_method("input_begin_frame"):
		return

	driver.input_begin_frame()

	# 控制台开启：清零移动 + 释放所有 held，避免卡键
	var console_open: bool = driver.has_method("is_console_open") and driver.is_console_open()
	if console_open:
		driver.input_set_move(0.0, 0.0)
		_release_all(driver)
		return

	# ★ 007: 相机变换来源改为 CameraRig（仅 yaw；pitch 隔离在 PitchArm 子节点）
	var camera_rig = get_node_or_null("../CameraRig")
	if camera_rig != null:
		driver.input_set_camera_transform(camera_rig.global_transform)

	# WASD → 相机相对移动（x = 右移, z = 前进）
	var mx: float = 0.0
	var mz: float = 0.0
	if Input.is_key_pressed(KEY_D): mx += 1.0
	if Input.is_key_pressed(KEY_A): mx -= 1.0
	if Input.is_key_pressed(KEY_W): mz += 1.0
	if Input.is_key_pressed(KEY_S): mz -= 1.0
	driver.input_set_move(mx, mz)

	# 离散/保持动作：上升沿 press，下降沿 release
	for b in KEY_BINDINGS:
		_edge(driver, b["code"], b["payload"], Input.is_key_pressed(b["key"]))
	for b in MOUSE_BINDINGS:
		_edge(driver, b["code"], b["payload"], Input.is_mouse_button_pressed(b["button"]))

func _edge(driver, code: int, payload: int, pressed: bool) -> void:
	var was: bool = _prev.get(code, false)
	if pressed and not was:
		driver.input_press(code, payload)
	elif not pressed and was:
		driver.input_release(code, payload)
	_prev[code] = pressed

func _release_all(driver) -> void:
	for code in _prev.keys():
		if _prev[code]:
			driver.input_release(code, 0)
			_prev[code] = false
