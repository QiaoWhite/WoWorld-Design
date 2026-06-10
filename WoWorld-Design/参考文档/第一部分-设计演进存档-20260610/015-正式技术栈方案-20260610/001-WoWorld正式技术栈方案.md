# 001-WoWorld 正式技术栈方案

> **版本**：1.0 — 可能是正式方案  
> **日期**：2026-06-10  
> **基础**：综合 004-014 的全部分析、审查、修正，形成单一权威技术方案  
> **定位**：本方案可能是 WoWorld 的技术实现蓝图——所有后续设计文档中的技术决策以本文档为准

---

## 〇、设计哲学的技术映射

WoWorld 的核心哲学在技术栈中的表达：

| 哲学原则 | 技术映射 |
|---------|---------|
| **只创造规则，不创造故事** | 参数驱动的涌现引擎——GOAP + 概率决策器。Mod 调节倾向性乘数，不编写行为脚本 |
| **创建另一个"人世"** | 100K NPC 的完整模拟——每个 NPC 有独立的心智、记忆、关系。分层模拟 (L1/L2/L3) 使这可行 |
| **每个存档都是新世界** | 种子确定性——地形、NPC、文化全部从种子参数化生成。存档仅保存与种子的差异 |
| **全球多元文化自然演化** | 文化种子参数 → 漂移/分化/融合 → 建筑/服饰/饮食的全链路影响。不由开发者预设模板 |
| **冒险与生活平等** | 所有玩法由同一模拟核心支撑——铁匠和冒险者在同一套规则下获得同等的系统深度 |
| **Mod 是涌现的延伸** | 数据驱动（TOML）+ 参数调节 API——Modder 创作的是"新规则"，不是"新脚本" |

---

## 一、总技术栈

### 1.1 选型总表

| 层 | 技术 | 版本 | 职责 |
|----|------|------|------|
| **模拟语言** | Rust | stable 1.80+ | NPC 心智、GOAP、情绪、记忆、世界生成、经济、骨骼矩阵、时间、天气 |
| **游戏引擎** | Godot | 4.6 LTS | 渲染、UI、音频、输入、玩家物理、粒子、后处理、天体 Sky shader |
| **引擎集成** | GDExtension (`godot-rust`) | 0.2+ | Rust ↔ Godot 数据通道（PackedByteArray 批量传输） |
| **ECS / 实体管理** | 无外部 ECS——Rust 模块化 struct + Vec | — | 与现有 NPC 文档架构一致，SoA 布局 + rayon 并行 |
| **数据库** | LMDB (`lmdb-rkv`) | 0.8+ | 记忆、关系、事件事实——双层架构（事实库 + 主观记忆库） |
| **序列化** | `serde` + `bincode` | 1.x | 存档 + 网络传输——二进制紧凑，比 JSON 快 10-100× |
| **并发** | `rayon` | 1.x | 数据并行（情绪更新、关系衰减、世界生成） |
| **体素等值面** | Transvoxel（Rust 自建） | — | 基于 `transvoxel-rs` 参考实现，分层密度场 + 8 级 LOD |
| **噪声生成** | `noise` crate + 远期 GPU compute | 0.9+ | Perlin / Simplex / Ridged / Worley / fBm |
| **动画** | Rust CPU 批量骨骼矩阵 → Godot GPU skinning shader | — | UAF 启发的模块化姿态系统 |
| **NPC 渲染** | Godot `MultiMeshInstance3D` + 自定义 shader | — | 1 个 Node，1 次 draw call，1000 个 NPC |
| **NPC 物理** | Godot `PhysicsServer3D` API（无 Node） | — | RID 直接操作——零场景树开销 |
| **Mod 脚本** | Rhai（远期） | 1.x | 仅用于参数调节 API——不暴露行为编写 |
| **Mod 数据** | TOML | — | NPC 模板、物品、建筑、文化参数、天体配置 |
| **UI** | Godot Control 节点 (GDScript 胶水) | — | 全部 UI——Godot 最成熟的部分 |
| **音频** | Godot `AudioServer` / `AudioStreamPlayer3D` | — | 3D 音频 + 天气环境音效 |
| **GLTF 加载** | `gltf` crate (Rust 侧) + Godot GLTF 导入 | — | 动画数据在 Rust 侧解析 → Pose Database |
| **数学** | `glam` | 0.28+ | SIMD 加速的向量/矩阵 |
| **日志** | `tracing` | 0.1+ | 结构化日志——比 `println!` 更适合大规模 NPC |
| **基准测试** | `criterion` | 0.5+ | 性能回归检测 |
| **Mod 工具链** | WoWorld Mod Kit CLI + VSCode 扩展 | — | 创建/验证/打包 Mod |

