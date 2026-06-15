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
- **无限探索**：25万km²初步世界（≈英国面积），海:陆≈7:3，700m+山峦，一望无际平原。Minecraft级无限世界为最终目标

**尚无游戏代码**，本仓库纯粹是设计文档。技术方案历经 001→015（v1.0）→016（v2.0）→**018（v3.0，当前权威）** 演进。

> ⚠️ **重要说明**：WoWorld 目前仅处于理念设计阶段。设计文档中提及的任何代码方案、伪代码、数据结构示例等，仅用于辅助阐明设计理念，在实际开发中随时可能大幅重构甚至推翻重写。
>
> **本仓库是纯设计文档仓库**——没有代码、没有构建系统、没有测试。唯一工具是 `git` 和 **Obsidian**（用于 `[[wikilink]]` 导航）。当前无 `woworld/` 代码目录。

**当前活跃的开发工作**：最新完成 [[WoWorld-Design/Happy Game/开发阶段/NPC活人感模块/04-进阶需求系统|进阶需求系统 v1.0]]（2026-06-15）——三层需求模型（生存→心理→成长）+2新维度（尊重/认可+胜任挫折）+ IntrinsicGoal形式化（commitment×relevance）+2跨层桥接（sigmoid生存抑制+ERG挫折回归）。模块累计 17 个独立系统，~62,300行正式开发规格。

## 文档结构

所有设计文档在 `WoWorld-Design/` 目录下，使用 **Obsidian** 编辑，大量使用 `[[wikilink]]` 交叉引用。文档语言为中文。

### `Happy Game/` — 核心设计文档
- `欢迎.md` — 项目总览与快速导航
- `想法/WoW World/总设计草稿.md` — 总设计文档（核心哲学+全部系统概要）
- `想法/` — 概念设计/脑暴（`#草稿` = 早期文档，允许矛盾模糊未决）
- `开发阶段/` — 正式开发规格（讨论确定的、预备投入开发的结果。权威规格）
  - `游戏概述.md` — **游戏愿景与设计哲学**（创建另一个"人世"、核心竞争力、参考精神）
  - `README.md` — 模块总索引
  - `技术栈方案/` — **★ 正式技术栈方案 v3.0**（Rust+Godot架构、世界生成、性能预算、开发路线——所有技术决策的权威依据）
  - `NPC活人感模块/` — NPC系统权威规格（**ver2.0**，Rust伪代码。含 `03-基本需求系统` — 7维需求统一框架 v1.0、`04-进阶需求系统` — 三层需求模型 v1.0（尊重/认可+胜任挫折+IntrinsicGoal形式化+跨层桥接））
  - `文化系统/` — **文化系统 8 篇**（CultureCoreParams 10核心参数+三层派生架构、障碍Voronoi空间模型、CommunicationNorms所有权转移、审美/技术派生、演变四路径、地名系统31种实体类型+命名价值评分、节日与仪式系统 RitualDef统一原子+四类节日生成+权力桥接零耦合。~10,000行）
      - `信仰系统/` -- **★ 信仰系统 10 篇**（最新完成 2026-06-15。实践优先模型 ReligiousPracticeProfile、FaithTheology 10连续参数、NPC→NPC接触传染5渠道+4改变路径、FaithCalendarQuery trait实现、Divine授权事件桥接零耦合。~3,750行）
      - `权力系统/` -- **权力系统 9 篇 + README**（17普适权力原子+PowerTopology有向多重图+8条获取路径+Legitimacy 5因子+Duty制裁塌缩链+Polity涌现+外交6因子。~4,100行）
  - `战斗/` — **战斗系统 14 篇**（三层模型/信息不对称/招式积木/魔法融入/半自动HUD）
  - `魔法/` — **魔法系统 19 篇**（七层结构：十元素→魔力→施法→工程→人文）
  - `世界生成/` — **世界生成 9 篇**（11阶段管线：全球大陆架构→海洋系统→自然基底→资源→聚落→WFC建筑→交通→NPC历史→世界边界。v3 重构）
  - `生命/` — **生命系统 12 篇**（Life基类架构、智能种族/动物(v2.0水陆空三域)/灵兽/怪物/亡灵/植物(v2.0·1850行)/神明/玩家、12维积木拼装(含水生/飞行感官+运动子类型)、四层质量防线、繁衍三层社会约束）
  - `历史/` — **历史系统 6 篇**（事件因果链三层模型·趋势→力量→事件、生命痕迹七种情境·双重驱动、Work→PhysicalBook书籍模型、灵元素印记·读树、文物痕迹·关系遗产、大日志——纯功能性外置大脑·全量记录·渐进验证·纠错·关系图谱）
  - `经济系统/` — **经济系统 9 篇 + README**（限价订单簿撮合引擎+分层定价、Market/Storefront市场模型、价格从交易涌现、交易主体四条件涌现、MarketRegulations参数化经济体制、PowerAtom权力原子框架、行为经济学×NPC心智映射、货币三管道+五大自动稳定器、LLM经济增强层）
  - `载具系统/` — **★ 载具系统 10 篇 + README**（最新完成 2026-06-15。VehicleId+契书双重身份、VehicleArchetype×文化涌现→VehicleDef、五种动力类型+MagicEngine魔法集成、L1-L3半自动操控+VehicleController trait、三通道连续损伤+紧急修补+沉没事件链、记忆优先契书可选产权+伪造/篡改/检测、移动容器货运+运费NPC心智涌现、铁路P9极低概率涌现、10模块素材注入命名、VehicleQuery/VoiceMut跨模块接口。~8,000行）

