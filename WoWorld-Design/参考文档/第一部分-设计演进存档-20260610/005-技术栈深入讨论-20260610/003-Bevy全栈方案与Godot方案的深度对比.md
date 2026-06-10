# 003-Bevy 全栈方案与 Godot 方案的深度对比

> 对应：`004-技术栈重新设计` §一·二  
> 重点：两个方案的技术细节、tradeoff、决策矩阵

---

## 一、问题的起点

`004-技术栈重新设计` 提出了"Rust 模拟核心 + Godot 客户端"的混合架构。但有一个自然的问题需要被诚实回答：

> **如果核心计算已经全部在 Rust 中，为什么还要保留 Godot？为什么不直接全用 Rust（Bevy 引擎）？**

这不是一个可以被绕过的技术决策。两种方案各有严重的利弊。

---

## 二、方案 A：Rust 模拟核心 + Godot 客户端（混合方案）

### 架构

```
Rust (模拟) ──GDExtension── Godot (渲染/UI/音频/输入/物理)
```

### 优势

1. **Godot 的编辑器是现成的**。场景编辑、UI 布局（Control 节点）、动画状态机（AnimationTree）、材质/着色器编辑——这些在 Godot 中都有成熟的 GUI 工具。Bevy 目前全部靠代码。

2. **Godot 的 UI 系统成熟**。Control 节点体系经过十多年的打磨，做角色面板、背包界面、对话 UI 非常简单。Bevy 的 UI（`bevy_ui`）正在快速迭代但仍然粗糙。

3. **Godot 的音频系统**。3D 音频衰减、混音总线、音频资源管理——Godot 内置。Bevy 需要自己搭建音频管线。

4. **物理引擎（Jolt）**。Godot 有成熟的 Jolt Physics 集成。角色移动、碰撞检测、射线查询——API 稳定且有文档。

5. **社区和文档**。Godot 的社区规模远大于 Bevy，遇到问题的搜索成本低。

6. **退路**：如果 Rust 模拟核心出了严重问题，可以暂时用 GDScript 写 stub 继续开发 Godot 侧——两个代码库在危机时可以解耦。

### 劣势

1. **GDExtension 的维护成本**。`godot-rust` crate 虽然活跃，但它的 API 跟随 Godot 版本变化。Godot 5.0 发布时可能需要大量适配工作。

2. **两套类型系统**。Rust 的 struct 需要映射为 Godot 的 Variant/Dictionary。数据在边界上被序列化和反序列化——虽然 bincode 很快，但这仍然是额外的开销。

3. **调试跨进程/跨 FFI 的问题困难**。NPC 行为异常时，需要同时查看 Rust 日志和 Godot 调试器才能定位——没有统一的调试视图。

4. **渲染数据拷贝**。NPC 的视觉状态（位置、动画参数）从 Rust → bincode 序列化 → GDExtension → Godot Variant → 场景节点属性。每一步都在拷贝数据。

5. **Solo 开发者需要维护两套构建系统**。Rust 侧是 `cargo build`，Godot 侧是 Godot Editor + GDScript 资源。CI 流程需要同时处理两者。

---

## 三、方案 B：全栈 Bevy（纯 Rust）

### 架构

```
Bevy App
├── bevy_ecs      ← 模拟核心（NPC、世界生成、经济）
├── bevy_render   ← 渲染（包括体素 Mesh）
├── bevy_ui       ← UI
├── bevy_audio    ← 音频
├── bevy_input    ← 输入
└── bevy_asset    ← 资产管理
```

### 优势

1. **统一类型系统**。NPC 决策代码可以直接访问渲染组件——没有序列化边界。NPC 的情绪数据就是 `EmotionalState` 组件，渲染系统直接读取。

2. **零拷贝数据流**。模拟 System 写入组件的同一帧，渲染 System 就可以读取——没有 FFI、没有序列化开销。

3. **统一的调试视图**。`bevy-inspector` 可以在运行时查看和修改任何 Entity 的任何 Component。NPC 的完整心智状态和渲染状态在同一个调试面板中。

4. **ECS 的极致发挥**。Bevy 本身就是 ECS 引擎。模拟（NPC、经济、世界生成）和渲染（culling、LOD、粒子）在同一个 ECS 世界中共存——System 调度是全局的、可配置的。