---

## 二、架构总图

```
┌──────────────────────────────────────────────────────────────────┐
│                      Rust 模拟核心 (独立进程内)                    │
│                                                                  │
│  ┌─────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────┐ │
│  │ NPC 系统 │ │ 世界生成  │ │ 时间天气  │ │  Mod 系统 │ │ 基础设施│ │
│  │·情绪引擎 │ │·分层密度场│ │·TimeMgr  │ │·TOML加载 │ │·LMDB   │ │
│  │·GOAP    │ │·Transvoxel│ │·Weather  │ │·Rhai(远期)│ │·bincode│ │
│  │·记忆    │ │·8级LOD   │ │·Celestial│ │·数据验证  │ │·rayon  │ │
│  │·关系    │ │·噪声叠加 │ │·Season   │ │·钩子注册  │ │·glam   │ │
│  │·骨骼矩阵│ │·奇幻特征 │ │          │ │          │ │·tracing│ │
│  │·多源加载│ │·Chunk管理│ │          │ │          │ │        │ │
│  └────┬────┘ └────┬─────┘ └────┬─────┘ └────┬─────┘ └───┬────┘ │
│       └───────────┴────────────┼────────────┴───────────┘       │
│                          事件总线                                │
└───────────────────────────────┼──────────────────────────────────┘
                                │ GDExtension
                                │ PackedByteArray 批量传输
                                │ ·骨骼矩阵 ·视觉状态 ·体素三角形
                                │ ·天体参数 ·天气数据 ·音频事件
┌───────────────────────────────┼──────────────────────────────────┐
│                         Godot 4.6 客户端                          │
│                                                                  │
│  SceneTree (< 100 Node):                                         │
│  ┌────────────────────┐ ┌──────────┐ ┌──────┐ ┌──────────────┐  │
│  │ NpcRenderer        │ │ Terrain  │ │Player│ │ UI (Control) │  │
│  │ MultiMesh × 3LOD  │ │ ~50 Mesh │ │CBody │ │ HUD/背包/对话 │  │
│  │ GPU skinning shader│ │Instance3D│ │Camera│ │ 地图/技能面板 │  │
│  └────────────────────┘ └──────────┘ └──────┘ └──────────────┘  │
│                                                                  │
│  PhysicsServer3D API (无 Node):                                  │
│  └─ NPC 碰撞体 RID × ~1000                                       │
│  └─ Terrain 碰撞体 RID × ~50                                     │
│                                                                  │
│  AudioServer API: 天气环境音效、3D NPC 声音                        │
│                                                                  │
│  RenderingDevice (远期): Compute shader——GPU 噪声 + GPU 骨骼      │
└──────────────────────────────────────────────────────────────────┘
```

---

## 三、体素世界生成

### 3.1 核心架构：分层密度场

```
最终密度 = 基础地形层(种子→噪声→生物群系)
         + 地质特征层(洞穴/悬崖/矿脉)
         + 奇幻特征层(浮空岛/巨菇林/水晶——DensityProvider trait 可插拔)
         + 文化覆盖层(建筑地基/道路/农田)
         + NPC修改层(挖掘/踩踏——Desire Path 道路形成)
         + 玩家修改层(SDF 雕刻)
         + 天气临时层(积雪/积水——不与存档混合)
```

**存档策略**：未修改的 chunk 从种子重现（零存储）。只保存被修改过的 chunk 的差异数据。4km² 初始世界存档约 81MB。

### 3.2 LOD 体系：8 级覆盖 0-6.4km+

```
LOD 0: 0.5m/voxel  →  0-50m     (Full Transvoxel)
LOD 1: 1m/voxel    →  50-100m   (Full Transvoxel)
LOD 2: 2m/voxel    →  100-200m  (Full Transvoxel)
LOD 3: 4m/voxel    →  200-400m  (Full Transvoxel)
LOD 4: 8m/voxel    →  400-800m  (Transvoxel 低精度)
LOD 5: 16m/voxel   →  800-1600m (Transvoxel 极低精度)
LOD 6: 32m/voxel   →  1.6-3.2km (Heightfield 替代)
LOD 7: 64m/voxel   →  3.2-6.4km (Heightfield 替代)
LOD 8: 128m/voxel  →  6.4km+    (Billboard + 大气散射融合)
```

