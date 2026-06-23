extends Node3D
## 地形渲染器 — 从 Rust TerrainChunk 查询高度，用 SurfaceTool 构建网格

@export var grid_size: int = 128         # 每边顶点数
@export var spacing: float = 2.0        # 顶点间距 (米)
@export var origin_x: float = -128.0    # 网格左下角 X
@export var origin_z: float = -128.0    # 网格左下角 Z

func _ready():
	# 找到 Rust GDExtension 节点
	var terrain = get_node_or_null("../TerrainChunk")
	if not terrain:
		push_error("TerrainRender: TerrainChunk node not found!")
		return

	# 检查 Rust 方法是否可用
	if not terrain.has_method("query_height"):
		push_error("TerrainRender: query_height method not found on TerrainChunk!")
		return

	print("TerrainRender: generating ", grid_size, "x", grid_size,
		  " mesh (spacing=", spacing, "m)")

	var st = SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)

	# 生成顶点
	for iz in range(grid_size):
		var wz = origin_z + iz * spacing
		for ix in range(grid_size):
			var wx = origin_x + ix * spacing
			var h = terrain.query_height(wx, wz)
			var mat_idx = terrain.query_material(wx, wz)

			# 顶点位置
			st.set_normal(Vector3.UP)  # 临时——generate_normals 覆盖
			st.set_color(material_color(mat_idx, h))
			st.add_vertex(Vector3(wx, h, wz))

	# 生成索引 (每 quad 两个三角形)
	for iz in range(grid_size - 1):
		for ix in range(grid_size - 1):
			var tl = iz * grid_size + ix
			var tr = iz * grid_size + ix + 1
			var bl = (iz + 1) * grid_size + ix
			var br = (iz + 1) * grid_size + ix + 1
			st.add_index(tl)
			st.add_index(bl)
			st.add_index(tr)
			st.add_index(tr)
			st.add_index(bl)
			st.add_index(br)

	# 生成法线 + 提交网格
	st.generate_normals()

	var mesh_instance = MeshInstance3D.new()
	mesh_instance.name = "GeneratedTerrain"
	st.commit(mesh_instance)

	add_child(mesh_instance)
	print("TerrainRender: mesh ready — ", grid_size * grid_size, " vertices")


## 根据材质索引和高度返回颜色
func material_color(mat_idx: int, height: float) -> Color:
	match mat_idx:
		0:  # Grass
			if height > 100.0:
				return Color(0.2, 0.55, 0.2)
			return Color(0.3, 0.65, 0.25)
		1:  # Sand
			return Color(0.76, 0.7, 0.5)
		2:  # Rock
			return Color(0.45, 0.42, 0.38)
		3:  # Stone
			return Color(0.35, 0.35, 0.35)
		4:  # Wood
			return Color(0.4, 0.3, 0.2)
		5:  # Metal
			return Color(0.5, 0.5, 0.5)
		6:  # Water
			return Color(0.1, 0.3, 0.8)
		7:  # Ice
			return Color(0.9, 0.95, 1.0)
		8:  # Mud
			return Color(0.4, 0.3, 0.2)
		9:  # Snow
			return Color(0.95, 0.95, 0.95)
		10: # Gravel
			return Color(0.5, 0.45, 0.4)
		_:
			return Color(0.4, 0.5, 0.3)
