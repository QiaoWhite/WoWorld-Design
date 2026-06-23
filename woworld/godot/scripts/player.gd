extends CharacterBody3D
## 玩家控制器 — WASD 移动 + 鼠标环顾 + Space 跳跃

@export var speed: float = 8.0
@export var jump_velocity: float = 5.0
@export var mouse_sensitivity: float = 0.003
@export var gravity: float = 20.0

var _mouse_captured: bool = false

func _ready():
	# 捕获鼠标
	Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
	_mouse_captured = true

func _input(event):
	if event is InputEventMouseMotion and _mouse_captured:
		# 水平旋转 (Y 轴) — 整个 Player 旋转
		rotate_y(-event.relative.x * mouse_sensitivity)
		# 垂直旋转 (X 轴) — 仅 Camera 旋转，限制角度
		var cam = $Camera3D
		cam.rotate_x(-event.relative.y * mouse_sensitivity)
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
	# 重力
	if not is_on_floor():
		velocity.y -= gravity * delta

	# 跳跃
	if Input.is_action_just_pressed("jump") and is_on_floor():
		velocity.y = jump_velocity

	# WASD 移动
	var input_dir = Input.get_vector("left", "right", "forward", "backward")
	var direction = (transform.basis * Vector3(input_dir.x, 0, input_dir.y)).normalized()

	if direction:
		velocity.x = direction.x * speed
		velocity.z = direction.z * speed
	else:
		velocity.x = move_toward(velocity.x, 0, speed)
		velocity.z = move_toward(velocity.z, 0, speed)

	move_and_slide()

	# 玩家 Y 跟随地形高度
	if is_on_floor():
		pass  # 未来: raycast 查询 HeightfieldTerrain