**大气遮罩**：高度雾(深海→地表→高空) + 距离雾(0→6km) + Godot Environment 天空色融合——隐藏全部 LOD 过渡。

### 3.3 奇幻特征：可插拔 DensityProvider

```rust
trait DensityProvider: Send + Sync {
    fn sample(&self, pos: Vec3) -> f32;
    fn bounds(&self) -> Aabb;
}
```

预置 8 种类型：浮空岛、巨菇林、水晶簇、魔法漩涡、浮空瀑布、龙巢、石化森林、虚空裂隙。由魔法浓度 × 群系 × 高度控制分布——不同的种子产生不同的奇幻景观。

### 3.4 高速移动：GPU 噪声 + 预测加载 + 分速降级

| 移动模式 | 速度 | 加载半径 | 初始 LOD | 加载策略 |
|---------|------|---------|---------|---------|
| 步行 | 5 m/s | 150m | LOD 0 | 完整生成——所有 LOD |
| 骑马/地面快跑 | 15-80 m/s | 250m | LOD 1 | 碰撞优先——扇形优先 |
| 飞龙/魔法飞行 | 50-200 m/s | 400m | LOD 3 | Heightfield fallback——逐步细化 |
| 传送/极速 | 瞬间位移 | — | — | 1-2s 加载过渡——与动画配合 |

**地面高速特殊处理**：碰撞优先管线——粗碰撞体 0.05ms 内就绪保证不穿模，视觉 mesh 0.1-0.3s 后到位。

**多源加载（NPC 高速）**：加载预算硬分割——玩家 70%、同伴 20%、无关 NPC 10%。无关 NPC 的最低 LOD 设为 3（不需要细节）。

---

## 四、NPC 系统架构

### 4.1 模拟核心

不引入外部 ECS（Flecs/Bevy ECS）——与现有 NPC 文档架构保持一致：

- NPC 数据以 **SoA（Struct of Arrays）** 布局存储在 `NpcRegistry` 中
- 模拟核心作为 Rust 模块组织：`emotion.rs` / `decision.rs` / `memory.rs` / `social.rs` / `goap.rs`——每个模块是独立的 `pub fn` 函数
- rayon 数据并行处理独立 NPC 的更新（情绪、关系衰减）
- GOAP 规划串行执行（有全局副作用），分帧批处理，2ms 硬超时

### 4.2 分层模拟

| | L1 全模拟 | L2 轻量 | L3 统计 |
|---|---------|--------|---------|
| 数量 | ≤1,000 | ≤10,000 | ~89,000 |
| 距离 | ≤50m | 50-150m | >150m |
| 心智 | 完整（情绪+GOAP+记忆+社交） | 情绪衰减+简化状态机 | 每日/周批量命运推演 |
| 渲染 | MultiMesh GPU skinning | MultiMesh 简化骨骼 | 无 |
| 物理 | PhysicsServer3D RID | 同 L1（简化） | 无 |

### 4.3 记忆系统

双层 LMDB 架构（CHG-003 提出）：
- **事件事实库**：客观事件去重存储（多人经历同一事件 = 1 条事实）
- **主观记忆库**：(npc_id, event_id) 复合键 → 情绪编码 + 归因偏差 + 压缩状态

每 NPC 上限 2000 条记忆。冷记忆自动压缩（摘要化）——自然的遗忘和记忆错乱设计基础。不同 NPC 对同一事件保留独立的主观版本——"罗生门"效应的技术基础。

---

## 五、动画系统：UAF 启发的模块化 GPU 动画

### 5.1 核心原理

不创建 1000 个 `AnimationTree` 节点。骨骼矩阵在 Rust CPU 侧批量计算（rayon 并行，1000 NPC × 50 bones = 0.05ms），结果通过 `PackedByteArray` 传到 Godot → 上传到 `TextureBuffer` → vertex shader 中做蒙皮。

