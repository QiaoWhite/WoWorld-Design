# Bug 索引

> **🐛 LLM 调试协议**：遇到 bug/异常行为 → 第一步 `grep "症状关键词"` 此文件 → 命中则打开对应条目。
> 格式参考 [[TEMPLATE]]。代码中 `// BUG:XX-NNN` 标记可反向查找对应条目。

## 模块缩写

| 缩写 | 模块 | 目录 |
|------|------|------|
| GLOBAL | 全局（跨模块） | `全局/` |
| WG | 世界生成 (WorldGen) | `世界生成/` |
| ECS | ECS | `ECS/` |
| GD | Godot 桥接 | `Godot桥接/` |
| RENDER | 渲染 | `渲染/` |
| TOOL | 工具链 | `工具链/` |
| SAVE | 存档系统 | `存档系统/` |
| ATMO | 大气系统 | `大气系统/` |
| LIFE | 生命系统 | `生命系统/` |
| WTHR | 天气系统 | `天气系统/` |

## 索引

| ID | grep_keys | 症状一行 | 模块 | 类型 | 状态 | 文件 |
|----|-----------|----------|------|------|------|------|
| WG-001 | crack,fp32,seam,裂缝,水密,watertight,LOD边界,面片缺失,透光,edge mismatch | Clipmap LOD边界 fp32 精度裂缝 | 世界生成 | 🟠架构限制 | ⚠️已知残留 | [WG-001](世界生成/WG-001-clipmap-fp32-edge-cracks.md) |
| WG-002 | stride,swap,y-z,狼牙棒,spike,transvoxel,marching cubes,密度数组,索引错误,density array | Transvoxel y-z stride swap 双向抵消 | 世界生成 | 🟡反直觉陷阱 | ✅已修复 | [WG-002](世界生成/WG-002-transvoxel-yz-stride-swap.md) |
| WG-003 | perlin,原点,origin,ocean,海洋,全种子,sea threshold,grid point | Perlin(0,0)=0 全种子原点永远海洋 | 世界生成 | 🟡反直觉陷阱 | ✅已修复 | [WG-003](世界生成/WG-003-perlin-origin-always-ocean.md) |
| TOOL-001 | cargo test,build,dll,不更新,cdylib,动态库,godot加载,extension | cargo test ≠ cargo build .dll 不更新 | 工具链 | 🟡反直觉陷阱 | ✅已修复 | [TOOL-001](工具链/TOOL-001-cargo-test-not-build-dll.md) |
| GD-001 | floating origin,浮点原点,camera relative,vertex shader,AABB,double offset,frustum culling | Floating Origin v1-v5 实现全失败 | Godot桥接 | 🟡反直觉陷阱 | ✅已修复 | [GD-001](Godot桥接/GD-001-floating-origin-v1-v5-failures.md) |
| GD-002 | camera_pos,浮点原点,floating origin,透明,transparent,看穿,see-through,背面,backface,cull_back,LOD0,VoxelChunk,眼高,eye height,body vs camera,相机高度,地形偏高 | 浮点原点 camera_pos 用 body 非相机位置 → 低眼高看穿地形 | Godot桥接 | 🟡反直觉陷阱 | ✅已修复 | [GD-002](Godot桥接/GD-002-camera-pos-uses-body-not-camera.md) |
| RENDER-001 | pbr,normal,法线,插值漂移,perspective interpolation,大三角形,dot(N,L),clipmap | PBR 法线透视插值漂移（大三角形） | 渲染 | 🔴回归 | ✅已修复 | [RENDER-001](渲染/RENDER-001-pbr-normal-perspective-drift.md) |
| RENDER-002 | LOD0,VoxelChunk,光照异常,正方形区域,全黑,锯齿亮斑,shadow,shadow acne,阴影贴图,shadows_disabled,自阴影 | VoxelChunk 阴影贴图深度不连续 → LOD0 视角依赖光照异常 | 渲染 | 🔴回归 | ✅已修复 | [RENDER-002](渲染/RENDER-002-voxel-shadow-depth-discontinuity.md) |
| TOOL-002 | canvas,obsidian,二进制,损坏,corrupt,json,文本编辑,画布 | .canvas 文件被 LLM 文本编辑损坏 | 工具链 | 🟡反直觉陷阱 | ✅已修复 | [TOOL-002](工具链/TOOL-002-canvas-binary-corruption.md) |
| ECS-001 | greeting,问候,打招呼,speech_bubble,ActionIntent,veto,否决,SeekSafety,安全需求,social_total,needs累积,全村沉默,早期正常后突停 | SeekSafety 否决静默掐断全村问候（needs 累积后全 SeekSafety） | ECS | 🟡反直觉陷阱 | ✅已修复 | [ECS-001](ECS/ECS-001-seeksafety-veto-silences-greetings.md) |
| GD-003 | set_surface_override_material,material_override,材质不生效,颜色不对,albedo,渲染颜色,surface,material,wrong color | `set_surface_override_material` 材质设值但不渲染（GPU输出≠CPU属性） | Godot桥接 | 🟡反直觉陷阱 | ✅已修复 | [GD-003](Godot桥接/GD-003-set-surface-override-material-not-rendered.md) |
| GD-004 | entity bits,overlap,重叠,复用,clear_nodes,hecs rebuild,load,color inherit,颜色继承,节点复用 | hecs World 重建后 entity bits 重叠 → EntityRenderer 复用旧节点 | Godot桥接 | 🟡反直觉陷阱 | ✅已修复 | [GD-004](Godot桥接/GD-004-hecs-entity-bits-overlap-after-rebuild.md) |
