shader_type spatial;
render_mode cull_disabled; // FIXME: 部分三角形绕序不一致，诊断后恢复 cull_back

// Camera-relative floating origin — 与 terrain.gdshader 一致
uniform vec3 camera_pos = vec3(0.0);

void vertex() {
	// MODEL_MATRIX 提供各 VoxelChunk 的世界位置（Node3D.position 为纯平移 → 数学等价于 VERTEX+node_pos）
	vec3 world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	vec3 rel = world_pos - camera_pos;
	POSITION = PROJECTION_MATRIX * vec4(mat3(VIEW_MATRIX) * rel, 1.0);
	// 法线: 模型空间 → 世界空间 (MODEL_MATRIX 纯平移时等价于直传) → 视图空间
	NORMAL = mat3(VIEW_MATRIX) * mat3(MODEL_MATRIX) * NORMAL;
}

void fragment() {
	ALBEDO = COLOR.rgb;
}
