# 002-围绕 Godot 的规模化方案：用对工具而非换工具

> 核心论点：Godot 已经内置了规模化渲染海量实体的工具。问题不在于 Godot 不够强——在于我们一直在用"少量精细实体"的方式来使用它。

---

## 一、MultiMesh：1000 个 NPC 一个 Draw Call

### 1.1 原理

`MultiMeshInstance3D` 是 Godot 内置的 GPU 实例化工具。它用**一次 draw call** 渲染成千上万个相同的 mesh（或通过 `MultiMesh.buffer` 定制每个实例）。

```
传统方式（不可规模化）:
  SceneTree
    ├── NPC_001 (Node3D + Skeleton3D + MeshInstance3D)
    ├── NPC_002 (...)
    └── NPC_999 (...)
  → 1000 次 draw call + 1000 个 Node 的 _process 回调

MultiMesh 方式（可规模化）:
  SceneTree
    └── MultiMeshInstance3D (单个 Node)
        └── MultiMesh: 1000 个实例
  → 1 次 draw call + 1 个 Node 的 _process 回调
```

### 1.2 Per-Instance 自定义数据

Godot 4.x 的 `MultiMesh.buffer` 允许为每个实例存储额外的浮点数据：

```gdscript
# 设置每个实例的自定义数据（在 shader 中通过 INSTANCE_CUSTOM 访问）
multimesh.buffer = packed_float_array
# 每实例 4 个 float（Godot 4.2+ 支持更多通道）
# 通道 0-3: animation_state, animation_time, emotion_blend, lod_level
```

在自定义 shader 中：

```glsl
// Godot shading language
shader_type spatial;
render_mode instance_custom;

// 从 MultiMesh.buffer 读取 per-instance 数据
instance uniform vec4 custom_data;

void vertex() {
    // custom_data.x = animation_state (0=idle, 1=walk, 2=run...)
    // custom_data.y = animation_time
    // custom_data.z = emotion_blend
    // custom_data.w = lod_level
    
    // 在这里做 GPU skinning...
}
```

### 1.3 限制与对策

| 限制 | 对策 |
|------|------|
| `MultiMesh.buffer` 只能传递少量 float（默认每实例 1 个 vec4） | Godot 4.3+ 可以用 texture buffer 或 storage buffer 传递更多数据 |
| 所有实例共享同一个 mesh | 使用 **texture array** 存储不同体型/服装的 mesh 变体，在 shader 中采样 |
| 不支持骨骼动画（MultiMesh 实例没有 Skeleton） | **在 vertex shader 中做 GPU skinning**——骨骼矩阵存储在 texture 或 storage buffer 中 |
| 不支持 LOD 切换（单个 MultiMesh 只有一个 mesh） | 创建 3 个 MultiMesh（LOD0/LOD1/LOD2），根据距离在它们之间分配 NPC |

---

## 二、GPU Skinning：告别 1000 个 AnimationTree

### 2.1 原理

将骨骼动画的蒙皮计算从 CPU（Godot AnimationTree）移到 GPU（自定义 vertex shader）。

```
传统 CPU Skinning:
  每个 Skeleton3D 计算骨骼矩阵（CPU）
    → 每个 MeshInstance3D 上传矩阵给 GPU
      → GPU 在 vertex shader 中蒙皮
  问题：1000 个 Skeleton3D 各自独立计算 → CPU 瓶颈

GPU Skinning:
  所有 NPC 的骨骼矩阵预先计算好，存储在一个大的 texture 中
    → vertex shader 从 texture 中查找当前 NPC 的骨骼矩阵
      → 在 GPU 上并行蒙皮所有顶点
  结果：CPU 只做数据上传，GPU 做所有蒙皮计算
```

### 2.2 实现方案

**方案 A：Rust 侧计算骨骼矩阵 → Texture Buffer → Godot Shader**

```
Rust 模拟核心:
  1. 遍历所有 L1 NPC 的动画状态
  2. 根据动画状态更新动画时间
  3. 计算骨骼矩阵（CPU 侧，Rust 编译优化后极快）
  4. 将所有矩阵打包为一个大的 byte array
  5. 通过 GDExtension 传递这个 byte array

Godot 侧:
  1. 接收 byte array
  2. 上传到 TextureBuffer 或 StorageBuffer
  3. Custom shader 在 vertex() 中根据 gl_InstanceID 索引正确的矩阵
  4. 对顶点做蒙皮变换
```

**方案 B：纯 Godot GPU Compute → Texture Buffer → Shader**