5. **编译到单一二进制**。没有 GDExtension 动态库的版本兼容问题。`cargo build --release` 产出单个可执行文件。

6. **类型安全贯穿全栈**。渲染着色器的绑定、UI 布局、音频事件——全部在 Rust 的类型系统内。编辑器拼写错误在编译期被捕获。

### 劣势

1. **没有 GUI 编辑器**。场景布局、UI 设计、动画状态机——全部需要手写代码。对于需要大量手工调整的场景（城镇布局、室内装饰），纯代码方式可能极其低效。

2. **UI 系统不成熟**。`bevy_ui` 使用 Flexbox 布局，功能在不断改进但仍然缺少很多 Godot Control 节点的便利特性（如 Theme、StyleBox、BBCode 文本、富文本）。

3. **动画系统原始**。Bevy 的 `AnimationGraph` 仍在早期阶段。Godot 的 `AnimationTree` + `BlendSpace` 做 NPC 的行走/跑步/情绪微动作混合要方便得多。

4. **音频管线待完善**。基本的 3D 音频可以工作，但混音、效果器、音频总线——这些在 Godot 中成熟的特性在 Bevy 中还在路上。

5. **资产导入管线**。GLTF 模型导入 Bevy 需要配置。Blender → Godot 的导入是经过多年优化的——材质自动转换、动画自动识别。

6. **物理引擎**。Bevy 主要使用 `bevy_rapier`（Rapier 物理引擎的 Bevy 绑定）。Rapier 是优秀的纯 Rust 物理引擎，但 Jolt Physics 在某些方面（如角色控制器）更成熟。

7. **生态年轻**。Bevy 社区活跃但规模小。遇到不常见的问题时可能没有现成的答案。

8. **编译速度**。全栈 Rust 项目的编译时间较长。Bevy 本身是一个大型依赖——`cargo build` 首次可能需要 10-20 分钟。

---

## 四、逐项对比

| 维度 | 方案 A (Rust + Godot) | 方案 B (Bevy 全栈) | WoWorld 的需求权重 |
|------|----------------------|---------------------|-------------------|
| **NPC 模拟性能** | ✅ 极快（Rust） | ✅ 极快（Rust） | 🔴 极高（100K NPC） |
| **UI 开发效率** | ✅ 成熟（Godot Control） | ⚠️ 代码驱动 | 🟡 中（后期才需要完整 UI） |
| **渲染管线** | ✅ 成熟（Forward+） | ⚠️ 可用但调整空间小 | 🟡 中（低多边形风格对渲染要求低） |
| **动画系统** | ✅ 成熟（AnimationTree） | ⚠️ 原始 | 🟡 中（阶段早期不需要复杂动画） |
| **体素/Mesh** | ⚠️ 需 Godot 侧构建 | ⚠️ 需手写 Mesh 构建 | 🔴 核心 |
| **3D 音频** | ✅ 成熟 | ⚠️ 基本可用 | 🟢 低（非核心） |
| **物理（碰撞）** | ✅ Jolt 成熟 | ⚠️ Rapier 可用 | 🟡 中 |
| **资产管理** | ✅ 成熟导入管线 | ⚠️ 手写 + 插件 | 🟡 中 |
| **调试体验** | ⚠️ 跨 FFI 调试困难 | ✅ 统一 inspetor | 🟡 中 |
| **编辑器** | ✅ Godot Editor | ❌ 无 | 🟡 中（一个人开发，不需要可视化编辑器给美术用） |
| **学习曲线** | 高（Rust + Godot 两者） | 高（纯 Rust 但全套自建） | — |
| **编译速度** | 中（仅 Rust 侧慢） | 慢（全栈编译） | 🟢 低（开发体验但不是阻断项） |
| **长期维护** | ⚠️ 两套系统 | ✅ 纯 Rust 单一系统 | 🔴 3-6 年周期 |
| **LLM 网关** | ✅ Godot HTTPRequest | ⚠️ 需 `reqwest` 等 | 🟢 低 |
| **社区规模** | 大（Godot） | 小（Bevy） | 🟡 中 |

---

## 五、决策矩阵：从 WoWorld 的具体情况出发

### 5.1 决定性的因素

对于 WoWorld 这个特定项目，以下因素可能比通用技术对比更重要：

