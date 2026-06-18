# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

**WoWorld (Wonder World)** — 与其说是一个游戏，不如说是在以游戏的方式创建另一个"人世"。程序化生成的体素开放世界RPG，独立开发中。

**核心设计哲学**：WoWorld 是一个**故事生成器**，不是一个故事容器。开发者只定义世界的底层规则（人格参数、行动规则、情绪规律、文化传播率），具体的故事由NPC之间的互动以概率和统计的方式自然涌现。每个存档都是一个完全不同的世界。游戏没有"通关"的概念。

**"人世"的关键维度**：
- **完整的人**：NPC 有记忆（上限2000条）、预测、情感、思想、人际关系、欲望、习惯——与"人"打交道是核心
- **历史深度**：玩家进入时世界已运转千百万年，书本和遗迹中保留着先民活过的证据
- **冒险与生活平等**：冒险者、铁匠、商人、政客——每条路都是"主线"，同一套底层系统支撑
- **全球多元文化**：西方剑与魔法为基底，全球文化元素自由融入，文明自行演化
- **无限探索**：25万km²初步世界（≈英国面积），海:陆≈7:3，700m+山峦。Minecraft级无限世界为最终目标

**尚无游戏代码**，本仓库纯粹是设计文档。技术方案历经 001→015（v1.0）→016（v2.0）→018（v3.0）→**v4.0（当前权威，经技术栈全量审计升级）** 演进。

> ⚠️ **重要说明**：WoWorld 目前仅处于理念设计阶段。设计文档中提及的任何代码方案、伪代码、数据结构示例等，仅用于辅助阐明设计理念，在实际开发中随时可能大幅重构甚至推翻重写。
>
> **本仓库是纯设计文档仓库**——没有代码、没有构建系统、没有测试。唯一工具是 `git` 和 **Obsidian**（用于 `[[wikilink]]` 导航）。当前无 `woworld/` 代码目录。

**当前活跃的开发工作**：最新完成 [[WoWorld-Design/Happy Game/开发阶段/模型动作与物理系统/001-模型动作与物理系统总纲|模型动作与物理系统 v1.0]]（2026-06-17）。模块累计 22 个独立系统（含22个子模块），~90,000行正式开发规格。

## 文档结构

所有设计文档在 `WoWorld-Design/` 目录下，使用 **Obsidian** 编辑，大量使用 `[[wikilink]]` 交叉引用。文档语言为中文。

### `Happy Game/` — 核心设计文档
- `欢迎.md` — 项目总览 | `想法/WoW World/总设计草稿.md` — 总设计文档
- `想法/` — 概念设计/脑暴（`#草稿` = 早期文档，允许矛盾模糊未决）
- `开发阶段/` — 正式开发规格（权威规格）。关键子目录：
  - `游戏概述.md` / `README.md` / **`技术栈方案/`**（★ v4.0 权威技术方案）
  - `NPC活人感模块/` — NPC ver2.0 + 基本需求(7维) + 进阶需求(三层) + 审美与艺术 + **认知与智慧**
  - `模型动作与物理系统/` — 模型定义(骨架/面部图集) + 动画(38姿态/15轨迹/9层栈) + 空间查询(4 trait) + 物理响应
  - `文化系统/` — 8篇~10,000行（含 `信仰系统/` 10篇 + `权力系统/` 9篇）
  - `战斗/` 14篇 | `魔法/` 19篇 | `世界生成/` 9篇 | `生命/` 12篇 | `历史/` 6篇
  - `经济系统/` 9篇 | `载具系统/` 10篇 | `音频系统/` 9篇 | `感官与知觉系统/` 8篇

### `Change/` — 设计变更追踪
按 `CHG-XXX-简短描述-YYYYMMDD.md` 命名。CHG-001~006 早期变更，CHG-007~013 审计与重构，CHG-014~019 地基模块，CHG-022~033 业务系统（经济→模型动作物理）。详见 `Change/README.md`。

**`Change/hand/`** — 用户直接设计反馈。修改涉及的设计决策时，需检查此目录是否有相关意见。

### `参考文档/` — 参考性设计文档
按 `NNN-简短描述-YYYYMMDD` 格式组织。001-015 已归档。活跃文档 017-030 覆盖测试记录、技术栈、各系统设计探讨。详见 `参考文档/README.md`。

## 技术栈（v4.0 权威方案）

