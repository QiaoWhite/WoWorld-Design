# 001-Rust+Godot 综合技术栈：UAF 动画 + Rust 生态 + LLM 辅助

> 约束条件全览：Rust + Godot · LLM 大量生成 · UE5 UAF 动画理念 · Rust 社区资源 · 边学边做 · 最大化 Godot 能力 · 长期稳定 · Mod 友好 · 性能与表现平衡

---

## 〇、技术栈全景

```
┌──────────────────────────────────────────────────────────────┐
│                     Rust 模拟核心                              │
│                                                              │
│  NPC 系统         世界生成         Mod 系统        基础设施     │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌────────┐ │
│  │ 情绪引擎    │  │ 噪声→生物群系│  │ Rhai 脚本   │  │ LMDB   │ │
│  │ GOAP 规划   │  │ 聚落→建筑   │  │ 数据驱动    │  │ bincode│ │
│  │ 记忆系统    │  │ 道路→NPC    │  │ 事件钩子    │  │ rayon  │ │
│  │ 关系/社交   │  │ 历史模拟    │  │ API 文档    │  │ flecs  │ │
│  │ GPU 骨骼    │  │ Transvoxel  │  │             │  │        │ │
│  └────────────┘  └────────────┘  └────────────┘  └────────┘ │
│                                                              │
│  Rust 生态 crates:                                           │
│  flecs-rs | lmdb-rkv | serde+bincode | rayon | noise | gltf │
│  egui(调试) | rhai(mod脚本) | wgpu(远期compute) | anyhow    │
└──────────────────────┬───────────────────────────────────────┘
                       │ godot-rust GDExtension
┌──────────────────────┴───────────────────────────────────────┐
│                    Godot 4.6 客户端                            │
│                                                              │
│  渲染层              UI 层              Mod 层                │
│  ┌────────────┐  ┌────────────┐  ┌────────────────────┐     │
│  │ MultiMesh  │  │ Control 节点│  │ .tscn 场景加载      │     │
│  │ Rendering  │  │ 主题/样式   │  │ .tres 资源导入      │     │
│  │  Device    │  │ 富文本/BBCode│  │ .gd 胶水脚本(可选)  │     │
│  │ 自定义Shader│  │ 控制台命令   │  │ GLTF/纹理/音频替换  │     │
│  │ 后处理     │  │             │  │                     │     │
│  └────────────┘  └────────────┘  └────────────────────┘     │
│                                                              │
│  玩家系统          音频              输入                     │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐             │
│  │ Character  │  │ AudioServer │  │ InputMap    │             │
│  │  Body3D    │  │ AudioStream │  │  + 自定义    │             │
│  │ Jolt Phys  │  │  Player3D   │  │             │             │
│  └────────────┘  └────────────┘  └────────────┘             │
└──────────────────────────────────────────────────────────────┘
```

---

## 一、Rust crates 选型总表

### 1.1 核心基础设施

| crate | 版本 | 职责 | 选择理由 |
|-------|------|------|---------|
| `flecs-rs` | 0.4+ | ECS 框架 | C 实现性能极致，Rust 绑定成熟，不绑定任何渲染引擎 |
| `lmdb-rkv` | 0.8+ | 记忆/关系数据库 | 读极快、ACID 事务、copy-on-write 天然适合增量存档 |
| `serde` + `bincode` | 1.x | 序列化 | 零成本抽象、二进制紧凑、比 JSON 快 10-100x |
| `rayon` | 1.x | 数据并行 | 极简 API——`.par_iter()`一键并行 |
| `anyhow` | 1.x | 错误处理 | LLM 生成的代码用 `anyhow::Result<T>` 而非 `unwrap()` |

### 1.2 世界生成

| crate | 职责 | 备注 |
|-------|------|------|
| `noise` | Perlin/Simplex/Worley 噪声 | Rust 噪声库标准选择 |
| `rand` + `rand_pcg` | 种子可控的随机数 | PCG 算法适合程序化生成 |
| `glam` | SIMD 加速的向量/矩阵数学 | 游戏数学的事实标准 |
| `hecs` (备选) | 更轻量的 ECS | 如果 flecs-rs 太重，hecs 是纯 Rust 替代 |

### 1.3 Mod 支持

| crate | 职责 | 备注 |
|-------|------|------|
| `rhai` | 嵌入式脚本引擎 | Rust 原生、语法接近 Rust、适合作为 Mod 脚本语言 |
| `toml` | 配置文件解析 | Mod 数据文件格式——比 JSON 更适合手写 |
| `gltf` | GLTF 模型加载 | Mod 作者提供自定义模型的基础 |

