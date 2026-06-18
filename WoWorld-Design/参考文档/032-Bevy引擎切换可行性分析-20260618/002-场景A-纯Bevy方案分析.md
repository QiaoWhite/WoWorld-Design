# 002-场景 A：切换至纯 Bevy 引擎

> **开发代号**: WoWorld (Wonder World)
> **文档类型**: 参考文档 — 引擎切换可行性分析
> **创建日期**: 2026-06-18
> **前置阅读**: [[001-现状回顾与既有评估|现状回顾]]
> **关联文档**: [[003-场景B-混合方案分析|场景 B]], [[005-跨维度综合比较与建议|综合比较]]

---

## 场景定义

**完全放弃 Godot，将整个项目迁移至纯 Bevy 引擎。**

- Bevy 负责：渲染、UI、音频、输入、物理（Rapier/Avian）、资产加载、窗口管理、平台导出
- Rust 模拟核心：直接运行在 Bevy ECS 世界内，消除所有 FFI 边界
- 所有 Godot 特性（编辑器、Control 节点、AnimationTree、AudioServer、NavMesh、资产管线）需要寻找或自建替代

---

## 一、可行性分析

### 1.1 渲染能力匹配度

WoWorld 的画面风格（3D 低多边形 + flat/cel 渲染）对渲染管线要求不高，Bevy 可以胜任。但具体任务需要不同程度的自建工作：

| 渲染需求 | 可行性 | 实现路径 |
|---------|--------|---------|
| **3D 低多边形渲染** | ✅ 原生支持 | Bevy PBR 管线 + glTF 加载。关闭 PBR 即为 flat 渲染 |
| **Cel/Flat/NPR 着色** | ⚠️ 社区 crate | `bevy_toon_shader` (tbillington) / `bevy_wind_waker_shader` v0.4.0 / 自定义 WGSL `ExtendedMaterial` |
| **Transvoxel 体素网格** | ⚠️ 需自建 | `bevy_marching_cubes` v0.18.1 (SDF→mesh) / `bevy_cube_marcher` v0.4.2 (GPU 计算着色器) / `bevy_meshem` v0.5 (体素网格+AO+O(1)更新)。没有现成的 Transvoxel 实现（含 LOD 过渡），需要基于上述 crate 构建 |
| **体积云** | ✅ 第一方 + 社区 | Bevy 0.14+ `VolumetricFogSettings` (体积雾/体积光) + `bevy_volumetric_clouds` v0.2.0 (Bevy 0.19) + 0.18 Atmosphere Occlusion + 广义大气散射介质 |
| **海洋 (Gerstner 波)** | ⚠️ 社区 crate | `bevy_water` v0.12.1 (动态海洋材质+可配置波参数) / `bevy_simple_water` |
| **MultiMesh 实例化** | ✅ ECS 原生 | Bevy 的 ECS + `Mesh2d`/`Mesh3d` 组件 + GPU instancing 自动处理。比 Godot MultiMesh 更灵活 |
| **GPU 双骨蒙皮** | ⚠️ 需自建 | Bevy 0.15+ 支持 GPU skinning（`bevy_animation`）。但 WoWorld 的 Rust CPU 骨骼矩阵批量计算 + GPU 双骨蒙皮方案需要自定义 shader |
| **CSM 阴影** | ✅ 原生支持 | Bevy 内置 Cascaded Shadow Maps |
| **后处理** | ✅ 原生支持 | Bevy 0.18 Fullscreen Materials——轻松添加自定义后处理 shader |

**总结**：核心渲染能力（低多边形/阴影/后处理）原生覆盖。体素和 cel 渲染需要基于社区 crate 构建，但不存在架构障碍。最大的工作量在 Transvoxel 实现和自定义双骨蒙皮 shader。

### 1.2 模拟核心迁移

**这是迁移中最简单的部分。** Rust 模拟核心被刻意设计为引擎无关——无 Godot API 依赖、无 GDScript 调用、纯 Rust 数据结构。