> 📘 **权威技术方案见 `开发阶段/技术栈方案/001-WoWorld正式技术栈方案v3.md`**。v4.0 经 [[参考文档/031-技术栈全量审计-20260618/|技术栈全量审计]] 升级。以下为概要。

- **引擎**: Godot 4.6 LTS | **模拟语言**: Rust (stable 1.80+) — GDExtension 集成
- **体素**: 自建 Transvoxel（Rust 侧）→ Godot ArrayMesh。垂直稀疏Chunk + Clipmap LOD。分层密度场11层。海拔~1500m（700m+山），8级LOD，3km切换Signed Heightfield
- **海洋**: Gerstner程序化波，`OceanProvider` trait 预留FFT升级。海:陆≈7:3
- **天空**: 混合体积云——2D impostor + 3D体积密度场 + 噪声高云。随天气/季节变化
- **画面**: 3D低多边形 + flat/cel渲染（TABS风格——单diffuse pass，无PBR）。512²面部图集驱动表情
- **NPC AI**: GOAP规划（安全网，9%）+ 概率行为树（日常，90%）+ 可选LLM增强（1%）。SoA + rayon并行。分层模拟 L1/L2/L3/L4
- **战斗**: 半自动——玩家AI = NPC AI = 同一套Rust代码。三层模型（本能→节奏→战略）。招式积木+流派涌现+信息不对称。详见 `开发阶段/战斗/`
- **动画**: 9层可叠加动画栈——Rust CPU批量骨骼矩阵(≤0.5ms) → Godot GPU skinning(双骨蒙皮)。38模块姿态+15基元轨迹。涌现式步态(9参数从BigFive派生)。面部表情512²图集驱动
- **世界生成**: 全球噪声→区域骨架→局部细节。11阶段管线。种子确定性+增量存档
- **载具**: 魔力驱动火车/帆船。移动参考系。NPC集体建造
- **建造/蓝图**: 三途径——编辑器/TOML导入/委托NPC。NPC团体集体施工
- **数据库**: LMDB — 流式Chunk存储。双层记忆架构
- **时间/天气**: Rust侧TimeManager + WeatherManager；Godot Sky shader天体渲染
- **Mod**: TOML数据驱动——调节涌现乘数，不编写行为脚本。无脚本引擎
- **模型/动画/物理**: 33骨(L1)/35骨(L0)人形骨架。9层动画栈+38模块姿态+15基元轨迹。面部512²图集驱动。Rust侧空间查询(4 trait)。仅玩家保留PhysicsServer3D。TABS式cel渲染
- **架构**: Rust模拟核心 → GDExtension → Godot客户端（渲染/UI/音频/输入/玩家物理）
- **硬件目标**: GTX 1660 SUPER 6GB VRAM。VRAM~4.2GB/6GB，内存~1.4GB/32GB，帧预算16.7ms（60fps）
- **美术**: AI生成(Stable Diffusion) + 手动调整(Blender)
- **平台**: Windows / Linux / macOS

## 文件格式

| 格式 | 用途 | 注意事项 |
|------|------|----------|
| `.md` | 所有设计文档（主要格式） | — |
| `.canvas` | Obsidian画布文件 | **二进制JSON，不可当文本编辑** |
| `.svg` | Mermaid导出图表 | — |
| `.png` | 粘贴的图片资产 | — |
| `.base` | Obsidian模板文件 | 用于创建新文档的模板 |

## 文档元数据约定（Frontmatter）

所有设计文档在开头使用 `> ` 块引用格式标注元数据（非 YAML frontmatter）：

```markdown
> **开发代号**: WoWorld (Wonder World)
> **引擎**: Godot 4.6.2
> **开发者**: 独立游戏开发者（Solo）
```

## 项目配置

- **Obsidian**: `.obsidian/` 目录包含工作区配置，无需手动编辑
- **.gitignore**: `.claude/` 目录已加入 gitignore
- **Git远程**: `git@github.com:QiaoWhite/WoWorld-Design.git` (SSH)

## 跨模块接口契约（关键——避免冲突复发）

> 📘 **完整契约表详见 [[CLAUDE-INTERFACES.md]]**。以下为 CHG-013 基座契约（最常涉及的跨模块概念）和各 CHG 摘要。

### 基座契约（CHG-013）