### 1.4 开发工具

| crate | 职责 | 备注 |
|-------|------|------|
| `egui` | 即时模式 GUI（调试面板） | Rust 原生、可嵌入 Godot 窗口 |
| `tracing` | 结构化日志 | 比 `println!` 更适合调试大规模 NPC |
| `criterion` | 性能基准测试 | 比内置 `#[bench]` 更丰富 |

---

## 二、UE5 UAF 动画理念的 Rust 实现

### 2.1 映射关系

| UE5 UAF 概念 | Rust 侧实现 | Godot 侧配合 |
|-------------|-----------|------------|
| Pose Driver | Rust struct `PoseDriver`（浮点参数集） | 每帧从 NPC 状态→Pose Driver |
| Motion Matching | Rust `PoseDatabase`（~200 个关键姿态） | GPU compute shader 查询+混合 |
| Modular Rig | Rust 骨骼模块定义（下半身/上半身/头/面） | vertex shader 按模块蒙皮 |
| GPU Compute Update | Rust CPU 批量骨骼矩阵（或 wgpu compute） | RenderingDevice compute shader |
| Data-Driven Animation | GLTF 动画→Rust 二进制姿态数据库 | 不经过 Godot AnimationTree |
| LOD by Component | Rust 控制——L1 全模块/L2 精简/L3 无 | Godot MultiMesh 三级别渲染 |

### 2.2 Pose Database 构建流程

```
Blender (动画制作/AI生成)
  ↓ 导出 GLTF
Rust `gltf` crate 解析
  ↓ 提取关键帧 + 骨骼层级
Rust 预处理
  ├── 生成 PoseEntry（姿态标签 + 骨骼变换）
  ├── 生成 PoseDatabase（~200 姿态 × 50 骨骼 × 32B = 320KB）
  └── 序列化为二进制文件（游戏启动时加载到 GPU）
Godot 侧
  └── 上传到 TextureBuffer → Compute Shader 访问
```

### 2.3 LLM 生成指导

这个管线的每个步骤都可以让 LLM 分别生成：
- `gltf` crate 的解析和关键帧提取 → LLM 生成 85%
- 姿态标签的自动分类（根据动画文件名和骨骼运动模式推断 action/fatigue/emotion 标签）→ LLM 生成启发式规则
- GPU compute shader（WGSL） → LLM 生成 70%，开发者调优
- Vertex shader 骨骼蒙皮 → LLM 生成 95%

---

## 三、最大化 Godot 引擎能力的边界

### 3.1 应该留在 Godot 的

| 系统 | 理由 |
|------|------|
| **渲染管线**（Forward+） | 低多边形风格对 Godot 绰绰有余——不要重造轮子 |
| **UI 系统**（Control 节点） | 背包、对话、HUD、地图、技能面板——Godot UI 成熟且高效 |
| **音频**（AudioServer） | 3D 音频衰减、混音、效果器——Godot 内置 |
| **输入**（InputMap） | 键盘/鼠标/手柄映射——Godot 内置 |
| **玩家物理**（Jolt） | CharacterBody3D + 碰撞——Godot 内置 |
| **场景管理**（SceneTree） | 但只用 `< 100` 个 Node——玩家、UI、少数场景根节点 |
| **动画**（仅玩家） | 玩家的 AnimationTree 保留——因为只有 1 个 |
| **粒子系统** | 雨、雪、魔法特效——Godot 内置 |
| **后处理**（Environment） | Bloom、SSAO、色调映射、雾——Godot 内置 |

### 3.2 应该从 Godot 中剥离到 Rust 的

| 系统 | 理由 |
|------|------|
| **NPC 渲染** | MultiMesh 在 Godot 中——但 NPC 实例数据由 Rust 管理 |
| **NPC 动画** | 骨骼矩阵在 Rust 计算——Godot shader 只做蒙皮 |
| **NPC 物理** | PhysicsServer3D API——物理在 Godot，但 NPC 无 Node |
| **体素生成** | 噪声+Transvoxel 在 Rust——Godot 接收三角形数组构建 ArrayMesh |
| **所有心智计算** | 情绪、GOAP、记忆、决策——100% Rust |

### 3.3 关键策略：Rust 生成 + Godot 消费

```
Rust 生成               Godot 消费
─────────              ──────────
NPC 视觉状态数组    →   MultiMesh buffer 更新
骨骼矩阵 (每帧)      →   TextureBuffer (shader 采样)
体素三角形           →   ArrayMesh surface
世界生成数据         →   MeshInstance3D 创建/更新
音频事件             →   AudioServer.playback_start()
```