迁移工作：
1. 将 `NpcRegistry` 等 SoA 结构映射为 Bevy `Component` + `Resource`
2. 将 rayon 并行改为 Bevy 系统调度（让 Bevy 自动并行）
3. 将 LMDB 存储保留（与引擎无关）
4. 将事件总线改为 Bevy `Event` 系统
5. 估算工时：**~2-3 周**（主要是 API 适配，非逻辑重写）

### 1.3 需自建或替代的 Godot 特性

这是迁移工作的主体，也是最大的风险区：

| Godot 特性 | 用途 | Bevy 替代 | 自建难度 |
|-----------|------|----------|---------|
| **编辑器** | 场景编辑/资产浏览/调试 | Jackdaw v0.4.1 (社区) | ⭐⭐⭐⭐⭐ 最大缺口 |
| **Control 节点系统** | 背包/对话/技能面板/蓝图编辑器/HUD | bevy_ui (基础 Flexbox) + bevy_egui | ⭐⭐⭐⭐ |
| **AnimationTree** | 9层动画栈混合/BlendSpace | bevy_animation (基础) | ⭐⭐⭐⭐ |
| **AudioServer** | 3D空间音频/混音/效果总线 | bevy_audio (基础播放) / kibaudio (社区) | ⭐⭐⭐ |
| **NavMesh** | NPC 寻路 | 需自建或集成 `navmesh` crate | ⭐⭐⭐ |
| **资产导入管线** | glTF/纹理/音频导入+预设 | bevy_asset (代码驱动加载) | ⭐⭐ |
| **输入映射** | 键位/手柄绑定 | leafwing-input-manager (社区) | ⭐⭐ |
| **Jolt 物理 (仅玩家)** | CharacterBody3D | bevy_rapier 0.32 / Avian (XPBD) | ⭐⭐ |
| **多平台导出** | Win/Mac/Linux 一键导出 | cargo build --target + 平台配置 | ⭐⭐ |
| **GDScript 热重载** | 快速迭代 | 无等价物。Rust 编译替代 | ⭐⭐ (效率损失) |

### 1.4 API 稳定性

**这是 pre-1.0 引擎的本质风险**：

- Bevy 0.18.1 → 0.19 (即将发布)：每约 3-4 个月 breaking release
- 迁移指南随每次发布提供，但需要非零工作量
- 社区 crate（bevy_rapier, bevy_water, bevy_toon_shader 等）需等待对应版本更新
- 类比：WoWorld 的 Rust 核心如果有 20+ 个 bevy 生态依赖，每次 Bevy 大版本升级时全部需要检查兼容性

**真实影响**：对于一个可能持续数年的项目，每季度花 1-3 天做 API 迁移会积累成可观的时间成本。

---

## 二、好处（切换到纯 Bevy 的理由）

### 2.1 消除 FFI 边界

这是最大的结构性收益：

- **零序列化开销**：Rust 模拟写入 Component，渲染同一帧直接读取，无 `PackedByteArray` 编解码
- **统一调试**：rust-gdb/lldb 一套工具覆盖模拟+渲染
- **无三组件独立演进风险**：不再有 "Rust struct → godot-rust → GDExtension C API → Godot 引擎" 的脆弱依赖链
- **类型安全贯穿全栈**：编译期检查从模拟核心直到 shader uniform

当前方案的 FFI 代价虽然不大（~0.5ms/1000 NPC），但隐藏成本在于跨边界调试和维护。消除这个边界从根本上简化了架构。

### 2.2 ECS 原生性能优势

Bevy 的 ECS 在实体密集场景下的优势是结构性的，不是微优化级别的：

| 场景 | Godot (GDScript) | Bevy (Rust ECS) | 倍数 |
|------|-----------------|-----------------|------|
| 500 boids | 98 FPS | 145 FPS | 1.5x |
| 1,000 boids | 58 FPS | 145 FPS | 2.5x |
| 5,000 boids | 13 FPS | 108 FPS | 8.3x |
| 10,000 boids | 6 FPS | 145 FPS | 23.4x |
| 20,000 boids | 2.5 FPS | 26 FPS | 10.4x |