### `Change/` — 设计变更追踪
按 `CHG-XXX-简短描述-YYYYMMDD.md` 命名。当前：
- CHG-001~006：早期变更（NPC数量、废止阶段规划、设计哲学、魔法系统）
- **[CHG-007](WoWorld-Design/Change/CHG-007-开发测试暴露的设计缺陷修正-20260610.md)**：30次双视角模拟测试暴露16个设计缺陷+Phase重新划分
- **[CHG-008](WoWorld-Design/Change/CHG-008-设计深度补充-世界尺度战斗天象载具城市蓝图-20260610.md)**：设计深度补充——700m山+海陆73+体积云+半自动战斗+城市规划+载具+蓝图
- **[CHG-009](WoWorld-Design/Change/CHG-009-NPC开发文档Rust重写-v3升级-20260611.md)**：NPC开发文档全面重写：GDScript→Rust伪代码+v3全系统植入
- **[CHG-010](WoWorld-Design/Change/CHG-010-世界生成v3重构-20260611.md)**：世界生成方案 v3 重构——自建Transvoxel+25万km²+海陆7:3+海洋系统+港口/游牧聚落+承载容量+两层WFC+全部参数重算
- **[CHG-011](WoWorld-Design/Change/CHG-011-生命系统v1.0设计-20260611.md)**：生命系统 v1.0 全新设计——Life基类+8大生命类型+12维程序化生成+四层质量防线+灵兽5驯服路径+亡灵7来源+神明三模式+繁衍三层约束
- **[CHG-012](WoWorld-Design/Change/CHG-012-开发文档全审计-矛盾冲突错误修复-20260611.md)**：开发阶段全文档审计——五大模块约77个矛盾/冲突/错误修复——十元素冰→电统一、饥饿/口渴方向修正、魔法wikilink全面修复、边界距离重算、材质ID命名空间分离等
- **[CHG-013](WoWorld-Design/Change/CHG-013-跨模块一致性冲突修正-20260612.md)**：跨模块一致性审计与修正——4并行代理审计67份文档发现~95冲突——修正全部11 CRITICAL + 20 HIGH——建立模块间接口契约
- CHG-014~019：物品系统/技能系统/天气系统/语言表达系统——地基模块的完整接口契约
- **[CHG-022](WoWorld-Design/Change/CHG-022-经济系统v1.0创建-20260613.md)**：经济系统 v1.0 创建——9篇开发规格+README，~7,000行。限价订单簿+分层定价+Storefront+四条件涌现+参数化体制+PowerAtom+行为经济学映射+货币稳定器
- **[CHG-023](WoWorld-Design/Change/CHG-023-权力系统v1.0创建-20260613.md)**：权力系统 v1.0 创建——9篇开发规格+README，~4,100行。17普适权力原子+PowerTopology有向多重图+8条获取路径+Legitimacy 5因子公式+Duty制裁塌缩链+Polity涌现+外交6因子公式
- **[CHG-024](WoWorld-Design/Change/CHG-024-文化系统v1.0创建-20260614.md)**：文化系统 v1.0 创建——首发6篇~3,400行。后续扩展：007-地名系统(~1,350行)+008-节日与仪式系统(~1,400行)。模块总规模 8 篇~10,000行。CultureCoreParams 10参数三层架构+障碍Voronoi空间模型+CommunicationNorms所有权转移+TechnologyProfile 8领域+RitualDef统一仪式原子+四类节日生成+权力桥接零耦合
- **[CHG-025](WoWorld-Design/Change/CHG-025-信仰系统v1.0创建-20260615.md)**：信仰系统 v1.0 创建——10篇+README，~3,750行。实践优先模型 ReligiousPracticeProfile+FaithTheology 10连续参数+NPC→NPC接触传染5渠道+4改变路径+FaithCalendarQuery trait实现+Divine授权事件桥接零耦合
- **[CHG-026](WoWorld-Design/Change/CHG-026-载具系统v1.0设计-20260615.md)**：载具系统 v1.0 创建——10篇+README，~8,000行。五种动力类型+MagicEngine魔法集成+L1-L3半自动操控+三通道损伤+记忆优先契书可选产权+移动容器货运+VehicleArchetype×文化涌现VehicleDef
- **[CHG-027](WoWorld-Design/Change/CHG-027-基本需求系统v1.0创建-20260615.md)**：基本需求系统 v1.0 创建——1篇主文档+5篇讨论草稿。7维需求统一框架（4旧升级+3新:元素平衡/libido/社交归属）+ urgency=deviation×sensitivity 统一公式 + bottleneck 瓶颈模型 + ConsumableEffect 数据合同。修改 Life/004、NPC ver2.0、Items/001/003
- **[CHG-028](WoWorld-Design/Change/CHG-028-进阶需求系统v1.0创建-20260615.md)**：进阶需求系统 v1.0 创建——1篇主文档+8篇讨论草稿。三层需求模型（生存→心理→成长）+2新维度（尊重/认可+胜任挫折）+ IntrinsicGoal形式化（commitment×relevance→偏好偏置）+2跨层桥接（sigmoid生存抑制+ERG挫折回归）。唯一新跨模块方法：CultureQueryExt::honor_weight_for_domain()。修改 03-基本需求系统、NPC ver2.0、文化系统/006
- 详见 `Change/README.md`

**`Change/hand/`** — 用户直接设计反馈。包含对跨模块冲突的具体裁决意见（如魔力恢复速度以Magic为准、部位伤害以Combat为准、spirit过载方案等）。修改涉及的设计决策时，需检查此目录是否有相关意见。

### `参考文档/` — 参考性设计文档
按 `NNN-简短描述-YYYYMMDD` 格式组织。001-015 已归档至 `第一部分-设计演进存档-20260610/`。

**当前活跃文档**：
| 编号 | 内容 |
|------|------|
| **020** | [战斗系统文档审查-缺陷不足矛盾与优化](WoWorld-Design/参考文档/020-战斗系统文档审查-缺陷不足矛盾与优化-20260611/) — 战斗系统 14 篇开发文档的综合审查报告 |
| **019** | [NPC文档重写-问题分析与优化方向](WoWorld-Design/参考文档/019-NPC文档重写-问题分析与优化方向-20260611/) — NPC ver2.0 重写中识别的 7 个潜在问题 + 4 个架构优化方向 |
| **022** | [节日系统设计探讨](WoWorld-Design/参考文档/022-节日系统设计探讨-20260614/) — 10篇原始设计~2,500行 + 5篇优化审查~1,200行。14大类79议题→正式规格008 |
| **023** | [载具系统设计探讨](WoWorld-Design/参考文档/023-载具系统设计探讨-20260615/) — 5篇讨论草稿~2,000行。载具身份+契书+动力+操控+损伤+产权+货运+Names+跨模块接口→正式规格 |
| **024** | [NPC基本需求系统设计探讨](WoWorld-Design/参考文档/024-NPC基本需求系统设计探讨-20260615/) — 5篇讨论草稿。7维需求统一框架+元素平衡+libido+社交/归属+决策器集成+性能预算→正式规格 [[../Happy Game/开发阶段/NPC活人感模块/03-基本需求系统|03-基本需求系统]] |
| **025** | [NPC进阶需求系统设计探讨](WoWorld-Design/参考文档/025-NPC进阶需求系统设计探讨-20260615/) — 8篇讨论草稿~4,000行。理论框架（马斯洛/ERG/SDT）+尊重/认可+胜任挫折+IntrinsicGoal形式化+跨层桥接+跨模块接口全面清单+决策器集成+性能预算→正式规格 [[../Happy Game/开发阶段/NPC活人感模块/04-进阶需求系统|04-进阶需求系统]] |
| **021** | [设计文档补全总体规划](WoWorld-Design/参考文档/021-WoWorld设计文档补全规划-20260613/) — Phase 13-19 全部缺失模块的总体规划 |
| **018** | [**正式技术栈方案 v3.0**](WoWorld-Design/Happy Game/开发阶段/技术栈方案/) ← **★ 当前权威方案（已迁移至开发阶段）** |
| **017** | [开发阶段测试记录](WoWorld-Design/参考文档/017-开发阶段测试记录-20260610/) — 方法论+50份双视角测试报告 |

详见 `参考文档/README.md`

## 技术栈（v3.0 权威方案）

> 📘 **权威技术方案见 `开发阶段/技术栈方案/001-WoWorld正式技术栈方案v3.md`**。以下为概要。

