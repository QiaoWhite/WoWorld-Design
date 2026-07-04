shader_type spatial;
render_mode cull_disabled;

// Camera-relative floating origin — 与 terrain.gdshader 一致
uniform vec3 camera_pos = vec3(0.0);

void vertex() {
	// MODEL_MATRIX 自动提供各 VoxelChunk 的世界位置——无需 per-chunk uniform
	vec3 world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	vec3 rel = world_pos - camera_pos;
	POSITION = PROJECTION_MATRIX * vec4(mat3(VIEW_MATRIX) * rel, 1.0);
	NORMAL = mat3(VIEW_MATRIX) * NORMAL;
}

void fragment() {
	ALBEDO = COLOR.rgb;
}