**对 WoWorld 的意义**：

- L1 NPC (≤1,000) 在 Godot SoA+rayon 下已足够（≤2.5ms）。但 Bevy ECS 的优势体现在：
  - **自动负载均衡**：Bevy 调度器自动将系统分配到可用核心。无需手动 rayon 调优
  - **缓存友好的数据布局**：Component 连续存储，迭代 10,000 实体的位置是纯数组遍历
  - **未来扩展性**：如果后续将 L2/L3 NPC 提升为更活跃的实体，Bevy ECS 有更大的性能余量

### 2.3 架构纯粹性与长期可维护性

- **单一语言**：100% Rust。无 GDScript/C# 上下文切换。Solo 开发者只需精通一门语言
- **单体二进制编译**：`cargo build` 产出单一可执行文件（+ assets 文件夹）。无 Godot 引擎二进制依赖
- **版本一致性**：`Cargo.toml` 锁定所有依赖版本。不像 Godot 需要匹配引擎版本 + GDExtension 版本 + godot-rust 版本
- **重构安全**：Rust 的编译器保证在重构跨越数十个模块时不会引入隐蔽的运行时错误
- **编译期保证 > 运行时错误**：在 WoWorld 这样复杂的 NPC 模拟系统中（记忆、预测、情感、欲望——全部交叉作用），Rust 的类型系统消除了大量 GDScript 中只能在运行时发现的 bug

### 2.4 WGSL 自定义渲染

- Bevy 的渲染基于 `wgpu`，使用 WebGPU 标准的 **WGSL** 着色语言。这是现代、跨平台的 GPU 原生语言
- `ExtendedMaterial<StandardMaterial>` 允许在 PBR 管线之上叠加自定义 shader 逻辑，无需替换整个渲染器
- Godot 的 GDSL（Godot Shader Language）仅在 Godot 生态内有用。WGSL 技能可以跨项目复用
- 对于 WoWorld 的 cel 渲染 + 自定义双骨蒙皮 + Gerstner 海洋 + 体积云，完全控制 GPU 管线是优势

### 2.5 社区趋势与长期生态

- Bevy 是 Rust gamedev 的**事实标准引擎**（>60% Rust gamedev 市场份额）
- GitHub ~46.5K stars, ~426 活跃贡献者，~1,967 依赖 crate
- 每 3-4 个月的发布节奏稳定
- **Tiny Glade** (100 万+ Steam 愿望单) 的商业成功是重要的信心信号
- 生态系统正从"实验品"向"可商业使用"过渡

---

## 三、坏处（不切换或暂缓切换的理由）

### 3.1 无生产级编辑器

**这是最大的实际障碍。**

- **无官方编辑器**：`bevy_editor_prototypes` 仍处于早期原型阶段（Phase 1: 最低场景编辑器）
- **Jackdaw v0.4.1** (2026.05) 是目前最好的社区替代品，但仍是早期软件：
  - 基于 egui 的 3D 编辑器
  - 刷子式 CSG 几何体、地形雕刻、场景序列化
  - 变换工具 + 网格吸附 + 撤销/重做
  - **但远不及** Godot 编辑器的成熟度（场景树可视化、属性检查器、动画编辑器、资产浏览器、脚本编辑器、性能分析器）
- **实际影响**：WoWorld 的大量 RPG 内容（NPC 配置、对话树、任务参数、UI 布局）在 Godot 中可通过编辑器以可视化方式创建和调整。在 Bevy 中，这些全部变成代码配置或需要自建工具

### 3.2 UI 系统薄弱