- **引擎**: Godot 4.6 LTS
- **模拟语言**: Rust (stable 1.80+) — 通过 GDExtension 与 Godot 集成
- **体素**: 自建 Transvoxel（Rust 侧）→ Godot ArrayMesh。**垂直稀疏Chunk**（仅地表附近存体素）+ **Clipmap LOD**。分层密度场11层（全球海陆+基础地形+地质+奇幻+植被(L6.5)+文化+NPC修改+玩家修改+天气）。海拔范围~1500m（700m+山），8级LOD覆盖0-10km+，3km切换Signed Heightfield+法线烘焙
- **海洋**: Gerstner程序化波（对标英灵神殿水面效果），`OceanProvider` trait 预留FFT升级接口。海:陆≈7:3
- **天空**: 混合体积云——2D impostor（地平线）+ 3D体积密度场（可穿越）+ 噪声高云。云随天气/季节变化。NPC温和感知天象（概率偏移，非决定）
- **画面**: 3D低多边形 + 像素纹理
- **NPC AI**: GOAP规划（安全网，占9%决策）+ 概率行为树（日常，占90%）+ 可选LLM增强（1%）。SoA数据布局 + rayon并行。分层模拟 L1/L2/L3/**L4**（超远距统计——远方城市人口基数）
- **战斗**: **半自动**——玩家AI = NPC AI = 同一套Rust代码。**三层模型**（本能逐帧→节奏0.5-1s→战略5-30s）。**招式积木**程序化生成+流派涌现。**武器模块化**（滑块参数）。**魔法融入**（七系+十元素+法术熟练度自动化）。**信息不对称**（无上帝视角）。**掩体意识+投技+地形伤害+骑乘+潜行**。详见 `开发阶段/战斗/`（14篇开发文档含README）
- **动画**: UAF启发的模块化GPU动画——Rust CPU批量骨骼矩阵 → Godot GPU skinning shader。无AnimationTree节点
- **世界生成**: 全球尺度噪声（大陆形状→海陆掩码）→ 区域地形骨架 → 局部细节。水文（河流侵蚀→入海口）。城市规划（单中心/多中心/放射/网格布局）→ WFC区域填充建筑。种子确定性+增量存档。世界边界：初步版自然海洋包裹，最终版种子无限延伸
- **载具**: 魔力驱动火车/帆船。移动参考系（载具本地NavMesh）。NPC在载具上角色分化（船员/乘客）。大型载具由NPC团体集体建造
- **建造/蓝图**: 三途径——游戏内编辑器/外部`.blueprint` TOML导入/自然语言委托NPC。NPC团体集体施工（收集材料→按蓝图逐步建造→历时数日-数月游戏时间）
- **数据库**: LMDB — 流式Chunk存储（无限世界）。双层记忆架构（事件事实库 + 主观记忆库）
- **时间/天气**: Rust侧TimeManager + WeatherManager；Godot Sky shader天体渲染；季节+天气→NPC行为乘数（温和偏移）；天气→Gerstner浪高+体积云形态联动
- **Mod**: TOML数据驱动——Modder调节涌现乘数，不编写行为脚本。无Rhai/Lua/Python脚本引擎
- **架构**: Rust模拟核心（NPC/世界生成/时间天气/Mod/战斗AI/**经济系统**/载具逻辑/蓝图施工）→ GDExtension → Godot客户端（渲染/UI/音频/输入/玩家物理/海洋shader/体积云compute）
- **硬件目标**: **GTX 1660 SUPER 6GB VRAM** 流畅运行，分发后大部分PC流畅。VRAM预算~4.2GB/6GB，内存~1.4GB/32GB，帧预算16.7ms（60fps）
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

经 CHG-013 审计确定的关键接口所有权和约定。修改以下任何概念时必须维护这些契约：

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| 生命体征 (Vitals) | **Life** `004-身体状态与生命过程` | NPC（Physiology 为计算视图，通过 `from_vitals()` 派生） | Physiology 是主观感知层（0-1归一化），不可删除——NPC的GOAP/情绪代码依赖它 |
| 魔力/法力 (Mana) | **Life** `§四` (`max_mana_ke: u32` + SpiritState + MagicAttributes) | Magic/Combat | 运行时全部用 `u32` 刻运算——避免浮点精度误差。恢复速度以Magic为准（慢速：1-3%/h活跃，5-10%/h冥想） |
| 元素体系 (10 Elements) | **Magic** `002-十元素` | Combat/Life/History | 雷=声音/振动，电=闪电/电荷——不可混淆。金木水火土风雷电血灵——10元素顺序固定 |
| 部位伤害 | **Combat** `012-战后过渡与伤势` | Life/NPC | 渐进三档软数值模型（轻/中/重伤）——不可用二值"该肢不可用"。仅影响数值，不影响攻击模组 |
| 群系 (Biome) | **World Gen** `002-自然景观`（输出连续参数场） | Life（以参数场为输入） | 离散群系标签仅用于显示/日志——生成逻辑直接消费参数场 |
| 灵元素印记 (AetherImprint) | **History** `004-灵元素印记` | Life（`CachedImprintView` 本地缓存，不独立存储） | 查询接口统一为 History 006 的 `AetherQuery` trait |
| Spirit 消耗 | **Life** `004 §四` | Combat | 常规消耗Mana（安全），过载消耗raw spirit（渐进症状→系统阻断→0%不可逆死亡） |
| 海洋深度分级 | **Life** `003`/`005` | World Gen | 深渊边界=4000m。透光层0-200m/中层200-1000m/深层1000-4000m/深渊4000m+ |
| 死亡原因 | **Life** `004 §九` | History（墓碑文本映射） | 25种，五大类：物理伤害7/环境6/生物5/魔法4/时间与特殊3 |

### CHG-014 新增契约（物品系统 v1.0）

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| 物品标识 (ItemDefId/ItemEntId) | **物品系统** `001`/`002` | 全部模块 | ItemDefId=u64全局恒定(8+56bit), ItemEntId=u64存档内唯一——旧MaterialId/MagicItemId/resource_type通过映射表桥接 |
| 物品属性 (ItemProperties) | **物品系统** `003` | 全部模块 | 核心属性+Quality(4档)×Rarity(5档)+AestheticProps——各模块在此之上叠加 |
| 装备槽位 (EquipmentSlots) | **物品系统** `004` | NPC/Life | BodyPlan自动派生——不预定义物种类型。双套Outfit切换由NPC自主决定 |
| 库存与仓储 | **物品系统** `005` | NPC/Player | 30基础槽位+容器五层体系——[TUNING]可调。货币独立于库存(CharacterWallet) |
| 物品装配 (Assembly) | **物品系统** `001` | Combat/Magic | 通用组件树框架+四种JointType(Rigid/Hinge/Chain/Flexible)——Combat/Magic注册slot_type |
| 附魔 (Enchantment) | **物品系统** `008` | Combat/Magic | 卡槽附魔(日常经济,可撤换)+直接附魔(历史锚点,永久)——两者可共存于同一物品 |
| 制造配方 (CraftingRecipe) | **物品系统** `006` | NPC/Magic | TOML数据驱动——不是代码。配方发现=学习/实验/购买/天启 |

### CHG-015 新增契约（技能系统 v1.0）

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| 技能标识 (SkillId) | **技能系统** `001`/`002` | 全部模块 | SkillId=u64全局恒定(8+8+16+32bit,5分类22子组)——NPC/Combat/Magic/物品系统的SkillId引用全部以技能系统为权威 |
| 技能实例 (SkillEntry) | **技能系统** `001` | 全部模块 | xp/level/innate_aptitude(0.7-1.3天生一次roll)/total_xp_earned/times_used——稀疏存储，不存在=untrained |
| 累积XP公式 | **技能系统** `001` | Combat/Magic/NPC | total_xp(L)=100×(e^(0.04L)-1)/0.04——[TUNING]可调。用进不退——无衰减 |
| 天赋模型 | **技能系统** `001`/`002` | NPC | 三层叠加：MentalAccess trait(0.7-1.0)/天生倍率(0.7-1.3每NPC每技能独立)/交叉训练(同组0.04-0.20,天花板min(40,0.5L),非递归) |
| 技能管辖范围 | **技能系统** `001` | NPC/物品/Combat/Magic | 5大类(Combat/Magic/Artisan/Academic/Survival)——社交/经济不在管辖。玩家MentalAccess+PhysicalAccess均返回1.0 |
| 教学系统 | **技能系统** `003` | NPC/Magic | 四种路径统一在技能系统：自学(1.0×)/师承(1.2-3.0×)/学院(1.3×)/秘传(→Magic调用add_xp_direct)。TeachingRisk trait空默认 |
| 交叉训练 | **技能系统** `002` | 全部模块 | 仅DirectAction/Teaching/DirectInfusion触发——CrossTraining不递归。天花板min(40, source_level×0.5)。元素组边界由Magic 002权威定义 |
| SkillCategory | **技能系统** `002` | NPC | 5类枚举（非NPC旧7类）——删除Social和Economic。NPC文档以技能系统为权威 |

