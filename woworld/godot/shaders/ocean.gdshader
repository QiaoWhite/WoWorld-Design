shader_type spatial;
render_mode blend_mix, depth_draw_opaque, cull_disabled;

// Gerstner 波参数
uniform float time_scale : hint_range(0.1, 2.0) = 0.5;
uniform vec3 wave_a = vec3(1.0, 0.3, 0.5);  // direction_xz, steepness, wavelength
uniform vec3 wave_b = vec3(0.7, 0.2, 0.8);
uniform vec3 wave_c = vec3(0.5, 0.15, 1.2);
uniform vec3 wave_d = vec3(0.3, 0.1, 1.8);
uniform vec3 wave_e = vec3(0.9, 0.08, 0.4);
uniform vec3 wave_f = vec3(0.4, 0.05, 2.5);

uniform vec4 deep_color : source_color = vec4(0.02, 0.08, 0.25, 1.0);
uniform vec4 shallow_color : source_color = vec4(0.05, 0.25, 0.45, 1.0);
uniform vec4 foam_color : source_color = vec4(0.9, 0.95, 1.0, 1.0);
uniform float depth_blend : hint_range(0.0, 100.0) = 20.0;

// Gerstner 波函数
vec3 gerstner_wave(vec4 wave_params, vec3 pos, float time, inout vec3 tangent, inout vec3 binormal) {
	float dir_x = wave_params.x;
	float dir_z = wave_params.y;
	float steepness = wave_params.z;
	float wavelength = wave_params.w;

	float k = 6.28318530718 / wavelength;
	float c = sqrt(9.8 / k);
	float d = normalize(vec2(dir_x, dir_z));
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

	pos += gerstner_wave(vec4(wave_a, 0.0), pos, time, tangent, binormal);
	pos += gerstner_wave(vec4(wave_b, 0.0), pos, time, tangent, binormal);
	pos += gerstner_wave(vec4(wave_c, 0.0), pos, time, tangent, binormal);
	pos += gerstner_wave(vec4(wave_d, 0.0), pos, time, tangent, binormal);
	pos += gerstner_wave(vec4(wave_e, 0.0), pos, time, tangent, binormal);
	pos += gerstner_wave(vec4(wave_f, 0.0), pos, time, tangent, binormal);

	vec3 normal = normalize(cross(binormal, tangent));
	NORMAL = normal;
	VERTEX = pos;
}

void fragment() {
	float depth_factor = abs(VERTEX.y) / depth_blend;
	depth_factor = clamp(depth_factor, 0.0, 1.0);
	ALBEDO = mix(shallow_color.rgb, deep_color.rgb, depth_factor);
	ROUGHNESS = 0.15;
	METALLIC = 0.1;
	SPECULAR = 0.5;
}