**1. 你是 Solo 开发者。** 
- Godot 编辑器 = 你不需要写场景编辑器、UI 编辑器、动画编辑器。这些都是现成的。
- Bevy = 你需要自己写或拼装所有这些工具。你是唯一的开发者——这些工具的时间投入从业务代码中扣除。

**2. 你的美术是 AI 生成 + Blender 手动调整。**
- Godot 对 GLTF 的导入支持成熟。
- Bevy 也能导入 GLTF，但可能需要更多的手动调整和代码配置。

**3. 你的渲染需求是低多边形 + 像素纹理。**
- Godot 的 Forward+ 渲染器对此绰绰有余。
- Bevy 的渲染器也足够，但 Godot 的材质/着色器编辑器更方便微调美术风格。

**4. 开发周期是 3-6 年。**
- 3-6 年后 Bevy 可能已经非常成熟。这是在"投资未来"还是"赌博"？
- 3-6 年后 Godot 5.x 应该已发布。GDExtension API 的稳定性如何？

### 5.2 我的判断

**方案 A（Rust + Godot）是当前更安全、更务实的选择。** 原因：

1. **核心矛盾已经解决**：NPC 模拟的性能瓶颈——这是唯一不可协商的需求——在方案 A 中已由 Rust 解决。Godot 保留的是它最擅长的东西。

2. **编辑器不是奢侈品——它是杠杆**：对于 Solo 开发者来说，可视化编辑器不是"方便"——它是"能不能做"的问题。在没有 UI 编辑器的情况下做背包界面、对话界面、技能面板、地图——这些不是 Showstopper 但会消耗大量本应投入核心系统的精力。

3. **Bevy 是更好的长期选择——但它现在还不够**：如果这个项目在 2028 年开始，我会推荐 Bevy 全栈。但 2026 年的 Bevy 在 UI、动画、音频方面仍然需要大量的自建工作。这些不是 WoWorld 的差异化竞争力——在它们上面花时间是离核心价值最远的事情。

4. **GDExtension 的维护风险可控**：godot-rust crate 由活跃社区维护。即使未来 Godot 5.x 需要适配，适配范围仅限于 GDExtension 桥接层——Rust 模拟核心不依赖任何 Godot API。

### 5.3 但有一个条件

**如果 Bevy 在 1-2 年内（即 WoWorld 进入需要大量 UI 工作的阶段之前）在 UI、动画、音频方面有飞跃式的成熟——可以考虑在那个时候迁移。** 这就是为什么在架构设计中特意使 Rust 模拟核心"不依赖 Godot"——它可以在未来插入任何渲染前端。

迁移路径：
```
现在：  Rust Core ──GDExtension── Godot
未来：  Rust Core ──bevy_ecs── Bevy (如果 Bevy 够成熟了)
```

---

## 六、不推荐的方案：纯 GDScript

为了完整性——**纯 GDScript/Godot 全栈方案在技术上是不可行的**。原因 `004-技术栈重新设计` §一已经充分论证。简言之：GDScript 是脚本语言，GOAP 规划在最坏情况下单次 5-20ms，10万 NPC 的热数据存储需要 1.8GB——这超出了任何消费级硬件的合理预算。

---

## 七、如果"必须全 Rust"的替代方案

如果未来决定废弃 Godot（出于任何原因——GDExtension 维护负担过重、Godot 方向改变、或 Bevy 已经足够成熟），完整的迁移清单：

1. **Bevy ECS**：替代 Flecs ECS（因为 Bevy 自带 ECS）
2. **bevy_render + 自定义 Transvoxel 管线**：替代 Godot 的体素渲染
3. **bevy_ui**：替代 Godot Control 节点
4. **bevy_animation**：替代 Godot AnimationTree
5. **bevy_rapier**：替代 Godot Jolt Physics
6. **bevy_asset**：替代 Godot 资产导入管线
7. **bevy_audio**：替代 Godot AudioServer
8. **egui 或 bevy-inspector**：替代 Godot 的调试面板

**估算迁移工作量**：相当于从零写一个轻量版 Godot——专门为 WoWorld 优化的渲染/UI/音频/物理前端。对 Solo 开发者，这可能在 6-12 个月的全职工作量范围。

> **结论**：不是现在。让 Bevy 再飞一会儿。用 Godot 做它最擅长的事，用 Rust 做它最擅长的事。这可能是 2026 年的最优解。
