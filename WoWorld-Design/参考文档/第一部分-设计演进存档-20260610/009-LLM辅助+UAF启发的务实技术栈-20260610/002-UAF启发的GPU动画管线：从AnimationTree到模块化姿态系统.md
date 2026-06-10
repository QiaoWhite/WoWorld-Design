# 002-UAF 启发的 GPU 动画管线：从 AnimationTree 到模块化姿态系统

> UE5.6 UAF（Unreal Animation Framework）的核心思想移植到 WoWorld 的技术语境中  
> 目标：设计一套不依赖特定引擎、可用 LLM 生成代码实现的模块化动画架构

---

## 〇、传统动画架构为何必死

```
Godot AnimationTree（传统）:
  每个 NPC 持有独立的 AnimationTree 实例
  → AnimationTree 内部运行一个状态机
  → 状态机决定当前播放哪个动画
  → BlendSpace 混合多个动画
  → 输出：骨骼的局部变换矩阵
  → 每个 AnimationTree 在 CPU 上独立计算

问题：
  1000 个 AnimationTree = 1000 个独立的状态机 + 1000 次骨骼矩阵计算
  全部在 CPU 上串行或有限并发执行
  → 即使优化到极致，这也是 O(n) CPU 负载
```

**UAF 的核心理念**：动画不是"每个角色播放各自的动画"——而是**"一个动画系统服务所有角色"**。

---

## 一、UAF 核心概念在 WoWorld 中的映射

### 1.1 Pose Driver（姿态驱动器）

**UAF 中**：动画状态不由硬编码的状态机决定——由**数据驱动**。速度、方向、加速度、情绪值……这些浮点数据直接映射到动画混合参数。

**WoWorld 映射**：
```
NPC 的情绪状态（pleasure, arousal, control）
NPC 的生理状态（fatigue, health）
NPC 的当前行动（walking, running, working, fighting）
NPC 的社交状态（talking, listening, arguing）

→ 这组浮点数值就是 Pose Driver 的输入
→ 不经过状态机，直接驱动动画混合
```

```rust
// 不是状态机——
// 是浮点数组 → 动画混合权重
struct PoseDriver {
    // 驱动参数（每帧从 NPC 状态计算）
    velocity: Vec3,          // 移动速度和方向
    fatigue: f32,            // 0=精力充沛 1=极度疲劳
    emotion_pleasure: f32,   // -1~1
    emotion_arousal: f32,    // 0~1
    is_talking: bool,
    is_holding_item: bool,
    holding_item_type: u8,   // 枚举：无/工具/武器/食物/...
    
    // 输出：混合权重（compute shader 计算）
    // blend_weights: [walk_normal, walk_tired, walk_happy, walk_sad, idle_normal, ...]
}
```

### 1.2 Motion Matching（运动匹配）

**UAF 中**：不预设"从走路到跑步的过渡动画"。而是从动画数据库中搜索与当前运动状态最匹配的帧。

**WoWorld 映射**：不需要完整的 Motion Matching（那需要大量动画数据）。但可以借鉴其**简化版——Pose Database**：

```
Pose Database（姿态数据库）:
  存储的不是"完整动画"——而是"关键姿态"
  
  每个姿态有标签：
    { action: "walk", fatigue_level: 0, emotion: "neutral" }
    { action: "walk", fatigue_level: 1, emotion: "sad" }
    { action: "idle", fatigue_level: 0, emotion: "happy" }
    ...
  
  运行时：
    根据 NPC 的当前参数 → 在数据库中找到最近的几个姿态 → 混合
```

这个 Pose Database 不需要手工制作——可以由 LLM 根据动画数据自动生成姿态标签和混合参数。

### 1.3 Modular Rig（模块化骨骼）

**UAF 中**：骨骼不是一整棵树。上半身、下半身、面部可以独立计算和混合。

**WoWorld 映射**：
```
NPC 骨骼拆分为独立模块：
  下半身模块（骨盆 + 腿）      → 由移动状态驱动
  上半身模块（脊椎 + 手臂）    → 由行动类型驱动 + 情绪影响
  头部模块（脖子 + 头）        → 由注意力目标驱动
  面部模块（表情骨骼）          → 由情绪状态驱动（LOD2+ 不需要）
  手部模块                     → 由持有的物品驱动
```

**好处**：
- 不需要为"边走路边挥手边说话边做悲伤表情"做一个专门的动画——每个模块独立混合
- 动画组合爆炸问题被模块化降解
- 远处的 NPC 可以只更新下半身（LOD 按模块降级）

