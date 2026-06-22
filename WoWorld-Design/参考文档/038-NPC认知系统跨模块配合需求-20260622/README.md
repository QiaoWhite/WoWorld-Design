> **参考编号**: 038
> **标题**: NPC认知系统跨模块配合需求
> **日期**: 2026-06-22
> **关联**: [[../../Happy Game/Change/CHG-057-NPC认知系统深度设计-20260622|CHG-057]] · [[../../Happy Game/开发阶段/NPC活人感模块/06-认知与智慧系统/README|06-认知与智慧系统]]

---

# 038 — NPC 认知系统跨模块配合需求

CHG-057 在 NPC 认知系统中引入了多项架构裁决。本文档枚举**其他模块需要怎么配合**以及**配合的详细理由**。

---

## 一、woworld_core（全局基础设施）

### 新增类型

| 类型 | 理由 |
|------|------|
| `PatternExpression` + `PatternStep` | 认知系统的数学地基。MentalModel、Creative Leap、学科涌现、修辞渲染全部依赖它。定义在 woworld_core 因为 Creative Leap 的 LSH 计算是纯数学（零域知识），且所有领域模块需要消费它 |
| `DomainSignature(u64)` | 替代 MentalModelDomain 枚举。领域分类从原子类型组合涌现。woworld_core 定义，所有模块平等引用 |
| `AtomClass(u16)` | 统一原子分类——物理/权力/未来扩展的命名空间 + 索引。`IntoAtomClass` trait 允许各域 crate 注册自己的原子类型而不需核心层知道具体类型 |
| `EmotionalCharge` | 情绪感知信号的统一类型。被感官系统生产、认知系统消费 |
| `GazeEstimate` | 视线感知信号。纯几何计算，零域知识 |
| `PerceptBatch` extension methods | `enrich()`, `bind_modalities()`, `estimate_gaze()`, `read_emotions()`。桥梁层——纯函数，所有消费者平等访问 |

### 不做什么

- **不修改 `PatternSignature`**——那是概念与语言地基的权力原子哈希，与 DomainSignature 用途不同
- **不修改 EntityKind**——抽象的 EntityContext 通过 PatternSignature 引用（而不是 EntityKind 的变体）

---

## 二、感官与知觉系统（10-感官与知觉系统）

### 需要做什么

1. **定义 `EmotionalCharge` struct 的完整字段**

   **理由**: `PerceptEntry.emotional_charge` 字段类型在 v1.0 中被引用但从未定义。认知系统的情绪传染、思维触发（ThoughtTrigger::check_emotional）、信念评估（assess_and_integrate_mental_model）全部依赖这个输入。没有它，这些功能只能在"假设有"的基础上讨论——无法进入实现。

2. **在 `PerceptEntry` 中加入 `gaze_estimate: Option<GazeEstimate>`**

   **理由**: Theory of Mind 中的"她在看我"是最基本的社交认知原子。计算是纯几何（目标朝向向量与观察者方向的夹角），应放在感知系统而不是认知系统。不增加感知成本——每 percept 一次角度运算。

3. **在 `EnvironmentPerception` 中加入 `crowd_emotional_field: f32` 和 `crowd_dominant_emotion: Option<BasicEmotion>`**

   **理由**: 人群情绪场是环境属性（如光照和噪音），不是个人属性。在感知系统内一次聚合（O(n) per percept batch），所有消费者平等访问。这个值驱动：
   - 群体可暗示性（modulates trust evaluation in belief assessment）
   - 记忆源混淆（modulates source_confidence at encoding）
   - 情绪传染的 amplification/damping

4. **实现 `PerceptBatch::enrich()`**

   **理由**: 跨模态绑定 + 情绪提取 + 视线估计的统一点。在 PerceptBatch 产出后、信念推导前执行。纯函数——NPC、Combat、Economy、Power 的 crate 都可以调用，零新依赖。

### 不做什么

- **不修改 "不整合跨模态" 的设计承诺**——跨模态绑定在 `enrich()` 中执行（不是感官系统内部），感官系统继续输出独立的视觉/听觉流

---

## 三、概念与语言地基（24-概念与语言地基）

### 需要做什么

1. **`UtteranceConcept` 加 `domain_sig: Option<u64>` 字段**

   **理由**: 修辞检测需要知道相邻概念是否来自不同领域。`domain_sig` 是 Optional——不影响现有渲染管道。当 None 时 TextGenerator 行为不变。

2. **在 `TextGenerator` 中加入修辞化渲染模式**

   **理由**: NPC 有类比思维能力（Creative Leap），但没有比喻表达能力。修辞渲染模式是这两个系统之间的连接——不新建"修辞系统"。需要扩展的 FragmentLibrary 模板（为明喻/暗喻/类比解释准备句法模板）。

3. **`compute_pattern_signature()` 扩展以接受非权力模式**

   **理由**: 当前只接受 PowerEdgeRef。抽象概念（数学定理、哲学命题）的 PatternSignature 无法被计算。需要重载/扩展以接受 PatternStep 的通用表示——使学术领域的创新能被概念系统分类。

### 不做什么

- **不 import 认知 crate**——domain_sig 是裸 u64，TextGenerator 不需要知道 DomainSignature 的语义

---

## 四、NPC 活人感模块（主文档）