```
Godot 侧:
  1. 用 RenderingDevice 运行一个 compute shader
  2. Compute shader 输入：动画数据（关键帧）、每个 NPC 的动画状态
  3. Compute shader 输出：所有 NPC 的骨骼矩阵（写入 texture）
  4. MultiMesh 的 vertex shader 读取这个 texture
```

### 2.3 方案 A vs B

| | 方案 A (Rust CPU) | 方案 B (Godot GPU) |
|---|---|---|
| 性能 | 1000 NPC × 50 bones = 50000 矩阵 × 0.001ms = 0.05ms | 1 个 compute dispatch = ~0.1-0.5ms |
| 实现复杂度 | 中（Rust 侧需要动画数据处理） | 高（需要写 compute shader + 理解 RenderingDevice） |
| 调试难度 | 低（Rust 侧 CPU 代码可直接 debug） | 高（GPU compute shader 调试工具有限） |
| Godot 依赖 | 最少 | 依赖 RenderingDevice API |
| 推荐度 | ⭐⭐⭐⭐ 务实首选 | ⭐⭐⭐ 性能更优但更复杂 |

**推荐方案 A**。原因是：
1. Rust 侧已经在处理 NPC 数据——动画状态是 NPC 数据的一部分
2. CPU 批量计算 50000 个骨骼矩阵在 Rust 编译优化下极快（远不到 1ms）
3. GDExtension 传递一个大的 byte array（50000 × 64 字节 = 3.2MB）是 GDExtension 的强项——大块数据传输的边际成本极低
4. Godot 侧只需把接收到的矩阵上传到 texture，shader 直接采样——逻辑简单

---

## 三、场景树轻量化：不让 Node 成为瓶颈

### 3.1 问题回顾

缺陷三：Godot 的 SceneTree 在数千个活跃 Node 时会降速——因为每个 Node 的 `_process`/`_physics_process` 需要被引擎逐一调度。

### 3.2 解决方案：单节点管理海量 NPC

```
原方案（糟糕）:
  SceneTree
    ├── NpcManager (Node)
    │   ├── NPC_001 (CharacterBody3D)  ← 每个 NPC 一个 Node
    │   ├── NPC_002 (CharacterBody3D)  ← 每个有自己的 _process
    │   └── ... × 1000

修订方案（正确）:
  SceneTree
    ├── NpcRenderer (Node3D)            ← 只有 1 个 Node
    │   ├── MultiMeshInstance3D (LOD0)  ← 近距离 NPC
    │   ├── MultiMeshInstance3D (LOD1)  ← 中距离 NPC
    │   └── MultiMeshInstance3D (LOD2)  ← 远距离 NPC
    ├── NpcPhysicsManager (Node)        ← 处理 NPC 碰撞（如果需要）
    │   └── 使用 PhysicsServer API 而非 CharacterBody3D 节点
    └── Player (CharacterBody3D)        ← 玩家还是正常 Node
```

### 3.3 不创建 Node 的物理

Godot 允许不创建 `CharacterBody3D` 节点就直接使用物理 API：

```gdscript
# 使用 PhysicsServer3D 直接创建物理体——不需要 Node
var body_rid = PhysicsServer3D.body_create()
PhysicsServer3D.body_set_space(body_rid, get_world_3d().space)
PhysicsServer3D.body_set_mode(body_rid, PhysicsServer3D.BODY_MODE_KINEMATIC)
PhysicsServer3D.body_set_state(body_rid, PhysicsServer3D.BODY_STATE_TRANSFORM, transform)

# 在 _physics_process 中批量更新
for npc in l1_npcs:
    var rid = npc.physics_rid
    var new_transform = compute_new_position(npc)
    PhysicsServer3D.body_set_state(rid, PhysicsServer3D.BODY_STATE_TRANSFORM, new_transform)
```

这样 1000 个 NPC 的碰撞检测在 Godot 的物理空间中运行——而不需要 1000 个场景节点。

### 3.4 音乐/音效的类似处理

Godot 允许直接使用 `AudioServer` API 而不创建 `AudioStreamPlayer3D` 节点：

```gdscript
# 不需要 1000 个 AudioStreamPlayer3D 节点
var playback = AudioServer.playback_start(stream, volume_db, pitch_scale, position)
```

---

## 四、体素地形：继续用 Voxel Tools，但换一种方式

### 4.1 Voxel Tools 的真正瓶颈

Voxel Tools 的性能瓶颈不在于它的体素算法——而在于它将每个 chunk 创建为一个 `VoxelTerrain` 节点并插入场景树。对于 16×16 区块的初始可玩区域，每个 32×32 chunk = 256 个 chunk 节点。虽然很多，但**16×16 区块 = 4096 个 chunk**——这就是 4096 个 Node。

