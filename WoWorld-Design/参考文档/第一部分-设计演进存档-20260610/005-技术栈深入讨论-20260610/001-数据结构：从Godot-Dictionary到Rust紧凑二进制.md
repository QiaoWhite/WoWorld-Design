# 001-数据结构：从 Godot Dictionary 到 Rust 紧凑二进制

> 对应：`004-技术栈重新设计` §三·Rust 模拟核心设计  
> 重点：NPC 数据、记忆、关系的具体结构定义与规模测算

---

## 一、问题的本质

当前设计文档中的 NPCData 以 Godot Dictionary 为基准——这是一种方便原型但不可规模化存储的数据格式。核心问题不在于"Dictionary 慢"，而在于：

1. **类型松散**：Dictionary 的 value 是 Variant，每次访问都有类型检查开销。NPC 热路径（情绪更新每 0.2s、概率决策每 10s、记忆检索每次社交）对这些开销敏感。
2. **内存膨胀**：Variant 的最小开销约 16-24 字节。一个 `f32` 在 Dictionary 中占 24 字节而非 4 字节。100K NPC 的这个膨胀系数意味着 ~1.2GB 的本体 vs ~500MB 的紧凑格式——差了一个数量级。
3. **缓存不友好**：Dictionary 是哈希表，键值对在内存中分散存储。遍历 1000 个 NPC 的情绪状态意味着 1000 次随机内存访问，每次都可能 cache miss。紧凑的结构体数组（SoA）使 1000 个 NPC 的 pleasure 值在连续内存中，一次 cache line 加载 16 个。

---

## 二、Rust 侧 NPC 数据结构

### 2.1 核心数据结构（紧凑二进制优先）

```rust
// ===== 固定大小热数据（常驻内存，按 SoA 布局） =====

/// NPC 身份的不可变核心
#[derive(Clone, Serialize, Deserialize)]
struct NpcIdentity {
    id: u64,                          // 全局唯一 ID
    name_seed: u32,                   // 名字生成种子
    birth_day: i32,                   // 出生游戏日（负值=世界生成前出生）
    birth_location_id: u64,           // 出生地
    species: u8,                      // 物种枚举（0=人类, 1-255 待扩展）
    sex: u8,                          // 0=女 1=男 2=其他
}

/// 人格（Big Five + 扩展特质），每个维度 f32
#[derive(Clone, Serialize, Deserialize)]
struct NpcPersonality {
    // Big Five (0.0-1.0)
    openness: f32,
    conscientiousness: f32,
    extraversion: f32,
    agreeableness: f32,
    neuroticism: f32,
    // 扩展特质
    courage: f32,       // 0=懦弱 1=无畏
    honesty: f32,       // 0=狡诈 1=诚实
    diligence: f32,     // 0=懒惰 1=勤勉
    generosity: f32,    // 0=贪婪 1=慷慨
    loyalty: f32,       // 0=多情 1=忠贞
    piety: f32,         // 0=世俗 1=虔诚
}

/// 生理状态（频繁变化）
#[derive(Clone, Serialize, Deserialize)]
struct NpcPhysiology {
    hunger: f32,        // 0=饱腹 1=极度饥饿
    thirst: f32,
    fatigue: f32,
    health: f32,        // 0=濒死 1=完全健康
    stamina: f32,       // 当前可用体力
    body_temp: f32,     // 体温（受环境温度影响）
    // 生命阶段
    age_stage: u8,      // 0=儿童 1=青少年 2=成年 3=中年 4=老年
    // 疾病/伤情 ID 列表（多数 NPC 为空，用 Option 避免浪费）
    conditions: heapless::Vec<u32, 4>,  // 栈分配，最多 4 个
}

/// 情绪状态（每 0.2s 更新，热路径）
#[derive(Clone, Serialize, Deserialize)]
struct NpcEmotion {
    // PAD 三维轴（慢变量）
    pleasure: f32,      // -1.0 ~ +1.0
    arousal: f32,       // 0.0 ~ 1.0
    control: f32,       // -1.0 ~ +1.0
    // 八基色强度（快变量）
    joy: f32,
    trust: f32,
    fear: f32,
    surprise: f32,
    sadness: f32,
    disgust: f32,
    anger: f32,
    anticipation: f32,
    // 复合情绪标签（带强度）
    composite_label: u8,    // 枚举：0=无, 1=爱, 2=悔恨, 3=敬畏, 4=嫉妒...
    composite_intensity: f32,
    // 无聊度（单独追踪，影响"创新"行为）
    boredom: f32,
}

/// 技能（高基数的稀疏数据，多数技能为 0）
#[derive(Clone, Serialize, Deserialize)]
struct NpcSkills {
    // 使用 SmallVec 或固定容量数组，只存储非零技能
    entries: heapless::Vec<SkillEntry, 32>,  // 最多 32 个非零技能
}

#[derive(Clone, Serialize, Deserialize)]
struct SkillEntry {
    skill_id: u16,
    level: f32,         // 0-100
    experience: f32,    // 距离下一级的进度
    last_use_day: i32,  // 用于衰减计算
}

/// 当前行为状态
#[derive(Clone, Serialize, Deserialize)]
struct NpcAction {
    current_action_id: u16,
    action_start_minute: u32,   // 当天第几分钟开始的
    action_duration_minutes: u16,
    target_entity_id: Option<u64>,
    target_location: Option<(f32, f32, f32)>,
    interruption_priority: u8,  // 0=正常 10=紧急生存
}

/// 志向/长线目标
#[derive(Clone, Serialize, Deserialize)]
struct NpcAmbitions {
    entries: heapless::Vec<Ambition, 3>,  // 最多 3 个志向
}

#[derive(Clone, Serialize, Deserialize)]
struct Ambition {
    ambition_type: u8,      // 枚举：致富/掌权/复仇/知识/艺术/...
    target_id: Option<u64>, // 目标对象（如复仇对象的 NPC ID）
    progress: f32,          // 0-1
    commitment: f32,        // 投入程度（低投入的在线下可能被放弃）
}

// ===== 热数据总装（SoA 布局：每个字段独立数组） =====

/// 模拟核心持有的全部 NPC 数据。
/// 使用 Struct-of-Arrays 布局以优化缓存。
struct NpcRegistry {
    // --- 不可变或慢变数据 ---
    identities: Vec<NpcIdentity>,       // 100K 个
    personalities: Vec<NpcPersonality>,  // 100K 个

    // --- 每 0.2s 更新的热数据（SoA 核心） ---
    physiology: Vec<NpcPhysiology>,     // 100K 个，但大多数 L3 不会频繁更新
    emotions: Vec<NpcEmotion>,          // 100K 个，L1+L2 频繁更新
    actions: Vec<NpcAction>,            // L1+L2 有实际值，L3 为 NONE

    // --- 稀疏数据 ---
    skills: Vec<NpcSkills>,             // 多数 NPC 的 vec 为空或很小
    ambitions: Vec<NpcAmbitions>,       // 多数 NPC 的 vec 为空

    // --- 索引（运行时构建，不序列化） ---
    id_to_index: HashMap<u64, usize>,    // ID → 数组索引
    // 空间八叉树：快速查询"玩家周围 50m 内的所有 NPC"
    spatial_index: Octree<NpcRef>,

    // --- LOD 状态（运行时） ---
    lod_levels: Vec<SimLod>,            // 每个 NPC 当前的模拟等级
}

#[derive(Clone, Copy, PartialEq)]
enum SimLod {
    L1, // 全模拟（~1000 个）
    L2, // 轻量（~10000 个）
    L3, // 统计（~89000 个）
}
```