| 概念 | Owner | 消费方 | 关键约定 |
|------|-------|--------|---------|
| Vitals / Physiology | **Life** 004 | NPC | Physiology=主观感知层(0-1归一化), from_vitals()派生——不可删除 |
| Mana (u32刻) | **Life** §四 | Magic/Combat | 运行时u32。恢复速度以Magic为准(活跃1-3%/h, 冥想5-10%/h) |
| 10元素 | **Magic** 002 | Combat/Life/History | 雷=声音/振动≠电=闪电。金木水火土风雷电血灵——顺序固定 |
| 部位伤害 | **Combat** 012 | Life/NPC | 渐进三档(轻/中/重伤)——非二值。仅影响数值,不影响攻击模组 |
| 群系 | **World Gen** 002 | Life | 离散标签仅显示/日志——生成消费连续参数场 |
| AetherImprint | **History** 004 | Life | 查询统一为History 006的AetherQuery trait |
| Spirit消耗 | **Life** 004 §四 | Combat | 常规=Mana(安全), 过载=raw spirit(渐进症状→不可逆死亡) |
| 海洋深度 | **Life** 003/005 | World Gen | 深渊=4000m。透光0-200/中200-1000/深1000-4000/深渊4000+ |
| 死亡原因 | **Life** 004 §九 | History | 25种5大类: 物理7/环境6/生物5/魔法4/时间与特殊3 |

### 后续 CHG 契约摘要

| CHG | 模块 | 核心契约要点（详见 [[CLAUDE-INTERFACES.md]]） |
|-----|------|----------------------------------------------|
| 014 | 物品系统 | ItemDefId/ItemEntId(u64)、Quality×Rarity、BodyPlan自动派生、Assembly组件树+JointType、Enchantment卡槽+直接双模式 |
| 015 | 技能系统 | SkillId(u64)5分类22子组、SkillEntry稀疏存储、三层天赋(MentalAccess×天生×交叉训练)、TeachingRisk trait、CrossTraining非递归 |
| 016 | 天气系统 | WeatherQuery::sample()零事件总线、SeasonClock纯时间函数(120d/y·48min/d)、双层温度、Markov 6状态+雾、极端天气参数组合 |
| 017 | 语言表达 | LanguageId(u32)/ScriptId(u16)、ExpressionRef 8B句柄、ContentResolver trait注册、片段组合文本(~430片段)、PhaticLayer五类应酬 |
| 018 | 语言表达v1.1 | 五传播通道+失真算子、DeceptionIntent四种、CommunicationNorms→文化系统(★所有权转移)、GestureCultureMapping→文化系统 |
| 019 | LLM增强层 | LlmSceneConfig 19场景开关、LlmBackend trait(本地5+云端6)、VoiceProfile、TtsEngine trait、VoicePriority五级 |
| 022 | 经济系统 | 价格从订单簿涌现(禁止直写)、Market≠物理地点、两阶段提交、NpcEconomicState trait注入、中间商四条件涌现、五大货币稳定器 |
| 023 | 权力系统 | 17 UniversalPowerAtom、PowerTopology有向多重图、Legitimacy 5因子公式、Duty制裁塌缩链、Polity 4条件涌现、外交6因子 |
| 024 | 文化系统 | CultureCoreParams 10核心参数(f32)、CommunicationNorms迁入、障碍Voronoi空间模型、RitualDef统一原子、四路径文化演变 |
| 025 | 信仰系统 | FaithTheology 10参数、实践优先模型(无"神学立场")、FaithQuery 30方法、NPC→NPC接触传染5渠道、ReligiousReproductionNorms迁入 |
| 027 | 基本需求 | ConsumableEffect schema(Life定义)、7维需求统一框架、element_surplus[8]、NeedSensitivity 8字段、GOAP安全网不扩展 |
| 028 | 进阶需求 | 三层需求(生存→心理→成长)、esteem_deficit/competence_frustration、survival_suppression() sigmoid、frustration_regression() ERG挫折回归 |
| 029 | 审美系统 | AestheticSignal 6维、judge()纯函数零副作用、HasAestheticSignal trait 12实现者、4事件原子(React/Articulate/Adopt/Embellish)、FineArts技能大类(0x06) |
| 030 | 音频系统 | SoundFootprint物理模型、AudioQuery 30方法、VoiceProfile迁入音频、五类声音、传播引擎(衰减/吸收/风/温度/遮挡/多普勒)、话语清晰度五档 |
| 031 | 感官系统 | PerceptBatch统一产出、VisionQuery/ScentQuery/SpatialQuery trait(woworld_core)、PerceptualCache LRU、DarkAdaptation指数松弛、AestheticFrameworks四过程、感官噪声确定性种子 |
| 032 | 认知与智慧系统 | CognitiveStyle 4维认知风格(含阻尼)、CognitiveTide 3维潮汐、MentalModel(≤20条·6路径跨代传递)、ThoughtTrigger 6类触发+ThoughtFragment 3级清晰度、睡眠正则化(过拟合大脑假说)、创新管线6阶段→6领域对接、MindAttribution Theory of Mind、零新trait·零新调参旋钮·全部已有维度派生 |
| 033 | 模型动作与物理 | TerrainQuery/EntityIndex/SpatialEventBus/VisibilityQuery四trait空间查询、33骨(L1)/35骨(L0)人形骨架、双骨蒙皮、512²面部图集(16嘴×16眉×8眼)、38模块姿态+15基元轨迹、9层动画栈、步态9参数从BigFive派生、CombatStyleParams8参数流派涌现、COM抛物体飞行、骨架松弛死亡(替代布娃娃)、仅玩家保留PhysicsServer3D |