### 需要做什么

1. **EventMemory 新增 `original_source: MemorySource` + `source_confidence: f32` + `decision_point: Option<DecisionPoint>`**

   **理由**: 记忆源混淆（虚假记忆）和反事实后悔（教训吸取）的数据基础。不需要新存储系统——在已有 EventMemory 中加字段。决策点在 macro_digestion 中惰性修剪（7 天窗口 ≤ 70 条）。

2. **MemoryStore 实现四层压缩架构**

   **理由**: 当前 ColdMemorySummary 无容量上限——老年 NPC 的内存泄漏。四层速率失真阶梯固定预算 ~617KB/NPC。压缩在睡眠期间执行（离线批处理，零帧开销）。L2 Era Digest 是"知识抽象"的枢纽——不需要新抽象系统。

3. **将 `crowd_suggestibility` 接入 `assess_and_integrate_mental_model()`**

   **理由**: 情绪传播和信念传播的速度差异是涌现群体心理的关键机制。crowd_suggestibility 调制 trust evaluation——情绪激烈时 NPC 更容易接受任何"提供解释"的来源。Le Bon 的 crowd suggestibility 的数学表达。

4. **deliberation_depth 接入 GOAP 决策循环**

   **理由**: 深思熟虑不应硬编码阈值。连续量 `trait_capacity × state_capacity × stakes`——醉汉不思、哲学家面对午饭也不思、哲学家面对职业改变深思。

### 不做什么

- **不修改 emotion contagion 的核心算法**——只增加 crowd_emotional_field 作为额外调制因子

---

## 五、技能系统

### 需要做什么

1. **在 Academic 分类中新增 Mathematics 子类**（5 技能：arithmetic/geometry/algebra/statistics/logic）

   **理由**: Mathematician/TaxCollector/Architect/Accountant 职业已定义但无技能支撑。这些职业需要技能来调制数学运算的精度（如 tax calculation 的 execution_noise）。走标准技能架构——不新建数学系统。

2. **新增 `COUNT`、`MEASURE`、`CALCULATE`、`DERIVE` 物理原子**（或标记为认知原子）

   **理由**: 数学运算——与 GRASP/STRIKE 一样是"身体-世界交互的基本单位"（这里身体是大脑）。DomainSignature 可以包含这些原子类型——使数学领域的 pattern 能被识别和类比。

### 不做什么

- **不建 "数学系统"**——数学运算走标准三层（物理原子 → 复合原子 → GOAP 可见），精度由 executive_noise_std(skill_level) 决定

---

## 六、存档系统

### 需要做什么

1. **认知系统的 `SaveableModule` 实现中包含 PatternExpression、四层压缩结构、DecisionPoint**

   **理由**: 存档系统按 `SaveableModule::snapshot_dirty()` 协议序列化 NPC 数据。新增字段需要被序列化——但序列化格式由 NPC crate 自决（存档系统不规定）。零新跨模块协调。

### 不做什么

- **不修改存档系统的 trait**——SaveableModule 不需要知道 PatternExpression 的存在

---

## 七、各领域 crate（Combat/Magic/Crafting/Aesthetic/Building/Power）

### 需要做什么

1. **在 startup 时调用 `register_innovation_consumer(ATOM_MASK, consumer_fn)`**

   **理由**: formalize_innovation() 从硬编码 match 改为注册制。各领域 crate 声明自己消费哪些 atom_class mask——cognitive crate 按 mask overlap 匹配。零新 trait。零新跨模块依赖方向。

2. **未注册的领域 crate → 自动走默认 AcademicWork 路径**

   **理由**: 史学、数学、哲学、天文、神学、法学等知识领域的创新自动形式化为学术著作——走已有的 WritingImpulse → LifeTrace → TextSegment → PhysicalBook 流程。不需要这些 crate 修改代码。

### 不做什么

- **不要求所有 crate 立即注册**——未注册的自动默认，零阻塞

---

## 八、协同理由总结

| 配合方 | 配合内容 | 核心理由 |
|--------|---------|---------|
| woworld_core | PatternExpression + DomainSignature + EmotionalCharge + 富化方法 | 数学地基——所有模块平等消费 |
| 感官系统 | EmotionalCharge + GazeEstimate + crowd_emotional_field | 认知系统的输入空白被填补 |
| 概念与语言地基 | domain_sig + 修辞渲染 | 类比思维→比喻表达——已有系统的连接 |
| NPC 主文档 | source_confidence + 四层压缩 + DecisionPoint | 记忆保真度→知识抽象→反事实后悔 |
| 技能系统 | Mathematics 子类 + 认知原子 | 数学职业的技能支撑 |
| 存档系统 | 新增字段序列化 | 按已有协议——零协调 |
| 领域 crate | 注册 innovation consumer | 解耦 formalize_innovation——不 import 认知 crate |

> **关联**: [[../../Happy Game/开发阶段/NPC活人感模块/06-认知与智慧系统/001-认知与智慧系统总纲|001-总纲]] · [[../../Happy Game/开发阶段/NPC活人感模块/06-认知与智慧系统/003-MentalModel与智慧积累|003-MentalModel]] · [[../../Happy Game/开发阶段/NPC活人感模块/06-认知与智慧系统/006-创新管线与跨领域对接|006-创新管线]]