```
传统 AnimationTree: 1000 个实例 —— CPU 瓶颈
WoWorld 动画管线:    Rust CPU 批量矩阵 → GPU shader 蒙皮 —— GPU 并行
```

### 5.2 模块化骨骼（UAF Modular Rig）

| 模块 | 骨骼数 | 驱动源 |
|------|--------|--------|
| 下半身 | 15-20 | 移动速度/方向/疲劳度 |
| 上半身 | 15-20 | 行动类型/情绪/持物 |
| 头部 | 5-8 | 注意力目标（看向谁/什么） |
| 面部 | 10-15 | 八基色情绪 → 面部混合（仅 L1） |
| 手部 | 10-15 | 持有的物品类型 |

每个模块独立动画时间轴。下半身在走路的同时上半身在挥手告别——互不干扰。LOD 按模块降级（L2 无面部/手部）。

### 5.3 Pose Database

~200 个关键姿态（远期目标）。GLTF 动画 → Rust 侧解析 → `PoseEntry`（姿态标签 + 骨骼变换）→ 二进制序列化 → GPU Buffer。

Phase 1：20-30 个基础姿态（走/跑/坐/吃/睡/工作/社交）。Phase 3+：AI 视频→3D 工具批量生成至 ~100。Phase 4+：Mod 社区贡献动画。

---

## 六、时间·天体·季节·天气

### 6.1 TimeManager（Rust 模拟核心）

```rust
struct TimeManager {
    elapsed_seconds: f64,      // 世界创建后的总秒数
    seconds_per_day: f64,      // 可配置——默认 1440s (24分钟)
    days_per_year: u32,        // 可配置——默认 365
    current_year: u32,
    time_scale: f64,           // 时间流速乘数（可暂停/加速）
}
```

### 6.2 季节

基于纬度 + 日期计算。热带（<15°）两季、温带（15°-75°）四季、寒带（>75°）常年冬季。

**纠错（vs 014）**：014 的 `lat_offset` 季节计算有误——纬度偏移不应简单乘以 90。正确方式是将纬度映射到太阳赤纬的余弦相似度。在 015 中修正为基于太阳赤纬 ±23.5° 的标准地理模型。

### 6.3 天气

`WeatherManager` 状态机——群系 + 季节 + 温度约束天气类型。`WeatherChangeEvent` 通过事件总线广播——NPC 系统收到后调节行为权重（不写死行为——只调节倾向性乘数，符合涌现哲学）。

**天气的视觉影响**：雨/雪粒子在 Godot 侧（`GPUParticles3D`）。积雪/积水通过 `WeatherDensityOverlay` 临时密度层实现——不与存档混合，天气变化时自动重新计算。

### 6.4 天体

太阳/月亮/星星的位置由 `TimeManager` 计算。Godot `Sky` shader 接收参数渲染。天体配置由 TOML 文件定义——Modder 可自定义太阳数量/颜色/大小，月亮相位周期，星星密度，极光参数。

---

## 七、Mod 系统

### 7.1 核心原则（012 修正后）

**Mod 调节参数，不编写行为。** Modder 修改的是涌现引擎的倾向性乘数——不是写 `if noon { go_tavern() }`。

| Mod 层级 | 内容 | 实现方式 |
|---------|------|---------|
| Level 1 | 纹理/模型/音频替换 | Godot 资源覆盖 |
| Level 2 | NPC 模板/物品/建筑/天体配置 | TOML 数据文件 |
| Level 3 | 倾向性参数调节 | TOML 修改——调高/调低特定行为的概率乘数 |
| Level 4 | 世界规则修改（文化参数/生成规则） | TOML 高级配置——需新存档 |
| Level 5 | Rhai 脚本（高级/可能破坏涌现） | 标注为 Expert——不鼓励但提供 escape hatch |

### 7.2 Mod 存档兼容

- Level 1-2：安全——Mod 删除后已有物品保留但不再生成新实例
- Level 3：安全——行为回退到默认值
- Level 4：**不兼容**——需要新建存档。诚实告知玩家

---

## 八、性能预算

### 8.1 60fps 帧预算分配（16.7ms）

