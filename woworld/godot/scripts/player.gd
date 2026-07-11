extends CharacterBody3D
## 玩家飞行/G 键/夺舍物理 (★ 007 收缩——相机逻辑已迁至 CameraRig)
## 用 Input.get_vector 配合 UI action，不依赖 Input Map 预配置
## INPUT TUNING: CharacterBody3D 物理参数，非世界模拟常量 (§14.1 合规)

const SPEED: float = 5.0
const SPRINT_MULTIPLIER: float = 3.0
const FLY_SPEED: float = 30.0
const FLY_FAST_MULTIPLIER: float = 3.0   # Shift 加速飞行 = 90 m/s
const JUMP_VELOCITY: float = 8.0
const GRAVITY: float = 9.8

var _flying: bool = false

func _input(event):
	# Sprint-059: 控制台开启时跳过所有玩家输入
	var driver = get_node_or_null("../WorldDriver")
	if driver and driver.has_method("is_console_open") and driver.is_console_open():
		return

	if event is InputEventKey and event.pressed:
		if event.keycode == KEY_G:
			_flying = not _flying
			# Sprint-063: 飞行 = Block A0 让权（Godot 节点权威旁路）；落地 = 交回 Block A0
			if driver and driver.has_method("set_block_a0_driving"):
				driver.set_block_a0_driving(not _flying)

func _physics_process(delta):
	# Sprint-059: 控制台开启时跳过所有玩家移动
	var driver = get_node_or_null("../WorldDriver")
	if driver and driver.has_method("is_console_open") and driver.is_console_open():
		return

	# Sprint-063: Block A0 行使渲染权威时（自由裸玩家 + 非飞行），
	#   Rust 角色控制器驱动位移，player.gd 退为纯相机骨架（鼠标环顾仍在 _input 处理）。
	#   夺舍 NPC / G 飞行时 is_block_a0_driving() 返回 false，走下方 legacy 物理。
	if driver and driver.has_method("is_block_a0_driving") and driver.is_block_a0_driving():
		return

	if _flying:
		_physics_fly(delta)
	else:
		_physics_walk(delta)

func _physics_fly(_delta):
	# WASD 水平移动
	var input_dir = Vector2.ZERO
	if Input.is_key_pressed(KEY_W): input_dir.y -= 1.0
	if Input.is_key_pressed(KEY_S): input_dir.y += 1.0
	if Input.is_key_pressed(KEY_A): input_dir.x -= 1.0
	if Input.is_key_pressed(KEY_D): input_dir.x += 1.0

	var direction = (transform.basis * Vector3(input_dir.x, 0, input_dir.y)).normalized()
	var fly_speed = FLY_SPEED * (FLY_FAST_MULTIPLIER if Input.is_key_pressed(KEY_SHIFT) else 1.0)

	if direction:
		velocity.x = direction.x * fly_speed
		velocity.z = direction.z * fly_speed
	else:
		velocity.x = move_toward(velocity.x, 0, fly_speed)
		velocity.z = move_toward(velocity.z, 0, fly_speed)

	# 垂直: Space 上升, Ctrl 下降
	velocity.y = 0.0
	if Input.is_key_pressed(KEY_SPACE):
		velocity.y = fly_speed
	if Input.is_key_pressed(KEY_CTRL):
		velocity.y = -fly_speed

	move_and_slide()

func _physics_walk(delta):
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
