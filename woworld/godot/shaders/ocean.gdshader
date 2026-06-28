shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_disabled;

// Gerstner 波参数
uniform float time_scale : hint_range(0.1, 2.0) = 0.5;
// (dir_x, dir_z, steepness, wavelength)
uniform vec4 wave_a = vec4(1.0, 0.0, 0.3, 4.0);
uniform vec4 wave_b = vec4(0.0, 1.0, 0.2, 6.0);
uniform vec4 wave_c = vec4(0.7, 0.5, 0.15, 8.0);
uniform vec4 wave_d = vec4(-0.3, 0.9, 0.1, 12.0);
uniform vec4 wave_e = vec4(0.9, -0.3, 0.08, 3.0);
uniform vec4 wave_f = vec4(-0.5, -0.4, 0.05, 18.0);

uniform vec4 deep_color : source_color = vec4(0.02, 0.08, 0.25, 1.0);
uniform vec4 shallow_color : source_color = vec4(0.05, 0.45, 0.55, 1.0);
uniform vec4 foam_color : source_color = vec4(0.9, 0.95, 1.0, 1.0);
uniform float depth_blend : hint_range(0.0, 500.0) = 200.0;
// 从 Rust 侧每帧更新——玩家脚下的海深（米）
uniform float water_depth : hint_range(0.0, 500.0) = 50.0;

// Gerstner 波函数
vec3 gerstner_wave(vec4 wave_params, vec3 pos, float time, inout vec3 tangent, inout vec3 binormal) {
	float dir_x = wave_params.x;
	float dir_z = wave_params.y;
	float steepness = wave_params.z;
	float wavelength = wave_params.w;

	float k = 6.28318530718 / wavelength;
	float c = sqrt(9.8 / k);
	vec2 d = normalize(vec2(dir_x, dir_z));
	float f = k * (dot(d, pos.xz) - c * time);
	float a = steepness / k;

	tangent += vec3(
		-d.x * d.x * (steepness * sin(f)),
		d.x * (steepness * cos(f)),
		-d.x * d.y * (steepness * sin(f))
	);
	binormal += vec3(
		-d.x * d.y * (steepness * sin(f)),
		d.y * (steepness * cos(f)),
		-d.y * d.y * (steepness * sin(f))
	);

	return vec3(
		d.x * (a * cos(f)),
		a * sin(f),
		d.y * (a * cos(f))
	);
}

void vertex() {
	float time = TIME * time_scale;
	vec3 pos = VERTEX;
	vec3 tangent = vec3(1.0, 0.0, 0.0);
	vec3 binormal = vec3(0.0, 0.0, 1.0);

	pos += gerstner_wave(wave_a, pos, time, tangent, binormal);
	pos += gerstner_wave(wave_b, pos, time, tangent, binormal);
	pos += gerstner_wave(wave_c, pos, time, tangent, binormal);
	pos += gerstner_wave(wave_d, pos, time, tangent, binormal);
	pos += gerstner_wave(wave_e, pos, time, tangent, binormal);
	pos += gerstner_wave(wave_f, pos, time, tangent, binormal);

	vec3 normal = normalize(cross(binormal, tangent));
	NORMAL = normal;
	VERTEX = pos;
}

void fragment() {
	// 深度驱动颜色：浅水绿松石 → 深海暗蓝
	float depth_factor = clamp(water_depth / depth_blend, 0.0, 1.0);
	ALBEDO = mix(shallow_color.rgb, deep_color.rgb, depth_factor);

	// 海岸白沫：水深 < 5m 时出现
	float foam = 1.0 - smoothstep(0.5, 5.0, water_depth);
	ALBEDO = mix(ALBEDO, foam_color.rgb, foam * 0.35);

	ROUGHNESS = 0.15;
	METALLIC = 0.1;
	SPECULAR = 0.5;
}
