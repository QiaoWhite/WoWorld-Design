# NPC活人感模块——正式开发规格

> **开发代号**: WoWorld (Wonder World)
> **引擎**: Godot 4.6 LTS + Rust (GDExtension)
> **开发者**: 独立游戏开发者（Solo）
> **版本**: v2.0 — Rust 重写版
> **日期**: 2026-06-20
> **状态**: 开发规格
> **定位**: WoWorld NPC 系统的权威实现规格。从数据合同到心智算法到分层调度到物理表达——覆盖 NPC 的完整心智与行为。
> **关联**: [[../技术栈方案/001-WoWorld正式技术栈方案v3|技术栈方案 v4.0]] · [[CHG-007]] · [[CHG-008]] · [[CHG-009]] · [[CHG-027]] · [[CHG-028]] · [[CHG-029]] · [[CHG-032]] · [[CHG-041]] · [[CHG-042]] · [[CHG-057]] · [[CHG-058]]

---

## 定位

NPC 活人感模块是 WoWorld "故事生成器"哲学的核心承载者。开发者定义人格参数、行动可能性、情绪规则、文化传播率；具体故事由 NPC 局部互动以概率和统计的方式自然涌现。

**核心设计原则**：
- **开发者只创造法则，不创造故事**——涌现优先
- **NPC 不能"查询数据库"**——必须通过模拟感官感知世界（详见 [[../感官与知觉系统/README|感官系统]]）
- **玩家 AI = NPC AI = 同一套 Rust 代码**（半自动战斗参见 [[../战斗/README|战斗系统]]）
- **Life 是基类，NPC 心智是上层**——BodyState 由 Life 定义，GOAP/记忆/情绪由 SapientMind 叠加
- **从行为模拟到存在模拟**——NPC 不是"执行任务"，而是"过日子"

## 文档结构

本模块采用**总纲 + 子模块**两层结构。总纲 (`NPC活人感开发文档ver2.0.md`) 定义数据合同、心智架构、分层调度和物理表达。各子模块深化专项领域。

### 根级文档

| 文件 | 内容 | 状态 |
|------|------|------|
| [NPC活人感开发文档ver2.0.md](NPC活人感开发文档ver2.0.md) | ★ 总纲——数据合同(NpcData struct)·六层模型·三大决策环·情绪引擎·记忆系统·社会关系·GOAP安全网·分层模拟L1-L4·物理表达 | ✅ v2.0 |
| [02-性别与吸引力系统.md](02-性别与吸引力系统.md) | 性别定义·吸引力参数·性取向分布·社会影响 | ✅ v2.0 |

### 子模块

| 编号 | 目录 | 内容 | 状态 | CHG |
|------|------|------|------|-----|
| **03** | [基本需求系统](03-基本需求系统/) | 6 篇——7维需求统一框架(饥饿/口渴/疲劳/温度/排泄/安全/社交)、ConsumableEffect schema(Life定义)、element_surplus[8]、NeedSensitivity 8字段、GOAP安全网不扩展 | ⚠️ 待审核 | CHG-027 |
| **04** | [进阶需求系统](04-进阶需求系统/) | 9 篇——三层需求(生存→心理→成长)、esteem_deficit/competence_frustration、survival_suppression() sigmoid、frustration_regression() ERG挫折回归 | ⚠️ 草稿 | CHG-028 |
| **05** | [审美与艺术](05-审美与艺术/) | 10 篇——AestheticSignal 6维、judge()纯函数零副作用、HasAestheticSignal trait 12实现者、4事件原子(React/Articulate/Adopt/Embellish)、FineArts技能大类 | ⚠️ 草稿 | CHG-029 |
| **06** | [认知与智慧系统](06-认知与智慧系统/) | 8 篇——CognitiveStyle 4维认知风格(含阻尼)、CognitiveTide 3维潮汐、MentalModel(≤20条·6路径跨代传递)、ThoughtTrigger 6类触发+ThoughtFragment 3级清晰度、睡眠正则化(过拟合大脑假说)、创新管线6阶段→6领域对接、MindAttribution Theory of Mind | ✅ v1.0 | CHG-032 |
| **07** | [生命周期系统](07-生命周期系统/) | 11 篇——NPC从受孕到死亡+死后痕迹的完整生命历程。6核心原则：零年龄门控/零系统开关/连续模型/中性事件/平等性/统一时间流速。AgeClock纯函数+Gompertz衰老+InfantDependency状态机+FertilityPotential曲线+DeathCause 30种6类+玩家双角色死亡继承 | ✅ v1.0 | CHG-041 |
| **08** | [NPC行动涌现与分类](08-NPC行动涌现与分类/) | 12 篇——三层原子架构(35物理基元+~40领域复合原子+~25社会抽象原子)。AgentSnapshot连续能力快照。MaterialProperties数据驱动涌现。IK+碰撞箱战斗管线。KnowledgeSeed知识种子。execution_noise技能精度。零年龄门控·零硬编码禁止·零预设战斗动画 | ✅ v1.0 | CHG-042 |

## 架构速览

```
六层模型（由内向外）:
1. 数据本体 (NpcData struct)  — ~4.3KB/L1 NPC
2. 心智核心 (Emotion + Memory + Decision + Social + Culture + Spatial)
3. 感官外壳 (Visual + Auditory + EmotionPerception + SkyPerception)
4. 物理躯体 (Rust骨骼矩阵 → Godot GPU skinning)
5. 社会投影 (其他NPC记忆 + 共识场中的"他")
6. 统计层 (L4 — 超远距纯统计NPC，无个体实例)

三大决策环:
- 本地概率决策 (~90%): 文化习俗×个人习惯×人格偏移×情绪修正×需求-行动匹配 → 加权随机采样
- GOAP 安全网 (~9%): 仅生存需求(饥饿<0.3/口渴<0.25/疲劳>0.9/重伤/致命威胁)
- LLM 增强 (~1%): 复杂社交场景——可选·安全网关·速率限制

分层模拟 L1-L4:
- L1: 玩家邻近50m — GOAP全量·完整记忆·完整物理
- L2: 50m-300m — 简化GOAP·记忆摘要·LOD骨骼·概率行为树
- L3: 300m-3km — 统计代理·天节奏·关键事件通知·统计输出
- L4: 3km+ — 纯统计·人口计数·无个体实例
```

## 关键参数速查

| 参数 | 值 | 说明 |
|------|-----|------|
| **NPC 总数** | ~100K | 全量预生成 |
| **NPC 记忆上限** | 2000 条 | 含遗忘曲线+记忆重构 |
| **L1 NPC 数量** | ≤50 | 50m 内全量模拟 |
| **NpcData 大小** | ~4.3KB | L1 NPC（含概念与语言地基新增字段） |
| **总内存** | ~430MB | 100K NPC @ 4.3KB |
| **CPU 预算** | ≤3ms/帧 | Rust rayon 并行 |
| **GOAP 搜索深度** | ≤5 | 安全网不扩展 |
| **面部表情** | 512² 图集 | 16嘴×16眉×8眼 |

---

> **关联**: [[../技术栈方案/001-WoWorld正式技术栈方案v3|技术栈方案 v4.0]] · [[../生命/README|生命系统]] · [[../感官与知觉系统/README|感官系统]] · [[../战斗/README|战斗系统]] · [[../概念与语言地基/README|概念与语言地基]]