| 类别 | 预算 | 负责方 |
|------|------|--------|
| **Rust 模拟核心** | ≤7.0ms | Rust |
| ├─ NPC 心智（情绪/决策/社交） | ≤3.0ms | Rust + rayon |
| ├─ GOAP 规划（分帧，2ms 硬超时） | ≤2.0ms | Rust |
| ├─ 骨骼矩阵计算 | ≤0.5ms | Rust + rayon |
| ├─ 世界生成（后台线程，主线程仅同步） | ≤0.5ms | Rust + 后台线程 |
| └─ 时间/天气更新（低频率） | ≤0.1ms | Rust |
| **Godot 渲染** | ≤8.0ms | Godot |
| ├─ 体素 Mesh 渲染（~50 chunk） | ≤2.0ms | Godot Forward+ |
| ├─ NPC MultiMesh 渲染（1000 个） | ≤1.0ms | Godot MultiMesh |
| ├─ GPU skinning shader | ≤0.5ms | Godot custom shader |
| ├─ 光照 + 阴影（方向光 + CSM） | ≤2.0ms | Godot |
| ├─ 后处理（Bloom + 雾 + Color Grading） | ≤1.0ms | Godot |
| ├─ UI 渲染 | ≤1.0ms | Godot Control |
| └─ 玩家 + 天气粒子 | ≤0.5ms | Godot |
| **Godot 物理** | ≤1.7ms | PhysicsServer3D |
| └─ NPC 碰撞体（RID，无 Node） | ≤1.5ms | Godot Jolt |
| **Rhai Mod 脚本** | ≤0.5ms | Rust Rhai 引擎 |
| └─ 所有 Mod 钩子总执行时间上限 | 0.5ms | 超预算 Mod 自动禁用 |
| **总计** | **≤16.7ms** | |

### 8.2 内存预算

| 数据 | 规模 | 内存 |
|------|------|------|
| NPC 数据本体（SoA，100K） | 100K × 30B avg | ~30 MB |
| NPC 记忆（LMDB 热缓存） | 600 万条有效 | ~480 MB |
| NPC 关系（500 万条） | 500 万 × 48B | ~240 MB |
| 活跃 Chunk（256 LOADED + 1024 CACHED） | 1280 个 × 平均 100KB | ~128 MB |
| 体素 Mesh 数据（GPU 驻留） | Godot 管理 | 不占 CPU |
| 其他（团体/事件/道路/经济） | — | ~122 MB |
| **总计** | | **~1 GB** |

目标硬件：16GB+ RAM（2026 年主流配置）——~1GB 内存占用可接受，留有充足余量。

---

## 九、开发路线

```
Phase 1 (2-4 周): 最小可行原型
  纯 Godot + GDScript——1 个胶囊体在体素世界中找苹果吃
  不做任何架构决策。

Phase 2 (4-8 周): 噪声地形 + 昼夜 + 第一个奇幻特征
  Rust: 分层噪声 → Transvoxel mesh。TimeManager 基础（昼夜）。
  Godot: MultiMesh 渲染 + Sky shader 天体。
  里程碑：昼夜交替 + 可探索的自然地形 + 一个浮空岛。

Phase 3 (4-8 周): 分层密度 + 修改 + 高速 + 基础天气
  SDF 修改存档。分速加载（空中+地面）。碰撞优先管线。
  WeatherManager 基础（雨/晴）。多源加载（玩家+NPC）。
  里程碑：飞龙速度下世界加载跟上；NPC 高速移动；存档读写。

Phase 4 (长期): 完整世界管线
  8 级 LOD。垂直生态（地表→虚空）。季节系统。
  天气-体素耦合（积雪/积水）。天体完整自定义。
  奇幻特征库（10-20 种）。GPU 生产级加速。
  里程碑：从深海到虚空的完整世界——日夜交替、四季轮转。
```

---

## 十、十四项自查与修正

基于对 004-014 全部文档的审查，以下问题已在 015 中修正：