### CHG-016 新增契约（天气与季节系统 v1.0）

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| 天气数据 (WeatherSample) | **天气系统** `001`/`004` | 全部模块 | 统一通过 `WeatherQuery::sample(pos, time)` trait 轮询——零事件总线。天气只提供客观物理事实——NPC 主观感知/魔法元素浓度/战斗环境判定均由各自模块自行推导 |
| 季节时钟 (SeasonClock) | **天气系统** `003` | 全部模块 | 纯时间函数（120天/年·48分钟/天·均匀四季 [TUNING]）——输入 GameTime，输出 SeasonState。虚数年号（种子决定），春分开局 |
| 温度模型 | **天气系统** `001` | Life/NPC/植物/动物 | 双层温度：regional_temperature（大气） + ground_temperature（群系修正——冠层/雪面/沙地/水体/城市）。NPC 根据高程上下文自选基准温度 |
| 天气状态机 | **天气系统** `002` | 全部 | 6状态 Markov 链（Clear→PartlyCloudy→Overcast→LightPrecip→ModeratePrecip→HeavyStorm）+ 雾独立布尔维度。转移矩阵参数化生成——非硬编码 |
| 极端天气 | **天气系统** `002` | NPC/战斗/海洋/载具/历史 | 不命名枚举类型——参数组合自然区分（风速×降水类型×强度×温度×位置）。三层 NPC 响应：L1温和偏移/L2警告强制偏移/L3灾难硬约束 |
| 群系微气候 | **天气系统** `002` | 天气系统内部 | 冠层遮阳(-2~6°C)+雪面反照+沙地辐射(+8~20°C)+水体缓冲(±2~5°C)+城市热岛(+1~5°C)+洞穴恒温——天气系统 OWN 修正公式，消费世界生成的 BiomeMicroclimateQuery |
| 视觉输出 | **天气系统** `001`/`004` | Godot 渲染/音频 | WeatherVisualPacket (~200 bytes/帧) → 调制已有体积云/天空/海洋 shader。降水粒子 ≤0.4ms GPU。transition_blend 仅存在于 WeatherVisualPacket——非 WeatherSample |
| 历史气象异常 | **天气系统** `002` | 历史系统 | 种子驱动极值采样 → ~5,000-20,000 条 HistoricalWeatherAnomaly（纯气象事实）。历史系统在 P9 消费并转化为 HistoricalEvent——必须查询气象异常数据，不能独立随机 |
| 时间/天参数 | **天气系统** `003` | 全部 | day_duration_seconds=2880(48分钟), days_per_year=120, days_per_season=30——全部 [TUNING]。日长=纬度+季节函数 |
| 性能预算 | **天气系统** `001` | 技术栈方案 | Rust CPU ≤0.25ms/帧（含全部轮询），降水粒子 ≤0.4ms GPU，VRAM 增量 ≤2.5MB。WeatherSample 栈上 Copy (~120 bytes)，零分配轮询 |

### CHG-017 新增契约（语言表达系统 v1.0）

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| 语言标识 (LanguageId/ScriptId) | **语言表达** `002` | 全部模块 | LanguageId=u32(高16位语系+低16位编号), ScriptId=u16——此前为幽灵类型，现正式定义。历史 Work.language、物品铭文、魔法文本均以语言表达为权威 |
| 语言能力 (Proficiency) | **语言表达** `002` | NPC/技能 | Proficiency=f32(0.0-1.0)——替代旧 bool literacy。NPC 语言学习由技能系统 linguistics 驱动 |
| 可读物句柄 (ExpressionRef) | **语言表达** `001`/`004` | 全部模块 | 8字节 Copy 类型(carrier_type u8 + local_id 7B)——对齐 ItemEntId 范式。可存入 NPC 记忆、大日志、对话锚点。不拥有内容——只是指针 |
| 内容解析 (ContentResolver) | **语言表达** `004` | 历史/魔法/物品 | 各模块实现 ContentResolver trait + 注册到 ExpressionRegistry。新载体类型零改动集成。LocatableReadable 扩展用于物理位置查询 |
| 文本生成 (TextGenerator) | **语言表达** `003` | 全部模块 | 片段组合模型（~430片段·~86KB）——非单体模板。参数命名遵循标准常量~50个。三种编码(Text/Geometric/Audio)统一产出文本。滤镜管道(PersonalityFilter/RelationshipFilter)后处理 |
| 对话系统 (Conversation) | **语言表达** `005` | NPC | 多参与者模型(2→1000+)。TurnMode 四种(FreeForm/Moderated/Speech/Ritual)。DialogueIntent 五种驱动 NPC 主动对话。ConversationTopic 三种来源(预设/大日志/NLU)。DialogueContext 依赖注入——lang_expression 不反向查询任何模块 |
| 社交层 (PhaticLayer/SocialField) | **语言表达** `006` | NPC | PhaticLayer 五类应酬(~210片段)——开场/穿插/反应/收尾/自发。SocialField 群体动力学(惯性/极化/从众)——听众=聚合统计非独立实体。1000人演讲≈1人文本成本+浮点计算 |
| 自然语言理解 (InputInterpreter) | **语言表达** `007` | NPC/Player | 三层回落：L1关键词(永远可用,~10µs)→L2嵌入(离线,~5ms)→L3 LLM(可插拔,~1-2s)。99%对话走L1+模板。PlayerInput 四种统一(预设选项/搜索框/语音/打字) |
| 跨模块文本管道 | **语言表达** `008` | 全部模块 | ExpressionRegistry::read() 统一入口。DialogueContext 纯数据注入——调用者从各模块收集数据后传入。lang_expression 不依赖任何业务模块。ContentMetadata.encoding 透传——Godot 层据此判断是否需要额外渲染(法阵图/音频) |