WoWorld 需要复杂的 UI 系统：
- 背包（网格布局、拖放、物品提示）
- 对话（富文本、分支选项、NPC 头像）
- 技能面板（树状图、节点状态、进度条）
- 蓝图编辑器（网格放置、旋转、材料预览、验证高亮）
- HUD（动态切换、小地图、状态栏）

**Bevy 的现状**：
- `bevy_ui`：基础 Flexbox 布局。`NodeBundle` + `TextBundle` + `ButtonBundle` + `ImageBundle`
- **没有**：滚动视图、树视图、标签页、滑块、下拉框、富文本、主题系统、BBCode
- `bevy_egui` (immediate mode) 适合编辑器/调试面板，但不太适合游戏内 UI
- `bevy_cosmic_editor` (retained mode widget) 仍在早期
- Godot 的 Control 节点系统 + Theme + StyleBox 是经过 10+ 年打磨的生产级 UI 方案

**估算**：在 Bevy 中构建 WoWorld 级别的 UI，可能意味着 30-40% 的开发时间花在 UI 基础设施上。

### 3.3 动画系统缺失

WoWorld 的动画系统是设计中最复杂的子系统之一：
- **9 层可叠加动画栈**：基础姿态、上半身覆盖、持握姿态、面部表情、物理响应、战斗风格、情感覆盖、受伤反应、环境交互
- **38 种模块姿态** + **15 种基元轨迹**
- **步态涌现**：从 BigFive 人格参数的 9 个参数派生
- **512² 面部图集**：16 嘴型 × 16 眉型 × 8 眼型
- **双骨蒙皮** GPU shader

**Bevy 的现状**：
- `bevy_animation` 提供基础动画播放（关键帧插值、动画图）
- **没有** AnimationTree、BlendSpace、动画层混合、状态机过渡
- Godot 的 `AnimationTree` + `AnimationNodeBlendSpace2D` + `AnimationNodeStateMachine` 是 WoWorld 动画栈设计的直接实现基础

**估算**：在 Bevy 中构建 9 层动画栈 + 面部图集驱动 + 涌现式步态系统，预计需要 **2-4 个月** 全职的动画基础设施开发。

### 3.4 音频系统基础

WoWorld 的音频系统（CHG-030）需要：
- AudioQuery 30 方法（空间查询、传播建模、遮蔽分析）
- 五类声音（环境/UI/语音/音乐/效果）
- 传播引擎（衰减、吸收、风、温度、遮挡、多普勒）
- 话语清晰度五档
- 音乐三层模型

**Bevy 的现状**：
- `bevy_audio`：基础空间音频播放（OGG/FLAC/WAV/MP3）
- **没有**：混音器、效果总线、DSP 效果链、自适应音乐系统
- `kibaudio` (社区)：基于 Kira 的数据驱动音频中间件，填补了部分空白（总线路由、自适应音乐、混合快照）
- Godot AudioServer + AudioBus 是成熟的生产级方案

### 3.5 迭代速度下降

Solo 开发中最宝贵的资源是**迭代速度**——快速试错、调整手感、验证想法。

| 迭代场景 | Godot | Bevy |
|---------|-------|------|
| 调整 UI 布局 | 编辑器拖放 + 实时预览 | 修改 Rust 代码 → 编译 → 运行 |
| 调整动画混合参数 | AnimationTree 实时滑块 | 修改代码 → 编译 → 运行 |
| 调整 NPC 行为参数 | GDScript 保存即生效 | 修改代码 → 编译 → 运行 |
| 调整着色器 | ShaderMaterial 实时预览 | 修改 WGSL → 重新加载 |
| 调整音频混合 | AudioBus 实时推子 | 修改代码 → 编译 → 运行 |

Rust 编译时间（增量构建 5-30 秒，全量构建数分钟）在迭代密集型阶段（调 UI、调动画、调手感）会成为显著瓶颈。Godot 的 GDScript 热重载（保存即生效）是 Solo 开发者的生产力倍增器。

### 3.6 商业化验证不足