### 2.2 内存规模测算

| 结构 | 单个大小 | ×100K | 合计 |
|------|---------|-------|------|
| NpcIdentity | 32 B | 100K | ~3.2 MB |
| NpcPersonality | 44 B | 100K | ~4.4 MB |
| NpcPhysiology | 44 B | 100K | ~4.4 MB |
| NpcEmotion | 56 B | 100K | ~5.6 MB |
| NpcAction | 40 B | 100K（L3 为默认值） | ~4.0 MB |
| NpcSkills | 平均 40 B | 100K | ~4.0 MB |
| NpcAmbitions | 平均 16 B | 100K | ~1.6 MB |
| 索引开销 | ~24 B/NPC | 100K | ~2.4 MB |
| **总计** | | | **~30 MB** |

> 对比：原 Dictionary 方案 ~180MB（100K NPC）。Rust 紧凑二进制方案仅 ~30MB——6x 压缩。加上记忆系统 ~480MB（见 004-技术栈重新设计 §3.5），运行时总内存约 **~510MB**。在现代 32GB PC 上完全可接受。

---

## 三、LMDB 中的记忆与关系数据

### 3.1 事件事实数据库

```rust
// Key: event_id (8 bytes, big-endian u64)
// Value: bincode 编码的 EventFact

struct EventFact {
    timestamp: i64,              // Unix 时间戳
    game_day: i32,               // 游戏内第几天
    location_id: u64,            // 发生地点
    event_type: EventType,       // u8 枚举
    participants: Vec<u64>,      // 参与者的 NPC ID 列表（最多 50 个）
    objective_summary: String,   // 简短的客观描述（最多 128 字节）
    impact_score: f32,           // 0-1
    is_public: bool,             // 公开事件（可被非参与者听闻）还是私密事件
}

// 单条 EventFact 约 40-180 字节（取决于 participant 数量和 summary 长度）
```

### 3.2 NPC 主观记忆数据库