### 1.4 GPU Compute Update（GPU 计算更新）

**UAF 中**：骨骼矩阵在 GPU 上批量计算。

**WoWorld 映射**：
```
每帧一次 compute dispatch：
  输入：
    - Pose Driver 参数（per NPC, 存储在 GPU buffer 中）
    - Pose Database（存储在 GPU texture 中）
    - Animation Time（per NPC, 存储在 GPU buffer 中）
  
  计算：
    - 根据 Pose Driver 参数 → 查找/混合姿态
    - 计算模块化骨骼的局部变换
    - 组合为最终骨骼矩阵
  
  输出：
    - 骨骼矩阵数组（per NPC, GPU buffer）
    - 直接用于 vertex shader 蒙皮
```

**关键数据流**：
```
模拟核心（CPU）                 GPU
─────────────                  ───
NPC 状态更新（Rust/C#/GDScript）
  → PoseDriver 参数打包
    → GPU Buffer 上传 ──────→ Compute Shader
                                → 姿态混合 + 骨骼矩阵计算
                                → Vertex Shader 蒙皮
                                → 像素着色
```

---

## 二、具体实现方案（引擎无关的架构描述）

### 2.1 Pose Database 的数据格式

```rust
// 一个姿态（Pose）的定义
struct PoseEntry {
    // 标签（用于匹配）
    action: u8,              // 枚举：idle/walk/run/work/fight/...
    fatigue_band: u8,        // 0=精力充沛 1=正常 2=疲劳 3=极度疲劳
    emotion_quadrant: u8,    // 愉悦-觉醒象限（简化版）
    
    // 骨骼数据（相对于 rest pose 的变换）
    bone_transforms: [BoneTransform; MAX_BONES],
}

struct BoneTransform {
    translation: Vec3,   // 相对 rest pose 的位移
    rotation: Quat,      // 相对 rest pose 的旋转
    scale: Vec3,         // 通常为 (1,1,1)
}

// Pose Database 在 GPU 上存储为 texture 或 storage buffer
// 大小：假设 200 个姿态 × 50 根骨骼 × 32 字节/骨骼变换 = 320KB
// 这对 GPU 微不足道
```

### 2.2 运行时查询

```
对于 NPC #42，当前状态为：
  action = "walk"
  fatigue = 0.7
  emotion_pleasure = -0.3
  
Pose Database 查询：
  1. 找到 action="walk" 的所有姿态
  2. 用 fatigue_band 筛选最近的 2 个 band
  3. 用 emotion_quadrant 筛选最近的 2 个象限
  4. 综合距离 → 2-4 个候选姿态的混合权重
```

这个查询在 GPU compute shader 上完成。对于 1000 个 NPC，每个查询 200 个姿态——总共 200,000 次距离计算——在 GPU 上这是一次 dispatch，微秒级。

### 2.3 动画时间管理

每个 NPC 的动画时间（每根骨骼的不行相位）在 GPU 上独立推进：

```wgsl
// GPU compute shader 伪代码
@compute @workgroup_size(64)
fn update_animation(
    @builtin(global_invocation_id) npc_idx: u32,
) {
    let npc = npc_data[npc_idx];
    
    // 推进动画时间
    let dt = time.delta;
    let speed = length(npc.velocity);
    npc_data[npc_idx].anim_time += dt * speed * WALK_CYCLE_SPEED;
    
    // 查询 Pose Database
    let poses = query_pose_database(npc.pose_driver);
    
    // 混合姿态（加权平均）
    var blended_pose: array<BoneTransform, MAX_BONES>;
    for (var i = 0u; i < poses.len; i++) {
        for (var b = 0u; b < MAX_BONES; b++) {
            blended_pose[b] += poses[i].bone_transforms[b] * poses[i].weight;
        }
    }
    
    // 模块组合：下半身 + 上半身 + 头部 + 面部
    var final_matrices: array<mat4x4f, MAX_BONES>;
    // 下半身：由移动状态驱动
    compute_lower_body(&blended_pose, npc.velocity, npc.fatigue);
    // 上半身：由行动类型驱动 + 情绪修饰
    compute_upper_body(&blended_pose, npc.current_action, npc.emotion);
    // 头部：由注意力目标驱动
    compute_head(&blended_pose, npc.attention_target);
    
    // 写入输出 buffer（vertex shader 消费）
    bone_matrices_out[npc_idx * MAX_BONES .. (npc_idx+1) * MAX_BONES] = final_matrices;
}
```

