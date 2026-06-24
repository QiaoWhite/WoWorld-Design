extends CharacterBody3D
## 玩家控制器 — WASD 移动 + 鼠标环顾 + Space 跳跃
## 用 Input.get_vector 配合 UI action，不依赖 Input Map 预配置
## INPUT TUNING: CharacterBody3D 物理参数，非世界模拟常量 (§14.1 合规)

const SPEED: float = 5.0
const SPRINT_MULTIPLIER: float = 3.0
const JUMP_VELOCITY: float = 8.0
const MOUSE_SENS: float = 0.003
const GRAVITY: float = 9.8

var _mouse_captured: bool = false

func _ready():
	Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
	_mouse_captured = true

func _input(event):
	if event is InputEventMouseMotion and _mouse_captured:
		rotate_y(-event.relative.x * MOUSE_SENS)
		var cam = $Camera3D
		cam.rotate_x(-event.relative.y * MOUSE_SENS)
		cam.rotation.x = clamp(cam.rotation.x, deg_to_rad(-89), deg_to_rad(89))

	if event is InputEventKey and event.pressed:
		if event.keycode == KEY_ESCAPE:
			if _mouse_captured:
				Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
				_mouse_captured = false
			else:
				Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
				_mouse_captured = true

func _physics_process(delta):
	# 查询地形高度
	var terrain = get_node_or_null("../WorldDriver")
	var ground_h: float = 0.0
	if terrain and terrain.has_method("query_height"):
		ground_h = terrain.query_height(global_position.x, global_position.z)
	else:
		ground_h = global_position.y - 10.0  # 回退

	var on_ground = global_position.y <= ground_h + 1.8

	if not on_ground:
		velocity.y -= GRAVITY * delta

	if Input.is_key_pressed(KEY_SPACE) and on_ground:
		velocity.y = JUMP_VELOCITY

	# WASD
	var input_dir = Vector2.ZERO
	if Input.is_key_pressed(KEY_W): input_dir.y -= 1.0
	if Input.is_key_pressed(KEY_S): input_dir.y += 1.0
	if Input.is_key_pressed(KEY_A): input_dir.x -= 1.0
	if Input.is_key_pressed(KEY_D): input_dir.x += 1.0

	var direction = (transform.basis * Vector3(input_dir.x, 0, input_dir.y)).normalized()

	var current_speed = SPEED * (SPRINT_MULTIPLIER if Input.is_key_pressed(KEY_SHIFT) else 1.0)

	if direction:
		velocity.x = direction.x * current_speed
		velocity.z = direction.z * current_speed
	else:
		velocity.x = move_toward(velocity.x, 0, current_speed)
		velocity.z = move_toward(velocity.z, 0, current_speed)

	move_and_slide()

	# 贴地
	if global_position.y < ground_h + 1.7:
		global_position.y = ground_h + 1.7
		velocity.y = 0.0