- Bevy 上发布的 Steam 游戏 ~12 款，Godot 数千款
- 大型开放世界 RPG 在 Bevy 上**尚无先例**
- 最成功的 Bevy 项目 **Tiny Glade** 使用了 Bevy ECS 但搭配**自建 Vulkan 渲染器**（不用 Bevy 的渲染管线）
- Godot 已有多个商业成功的开放世界/RPG 项目
- 遇到罕见 Bug 时，Godot 社区规模和积累让你更快找到答案

### 3.7 API 不稳定

- 每次 Bevy 版本升级（每 3-4 个月）都可能需要迁移代码
- 社区 crate 的兼容性链条更长（crate 作者需要先适配新版本 Bevy，然后你才能适配）
- 对于一个可能持续 3-5 年的项目，累积的 API 迁移工作量不可忽视

---

## 四、估算的迁移总工时

基于既有分析中"6-12 个月全职"的估算，结合 WoWorld 具体需求细化：

| 迁移任务 | 估算工时 | 风险 |
|---------|---------|------|
| 模拟核心适配（SoA→ECS） | 2-3 周 | 低 |
| Transvoxel 实现（基于 bevy_meshem/bevy_cube_marcher） | 2-3 月 | 中 |
| 自定义 cel 渲染 shader | 2-4 周 | 低 |
| 体积云/海洋/大气渲染 | 3-5 周 | 中 |
| 双骨蒙皮 + GPU skinning | 3-5 周 | 中 |
| **9 层动画栈 + 面部图集 + 步态涌现** | **2-4 月** | 高 |
| **UI 基础设施**（背包/对话/技能面板/蓝图/HUD） | **2-4 月** | 高 |
| NavMesh 寻路 | 3-5 周 | 中 |
| 音频渲染管线 | 3-5 周 | 中 |
| 物理集成 (Rapier/Avian) | 2-3 周 | 低 |
| 输入系统 + 映射 | 1-2 周 | 低 |
| 资产管线搭建 | 2-3 周 | 低 |
| 编辑器工作流建立 (Jackdaw) | 2-4 周 | 中 |
| **总计** | **10-18 个月** | — |

注意：这是**纯迁移工作量**——在这期间几乎不会推进游戏内容的实际开发。对于 Solo 开发者，这意味着一年或更长时间的"引擎建设期"。

---

## 五、小结

| 维度 | 评级 | 说明 |
|------|------|------|
| 渲染能力 | ⭐⭐⭐☆☆ | 核心具备，需自建 cel/Transvoxel/动画 shader |
| 模拟核心迁移 | ⭐⭐⭐⭐⭐ | 引擎无关设计使这部分几乎零成本 |
| 编辑器/工具 | ⭐⭐☆☆☆ | Jackdaw 在进步，但远未成熟 |
| UI 系统 | ⭐⭐☆☆☆ | 需大量自建才能达到 WoWorld 的复杂 UI 需求 |
| 动画系统 | ⭐⭐☆☆☆ | 9 层动画栈需从零构建 |
| 音频系统 | ⭐⭐⭐☆☆ | 基础 + kibaudio 可用，需适配 AudioQuery trait |
| API 稳定性 | ⭐⭐☆☆☆ | Pre-1.0，季度性 breaking changes |
| 生态成熟度 | ⭐⭐⭐☆☆ | 增长快，但仍不足以支撑复杂 RPG 开箱即用 |
| **总体可行性** | **技术上可行，时间上高风险** | 10-18 个月纯迁移工时 + 持续 API 维护 |

**纯 Bevy 方案在技术上完全可行。** 但它在 2026 年意味着：用一年以上的时间构建 Godot 已免费提供的工具链，换来的回报是结构更优雅的架构和更高的性能天花板。这笔"时间换架构"的交易是否值得，取决于项目的时间约束和对架构纯粹性的重视程度。

---

> **下一篇**: [[003-场景B-混合方案分析|场景 B：Godot + Bevy ECS 混合方案]]
