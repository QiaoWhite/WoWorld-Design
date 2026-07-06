shader_type spatial;
render_mode cull_back, shadows_disabled;

uniform vec3 camera_pos = vec3(0.0);

varying vec3 world_pos_frag;

void vertex() {
	vec3 world_pos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	world_pos_frag = world_pos;
	vec3 rel = world_pos - camera_pos;
	POSITION = PROJECTION_MATRIX * vec4(mat3(VIEW_MATRIX) * rel, 1.0);
	COLOR = COLOR;
}

void fragment() {
	// Per-fragment geometric normal from screen-space derivatives
	// — matches terrain shader's per-fragment approach
	vec3 dx = dFdx(world_pos_frag);
	vec3 dy = dFdy(world_pos_frag);
	vec3 world_n = normalize(cross(dx, dy));
	if (world_n.y < 0.0) {
		world_n = -world_n;
	}
	NORMAL = mat3(VIEW_MATRIX) * world_n;

	ROUGHNESS = 1.0;
	METALLIC = 0.0;
	SPECULAR = 0.0;
	ALBEDO = COLOR.rgb;
}
