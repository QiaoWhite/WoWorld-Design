shader_type spatial;
render_mode cull_back, shadows_disabled;

uniform sampler2D heightmap : filter_linear, repeat_disable;
uniform sampler2D material_map : filter_linear, repeat_disable;
uniform vec3 camera_pos = vec3(0.0);
uniform vec3 node_pos = vec3(0.0);
uniform vec2 grid_origin = vec2(0.0);
uniform float hm_extent = 256.0;

uniform sampler2D fine_heightmap : filter_linear, repeat_disable;
uniform vec2 fine_grid_origin = vec2(0.0);
uniform float fine_hm_extent = 256.0;
uniform float inner_bound = -1.0;
uniform float blend_zone = 1.0;

uniform sampler2D coarse_heightmap : filter_linear, repeat_disable;
uniform vec2 coarse_grid_origin = vec2(0.0);
uniform float coarse_hm_extent = 256.0;
uniform float outer_bound = 99999.0;
uniform float outer_blend_zone = 1.0;

varying vec2 frag_uv;

void vertex() {
	vec3 world_pos = VERTEX + node_pos;
	float cheb = max(abs(VERTEX.x), abs(VERTEX.z));

	vec2 uv = (world_pos.xz - grid_origin) / hm_extent;
	frag_uv = uv;
	float current_h = texture(heightmap, uv).r;

	vec2 fine_uv = (world_pos.xz - fine_grid_origin) / fine_hm_extent;
	float fine_h = texture(fine_heightmap, fine_uv).r;
	float t_inner = smoothstep(inner_bound, inner_bound + blend_zone, cheb);
	float h = mix(fine_h, current_h, t_inner);

	vec2 coarse_uv = (world_pos.xz - coarse_grid_origin) / coarse_hm_extent;
	float coarse_h = texture(coarse_heightmap, coarse_uv).r;
	float t_outer = smoothstep(outer_bound - outer_blend_zone, outer_bound, cheb);
	h = mix(h, coarse_h, t_outer);
	world_pos.y = h;

	vec3 rel = world_pos - camera_pos;
	vec3 view_pos = mat3(VIEW_MATRIX) * rel;
	POSITION = PROJECTION_MATRIX * vec4(view_pos, 1.0);

	// NORMAL computed per-fragment from heightmap — see fragment()
	COLOR = texture(material_map, uv);
}

void fragment() {
	// Per-fragment normal from heightmap — eliminates vertex interpolation drift on large triangles
	float ts = hm_extent / float(textureSize(heightmap, 0).x);
	float h_c = texture(heightmap, frag_uv).r;
	float h_r = texture(heightmap, frag_uv + vec2(ts / hm_extent, 0.0)).r;
	float h_u = texture(heightmap, frag_uv + vec2(0.0, ts / hm_extent)).r;
	vec3 world_normal = normalize(vec3(-(h_r - h_c) / ts, 1.0, -(h_u - h_c) / ts));
	NORMAL = mat3(VIEW_MATRIX) * world_normal;

	ROUGHNESS = 1.0;
	METALLIC = 0.0;
	SPECULAR = 0.0;
	ALBEDO = COLOR.rgb;
}