| # | 014 或其他方案的问题 | 015 中的修正 |
|---|-------------------|------------|
| 1 | 014 地面高速的碰撞优先管线——小石头暂时无碰撞（玩家可穿过）| 增加碰撞体延伸策略——0.3s 窗口内使用相邻 chunk 的碰撞体延伸覆盖。如果延伸不够，使用半径为 1m 的球形 fallback 碰撞体 |
| 2 | 014 多源加载预算硬分割可能导致无关 NPC 永远加载不到 chunk | 无关 NPC 的 10% 预算有最低保障——每 10 帧至少调度 1 个无关 NPC 的 chunk 请求 |
| 3 | 014 `TimeManager.day_of_year()` 溢出后 `+1` 产生 1-366 而非 1-365 | 修正为 `(elapsed / spd) as u32 % dpy`——产生 0-364，无需 `+1` |
| 4 | 014 没有天气音效 | 新增：`AudioServer` API 处理天气音效（雨声、风声、雷声）——3D 衰减，根据 `WeatherState` 动态交叉淡入淡出 |
| 5 | 014 天气对 NPC 的影响通过事件总线——但未明确"调节权重而非硬编码行为" | 明确：`WeatherChangeEvent` 携带浮点乘数（如 `outdoor_activity_multiplier: 0.3`）——NPC 概率决策器读取乘数而非硬编码"下雨→回家" |
| 6 | 014 缺少季节对体素纹理的影响 | 新增：Godot `ArrayMesh` 的材质参数随季节渐变——草地颜色、积雪覆盖度由 `SeasonManager` 输出的浮点值驱动 |
| 7 | 014 缺少时间状态的存档 | 新增：`TimeManager` 和 `WeatherManager` 实现 `Serialize/Deserialize`——存档包含游戏时间、天气状态快照 |
| 8 | 014 NPC 高速移动——未讨论 NPC 寻路在高速下的表现 | 新增：NPC 在高速移动时使用更粗粒度的导航图（2m 网格替代 0.5m）——路径规划更快，精度损失在高速下不可感知 |
| 9 | 014 未明确 Godot RenderingDevice 的使用边界 | 明确：RenderingDevice 仅在 Phase 3+ 用于 GPU compute shader（噪声生成 + GPU 骨骼矩阵）。Phase 1-2 全部 CPU |
| 10 | 011 方案中残存的 Rhai 行为脚本 API 与涌现哲学冲突 | 012 已修正。015 确认：Rhai 仅用于参数查询和事件注册。不暴露 `npc.set_goal()` 或 `npc.say()` |
| 11 | 013/014 未讨论植被系统（树/草） | 015 明确：植被使用 GPU Instancing (`MultiMeshInstance3D`)——生物群系的植被密度在 chunk metadata 中，渲染在 Godot 侧。不通过体素 mesh |
| 12 | 014 未明确水面渲染 | 015 明确：海平面使用半透明平面 + Godot `ShaderMaterial`。河流使用多段 mesh + 流动 UV。水面不参与体素生成 |
| 13 | 014 TimeManager 的 season 计算过于简化 | 修正为太阳赤纬模型：`declination = -23.44° × cos(360°/365 × (day + 10))`。纬度 < |declination| 为夏季，反之为冬季 |
| 14 | 014 帧预算缺少初始世界生成的等待时间 | 新增：WorldGen 进度条——Phase 1 生成玩家周围 500m（< 2s），其余后台异步生成。玩家立即可进入游戏 |

---

## 十一、风险矩阵

| 风险 | 等级 | 缓解 |
|------|------|------|
| `godot-rust` 停止维护 | 🟡 中 | GDExtension C API 可直接调用——封装在 `gdext-abi` crate 中。无需 godot-rust 也可工作 |
| Rust 学习曲线拖延进度 | 🟡 中 | LLM 辅助——按需学习。Phase 1 用 GDScript 原型开始——Rust 逐步引入 |
| GPU 噪声 readback 延迟 | 🟡 中 | Phase 3 性能测试验证——如果 > 50ms，降级 CPU heightfield |
| 100K NPC 的内存超预算 | 🟢 低 | SoA 布局 + LMDB 冷记忆——运行时内存 < 1GB |
| 地面高速碰撞不精准→穿模 | 🟡 中 | 多级 fallback——粗碰撞体 → 相邻延伸 → 球形 fallback |
| Mod 系统破坏涌现哲学 | 🟡 中 | API 设计审查——不暴露行为编写 API。默认 Mod 只调节参数 |
| Godot 5.0 GDExtension 不兼容 | 🟡 中 | 锁定 Godot 4.6 LTS 直至 5.x 稳定。GDExtension API 通过 trait 隔离 |

---

> **本方案可能是 WoWorld 的正式技术蓝图。** 它综合了 004-014 共十一份参考文档的全部分析、对比、审查、批判和修正。除非有新的根本性约束条件变化，后续所有技术决策以本文档为准。版本 1.0。
