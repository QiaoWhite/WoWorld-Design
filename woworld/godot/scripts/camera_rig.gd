extends Node3D
## CameraRig — 第三人称自由轨道相机
##
## 帧序 (spec 007 §五.3):
##   ① _input —— 鼠标 → yaw(rig)/pitch(PitchArm), 滚轮 → target_arm, V/ESC
##   ② InputBridge._process(-100) —— 读 CameraRig.global_transform
##   ③ WorldDriver._process(0) —— ECS → SNAP → SmoothDamp 跟随 → 碰撞 → Zoom → 设本节点位置
##   ④ 本脚本._process(+1) —— Sprint FOV kick
##
## 节点树: CameraRig (yaw) → PitchArm (pitch, local y=0) → Camera3D (local z=+arm)

const MOUSE_SENS: float = 0.003
const ZOOM_STEP: float = 0.5
const PITCH_TP_MIN: float = deg_to_rad(-80.0)
const PITCH_TP_MAX: float = deg_to_rad(80.0)
const PITCH_FP_MIN: float = deg_to_rad(-89.0)
const PITCH_FP_MAX: float = deg_to_rad(89.0)
const ARM_MIN: float = 0.0
const ARM_MAX: float = 8.0
const ARM_DEFAULT: float = 4.0
const SPRINT_FOV_KICK: float = 7.0

var _mouse_captured: bool = false
var _fp_mode: bool = false
var _target_arm: float = ARM_DEFAULT
var _cached_tp_arm: float = ARM_DEFAULT
var _base_fov: float = 75.0

@onready var _pitch_arm: Node3D = $PitchArm
@onready var _camera: Camera3D = $PitchArm/Camera3D
@onready var _driver = get_node_or_null("../WorldDriver")


func _ready() -> void:
	process_priority = 1
	Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
	_mouse_captured = true
	if _camera:
		_camera.current = true
		_base_fov = _camera.fov


func _input(event: InputEvent) -> void:
	var console_open: bool = _driver and _driver.has_method("is_console_open") and _driver.is_console_open()
	if console_open:
		return

	# ── Mouse look: yaw(rig), pitch(PitchArm) ──
	if event is InputEventMouseMotion and _mouse_captured:
		rotate_y(-event.relative.x * MOUSE_SENS)
		if _pitch_arm:
			_pitch_arm.rotate_x(-event.relative.y * MOUSE_SENS)
			var clamp_min = PITCH_FP_MIN if _fp_mode else PITCH_TP_MIN
			var clamp_max = PITCH_FP_MAX if _fp_mode else PITCH_TP_MAX
			_pitch_arm.rotation.x = clampf(_pitch_arm.rotation.x, clamp_min, clamp_max)

	# ── Zoom (scroll wheel) ──
	if event is InputEventMouseButton and event.pressed and _mouse_captured:
		if event.button_index == MOUSE_BUTTON_WHEEL_UP:
			_target_arm = maxf(_target_arm - ZOOM_STEP, ARM_MIN)
		elif event.button_index == MOUSE_BUTTON_WHEEL_DOWN:
			_target_arm = minf(_target_arm + ZOOM_STEP, ARM_MAX)
		# arm < 0.4 自动 FP (spec §VII.2)
		_fp_mode = _target_arm < 0.4
		_push_target_arm()

	# ── Key handling ──
	if event is InputEventKey and event.pressed:
		# ESC: toggle mouse capture
		if event.keycode == KEY_ESCAPE:
			if _mouse_captured:
				Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
				_mouse_captured = false
			else:
				Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
				_mouse_captured = true

		# V: toggle first-person / third-person
		if event.keycode == KEY_V:
			_fp_mode = not _fp_mode
			if _fp_mode:
				# 进 FP：缓存当前 TP 臂长，拉到 0
				_cached_tp_arm = maxf(_target_arm, 1.0)
				_target_arm = 0.0
			else:
				# 退 FP：恢复上次 TP 臂长 (spec §VII.2)
				_target_arm = _cached_tp_arm
			_push_target_arm()


func _process(_delta: float) -> void:
	# ── Sprint FOV kick ──
	if _camera and _driver and _driver.has_method("is_player_sprinting"):
		var sprinting: bool = _driver.is_player_sprinting()
		var target_fov: float = _base_fov + (SPRINT_FOV_KICK if sprinting else 0.0)
		_camera.fov = lerpf(_camera.fov, target_fov, 6.0 * _delta)
		_camera.fov = clampf(_camera.fov, 60.0, 120.0)


func _push_target_arm() -> void:
	if _driver and _driver.has_method("set_target_arm_distance"):
		_driver.set_target_arm_distance(_target_arm)
