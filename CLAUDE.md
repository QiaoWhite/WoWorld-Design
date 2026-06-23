# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 项目概述

**WoWorld (Wonder World)** — 与其说是一个游戏，不如说是在以游戏的方式创建另一个"人世"。程序化生成的体素开放世界RPG，独立开发中。

**核心设计哲学**：WoWorld 是一个**故事生成器**，不是一个故事容器。开发者只定义世界的底层规则（人格参数、行动规则、情绪规律、文化传播率），具体的故事由NPC之间的互动以概率和统计的方式自然涌现。每个存档都是一个完全不同的世界。游戏没有"通关"的概念。

**涌现优先原则**：玩家与 NPC 无本质区别——NPC 能正常运作，玩家就只需"夺舍"NPC再优化细节。许多内容从现有原子系统涌现，不轻易建立独立模块：法律从 PrescribeRule+Sanction+Adjudicate+NPC决策+信息传播涌现；名声从分散式 NPC记忆×信息传播×共识涌现。战斗/魔法等细节在开发中边做边迭代，不在开发前过度设计。

**"人世"的关键维度**：
- **完整的人**：NPC 有记忆（上限2000条）、预测、情感、思想、人际关系、欲望、习惯——与"人"打交道是核心
- **历史深度**：玩家进入时世界已运转千百万年，书本和遗迹中保留着先民活过的证据
- **冒险与生活平等**：冒险者、铁匠、商人、政客——每条路都是"主线"，同一套底层系统支撑
- **全球多元文化**：西方剑与魔法为基底，全球文化元素自由融入，文明自行演化
- **无限探索**：25万km²初步世界（≈英国面积），海:陆≈7:3，700m+山峦。Minecraft级无限世界为最终目标

**设计→编码过渡中**：技术方案历经 001→015（v1.0）→016（v2.0）→018（v3.0）→**v4.0（当前权威，经技术栈全量审计升级）** 演进。四轨开发路线图已启动——设计文档与代码脚手架并存。

> ⚠️ **重要说明**：WoWorld 处于**设计+编码并行阶段**。设计文档中的伪代码和数据结构示例用于阐明设计理念，实际 Rust 实现可能根据工程发现重构。`woworld/` 目前为项目脚手架（轨A 阶段一），代码量极少——核心类型和 trait 正按路线图逐步填充。
>
> **仓库构成**：设计文档（`WoWorld-Design/`）+ Rust workspace（`woworld/`）+ 开发治理（`woworld-dev-plan/`）。设计文档用 **Obsidian** 编辑（`[[wikilink]]` 导航），Rust 代码用标准 Cargo 工具链。

**当前规格版本**: v4.0。模块累计 **~25 个独立系统** + 1 个子模块（家具与放置物品）+ 交互配方表系统 + 存档系统 v2.0（CHG-055/056）。★ 2026-06-22 Loop Audit 全量审计完成。★ NPC 认知系统 v1.1（CHG-057/058/059）。★ 开发路线图优化（CHG-060）。最新 CHG 序列见 `WoWorld-Design/Change/`。

## 快速导航

> 📘 **两份文件的分工**: **CLAUDE.md** = 项目大局观 + 工作约定。**[[CLAUDE-INTERFACES.md]]**（938行）= 跨模块契约完整参考——概念所有权、trait 签名、关键约定。修改跨模块概念时以 CLAUDE-INTERFACES.md 为权威。