**冲突修正原则**：不删除原有设计。通过建立正确的派生/引用/映射关系消除冲突。两个模块定义同一概念的不同抽象层时——建立派生关系而非强制合并。有疑问时先与用户确认，不要从根上削减原有设计。

## 工作约定

- 所有新设计文档使用 `.md` 格式，放在对应的 `想法/` 或 `开发阶段/` 子目录下
- 使用 Obsidian wikilink 语法 `[[路径/文件名]]` 引用其他文档。**跨模块引用必须加 `[[]]`**——这是用户明确要求，方便 Obsidian 导航
- 新文档跟随 `> ` 块引用 frontmatter 风格
- 后续Godot项目代码将放在 `woworld/` 目录下（目前尚不存在）
- **设计变更**：对多个文档的结构性修改，在 `Change/` 按 `CHG-XXX-简短描述-YYYYMMDD.md` 创建变更文档。CHG文档之间及CHG与参考文档之间用 `[[]]` 交叉引用
- **用户设计反馈**：`Change/hand/` 目录存储用户对具体设计问题的直接裁决。涉及已有模块的修改时，先检查是否有相关用户意见
- **参考文档**：在 `参考文档/` 中创建 `NNN-简短描述-YYYYMMDD` 格式子文件夹，内部文档从 001 编号
- **技术决策**：以 `开发阶段/技术栈方案/` 为权威依据。所有 001-016 为历史演进存档，017 为测试方法论，018 已正式迁移至开发阶段
- **规划文件**：项目根目录的 `task_plan.md`、`findings.md`、`progress.md` 为 planning-with-files 工作流文件，用于追踪任务进度和设计决策。这些文件由 Claude 自动维护，不应手动编辑
- **修改后必须自检**：完成跨模块修改后，重新审计所涉及的模块间接口——确保没有引入新冲突

## 新模块设计标准工作流程

创建或重大扩展一个模块时，遵循以下流程（参考 CHG-025/026/027/028 的实践）：

1. **参考文档草稿**：在 `参考文档/` 创建 `NNN-模块名设计探讨-YYYYMMDD/设计草稿/` 文件夹
   - 001-理论框架与维度论证
   - 002-NNN-维度/子系统深化设计（可拆分为多篇）
   - 00N-跨模块依赖与接口全面清单 ★（关键——枚举所有涉及本模块的现有+潜在模块）
   - 00N-决策器/集成方案
   - 00N-性能预算与存储分析
   - 草稿阶段可自由创建多层子文件夹和文档——目的是深入讨论、理清思路、反复调整
2. **跨模块审计**：基于草稿分析，完整列举所有其他模块涉及本模块的内容
   - 本模块为权威 Owner——其他模块中冲突/遗漏/不合理的部分应以本模块设计为准
   - 同时列举与潜在未来模块的联系
3. **正式开发规格**：整合讨论结果，写入 `开发阶段/` 对应目录
   - 极尽清晰完整——关键概念必须有明确说明
   - 自审：是否与讨论内容有偏移、遗漏、矛盾冲突
4. **修改关联文档**：检查本文档与其他文档配合的地方，在原文档需要改动处进行改动
5. **审查**：对所有改动进行审查——内部一致性 + 跨文档对齐 + 讨论→规格漂移
6. **CHG 文档**：在 `Change/` 创建 `CHG-XXX-模块名v1.0创建-YYYYMMDD.md`
7. **更新 CLAUDE.md**：添加新模块条目 + CHG 条目 + 接口契约表。同步更新 [[CLAUDE-INTERFACES.md]] 契约表