```rust
// Key: (npc_id: u64, event_id: u64) 复合键，共 16 字节
// Value: bincode 编码的 NpcMemory

struct NpcMemory {
    emotional_encoding: EmotionalEncoding,
    personal_attribution: u8,  // 枚举：自我归因/他人归因/环境归因/命运归因
    perspective_bias: u8,      // 枚举：无/自我辩护/效价扭曲/掌控感扭曲
    compression_level: u8,     // 0=完整 1=轻度压缩 2=中度 3=摘要
    last_access_day: i32,      // 上次回忆的日期（用于遗忘曲线）
    is_direct: bool,           // 直接经历还是从他人处听来
    source_npc_id: Option<u64>,// 如果是听来的，来源是谁
    // 压缩后字段：
    //   level 0: 完整，约 48 字节
    //   level 1: 去除了 attribution/bias 细节，约 32 字节
    //   level 2: 仅保留 emotional_encoding 概要和 impact，约 16 字节
    //   level 3: 仅保留 event_id 引用和 impact，约 8 字节
}

struct EmotionalEncoding {
    pleasure: f32,          // 当时感受到的愉悦度
    arousal: f32,           // 当时的激活度
    control: f32,           // 当时的掌控感
    dominant_emotion: u8,   // 主导情绪枚举
    dominant_intensity: f32,// 主导情绪强度
}
```

### 3.3 关系数据库

```rust
// Key: (npc_a: u64, npc_b: u64) 复合键，其中 npc_a < npc_b（保证唯一性）
// Value: bincode 编码的 Relationship

struct Relationship {
    affection: f32,          // -1.0(恨) ~ +1.0(爱)
    trust: f32,              // 0.0 ~ 1.0
    familiarity: f32,        // 0.0 ~ 1.0（互动越多越熟悉）
    dominance: f32,          // -1.0(a服从b) ~ +1.0(a支配b)
    relationship_tag: u8,    // 枚举：陌生人/熟人/朋友/密友/恋人/配偶/对手/仇人/...
    first_met_day: i32,      // 第一次互动日期
    last_interaction_day: i32,  // 最近互动日期（用于衰减）
    interaction_count: u32,  // 总互动次数
    notable_shared_events: heapless::Vec<u64, 8>,  // 共同经历的重要事件 ID
}

// 单条 Relationship 约 48 字节
// 实际关系数：100K NPC × 平均 50 条关系 = 500 万条 × 48 B ≈ 240 MB
```

---

## 四、紧凑二进制的序列化格式

### 4.1 存档格式（`npc_data.bin`）

```
文件头:
  magic: [u8; 4] = b"WOWO"
  version: u32
  npc_count: u32
  checksum: u32  (CRC32 of remaining data)

数据段（SoA 布局，按字段顺序排列）:
  [所有 identity 连续排列]      // npc_count × 32 bytes
  [所有 personality 连续排列]   // npc_count × 44 bytes
  [所有 physiology 连续排列]    // npc_count × 44 bytes
  [所有 emotion 连续排列]       // npc_count × 56 bytes
  ...
```

SoA 布局的优势：
- **存档时**：直接将整个 Vec 写入文件，零复制
- **选择性加载**：加载存档时可以先只加载 identity 以建立索引，其余段延迟加载
- **部分更新**：增量存档时可以只写入发生变化的段

### 4.2 对比 JSON

| | JSON (原方案) | Binary SoA (新方案) |
|---|---|---|
| 100K NPC 存档大小 | ~450 MB | ~30 MB |
| 存档写入时间 | 数秒-数十秒 | 磁盘带宽限制（~0.1s @ SSD） |
| 选择性加载 | 不可行（需完整解析 JSON） | 天然支持（每个段独立） |
| 可读性 | ✅ 人类可读 | ❌ 需专用工具查看 |
| 方案 | 调试用：同时导出 JSON 快照（仅 L1 NPC 的完整数据） | 生产存档：二进制 |

---

## 五、为什么这件事必须在一开始就做对

数据结构是架构决策中最难逆转的一种。以下是一个具体的迁移场景来说明这个代价：

**场景**：你从 Godot Dictionary 开始，经过一年的开发，NPC 系统（情绪、GOAP、记忆、社交）全部围绕 Dictionary 的键值读写构建。此时你发现 1000 个 NPC 的内存已经 300MB，性能开始显著下降。你决定迁移到紧凑二进制。

**你需要改什么**：
- 每一个读写 `npc["emotion"]["pleasure"]` 的地方（可能有数百处分散在数十个 `.gd` 文件中）
- 序列化/反序列化逻辑
- GOAP 规划器访问世界状态的方式
- 内存中所有 NPC 数据的布局
- 调试工具和可视化面板

**这不是"重构"——这是重写。** 而如果你一开始就用紧凑结构体，这个迁移根本不需要发生。

> **原则**：数据结构一旦被多个模块依赖，更改成本就指数增长。所以数据结构的选择必须在任何模块开始构建之前做对。