### CHG-018 新增契约（语言表达系统 v1.1 完善——信息传播·非语言联动·记忆消化）

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| 信息传播通道 | **语言表达** `009` | NPC/历史 | 五通道：Oral(面对面·衰减+失真)/Letter(跨远距·延迟=旅行时间)/Courier(信鸽·丢失风险)/Magical(即时·高门槛·魔耗)/Official(公告·一对多)。PropagationPayload 参数化传播数据包 |
| 失真算子 | **语言表达** `009` | NPC | 五算子：ScaleDistortion(数量膨胀/缩小)/DetailLoss(细节丢失)/EmotionalAmplification(情感放大)/AttributionShift(归因替换)/Moralization(道德评价附加)。每次传播应用——传播次数越多失真越严重 |
| 欺骗与谎言 | **语言表达** `009` | NPC | DeceptionIntent 四种(Honest/Exaggeration/Omission/Fabrication)。DeceptionDetection 检测概率=(intelligence×0.3)+(contradicting_info×0.3)-(trust×0.4) |
| 谣言生命周期 | **语言表达** `009` | 历史 | 五阶段：Outbreak(24h)→Sustained(1-7d)→Decaying(7-30d)→Folklorized(30d+)→Dead。逻辑斯谛增长模型 `dR/dt = k·R·(1-R/N)` |
| NPC间对话渲染 | **语言表达** `009` | Godot/NPC | 四层可见性：Full(完全字幕)/Partial(模糊字幕)/BarelyPerceptible("两个人在交谈")/Invisible(悄悄话·不可见)。PlayerPerception 随距离衰减 |
| 悄悄话/密谋 | **语言表达** `009` | NPC | PrivateMode::Whisper(0.5m)/Conspiracy(0.3m+扫描)。偷听检测清晰度=指数衰减 `0.5^(excess_m)`。发现后果：FeignNormal/Confront/Recruit/Flee |
| 非语言表达数据模型 | **语言表达** `010` | NPC/Godot | NonVerbalSignal 六类(面部/手势/姿态/视线/空间/触觉)。synthesize_nonverbal() 从对话内容+情绪+个性派生手势和表情。文化差异：GestureCultureMapping 跨文化误解 |
| 对话→记忆消化 | **语言表达** `005` | NPC | EventMemory 新增字段：learning_method(LearningMethod)/source_expression: Option<ExpressionRef>/conversation_id: Option<ConversationId>/told_by: Option<NpcId>。digest_conversation_to_memory() 对话结束→为每个参与者生成记忆。digest_reading_to_memory() NPC阅读→记忆 |
| 文化沟通规范 | **文化系统** `003`（★ v1.0 所有权从语言表达 010 转移） | NPC/语言表达/世界生成 | CommunicationNorms：interruption_tolerance/eye_contact_norm/personal_space/directness/silence_tolerance/emotional_expressiveness/honorifics/touch_norms。文化系统定义字段和派生公式——语言表达通过 CultureQuery 只读消费。影响 TurnMode 打断阈值/沉默处理/社交距离/敬语选择 |
| 对话中断与恢复 | **语言表达** `005` | NPC | 五种中断(CombatStarted/ThirdPartyJoin/Environmental/PlayerLeft/NpcUrgentGoal)。ConversationSnapshot 快照→战斗后可恢复("刚才说到一半——")。超游戏内1小时过期 |

### CHG-019 新增契约（LLM增强层 + 语音输出接口 v2.0）

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| LLM 场景粒度开关 | **语言表达** `011` | 全部 | LlmSceneConfig 19场景独立开关+master_switch。LLM 装饰器模式——包裹模板管道，失败/关闭/不可用→无缝回退模板 |
| LLM 后端抽象 | **语言表达** `011` | 无(玩家配置) | LlmBackend trait 统一本地(5种)+云端(6种)+Mock。LlmBackendRegistry 多后端管理+场景路由+故障转移。LlmRequest/LlmResponse 后端无关通用格式 |
| NPC 自主对话意愿 | **语言表达** `011` | NPC | NpcDialogueWillingness 六维(个性/情绪/关系/话题/情境/文化)——不设系统信任硬阈值。willingness 驱动 LLM 回应深度而非调用开关 |
| 旅伴多人对话 | **语言表达** `011` | NPC | multi_travel_turn() 四人触发源(环境景色/时事流言/沉默时间/随机自发)+SpeakUrge竞争+TravelUtteranceStyle(自言自语/对全体/对特定/续前)。非LLM模式必须工作 |
| 灵活可闻半径 | **语言表达** `011` | NPC/环境 | EffectiveAudibleRadius 六因子(有意控制/环境噪音/地形/天气/文化/个性)。替代固定米数 |
| 语音身份 | **语言表达** `012` | NPC | VoiceProfile 物理(种族/性别/年龄→音高/音色/语速)+个性(外向性→表现力, 宜人性→粗砺度, 神经质→气声)。生成时确定 |
| 语音情绪修饰 | **语言表达** `012` | NPC | VoiceEmotionModulation 五参数(音高偏移/语速/音量/颤抖/气声)←当前情绪。VoicePacket 统一 Rust→Godot 语音包 |
| TTS 后端 | **语言表达** `012` | Godot | TtsEngine trait 五种(System/LocalAI/CloudAPI/PreRecorded/None)。与 LLM 无关——模板管道同样可用。TtsConfig 玩家开关+字幕模式 |
| 语音优先级 | **语言表达** `012` | Godot | VoicePriority 五级(Critical打断/High/Normal/Ambient排队/Background)。多NPC同时说话时高优先先播 |