### 4.2 绕过瓶颈：自定义 VoxelStream

Voxel Tools 支持自定义 `VoxelStream`——你可以自己控制哪些 chunk 被加载、它们的生成优先级、以及它们如何在内存中管理。

关键策略：
1. **限制活跃 chunk 数**：玩家周围 50 个 chunk 保持活跃，其他 chunk 的数据保留在内存中但不在场景树中
2. **异步生成**：chunk 生成（噪声→密度场→Transvoxel→Mesh）全部在 Rust 侧的后台线程中完成
3. **Rust 生成 → Godot 渲染**：Rust 生成体素密度场和 Transvoxel 结果→ 传给 Godot 的 `ArrayMesh` 构建
4. **碰撞在 Rust 侧**：地形碰撞（NPC 导航需要）在 Rust 侧用 Rapier 的 heightfield 或 trimesh 碰撞体

### 4.3 长期看：还是自建 Transvoxel

即使保持 Godot 渲染，Transvoxel 的等值面提取仍然应该在 Rust 侧（因为性能）——但**mesh 构建和渲染在 Godot 侧**（因为方便）。

```
Rust 侧：噪声→密度场→Transvoxel 等值面提取→三角形数组
GDExtension：三角形数组（vertex positions + normals + indices）→ byte array
Godot 侧：byte array → ArrayMesh → MeshInstance3D（每个 chunk 一个 MeshInstance3D，共 ~50 个活跃 chunk）
```

50 个 `MeshInstance3D` 的 Node 开销微不足道——这不是瓶颈。

---

## 五、C++ GDExtension：消除序列化边界

### 5.1 Rust GDExtension 的隐藏成本

Rust `godot-rust` crate 在 Rust struct 和 Godot Variant 之间有一个序列化层。虽然 bincode 很快，但它仍然是额外的拷贝。

### 5.2 C++ GDExtension 的优势

Godot 是用 C++ 写的。`gdextension` C API 的原生绑定就是 C++。使用 C++ GDExtension：
- 可以直接使用 `godot::Vector3`、`godot::Dictionary`、`godot::Array`——与 Godot 内部的类型完全相同
- 不需要序列化——Rust struct 和 Godot Variant 之间的转换消失了
- 可以直接分配 Godot 的 `PackedFloat32Array`、`PackedByteArray`——这些类型的内存布局与 C++ 原生数组兼容

### 5.3 代价：放弃 Rust 的内存安全

C++ 没有 borrow checker。在 NPC 系统的复杂度下，use-after-free、data race、悬垂指针的风险是真实的。但可以通过以下方式缓解：
- 将最复杂的逻辑（GOAP、情绪引擎）封装在严格限定副作用的模块中
- 使用 `std::unique_ptr`/`std::shared_ptr` 管理所有权
- 使用 AddressSanitizer + ThreadSanitizer 在 CI 中检测内存和并发错误
- 遵循 RAII 模式

### 5.4 务实判断

如果开发者对 C++ 的熟悉程度 >= 对 Rust 的熟悉程度，C++ GDExtension 可能是最务实的方案——它直接消除了 Rust↔Godot 之间的所有序列化开销，同时保持了与 Godot 引擎内部的天然兼容性。

如果开发者已经在学习 Rust 并且享受 borrow checker 带来的安全感，继续 Rust。两种语言在 GDExtension 中的性能是同级的。

---

## 六、方案总结：Godot 优先的规模化策略

| 瓶颈 | 旧方案（不可规模化） | Godot 优先方案 |
|------|-------------------|--------------|
| NPC 渲染 | 1000 个 Skeleton3D + MeshInstance3D 节点 | **MultiMeshInstance3D**（1 个 Node，1 次 draw call） |
| NPC 动画 | 1000 个 AnimationTree（CPU） | **Rust CPU 批量骨骼矩阵 + GPU skinning shader** |
| NPC 物理 | 1000 个 CharacterBody3D 节点 | **PhysicsServer3D API**（无节点，RID 直接操作） |
| NPC 音频 | 1000 个 AudioStreamPlayer3D 节点 | **AudioServer API**（无节点，直接播放） |
| 体素 chunk | 4096 个 VoxelTerrain 节点 | **~50 个 MeshInstance3D**（仅活跃 chunk）+ Rust 侧生成 |
| 数据传递 | Rust struct → bincode → Variant | **PackedByteArray** 零拷贝（或 C++ GDExtension 直接使用 Godot 类型） |
| 场景树规模 | 10000+ 个活跃 Node | **< 100 个活跃 Node** |

**这六个策略共同使 Godot 能够支撑 WoWorld 的规模——而不需要把渲染器搬到 Rust。**