| 我想… | 去… |
|------|------|
| 理解项目设计哲学 | `WoWorld-Design/Happy Game/欢迎.md` |
| 查找某模块的设计规格 | `开发阶段/<模块名>/README.md` |
| 查看跨模块契约（谁 own 什么概念、trait 签名） | [[CLAUDE-INTERFACES.md]] |
| 查我的模块被谁依赖 | `开发阶段/模块接头总览/<NN>-<模块>/003-变更影响链.md` |
| 看四轨开发路线图（设计→编码过渡） | `WoWorld-Design/开发路线图/README.md` |
| 查看开发宪法与冲刺工作流规则 | `woworld-dev-plan/CONSTITUTION.md` |
| 查看各模块开发准入状态（🔴🟡🟢） | `woworld-dev-plan/DEVELOPMENT_STATUS.md` |
| 查中英术语映射 | `woworld-dev-plan/GLOSSARY.md` |
| 查模块实现依赖图 | `woworld-dev-plan/DEPENDENCY_GRAPH.md` |
| 查架构决策记录（为什么这样设计） | `woworld-dev-plan/ARCHITECTURE_DECISIONS.md` |
| 新开发者/新设备接入 | `woworld-dev-plan/ONBOARDING.md` |
| 看最近的设计变更 | `WoWorld-Design/Change/README.md` |
| 查用户的设计裁决意见 | `WoWorld-Design/Change/hand/` |
| 看历史设计讨论存档 | `参考文档/` |
| 查阅NPC认知v1.1传播审计 | `参考文档/039-NPC认知传播审计-20260622/README.md` |
| 验证技术栈决策 | `开发阶段/技术栈方案/001-WoWorld正式技术栈方案v3.md` |
| **写/读 Rust 代码** | `woworld/` — workspace 结构见下方「代码架构」 |
| **构建项目** | `cd woworld && cargo build --workspace` |
| **启动 Godot 编辑器** | `tools/godot/Godot_v4.7-stable_win64.exe woworld/godot/project.godot` |

## 文档结构

所有设计文档在 `WoWorld-Design/` 目录下，使用 **Obsidian** 编辑，大量使用 `[[wikilink]]` 交叉引用。文档语言为中文。

### `Happy Game/` — 核心设计文档
- `欢迎.md` — 项目总览 | `想法/WoW World/总设计草稿.md` — 总设计文档
- `想法/` — 概念设计/脑暴（`#草稿` = 早期文档，允许矛盾模糊未决）
- `开发阶段/` — 正式开发规格（权威规格）。~25 个独立系统 + 1 个深化子模块（[[WoWorld-Design/Happy Game/开发阶段/物品系统/家具与放置物品/README|家具与放置物品]]），按层级组织：

| 层级 | 包含模块 | 角色 |
|------|---------|------|
| **全局基础** | 技术栈方案、模块接头总览、存档系统 | 决策依据、接口枢纽、持久化契约 |
| **世界框架** | 世界生成、生命、历史、天气与季节系统 | 物理和生物基底 |
| **NPC 核心** | NPC活人感模块（含需求/审美/认知/生命周期/行动涌现）、概念与语言地基 | NPC 完整心智 |
| **社会系统** | 经济系统、权力系统、文化系统、信仰系统 | 涌现式社会结构 |
| **交互/表现/建造/辅助** | 战斗、魔法、物品系统（含家具与放置物品）、技能系统、语言表达、模型动作与物理系统、音频系统、感官与知觉系统、大气与氛围系统、建筑模块、载具系统、小精灵系统 | 交互方式、视听表现、物理改造、玩家辅助 |

### 关键架构关系