### CHG-022 新增契约（经济系统 v1.0）

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| 价格形成 (Market.last_price/TradeRecord) | **经济系统** `002` | 全部模块 | 价格从订单簿撮合交易中涌现——不存在中央定价公式。last_price是最近成交价的EMA。**任何模块不得直接写入价格** |
| 市场机制 (Market/Storefront) | **经济系统** `002` | NPC/物品/世界生成 | Market≠物理地点——是订单簿+范围+交易模式。Storefront是交易原子单位——任何NPC在任何地点都可以开店 |
| 交易执行 (TradeExecutor) | **经济系统** `002` | NPC/物品 | 玩家和NPC走同一代码路径。两阶段提交(钱包±库存)保证原子性 |
| NPC经济子状态 (NpcEconomicState) | **经济系统** `001` | NPC | 通过trait注入NpcData——不修改NpcData本体。包含钱包/已知价格表/EconomicCognition(六维缓存)/贸易偏好/商人状态 |
| 交易主体涌现 | **经济系统** `004` | NPC/世界生成 | 供给侧=盈余驱动。需求侧=匮乏驱动。中间商=四条件涌现(信息+资本+心理+物流)。**不指定"商人NPC"** |
| 借贷 | **经济系统** `004` | 物品系统/NPC | 借贷=特化交易(赊购铜币)。欠条=ItemCategory::Financial的ItemDefId。**无独立银行系统**。放贷决策和违约处理由NPC心智驱动 |
| 经济体制 (MarketRegulations) | **经济系统** `005` | 法律系统(待)/政治系统(待) | 全部参数连续可调——同一套订单簿引擎适应自由市场至军国经济。四种预设为[TUNING]起点 |
| 市场权力 (PowerAtom/MarketAuthority) | **经济系统** `006` | 政治系统(待)/法律系统(待) | 权力=PowerAtom集合(~15种原子操作)。四种来源(正式/购买/事实垄断/暴力)。玩家和NPC走同一exercise_power() |
| 行为经济学×NPC心智映射 | **经济系统** `007` | NPC | 十个行为经济学概念全部从NPC已有字段派生——**不新增人格维度**。EconomicCognition为计算缓存(从人格×技能×经验派生) |
| 货币总量 (MoneySupply) | **经济系统** `008` | 世界生成/物品系统 | 三条管道(铸币/消费回收/跨区流动)。五大自动稳定器。货币总量增速与商品总量增速对齐 |
| 职业标签 (ProfessionTag) | **世界生成**(初始分配)/**NPC身份系统**(运行时) | 经济系统(消费收入来源类型) | ~80-100个原子标签——TOML数据驱动+预留新增接口。proficiency从技能系统派生。任意2-4个排列组合→职业涌现。incongruity标记不寻常组合(不阻止) |
| LLM经济增强 | **经济系统** `009` | 语言表达系统 | LLM不参与任何经济计算。结构化数据注入→自然语言包装。模板回退覆盖100%事件类型 |

### CHG-023 新增契约（权力系统 v1.0）

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| UniversalPowerAtom (17原子) | **权力系统** `002` | 全部模块 | 17 原子分 5 类（结构/自指/关系/规范/裁决）——是 WoWorld 中所有权力关系的原子基础。经济 PowerAtom 通过调度层 `PowerToEconomicBridge` 单向映射 |
| PowerTopology (权力拓扑图) | **权力系统** `003` | 全部模块（通过 PowerTopologyQuery trait） | 有向多重图。分表 SoA 存储 + 4 重索引（出边/入边/空间/原子类型）。EdgeId=u32。边生命周期：创建→行使→衰减→失效→软删除。循环委托 DFS 检测 |
| PowerDomain (权力域) | **权力系统** `002` | 经济/文化/法律 | 6 种域——Territory(领土)/Market(市场)/Behavior(行为)/Information(信息)/Identity(身份)/Universal(通用)。领土域通过四叉树空间索引，16m Chunk LRU 缓存优化 NPC 移动触发 |
| PowerSource (8条获取路径) | **权力系统** `004` | NPC/历史/战斗 | Inherited/Appointed/Elected/Purchased/Conquered/Divine/Emergent/Contractual。路径决定初始 legitimacy 和继承行为。玩家和 NPC 走同一套 8 条路径 |
| SuccessionRule (继承规则) | **权力系统** `003` | NPC/生命 | 6 种——DesignatedHeir/Primogeniture/ElectedBy/RevertToSuperior/ExtinguishWithHolder/Unspecified。holder 死亡时自动触发。Unspecified → 继承危机事件 |
| Legitimacy (合法性) | **权力系统** `004` | NPC/文化/天气 | subject 对 holder 权力的主观认可度(0-1)。5 因子可配置公式（程序正当性 0.35/结果满意度 0.20/文化契合度 0.20/时间惯性 0.15/仪式加持 0.10）。正反馈阻尼设计。不锁定，可崩溃。每游戏日分片并行重算 |
| Duty (义务) | **权力系统** `005` | NPC/法律 | 权力边创建时自动生成对应 Duty。4 种类型——Obligation/Prohibition/Toleration/Remediation。违约 → 制裁塌缩链（有权者→有意愿者→执行或 legitimacy 下降）。无人制裁 → legitimacy 危机 → 革命检测 |
| ImmunitySet (免疫) | **权力系统** `005` | 法律/NPC | subject 属性——非原子。5 种来源——Legal/Contractual/StatusBased/Divine/Customary。与 Derogate 原子互补（Immunity=subject 盾牌，Derogate=holder 让步）。覆盖外交豁免/议会免责/贵族特权/年幼儿童/神职人员 |
| 规范层级规则 | **权力系统** `005` | 法律 | 三级优先级硬编码元规范：委托链距离(近优先) > 领域专属(domain-specific优先) > legitimacy(高优先)。平局 → 触发 Adjudicate 事件由上级裁决。RulePriority 字段作为 tiebreaker |
| Contract 双边处理 | **权力系统** `002/005` | 经济/法律 | 唯一的双边原子——需双方同意。ContractRecord 关联对称边。interdependent 字段控制一边失效时另一边联动。双边契约 + Pledge 可完整表达婚姻/盟约/信托 |
| Polity (政治实体) | **权力系统** `007` | 世界生成/历史/NPC | 4 条件涌现标签——领土连续性+统一权威+平均 legitimacy≥0.30+持续≥365天。惰性快照，不锁定内部边。弱惯性反馈(legitimacy 加成≤0.15)。每游戏年重算。PolityId=u32 |
| GovernmentForm (政府形式) | **权力系统** `007` | 世界生成/文化 | 9 种——从权力边模式推断(AbsoluteMonarchy/ConstitutionalMonarchy/Oligarchy/DemocraticRepublic/Theocracy/MilitaryDictatorship/TribalConfederation/CityState/Stateless)。不预设，不从枚举创建 |
| DiplomaticRelation (外交) | **权力系统** `007` | 经济/战斗/历史 | 连续分数(-1~+1)→离散状态(Allied/Friendly/Neutral/Cold/Hostile/War)。6因子公式(契约0.25+领土争议0.20+贸易依存0.15+近期冲突0.20+文化亲和0.10+历史深度0.10)。War状态有硬效果——临时战争权力边(lazy evaluation)+Immunity撤销+Conscript门槛降低+贸易冻结 |
| Group 治理递归 | **权力系统** `006` | NPC | EntityId::Group 作为第一等 holder/subject。5 种治理类型——Autocracy/Oligarchy/Democracy/Consensus/CouncilOfElders。内部权力边递归表达。Group 行使权力时查询治理规则找代表 |
| PowerToEconomicBridge | **调度层**（不属任一模块） | 经济系统 | 普适原子 → 经济 PowerAtom 单向映射。13 个经济原子中 4 个有普适对应(SetTaxRate/ToggleItemBan/GrantLicense/SetPriceCeiling)，9 个保留为经济专属。两模块互不导入 |
| PowerTopologyQuery trait | **权力系统** `008` | 全部模块 | 14 个只读方法覆盖所有查询模式。PowerTopologyMut 仅为权力系统和授权调度代码使用。exercise_power() 返回(PowerExerciseResult, Vec\<PowerEvent\>)——模块不反向依赖任何消费方 |

### CHG-024 新增契约（文化系统 v1.0）

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| CultureId | **文化系统** `001` | 全部模块 | CultureId=u32 扁平全局标识——替代所有幽灵类型(CultureSeed/CultureStyleId/CultureClusterId)。CultureGenealogy 独立存储谱系关系——不编码在 ID 中 |
| CultureCoreParams | **文化系统** `002` | 全部模块 | 10 个 f32 核心参数(individualism/power_distance/uncertainty_avoidance/competition_orientation/long_term_orientation/indulgence/openness_to_outsiders/religiosity/militarism/artistry)——0-1 连续值。种子生成，代际漂移(σ=0.003/年)。不可再分原子——所有文化特征从此派生 |
| CommunicationNorms | **文化系统** `003`（★ v1.0 从语言表达 010 正式转移） | 语言表达 `005`/`010`、NPC | 8 字段(interruption_tolerance/eye_contact_norm/personal_space_radius_m/directness/silence_tolerance/emotional_expressiveness/honorifics/touch_norms)——从 CultureCoreParams 确定性派生。语言表达保留 norms 的消费逻辑（如何调制对话行为） |
| GestureCultureMapping | **文化系统** `003` | 语言表达 `010` | 与 CommunicationNorms 同步转移——文化系统定义手势含义，语言表达消费跨文化误解检测逻辑 |
| BuildingStylePreferences | **文化系统** `004` | 世界生成 `005`（建筑 WFC）、家具系统 | 屋顶样式×墙体材质×装饰水平×色彩调色板×对称性×尺度×邻接修正——从核心参数×群系派生，不独立存储 |
| CulturalBeautyStandard | **文化系统** `004` | NPC `02-性别与吸引力系统` | 审美标准从核心参数+统治阶层外观派生——所有权从 NPC 模块迁至文化系统。NPC 保留消费逻辑 |
| DietaryBasePreferences | **文化系统** `004` | 经济系统（消费偏好） | 主食类型/肉食角色/饮酒文化/共食方式/香料偏好——从核心参数×群系食材派生 |
| FertilityNorms | **文化系统** `004` | 生命 `012`（繁衍系统） | 理想家庭规模/非婚生污名/性别偏好/婚姻压力——从核心参数派生 |
| honor_weight | **文化系统** `004` | 历史 `003`（立碑权重）、NPC、权力 `004`、战斗 | 荣誉权重——从 7 个核心参数的组合中涌现（非独立参数）。消费者的统一查询接口 |
| TechnologyProfile | **文化系统** `005` | 技能系统（实现 SettlementTechQuery）、物品系统（可制造门槛） | 8 独立领域(metallurgy/construction/agriculture/navigation/textiles/medicine/writing/magic_tech)——各自独立逻辑斯谛增长。替代单一 TechEra 幽灵类型。不预设单线时代 |
| SettlementTechQuery trait | **技能系统** `001`（trait 定义）→ **文化系统**（实现） | 技能系统 `003`（消费） | 接口由消费方模块定义——技能系统不需要知道 TechnologyProfile。Survival 技能不受技术天花板限制 |
| CultureName | **文化系统** `001` | 权力 `007`（Polity 命名）、语言表达、UI | Endonym(音系规则生成)+Exonym(5源加权随机他称)双层体系 |
| CultureGenealogy | **文化系统** `001` | 历史 `002`（事件因果链） | 文化谱系——父子/分裂/融合边+文化消亡标记 |
| 文化空间模型 | **文化系统** `002` | 世界生成（P2.5 集成）、全部空间查询 | 障碍 Voronoi 离散区域+渐变边界带。3-8 个起源点从种子生成。P2.5 管线插入点——在 P2 自然基底后、P3 资源分布前 |
| 文化演变 | **文化系统** `006` | 世界生成（P9 历史模拟引擎） | 四路径：代际漂移(σ=0.003/年)/贸易影响(传染性系数 0.3-0.8)/征服强制(被抵抗率减弱)/事件驱动(单次≤±0.15)。亚文化分裂(隔离>200年+距离>0.15)/克里奥尔化(混合>300年+距离>0.2)/文化消亡 |
| CultureQuery trait | **文化系统** `006` | 全部模块 | pub trait(Send+Sync)——所有模块获取文化数据的唯一入口。零分配，实现方不透明。高频方法(culture_at/core_params/communication_norms)通过 SoA 缓存+四叉树 O(log n) |
| CultureMut trait | **文化系统** `006` | 世界生成管线、历史模拟引擎 | pub(crate)——文化修改的唯一入口。消费模块不可调用 |
| 群系对文化的修正 | **文化系统** `002` | 世界生成 `002`（提供群系数据） | 群系对初始文化参数仅温和偏移(±0.05上限)——文化性格主要来自种子随机性，非地理决定论 |
| 文化与信仰的边界 | **文化系统** `001` | **信仰系统** `001` | 文化只提供 religiosity 单参数——不决定信什么神。信仰系统为独立模块。religiosity 作为信仰分配加权因子，信仰反馈通过历史 CulturalShift 事件 |
| GeographicEntity/GeographicName | **文化系统** `007` | 全部模块 | GeographicEntityId=u32——31种实体类型(山峰/河流/海洋/森林...)。nameworthiness 四因子评分(物理显著性×文化相关性×邻近性×独特性)—≥0.7必有专名。命名模板系统+Exonym五源懒生成。感知分组(客观实体层级+parent_entity)+五类地名变更场景。地名历史层积(旧名不删除——老年NPC自然使用旧名) |
| RitualDef | **文化系统** `008` | NPC(个人仪式)、魔法(魔法仪式成分)、语言表达(TurnMode::Ritual) | 所有仪式的统一原子结构——节日仪式/个人仪式/魔法仪式三种语境复用。不存储强制性(强制性来自权力系统Duty) |
| FestivalQuery/FestivalQueryExt | **文化系统** `008` | 全部模块 | 分层trait—高频4方法(festivals_on_date等)+低频6方法(npc_attending等) |
| RitualQuery trait | **文化系统** `008`(定义) | 权力系统 `004`(消费—合法性ritual因子) | ★对标MentalAccess模式—消费方定义trait。权力系统主动拉取last_ritual_at |
| FestivalEconomicImpact | **文化系统** `008` | 经济系统 `002` | 只包含需求信号(consumption_propensity_boost+additional_demand)—不设价格乘数。价格由订单簿撮合涌现(CHG-022契约) |

### CHG-025 新增契约（信仰系统 v1.0）

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| FaithId | **信仰系统** `001` | 全部模块 | FaithId=u32 扁平全局标识——统一历史系统的 `ReligionId`。FaithGenealogy 独立存储谱系关系——不编码在 ID 中 |
| FaithDefinition | **信仰系统** `002` | 全部模块 | 一个具体宗教实例的完整定义——FaithTheology 10 连续参数+教义+禁忌+万神殿+HolyDay+FastingRule。~500B/faith，种子驱动生成(P2.5) |
| FaithTheology | **信仰系统** `002` | 文化 005（技术派生）、UI | 10 个 f32 连续参数(deity_count/ancestor_importance/nature_sacredness/hierarchy_degree/scripture_centrality/ritual_formality/exclusivity/mysticism/orthodoxy_vs_orthopraxy/faith_as_identity)——"信仰类型"标签从此派生，仅用于 UI |
| ReligiousPracticeProfile | **信仰系统** `003`（定义）→ **NPC 模块**（存储） | NPC、信仰系统、战斗 | 实践优先模型——NPC 无"神学立场"，有参与实践(participation/motivation/theological_depth)。~40B/NPC，热路径本地读取 |
| FaithQuery trait | **信仰系统** `010` | 全部模块 | pub trait(Send+Sync)——30 个只读方法。零分配。高频方法 O(1)缓存。对标 CultureQuery |
| FaithCalendarQuery trait | **文化系统** `008`（定义）→ **信仰系统** `006`（实现） | 节日系统 | 对标 MentalAccess 模式。`holy_days()`/`primary_deities()`/`fasting_rules()`——信仰系统实现，节日系统消费 |
| FaithMut trait | **信仰系统** `010` | 世界生成管线、历史模拟引擎 | pub(crate)——信仰修改的唯一入口。消费模块不可调用 |
| FaithAgent trait | **信仰系统** `004`（定义）→ NPC/Player（实现） | 信仰系统 | 信仰系统不区分调用者是 NPC 还是玩家。热路径本地字段读取 |
| MagicReligionRelation | **信仰系统** `008` | 文化 005（技术派生） | 四种关系(Gift/Blasphemy/Independent/Unified)——per-faith 属性，覆盖文化 005 的 `religiosity × 0.5` 假设 |
| ReligiousReproductionNorms | **信仰系统** `002**（从 Life 012 迁移所有权） | Life 012（繁衍——消费） | 所有权从生命模块迁至信仰系统。Life 012 通过 FaithQuery 消费 |
| DivineAuthorizationEvent | **信仰系统** `007`（发出） | 权力系统 004（消费） | 事件驱动——信仰系统不操作 PowerTopology。权力系统订阅 DivineAuthorization 事件→创建 PowerSource::Divine 的 PowerEdge |
| SacredArchitectureParams | **信仰系统** `002` | 世界生成 P5-P6（建筑） | 神殿建筑参数——从 FaithTheology 派生。几何/方向/高度/光线/图像/材质/布局 7 维度 |
| FuneralCustoms | **信仰系统** `006` | 生命 DeathCustom、历史墓碑 | 葬仪类型(土葬/火葬/天葬等 7 种)+陪葬品规则+哀悼期+对亡灵态度 |
| SettlementFaithSnapshot | **信仰系统** `010**（惰性缓存） | 全部空间查询 | 从 NPC 聚合——不存储独立"区域信仰地图"。每游戏日增量更新(dirty settlements only)。"有人的地方才有信仰"原则的架构实现 |
| FaithGenealogy | **信仰系统** `005** | 历史 002、UI | 信仰谱系——Schism/Fusion/Reformation/Revival 四种谱系边+消亡标记。对标 CultureGenealogy |
| faith_morale_bonus() | **信仰系统** `010**（定义） | 战斗系统 008（TeamMorale） | 信仰系统定义 FaithCombatContext 枚举。战斗系统在计算 individual_morale 时调用——信仰不拥有 morale 公式 |
| 实践优先模型 | **信仰系统** `001/003** | 全部模块 | NPC 无"神学立场"——标签从实践派生。废除 5 档 piety 标量。改用 DerivedReligiosity 三维度(behavioral_intensity/theological_depth/motivational_autonomy)——消费者按需取用 |
| "有人的地方才有信仰"原则 | **信仰系统** `001** | 全部模块 | 信仰数据以 NPC 的 ReligiousPracticeProfile 为存储单元，空间分布从 NPC 人口聚合派生。信仰随 NPC 移动/交谈/死亡而传播/演变/消亡 |
| 信仰塑造社会组织 | **信仰系统** `004** | Group/权力/CulturalNorm/战斗/经济 | 信仰只定义条件与修饰——执行全走现有系统。教众→Group。神职层级→PowerTopology 委托链。禁忌→CulturalNorm(NormScope::SpecificFaith)。神权政体→权力系统 GovernmentForm::Theocracy |
| ChildFaithProfile 继承 | **信仰系统** `003** | 生命 012（繁衍） | 子女继承双亲的 participation 混合(0.7 因子)+社区温和引力。motivation 初始=Habitual。成年礼后可转变 |

### CHG-027 新增契约（基本需求系统 v1.0）

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| ConsumableEffect | **Life 模块** `004 §十三` | 物品系统(ItemProperties存储)、NPC(进食决策消费) | Life 定义 schema——物品模块只存储不解析。对齐已有 EnchantmentSchema/AssemblySchema 模式 |
| element_surplus[8] | **Life.Vitals** | NPC Physiology(派生 element_balance_urgency) | 仅八元素(金木水火土风雷电)——不含血和灵。索引固定。进食累积,主动释放或睡眠被动释放归零 |
| libido (Vitals) | **Life.Vitals** | NPC Physiology(直通)、概率决策器 | bio-signal——非生存生命值。归零不死亡。周期波动由 LibidoType 决定 |
| blood_element_ratio[8] | **Life.RaceTraits** | Life(ingest_food 瓶颈模型)、(预留)Magic | 种族血元素的八元素合成配比。种子生成，Σ≈1.0 |
| libido_type / libido_cycle_days | **Life.RaceTraits** | Life(compute_libido) | 三种周期模式：Continuous(持续型)/Seasonal(季节型)/EventTriggered(触发型) |
| social_deficit | **NPC.NpcData** | NPC 决策器(need_action_match)、情绪引擎 | 心理状态——不是生物性的,不属于 Physiology。累积+0.02/游戏日,社交互动恢复-0.03~0.15 |
| NeedSensitivity | **NPC.NpcData** | NPC 决策器(need_action_match) | 大五人格一次性派生——终身不变。★ v2.0: 8 个 f32 字段覆盖 hunger/thirst/fatigue/element_balance/libido/social/esteem/competence |
| need_action_match | **NPC 决策器** | 概率决策器权重链 | 替代 physiology_modifier——统一 7 维需求的 urgency→行动权重映射。公式: avg_urgency×sensitivity → 权重 1.0~2.0 |
| element_balance_urgency | **NPC.Physiology** | NPC 决策器、情绪引擎 | 从 Vitals.element_surplus 派生：max(surplus)×0.5 + avg(surplus)×0.5 |
| GOAP 安全网边界 | **NPC GOAP** | 全部模块 | 基本需求系统**不修改 GOAP** ——只有 survival 需求(hunger/thirst/fatigue/health/combat)进安全网。新增元素平衡/libido/social 不进 |
| ItemRegistry::get_consumable() | **物品系统** | Life(ingest_food)、NPC(进食选择) | struct 固有方法(非 trait)。查询 ConsumableEffect——None=不可食用 |

### CHG-028 新增契约（进阶需求系统 v1.0）

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| `esteem_deficit` | **NPC.NpcData** | NPC 决策器(need_action_match)、情绪引擎 | 心理状态——不入 Life.Vitals。+0.01/游戏日被动累积，社交钦佩事件恢复。**零新跨模块推送**——钦佩信号从已有社交管道检测 |
| `competence_frustration` | **NPC.NpcData** | NPC 决策器、情绪引擎、SelfNarrative | Allostatic 模型——设定点 = aspiration skill gap。每游戏日更新。无 SkillMastery aspiration → 恒为 0。通过 `npc.skills` 内部查询 |
| `honor_weight_for_domain()` | **文化系统**（CultureQueryExt 新增方法） | NPC 模块（esteem 计算） | ★ **唯一新跨模块方法**。`domain_code: u8` 参数——零类型依赖。从已有 CultureCoreParams 派生——零新存储 |
| `NeedSensitivity::esteem` | **NPC 模块** | NPC 决策器 | `0.2 + E×0.5 + (1-A)×0.3`。终身不变 |
| `NeedSensitivity::competence` | **NPC 模块** | NPC 决策器 | `0.2 + C×0.5 + N×0.3`。终身不变 |
| `survival_suppression()` | **NPC 模块**（概率权重链内部） | 仅概率引擎 | sigmoid `1/(1+e^(10(x-0.7)))`——软衰减。**不影响 GOAP** |
| `frustration_regression()` | **NPC 模块**（决策预处理） | NeedSensitivity 临时调制 | avg_frustration > 0.6 触发——ERG 挫折回归。社交+50%/尊重+40%/饥饿+30%/libido+25%。**不持久化** |
| `intrinsic_motivation_weight()` | **NPC 模块**（权重链） | 仅概率引擎 | `∏(1 + relevance×commitment×0.5)`, clamp [1,3]。relevance 为纯函数——可编译时优化 |
| `NeedTag::Esteem` / `NeedTag::Competence` | **NPC 模块** | ActionType 定义 | 新增 2 个需求标签。SeekRecognition/Compete/TrainSkill/SeekMentor 等行为映射 |
| GOAP 安全网边界 | **NPC GOAP** | 全部模块 | 进阶需求系统**不修改 GOAP**——心理需求不进安全网。只有生存需求+配偶压力进 |

**冲突修正原则**：不删除原有设计。通过建立正确的派生/引用/映射关系消除冲突。两个模块定义同一概念的不同抽象层（如 Physiology vs Vitals）时——建立派生关系而非强制合并。有疑问时先与用户确认，不要从根上削减原有设计。

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
7. **更新 CLAUDE.md**：添加新模块条目 + CHG 条目 + 接口契约表
