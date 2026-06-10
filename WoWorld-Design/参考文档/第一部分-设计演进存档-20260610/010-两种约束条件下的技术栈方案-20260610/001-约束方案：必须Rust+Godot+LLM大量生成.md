# 001-约束方案：必须 Rust + Godot + LLM 大量生成

> 约束条件：Rust 不可协商、Godot 不可放弃、LLM 是主要代码产出手段  
> 核心问题：在 Rust 学习曲线是既定事实的前提下，如何让技术栈对"边学边做"的 Solo 开发者可行？

---

## 〇、接受约束，改变应对方式

009 方案放弃 Rust 的理由是"学习曲线对新手太陡"。但如果 Rust 是不可协商的选择——出于对内存安全的执念、对长期维护成本的考量、或者单纯是个人偏好——那么问题从"选什么语言"变为**"如何让 Rust + Godot + LLM 这个组合成功"**。

答案是：**别学 Rust 的全部。只学 LLM 帮你处理不了的那 15%。**

---

## 一、Rust 的最小可行知识集

不是"掌握 Rust"——而是"掌握足够审查 LLM 生成的 Rust 代码的知识"。

### 1.1 必须理解的概念（无法外包给 LLM）

| 概念 | 为什么必须理解 | 学习时间 |
|------|-------------|---------|
| **Ownership & Borrowing** | LLM 生成的代码被 `borrow checker` 拒绝时，你必须知道怎么改——而不是瞎试 | 2-3 周 |
| **`&` vs `&mut` vs 所有权转移** | 这是 Rust 编译错误中 80% 的来源 | 1-2 周 |
| **`String` vs `&str` vs `Vec<T>` vs `&[T]`** | 字符串和切片是 GDScript 开发者最困惑的 Rust 概念 | 1 周 |
| **`struct` + `impl` + trait 基础** | 没有继承——trait 是 Rust 的接口机制 | 1 周 |
| **`Result<T, E>` vs `Option<T>` vs `panic!`** | Rust 没有异常。错误处理通过返回值 | 1 周 |
| **`cargo build` / `cargo test` / `cargo run`** | 基本开发循环 | 1 天 |

**总计**：约 6-9 周达到"能读懂 LLM 生成的 Rust 代码、能 fix borrow checker 错误、能写简单 struct/impl"的水平。LLM 全程辅助——但这 6-9 周不能跳过。

### 1.2 可以完全外包给 LLM 的知识

- 宏（`#[derive]`、`serde::Serialize` 等——只需要知道"它能做什么"，不需要知道"怎么写"）
- 泛型约束和 trait bound 的高级用法
- `unsafe` 代码（WoWorld 不需要）
- 汇编级别的性能优化
- 异步 Rust（`tokio`、`async fn`——只在存档 I/O 时用到，LLM 生成 + 开发者理解基本模式即可）

### 1.3 Rust 学习策略

**不要从"Rust Book"开始。** 从"让 LLM 生成一个简单的 Godot NPC 移动代码 → 编译 → 读 borrow checker 的错误 → 让 LLM 解释这个错误 → 修 → 编译通过 → 看 NPC 动起来"。

这是**以项目驱动的学习**——每一步都有可见的正反馈。相比之下，先啃完 300 页的 Rust Book 再写第一行代码——对 Solo 开发者是动力杀手。

---

## 二、LLM 辅助下的 Rust + Godot 开发流程

### 2.1 代码生成协议

为每个 Rust 模块建立固定的 LLM 提示词模板：

```
你是 Rust + Godot 4.x GDExtension 专家。项目约束：
- 模拟核心用 Rust (stable 1.80+)，游戏客户端用 Godot 4.6+
- Rust struct 需要 derive Serialize/Deserialize/Clone
- 热路径避免堆分配——用 `&[T]` 而非 `Vec<T>` 作为函数参数
- 错误处理用 `anyhow::Result<T>` 而非 `unwrap()`
- 骨骼矩阵等大块数据通过 `PackedByteArray` 在 Rust↔Godot 间传递
- NPC 数据在 Rust 侧用 SoA (Struct of Arrays) 布局

任务：[具体描述]
先生成 struct 定义 + 接口 trait + 单元测试框架，再生成实现代码。
```

### 2.2 编译→修复循环

这是 LLM 辅助下 Rust 开发的**核心循环**：

```
1. 开发者写出模块的功能描述
2. LLM 生成 Rust 代码
3. cargo build
4. 如果有编译错误：
   a. 复制错误信息给 LLM
   b. LLM 解释错误原因 + 生成修复代码
   c. 应用修复 → cargo build
   d. 重复直到编译通过
5. cargo test（LLM 已生成测试）
6. 如果测试失败 → LLM 分析失败原因 + 修复
7. 集成测试 → 在 Godot 中验证行为

步骤 4 的循环通常只需要 1-3 轮——因为 borrow checker 的错误信息是 Rust 最大的教学资产
```

### 2.3 GDExtension 胶水代码：LLM 最擅长的领域

GDExtension 的注册、类型映射、`#[godot_api]` 标记——这些是典型的"模板化代码"。LLM 可以生成 95% 的胶水层。

```rust
// 这种代码完全由 LLM 生成，开发者只需要验证接口是否正确
#[derive(GodotClass)]
#[class(base=Node)]
struct NpcSimulationBridge {
    core: SimulationCore,
}

#[godot_api]
impl INode for NpcSimulationBridge {
    fn init(base: Base<Node>) -> Self {
        Self { core: SimulationCore::new() }
    }
    
    fn ready(&mut self) {
        // LLM 生成的初始化逻辑
    }
    
    fn process(&mut self, delta: f64) {
        // LLM 生成的每帧更新逻辑
        self.core.tick(delta as f32);
        let visual_states = self.core.get_visual_states();
        // 将 visual_states 转化为 Godot 场景更新
    }
}
```