- **技能系统 ↔ 物理原子层**: 正交维度——AgentSnapshot 连续参数决定**身体能不能执行**（force_check），SkillEntry 追踪**脑子练过多少次**（用进废退）。汇合点：复合原子 execute() 同时消费两者，`execution_noise_std` 签名 f32——零耦合。技能是事后记录标签，不是事前门控规则——门槛从物理涌现（MaterialProperties × noise）。
- **ID 类型所有权**: 所有 ID 类型（`ItemDefId`·`SkillId`·`EntityId` 等）统一定义在 `woworld_core`——零依赖 crate。TOML 数据文件为中性数据——各消费模块通过 `include_str!()` 平等加载。
- **交互配方表**: 物品获取的物理路径（采集/拆解/屠宰）统一走 TOML 配方表——`(EntityKind, tool_tags) → (composite_atom, yield_resolver, xp)`。配方表和 `CraftingRecipe` 是两张独立表——采集不需要 SkillRequirement。
- **世界生成 ←→ 存档系统**: ★ P13 不再写 LMDB——输出纯内存 `ValidatedWorldState`，持久化由存档系统接管。`SaveableModule::snapshot_dirty() → SaveSystem → LMDB` 是唯一持久化路径。详见 `开发阶段/存档系统/README` 或 [[CLAUDE-INTERFACES.md#CHG-055]]。

### `WoWorld-Design/Change/` — 设计变更追踪

> ⚠️ **Change 文件夹约定**：编号大的覆盖编号小的。以 `开发阶段/` 实际内容为权威。最新 CHG 序列（053-060）详见 `WoWorld-Design/Change/README.md`。

近期关键变更：**CHG-053**（Godot 4.7·12子系统）→ **CHG-054**（世界生成 v2.1）→ **CHG-055/056**（存档系统 v1.0→v2.0）→ **CHG-057**（NPC认知 v1.1·PatternExpression数学地基）→ **CHG-058**（NPC认知系统自审修正）→ **CHG-059**（NPC认知v1.1全模块传播审计）→ **CHG-060**（开发路线图优化·四轨重定义·孤儿接口修复）。

**`WoWorld-Design/Change/hand/`** — 用户直接设计反馈。修改涉及的设计决策时，需检查此目录是否有相关意见。

### `WoWorld-Design/开发路线图/` — 四轨并行开发规划（2026-06-22）

项目从**纯设计阶段**过渡到**设计+编码并行阶段**的路线图。四轨可同时推进：

| 轨道 | 内容 | 关键里程碑 |
|------|------|-----------|
| **轨A：正式开发** | Rust workspace 脚手架 → woworld_core → 空间索引 → 世界生成 → Godot 客户端 | Week 6 可运行原型 |
| **轨B：文档补全** | UI/UX·玩家扩展·名声·法律 优先补全 | Week 4 UI/UX 规格完成 |
| **轨C：问题修复** | 5个孤儿接口所有权冲突·CHG-047延期·性能预算·模块保鲜 | Week 2 孤儿接口清零 |
| **轨D：创意探索** | 属性注册表·冒险小队·Storyteller·魔法场论 | 按兴趣推进 |

> **核心理念**：不追求设计"冻结"再写代码——编码会反向暴露设计缺陷，设计↔编码↔反馈循环推进。详见 `WoWorld-Design/开发路线图/README.md`（含 Week 1-9+ 执行路线图、6项关键决策、里程碑验证标准）。

### `参考文档/` — 参考性设计文档
按 `NNN-简短描述-YYYYMMDD` 格式组织。001-015 已归档至 `第一部分-设计演进存档-20260610/`。活跃文档 031-041 覆盖技术栈审计、各系统设计探讨、引擎分析、**世界生成重构跨模块补充需求(037)**、**法律涌现与执法机制分析(035)**、**玩家游玩内容全貌设计(036)**。详见 `参考文档/README.md`。

## 技术栈（v4.0 权威方案）

> 📘 **权威技术方案**: `开发阶段/技术栈方案/001-WoWorld正式技术栈方案v3.md`（文件名为v3，内容已升级为v4.0）。仅关键决策列于下方，完整细节见该文档。

- **引擎**: Godot 4.7 | **模拟语言**: Rust (stable 1.80+) — GDExtension 集成。引擎评估结论：保持 Godot+Rust，Rust 模拟核心保持引擎无关以降低迁移成本（详见 [[参考文档/032-Bevy引擎切换可行性分析-20260618/README|Bevy分析]]）
- **体素**: 自建 Transvoxel（Rust 侧）→ Godot ArrayMesh。垂直稀疏Chunk + Clipmap LOD。分层密度场(L0-L10 + L6.5植被)。3km切换Signed Heightfield
- **海洋**: Gerstner程序化波，`OceanProvider` trait (woworld_core · 6方法) 预留FFT升级。海:陆≈7:3
- **画面**: 3D低多边形 + Toon 2-tone cel渲染。512²面部图集+shader合成驱动表情。★ CHG-053
- **NPC AI**: GOAP（9%）+ 概率行为树（90%）+ 可选LLM增强（1%）。SoA + rayon并行。分层模拟 L1-L4
- **战斗**: 半自动——玩家AI = NPC AI = 同一套Rust代码。三层模型（本能→节奏→战略）。详见 `开发阶段/战斗/`
- **动画**: 9层可叠加动画栈——Rust CPU批量骨骼矩阵(≤0.5ms) → Godot GPU skinning。涌现式步态(9参数从BigFive派生)
- **世界生成**: 编排器模式——15阶段管线(P0-P13 + P2.25)。Bootstrap哲学+五Pass混合构造(含同性/多配偶/非婚生社会关系初始化)+KnowledgeSeed事件驱动传播。★ P13 输出纯内存 ValidatedWorldState，持久化由存档系统接管。详见 `开发阶段/世界生成/`（15篇·v2.1）
- **数据库**: LMDB — 单文件 + 多 named_db。全量快照 + 脏数据增量。详见 `开发阶段/存档系统/`
- **物理架构**: ⚠️ CHG-033 — 仅玩家保留PhysicsServer3D，其余全部Rust侧空间查询(TerrainQuery/EntityIndex/SpatialEventBus/VisibilityQuery四trait)
- **LOD架构**: 场景8层(0-7)×角色5层(0-4)双层体系 + LODCoordinator 8步冲突解决 + 7维LodPrescription。统一距离带覆盖12km。
- **架构**: Rust模拟核心 → GDExtension → Godot客户端（渲染/UI/音频/输入/玩家物理）
- **硬件目标**: GTX 1660 SUPER 6GB VRAM（理论估算，待原型RenderDoc验证）
- **Mod**: TOML数据驱动——调节涌现乘数，无脚本引擎
- **平台**: Windows / Linux / macOS

## 开发命令（woworld/ Rust workspace）

项目脚手架已就位（轨A 阶段一）。当前代码量极少，主要为类型占位。

```bash
cd woworld

# 编译整个 workspace（含 GDExtension 动态库）
cargo build --workspace

# 编译 release（启用 LTO + 单 CGU + panic=abort，用于性能测试）
cargo build --release --workspace

# 快速检查（不生成二进制，比 build 快）
cargo check --workspace

# 运行测试（当前无测试，占位）
cargo test --workspace

# Clippy lint（Rust 最佳实践检查）
cargo clippy --workspace -- -D warnings

# 格式化
cargo fmt --all

# 启动 Godot 编辑器（Windows — Godot 4.7 可执行文件）
#   _console.exe 变体可显示 stdout/stderr，调试 GDExtension 加载问题时使用
../tools/godot/Godot_v4.7-stable_win64.exe godot/project.godot
```

> **当前状态**：`cargo check` 通过（2026-06-23 验证）。`woworld_core` 仅有 `WorldPos` 占位类型，`woworld_godot` 仅有 GDExtension 入口标记。按 `WoWorld-Design/开发路线图/` 轨A 阶段二逐步填充核心类型和 trait。

## 代码架构（woworld/ Rust workspace）

```
woworld/
├── Cargo.toml                  # workspace 清单（resolver="2", edition=2021）
├── crates/
│   ├── woworld_core/           # 核心类型 + trait 定义（零依赖）
│   │   └── src/lib.rs          #   WorldPos 占位 — 后续: types, spatial, material, id 模块
│   └── woworld_godot/          # GDExtension 桥接（cdylib → Godot 4.7）
│       └── src/lib.rs          #   WoWorldExtension 入口标记
├── godot/                      # Godot 4.7 项目
│   ├── project.godot           #   引擎配置（Forward+ 渲染器, GodotPhysics3D）
│   ├── WoWorld.gdextension     #   GDExtension 配置文件（Windows/Linux/macOS 动态库路径）
│   ├── scenes/                 #   场景文件（待填充）
│   └── scripts/                #   GDScript 脚本（待填充）
└── assets/                     # TOML 数据文件（群系、物品等 — 尚未填充）
```

### 架构原则

- **`woworld_core` — 零依赖**：所有 ID 类型（`ItemDefId`, `SkillId`, `EntityId` 等）、空间查询 trait（`TerrainQuery`, `EntityIndex`, `SpatialEventBus`, `VisibilityQuery`）、共享数据结构均在此定义。引擎无关，不依赖 Godot 或任何外部 crate。
- **`woworld_godot` — 薄桥接层**：仅负责 Rust 类型 ↔ Godot GDExtension API 的转换。不包含游戏逻辑。编译为 `cdylib`，由 Godot 运行时动态加载。
- **Godot 项目 — 纯表现层**：渲染、UI、音频、输入、玩家物理（仅玩家保留 `PhysicsServer3D`——其他全部 Rust 侧空间查询）。
- **数据流**：`TOML 数据文件` → `woworld_core`（加载 & 验证）→ `woworld_godot`（序列化桥接）→ Godot 场景树。
- **godot-rust 版本**：`godot` crate 0.5.x（GDExtension API）。

### 关键跨层契约（代码侧）

> 完整契约见 [[CLAUDE-INTERFACES.md]]。以下为代码层面的关键约束：

| 契约 | 说明 |
|------|------|
| ID 类型所有权 | 所有 ID 类型统一定义在 `woworld_core`（零依赖）——各消费 crate 平等引用 |
| TOML 数据加载 | TOML 为中性格式——各消费模块通过 `include_str!()` 平等加载，无单点解析 |
| 空间查询 | 仅玩家保留 Godot `PhysicsServer3D`，NPC/生物/载具全部走 Rust 侧四 trait |
| 持久化 | `SaveableModule::snapshot_dirty() → SaveSystem → LMDB`——世界生成 P13 不直接写磁盘 |
| 引擎无关 | `woworld_core` 不依赖 Godot——降低未来引擎迁移成本（Bevy 分析见参考文档 032） |

## 关键目录与文件规则

### `想法/` vs `开发阶段/` — 两级文档体系

| 目录 | 性质 | 规则 |
|------|------|------|
| `想法/` | 概念设计/脑暴 | 可带 `#草稿` 标签，表示**允许矛盾、模糊、未决**——是思考过程而非最终结论 |
| `开发阶段/` | 正式开发规格 | **权威规格**。必须清晰、完整、无内部矛盾。优先以此为准 |

### README ≥60 行约定

所有模块根目录 README.md 必须 ≥60 行（CHG-048 规定，2026-06-22 Loop Audit 全量达标）。README 模板：模块定位 + 文档索引 + 架构速览 + 关键参数表 + 性能预算 + 消费模块导航。子模块 README（如 `06-认知与智慧系统/`、`08-NPC行动涌现与分类/`）同样适用此门槛。

### `.canvas` 文件 — ⚠️ 禁止文本编辑

Obsidian 画布文件（`.canvas`）是**二进制 JSON**——用 Edit/Write 工具编辑会损坏文件。只能通过 Obsidian 修改。

### `文件备份/` — ⚠️ 禁止修改

`WoWorld-Design/文件备份/` 下的所有目录（如 `20260618/`、`20260619skill设计前/` 等）是**历史快照**，用于回溯设计演进。**绝不能修改或删除**这些文件。

## 文件格式

| 格式 | 用途 | 注意事项 |
|------|------|----------|
| `.md` | 所有设计文档（主要格式） | — |
| `.canvas` | Obsidian画布文件 | ⚠️ 见上方规则 |
| `.svg` | Mermaid导出图表 | — |
| `.png` | 粘贴的图片资产 | — |
| `.base` | Obsidian模板文件 | 用于创建新文档的模板 |

## 文档元数据约定（Frontmatter）

所有设计文档在开头使用 `> ` 块引用格式标注元数据（非 YAML frontmatter）：

```markdown
> **开发代号**: WoWorld (Wonder World)
> **引擎**: Godot 4.7
> **开发者**: 独立游戏开发者（Solo）
```

## 项目配置

- **开发宪法系统**: `woworld-dev-plan/` — CONSTITUTION.md（元规则层）+ DEVELOPMENT_STATUS.md（全局状态）+ sprint-proposals/ + handoff/。宪法包裹 `开发路线图/`，定义冲刺工作流和决策规则
- **Obsidian**: `.obsidian/` 目录包含工作区配置，无需手动编辑
- **.gitignore**: `.claude/` 和 `woworld/target/` 已加入 gitignore
- **Git远程**: `git@github.com:QiaoWhite/WoWorld-Design.git` (SSH)
- **可复用 Skill**: `/woworld-tech-stack-audit` — 技术栈全量审计 8 阶段方法论。仅手动调用，不自动触发
- **可复用 Skill**: `/woworldidea-design` — 设计文档同步引擎（铁匠学徒）。sync(改后同步)/impact(改前预览)/gate(新模块闸门)/audit(旧文件修复)。详见 `.claude/skills/woworldidea-design/SKILL.md`
- **可复用 Skill**: `/woworld-loop-audit` — 设计文档自动审计循环系统。手动触发，最多 10 轮。全量轻扫（硬错误自动修复）+ 热点模块 16 维深度审计（软问题报告）。子代理工厂并行审计 + state.json 跨轮进度持久化。适用项目重大变更、里程碑审计、定期保鲜检查。详见 `.claude/skills/woworld-loop-audit/SKILL.md`

### `.claude/` 配置（自动化 Hook）

- **`settings.local.json`**: 唯一的 settings 文件（无 `settings.json`）——权限允许列表 + 两个自动化 hook：
  - **PostToolUse** (Write/Edit 触发): 调用 `.claude/hooks/posttooluse-detect.sh` — 检测到修改 `开发阶段/` 下文件时写入标记（`.claude/temp/modified_files.txt`）
  - **Stop** (回合结束触发): 调用 `.claude/hooks/stop-remind.sh` — 读取标记文件，若30分钟内有改动则在下轮注入同步提醒（`/woworldidea-design sync` 的兜底机制）
- **`audit-reports/`**: Loop Audit 输出目录，按 `YYYYMMDD-NNN/` 组织，含各模块审计报告 + `FINAL-SUMMARY.md` + `state.json`（跨轮进度）
- **`scheduled_tasks.lock`**: 计划任务（CronCreate 持久化任务）的锁文件。持久化任务数据写至 `.claude/scheduled_tasks.json`
- **仓库根目录 `task_plan.md` / `findings.md` / `progress.md`**: planning-with-files 工作流文件，由 Claude 自动维护，不应手动编辑

### 持久记忆系统

项目记忆存储于 `~/.claude/projects/C--Entertainment-GAME-DEV-The-Development/memory/`，由 `MEMORY.md` 索引。每个记忆一个 `.md` 文件（带 frontmatter: name/description/metadata）。分类：`user`（用户偏好）、`feedback`（反馈纠正）、`project`（项目事实）、`reference`（外部引用）。涉及跨会话需要记住的项目决策、用户反馈、非代码库可推导的背景时写入。

## 跨模块接口契约

> 📘 契约详情见 [[CLAUDE-INTERFACES.md]]。修改跨模块概念时**必须同步维护它**。

**冲突修正原则**：不删除原有设计。通过建立正确的派生/引用/映射关系消除冲突。两个模块定义同一概念的不同抽象层时——建立派生关系而非强制合并。有疑问时先与用户确认，不要从根上削减原有设计。

## 模块接头总览

> 📘 位于 `开发阶段/模块接头总览/`（00~26 共 27 个模块文件夹 + 变更追踪 + 孤儿接口汇总）。**定位**: 介于 CLAUDE-INTERFACES.md（契约宪法）和具体模块文档之间的接口地图。设计新模块时先查可消费接口；修改旧模块时查 `003-变更影响链.md` 看波及谁。

**保鲜协议**: 修改模块文档→同步更新接头条目。每模块维护 000-变更日志。新 CHG 必须包含"接头总览更新"章节。每 5 个 CHG 后检查时间戳。
> ⚠️ **保鲜延迟声明** (2026-06-22): 关键模块接头已更新至最新，剩余模块时间戳待后续增量更新。

---

## 工作约定

- **禁止自主循环审查**：除非用户手动触发（如 `/woworld-loop-audit` 或明确要求"重复直到没问题"），否则**不要**进入一轮又一轮的自我审查和纠错循环。发现问题→提出方案→执行修复→报告结果，一轮完成。不要自主反复自我批判。
- **冲刺工作流**：编码工作按 `woworld-dev-plan/CONSTITUTION.md` 定义的冲刺循环推进——Claude 自主提出冲刺提案（§8 格式）→ 用户审批 → 执行 → 自检（§4 三层+附录A）→ 用户审核 → 交接摘要（§10）→ 下一个冲刺。触及新模块时第一步逐字精读 `开发阶段/` 原文档。编码中遭遇文档矛盾/模糊/不可行时，按 §5 立刻停止并请示，不可自行填补设计空白。

- 所有新设计文档使用 `.md` 格式，放在对应的 `想法/` 或 `开发阶段/` 子目录下
- 使用 Obsidian wikilink 语法 `[[路径/文件名]]` 引用其他文档。**跨模块引用必须加 `[[]]`**——这是用户明确要求，方便 Obsidian 导航
- 新文档跟随 `> ` 块引用 frontmatter 风格
- Godot 项目代码在 `woworld/` 目录下（Rust workspace + Godot 项目脚手架，轨A 阶段一）
- **设计变更**：对多个文档的结构性修改，在 `WoWorld-Design/Change/` 按 `CHG-XXX-简短描述-YYYYMMDD.md` 创建变更文档。CHG文档之间及CHG与参考文档之间用 `[[]]` 交叉引用
- **用户设计反馈**：`WoWorld-Design/Change/hand/` 目录存储用户对具体设计问题的直接裁决。涉及已有模块的修改时，先检查是否有相关用户意见
- **参考文档**：在 `参考文档/` 中创建 `NNN-简短描述-YYYYMMDD` 格式子文件夹，内部文档从 001 编号
- **技术决策**：以 `开发阶段/技术栈方案/` 为权威依据。所有 001-016 为历史演进存档，017 为测试方法论，018 已正式迁移至开发阶段
- **规划文件**：项目根目录的 `task_plan.md`、`findings.md`、`progress.md` 为 planning-with-files 工作流文件，用于追踪任务进度和设计决策。这些文件由 Claude 自动维护，不应手动编辑
- **修改后必须自检**：完成跨模块修改后，重新审计所涉及的模块间接口——确保没有引入新冲突
- **设计文档同步提醒（hook兜底）**：当本轮对话中修改了 `开发阶段/` 下的任何 `.md` 文件后，必须在下一轮对话开始前主动询问用户："检测到设计文档修改，需要运行 `/woworldidea-design sync` 进行同步吗？"。这是 hook 失效时的兜底规则，不得跳过
- **Skill 调用约定**：`/woworldidea-design` 为纯手动调用 skill。用户需显式输入命令。被动提醒仅通过 hook 或本规则触发，不自动执行任何 skill 命令

### ⚠️ 数学计算铁律

> **任何涉及数学计算时，必须用 `python -c "…"` 执行，禁止 LLM "心算"。** 包括但不限于：公式求值、参数验证、数值比较、概率推算、单位换算、范围边界检查。计算结果需标注来源（Python 执行输出）。设计文档中的关键公式应附 Python 验证脚本。

## 常用操作

| 操作 | 方式 |
|------|------|
| 创建设计变更 | 在 `WoWorld-Design/Change/` 创建 `CHG-XXX-描述-YYYYMMDD.md` |
| 修改后同步接口 | `/woworldidea-design sync`（同步所有关联文件） |
| 改前预览影响 | `/woworldidea-design impact` |
| 新模块脚手架 | `/woworldidea-design gate`（创建模块目录和接头文件） |
| 旧文件修复 | `/woworldidea-design audit` |
| 全量设计审计 | `/woworld-loop-audit`（最多10轮，硬错误自动修复 + 热点模块深度审计） |
| 技术栈全量审计 | `/woworld-tech-stack-audit`（8阶段3层嵌套，手动触发） |
| 新建模块 | 8步标准流程（见下方"新模块设计标准工作流程"完整说明） |

## 新模块设计标准工作流程

创建或重大扩展一个模块时，遵循8步流程（详见 `WoWorld-Design/Change/` 中 CHG-025~028 实践）：

1. **参考文档草稿** → 2. **跨模块审计**（本模块为Owner，其他模块冲突以本模块为准）→ 3. **正式开发规格**（极尽清晰完整，自审偏移/遗漏/冲突）→ 4. **修改关联文档** → 5. **审查**（内部一致性+跨文档对齐+规格漂移）→ 6. **CHG 文档**（在 `WoWorld-Design/Change/` 创建）→ 7. **更新 CLAUDE.md + [[CLAUDE-INTERFACES.md]]** → 8. **更新模块接头总览**（新模块接头文件夹 + 受影响模块条目）