### 2.4 LOD 按模块降级

```
L1 (~1000 NPC, <50m):
  全模块：下半身 + 上半身 + 头部 + 面部
  全骨骼数（~50 根）
  每帧更新动画时间

L2 (~10000 NPC, 50-150m):
  下半身 + 上半身（无面部）
  简化骨骼数（~20 根）
  每 3 帧更新一次动画时间

L3 (>10000 NPC, >150m):
  不进入 GPU 动画管线
  位置更新由 CPU 侧的简化状态机处理
```

---

## 三、这个方案为什么适合 LLM 辅助开发

### 3.1 可以分模块让 LLM 生成

```
模块 1: Pose Database 的数据结构 + 加载器（GLTF 解析 → PoseEntry[]）
  → LLM 生成，开发者验证几个姿态是否正确加载

模块 2: GPU Compute Shader（姿态查询 + 混合 + 骨骼矩阵计算）
  → LLM 生成 WGSL/GLSL，开发者用 RenderDoc 验证输出

模块 3: CPU 侧的 Pose Driver 参数打包
  → LLM 生成，非常简单（struct → byte array）

模块 4: Vertex Shader 骨骼蒙皮
  → LLM 生成，标准公式
```

每个模块独立、有明确的输入输出、可以单独测试——这是 LLM 最擅长的任务类型。

### 3.2 渐进实现

不追求一次性实现完整的 UAF。三步走：

**Step 1 (1-2 周)**：CPU 侧实现简化版 Pose Database（10 个姿态，1 个模块——全骨骼）。用 GDScript 原型验证逻辑正确性。

**Step 2 (2-4 周)**：将姿态混合和骨骼矩阵计算迁移到 GPU compute shader。保持简单的 Pose Database。

**Step 3 (4-8 周)**：扩展 Pose Database 到 ~200 个姿态，拆分为模块化骨骼，实现 LOD 降级。

### 3.3 与引擎的集成

**在 Godot 中**：
- 使用 `RenderingDevice` 提交 compute shader
- Compute shader 输出写入 `StorageBuffer`
- `MultiMeshInstance3D` 的 vertex shader 读取这个 buffer
- 不创建 AnimationTree 节点

**不依赖 RenderingDevice 的引擎**：
- 使用 OpenGL/Vulkan compute shader
- 骨骼矩阵写入 Texture Buffer Object（TBO）
- 同样的 vertex shader 逻辑

---

## 四、与传统 AnimationTree 的对比

| | AnimationTree (Godot) | UAF 启发的 GPU 方案 |
|---|---|---|
| 状态机 | 每 NPC 一个 | **无状态机——数据驱动** |
| 计算位置 | CPU | **GPU** |
| 1000 NPC 耗时 | ~10-20ms（CPU瓶颈） | **~0.5-1ms（GPU并行）** |
| 新动画添加 | 修改 AnimationTree + 添加 BlendSpace 节点 | **添加 PoseEntry 到数据库** |
| 混合复杂度 | 2-4 个动画混合（BlendSpace限制） | **可混合 N 个姿态** |
| 模块化 | 全身骨骼一棵树 | **独立模块（下半身/上半身/头/面）** |
| LOD | 无内置 | **内置按模块/骨骼数/更新频率降级** |
| 与 WoWorld 情绪引擎的耦合 | 松（需要手动映射） | **紧（emotion → Pose Driver 直接驱动姿态）** |
| 调试 | Godot 内置 AnimationTree 调试器 | **需自建（但可写 CPU 侧验证路径）** |

---

## 五、LLM 可以生成的核心代码量估算

| 组件 | 代码行数 | LLM 可生成比例 | 开发者需手写 |
|------|---------|--------------|------------|
| Pose Database 数据结构 | ~100 | 90% | 验证 |
| GLTF 动画提取器 | ~300 | 85% | 验证 + 调试 |
| GPU Compute Shader | ~200 | 70% | 性能调优 |
| Vertex Shader 蒙皮 | ~50 | 95% | 几乎不需要 |
| CPU 侧 Pose Driver 更新 | ~150 | 90% | 与 NPC 状态的集成 |
| Buffer 管理 | ~100 | 90% | 平台差异处理 |
| **总计** | **~900** | **~85%** | **~150 行 + 集成调试** |

> 一个下午的 LLM 会话可以生成这 900 行的初版。剩下的工作是理解、审查、集成、性能调优——这才是开发者真正花时间的地方。