---

## 三、技术栈

| 层 | 技术 | 职责 |
|----|------|------|
| **模拟语言** | **Rust** (stable 1.80+) | NPC 心智、GOAP、情绪、记忆、世界生成、经济、骨骼矩阵计算 |
| **游戏引擎** | **Godot 4.6+** | 渲染、UI、音频、输入、玩家物理、粒子、后处理 |
| **引擎集成** | `godot-rust` GDExtension | Rust ↔ Godot 数据通道 |
| **数据库** | LMDB (via `lmdb-rkv`) | 记忆、关系、事件事实 |
| **ECS** | Flecs (via `flecs-rs`) | 模拟核心实体管理 |
| **并发** | `rayon` | 世界生成 + 批量 NPC 更新的数据并行 |
| **序列化** | `serde` + `bincode` | 存档 + GDExtension 数据传输 |
| **NPC 渲染** | Godot MultiMeshInstance3D | 批量渲染（1 draw call） |
| **NPC 动画** | **Rust CPU 批量骨骼矩阵** → Godot GPU skinning shader | 避免 AnimationTree 瓶颈 |
| **GPU 动画** | 可选——Rust 侧 wgpu compute (如必要) | 远期：如 CPU 骨骼矩阵成为瓶颈 |

---

## 四、Rust 特有优势的发挥

### 4.1 枚举驱动的状态机

WoWorld 的 NPC 有大量状态——情绪标签、行动类型、LOD 等级、关系标签。Rust 的 `enum` + `match` 是建模这些的天然工具：

```rust
// LLM 生成这种代码质量极高——
// 因为 Rust enum 的穷尽性匹配让 LLM 不可能"漏掉一个状态"
enum NpcAction {
    Idle,
    Walking { target: Vec3, speed: f32 },
    Working { task: WorkTask, progress: f32 },
    Eating { food_item: ItemId, location: Vec3 },
    Sleeping { bed_location: Vec3, quality: f32 },
    Socializing { partner: u64, topic: ConversationTopic },
    Fighting { target: u64, stance: CombatStance },
}

impl NpcAction {
    fn interruption_priority(&self) -> u8 {
        match self {
            Self::Fighting { .. } => 10,    // 不可中断
            Self::Sleeping { .. } => 8,      // 紧急情况可中断
            Self::Eating { .. } => 6,
            Self::Socializing { .. } => 3,
            Self::Working { .. } => 2,
            Self::Walking { .. } => 2,
            Self::Idle => 0,
        }
    }
}
```

### 4.2 borrow checker 帮你发现数据竞争

NPC 心智系统是一个"多个模块读写共享数据"的复杂系统。Rust 的 borrow checker 在编译期就能发现"你在 A 模块中修改了 NPC 的情绪，但 B 模块同时也在读它"的问题——这在任何其他语言中都是运行时 bug。

### 4.3 `serde` 零成本存档

`#[derive(Serialize, Deserialize)]` 贴在一个 struct 上，存档/读档就完成了。LLM 会生成这个 derive——开发者甚至不需要理解 serde 的内部机制。

---

## 五、学习路径：以 WoWorld 为课堂

### Month 1：Rust 基础 + 单 NPC 模拟

```
Week 1: 用 LLM 生成第一个 Rust struct（NpcIdentity + NpcPhysiology）
        cargo build → fix errors → 编译通过
        在 Rust 的 main() 中创建 1 个 NPC，打印它的状态

Week 2: 让 NPC "活"——情绪引擎（PAD 三维轴 + hunger → pleasure↓）
        纯 Rust，不涉及 Godot
        cargo test 验证：饥饿确实降低愉悦度

Week 3: 概率决策——NPC 饿了 → 找食物
        纯 Rust，输出日志：NPC #1 decided to go to the bakery

Week 4: GDExtension 原型——Godot 中显示 Rust NPC 的状态
        1 个胶囊体在 Godot 场景中，其位置由 Rust 侧驱动
```

### Month 2-3：多 NPC + 完整系统

```
- GOAP 规划器（Rust A* 搜索）
- 简易记忆系统（LMDB 读写）
- 20 个 NPC 同时在场景中运行
- MultiMeshInstance3D 批量渲染
```

---

## 六、风险与诚实评估

| 风险 | 等级 | 说明 |
|------|------|------|
| Rust 学习期过长 | 🟡 中 | 6-9 周才能达到"能改 LLM 代码"的水平——但这 6-9 周已经在写 NPC 代码了，不是纯理论学习 |
| `godot-rust` 社区 crate 的稳定性 | 🟡 中 | 不如官方 C++/C# 绑定稳定。但 LLM 可以帮助适配 breaking change |
| LLM 对 `godot-rust` API 的了解不如对 Godot C# | 🟡 中 | 在提示词中提供 `godot-rust` 的文档链接 + 示例代码作为上下文 |
| 调试跨 FFI 问题 | 🟡 中 | Rust 侧加详细日志；Godot 侧展示 Rust 传来的原始数据——两者对照 |

---

## 七、一句话总结

> **Rust 的学习曲线是真实的——但 LLM 把它从"不可逾越"变成"可以攀登"。**
>
> 不学 Rust 的全部。只学 LLM 帮不了你的那 15%——ownership、borrowing、enum/match、Result/Option、cargo。剩下的让 AI 生成。6-9 周的集中学习 + 以 WoWorld 为课堂的项目驱动——这是 Rust + Godot + LLM 组合的成功公式。