---

## 四、长期稳定策略

### 4.1 Godot 版本锁定

- **开发期间锁定 Godot 4.6 LTS**。不追新版本。
- Godot 5.0 发布后观察 6-12 个月再评估迁移。
- GDExtension API 兼容性：用 `godot-rust` 的 trait 封装隔离 Godot 版本差异。

### 4.2 Rust crate 版本策略

- 优先选择 **1.0+** 的 crate（serde、rayon、anyhow、rand、glam、tracing）
- 对未达到 1.0 但重要的 crate（`flecs-rs`、`godot-rust`、`rhai`）：锁定 minor 版本，定期评估更新
- 最小化依赖树——每个新增依赖都需要 justify

### 4.3 编码约定强制执行

用 `rustfmt` + `clippy` + CI 确保代码一致性。LLM 生成的代码可能风格不一致——自动化工具统一。

### 4.4 文档生成

从 Rust 代码的文档注释自动生成 API 参考：
- `cargo doc` → Rust 侧的内部文档
- 自定义脚本从 Rust struct 的 `#[doc]` 提取 Mod API 文档

---

## 五、LLM 辅助的 Rust 学习路径（针对性的）

与 010-001 的学习路径不同——这次是**按需学习**。

### 5.1 不是"先学 Rust 再写 WoWorld"——而是"WoWorld 需要什么就学什么"

```
第 1 个需求：创建一个 NPC struct
  → 学：struct 定义、字段类型、#[derive]
  → LLM 生成 NpcIdentity → 开发者读 → 改 → 理解

第 2 个需求：NPC 的数据需要存到磁盘
  → 学：serde Serialize/Deserialize
  → LLM 生成 #[derive] + 保存/加载代码 → 开发者理解 trait 的概念

第 3 个需求：100 个 NPC 的情绪需要并行更新
  → 学：rayon par_iter
  → LLM 生成并行代码 → 开发者理解"数据并行"和"Rayon 自动分配工作量"

第 4 个需求：GOAP 规划器需要搜索
  → 学：Vec、HashMap、迭代器、闭包
  → LLM 生成 A* 搜索 → 开发者理解"Rust 的集合类型"

...
```

每一步学到的概念**直接用于下一个需求**。没有"先学 6 周理论"的阶段——从第一天就在写 WoWorld 代码。

### 5.2 LLM 作为 Rust 导师的提示词

```
你是一个 Rust 导师。我正在实现一个 NPC 系统。当前我的 Rust 水平：理解了 struct、enum、Vec、HashMap、serde 的 derive 宏。不理解：泛型、trait object、生命周期标注。

请为以下需求生成代码：
[需求描述]

要求：
1. 代码可以编译通过（不需要完整实现，stub 即可）
2. 在关键行添加注释：这一行为什么这样写、涉及什么 Rust 概念
3. 如果代码中出现了我还不能理解的概念——用 `// TODO-LEARN: [概念名]` 标注
4. 在代码后附上一个"本次学到的概念"小结（不超过 5 个要点）

这样我可以边实现边学习。
```

---

## 六、与 009 方案（C# + Godot）的对比

| 维度 | 009 方案 (C#) | 本方案 (Rust) |
|------|-------------|-------------|
| 学习曲线 | 温和 | 陡——但 LLM 降低了有效坡度 |
| 内存安全 | GC + 手动优化 | 编译期保证 |
| 长期维护 | C# 生态稳定 | Rust + borrow checker = 重构信心 |
| Mod 脚本 | C# 天然不适合嵌入 | **Rhai**——Rust 原生脚本引擎 |
| 性能上限 | NativeAOT 接近 Rust | 零成本抽象——理论最优 |
| LLM 训练数据 | 更多 | 较多但增长快 |
| Godot 集成 | 官方 SDK | 社区 crate (活跃) |
| 并发安全性 | 运行时检查 | **编译期保证**——NPC 系统天然并行 |

**选择 Rust 的核心原因**（在此约束下）：不是"更快"——是"borrow checker 保证并发安全"+"Rhai mod 脚本"+"长期重构信心"+"生态与 UAF 理念更匹配"。

---

> **本方案是 009（C# 务实方案）在"必须 Rust"约束下的对应版本。两者共享相同的架构哲学（Godot 优先、GPU 动画、MultiMesh 渲染）——仅在模拟语言和 Mod 系统上分叉。**
