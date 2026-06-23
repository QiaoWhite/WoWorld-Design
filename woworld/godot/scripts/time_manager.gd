extends Node
## 昼夜循环：从引擎运行时间直接计算太阳位置 + 天空色。

const SECONDS_PER_DAY: float = 60.0     # 60 秒一天（测试用）
const DAYS_PER_SEASON: float = 30.0     # 30 天换季（测试用，正式=120）
var _first_frame: bool = true
const SUN_RADIUS: float = 500.0         # 太阳轨道半径

# ── 关键帧色板 ──
const SKY_DAWN   = Color(0.7, 0.4, 0.3)
const SKY_NOON   = Color(0.3, 0.5, 0.9)
const SKY_DUSK   = Color(0.8, 0.35, 0.2)
const SKY_NIGHT  = Color(0.05, 0.05, 0.15)
const HOR_DAWN   = Color(0.9, 0.6, 0.3)
const HOR_NOON   = Color(0.6, 0.7, 0.9)
const HOR_DUSK   = Color(0.95, 0.5, 0.15)
const HOR_NIGHT  = Color(0.02, 0.02, 0.06)
const AMB_DAWN   = Color(0.4, 0.3, 0.25)
const AMB_NOON   = Color(0.6, 0.6, 0.65)
const AMB_DUSK   = Color(0.45, 0.3, 0.2)
const AMB_NIGHT  = Color(0.04, 0.04, 0.08)

func _process(_delta):
	if _first_frame:
		print("[TimeManager] _process running OK, sun will move over ", SECONDS_PER_DAY, "s cycle")
		_first_frame = false

	var elapsed = float(Time.get_ticks_msec()) / 1000.0
	var dp = fmod(elapsed / SECONDS_PER_DAY + 0.25, 1.0)

	# ── 季节偏移 ──
	var season = fmod(elapsed / (SECONDS_PER_DAY * DAYS_PER_SEASON), 1.0)
	var season_shift = sin(season * TAU) * 0.4          # ±23° 日出方位偏移
	var max_elev = PI / 2.0 * (0.6 + 0.4 * sin(season * TAU))  # 夏至高弧 冬至低弧

	# ── 太阳轨道 ──
	# 方位角: 东(π/2) → 南(π) → 西(3π/2)，线性 + 季节偏移
	var azim = PI / 2.0 + (dp - 0.25) * TAU + season_shift
	# 高度角: 正弦弧线 × 季节振幅
	var elev = asin(sin((dp - 0.25) * TAU) * max_elev)

	var sun_pos = Vector3(
		cos(elev) * sin(azim) * SUN_RADIUS,
		sin(elev) * SUN_RADIUS,
		cos(elev) * cos(azim) * SUN_RADIUS
	)

	var sun = get_node_or_null("../Sun")
	if sun:
		sun.position = sun_pos
		var up = Vector3.UP
		if abs(sun_pos.normalized().dot(up)) > 0.99:
			up = Vector3.FORWARD
		sun.look_at(Vector3.ZERO, up)

	# ── 天空色 lerp ──
	var env = get_node_or_null("../WorldEnvironment")
	if not (env and env.environment):
		return

	var sky_mat = null
	if env.environment.sky:
		sky_mat = env.environment.sky.sky_material

	if sky_mat:
		sky_mat.sky_top_color = _keyframe_lerp(dp, SKY_DAWN, SKY_NOON, SKY_DUSK, SKY_NIGHT)
		sky_mat.sky_horizon_color = _keyframe_lerp(dp, HOR_DAWN, HOR_NOON, HOR_DUSK, HOR_NIGHT)

	env.environment.ambient_light_color = _keyframe_lerp(dp, AMB_DAWN, AMB_NOON, AMB_DUSK, AMB_NIGHT)


## 色板 lerp（带过渡窗收紧 + 夜间/白天平台）
##   夜间平台 0.0-0.20 | 夜→晨 0.20-0.25 | 晨→午 0.25-0.35
##   白天平台 0.35-0.65 | 午→昏 0.65-0.75 | 昏→夜 0.75-0.80 | 回夜间平台
func _keyframe_lerp(dp: float, dawn: Color, noon: Color, dusk: Color, night: Color) -> Color:
	if dp < 0.20:
		return night  # 深夜间平台
	elif dp < 0.25:
		return night.lerp(dawn, (dp - 0.20) / 0.05)   # 夜→晨 急速过渡
	elif dp < 0.35:
		return dawn.lerp(noon, (dp - 0.25) / 0.10)    # 晨→午 舒展过渡
	elif dp < 0.65:
		return noon  # 白天平台
	elif dp < 0.75:
		return noon.lerp(dusk, (dp - 0.65) / 0.10)    # 午→昏 舒展过渡
	elif dp < 0.80:
		return dusk.lerp(night, (dp - 0.75) / 0.05)   # 昏→夜 急速过渡
	else:
		return night  # 深夜间平台
