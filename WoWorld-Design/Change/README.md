# Change 文件夹

本文件夹记录 WoWorld 设计文档的结构性变更。每份变更文档独立编号，追溯所有重大修改的动机、内容和影响。

## 命名规范

```
CHG-XXX-简短描述-YYYYMMDD.md
```

| 部分 | 说明 |
|------|------|
| `CHG` | 固定前缀（Change） |
| `XXX` | 三位数字序号，从 001 开始递增 |
| `简短描述` | 中文简要说明变更内容 |
| `YYYYMMDD` | 变更日期 |

## 变更索引

### 1. CHG-001 ~ CHG-006：早期变更 (Early Changes)

| 编号 | 文件名 | 日期 | 描述 | 状态 |
|------|--------|------|------|------|
| CHG-001 | [CHG-001-NPC数量目标变更-20260609.md](CHG-001-NPC数量目标变更-20260609.md) | 2026-06-09 | NPC数量目标变更：10,000+→100,000，三层LOD全部重新规划 | 完成 |
| CHG-002 | [CHG-002-废止阶段性实现规划-20260609.md](CHG-002-废止阶段性实现规划-20260609.md) | 2026-06-09 | 废止所有阶段性实现规划，删除8阶段路线图 | 完成 |
| CHG-003 | [CHG-003-设计哲学深化与风格定义-20260609.md](CHG-003-设计哲学深化与风格定义-20260609.md) | 2026-06-09 | 深化设计哲学：故事生成器定位、东西融合风格、每存档新世界 | 完成 |
| CHG-004 | [CHG-004-创建人世哲学与NPC完整性深化-20260609.md](CHG-004-创建人世哲学与NPC完整性深化-20260609.md) | 2026-06-09 | 创建"人世"哲学、全球多元文化、历史深度、冒险与生活平等、NPC完整性、记忆200→2000 | 完成 |
| CHG-005 | [CHG-005-魔法系统冲突修正-20260609.md](CHG-005-魔法系统冲突修正-20260609.md) | 2026-06-09 | 修正魔法系统6份设计文档的7项冲突 | 完成 |
| CHG-006 | [CHG-006-魔法系统模块化重组-20260609.md](CHG-006-魔法系统模块化重组-20260609.md) | 2026-06-09 | 魔法系统模块化重组：新建13文档+重构中心索引+更新5文档交叉引用（6→19） | 完成 |

### 2. CHG-007 ~ CHG-009：审计与重构 (Audit & Restructure)

| 编号 | 文件名 | 日期 | 描述 | 状态 |
|------|--------|------|------|------|
| CHG-007 | [CHG-007-开发测试暴露的设计缺陷修正-20260610.md](CHG-007-开发测试暴露的设计缺陷修正-20260610.md) | 2026-06-10 | 30次双视角模拟测试暴露16个设计缺陷：11修正+3待后续+2需新设计。含Phase重新划分建议 | 完成 |
| CHG-008 | [CHG-008-设计深度补充-世界尺度战斗天象载具城市蓝图-20260610.md](CHG-008-设计深度补充-世界尺度战斗天象载具城市蓝图-20260610.md) | 2026-06-10 | 设计深度补充：700m山+海陆73+体积云+半自动战斗+城市规划+载具+蓝图系统。触发技术栈v3重写 | 完成 |
| CHG-009 | [CHG-009-NPC开发文档Rust重写-v3升级-20260611.md](CHG-009-NPC开发文档Rust重写-v3升级-20260611.md) | 2026-06-11 | NPC活人感开发文档全面重写：GDScript→Rust伪代码，植入v3全部新系统，017测试发现反向修正 | 完成 |

### 3. CHG-010 ~ CHG-012：世界生成/生命/跨模块审计 (World Gen / Life / Cross-module Audit)

| 编号 | 文件名 | 日期 | 描述 | 状态 |
|------|--------|------|------|------|
| CHG-010 | [CHG-010-世界生成v3重构-20260611.md](CHG-010-世界生成v3重构-20260611.md) | 2026-06-11 | 世界生成v3重构：编排器模式、多阶段管线、Bootstrap哲学 | 完成 |
| CHG-011 | [CHG-011-生命系统v1.0设计-20260611.md](CHG-011-生命系统v1.0设计-20260611.md) | 2026-06-11 | 生命系统v1.0设计：生命世界初始化、种群模板、食物链顺序 | 完成 |
| CHG-012 | [CHG-012-开发文档全审计-矛盾冲突错误修复-20260611.md](CHG-012-开发文档全审计-矛盾冲突错误修复-20260611.md) | 2026-06-11 | 开发文档全审计：矛盾、冲突、错误全面修复 | 完成 |

### 4. CHG-013 ~ CHG-019：跨模块一致性 + 地基模块 (Cross-module Consistency + Foundation Modules)

| 编号 | 文件名 | 日期 | 描述 | 状态 |
|------|--------|------|------|------|
| CHG-013 | [CHG-013-跨模块一致性冲突修正-20260612.md](CHG-013-跨模块一致性冲突修正-20260612.md) | 2026-06-12 | 跨模块一致性冲突修正：确立基座契约（Vitals/Mana/10元素/部位伤害/群系/AetherImprint），冲突修正原则 | 完成 |
| CHG-014 | [CHG-014-物品系统v1.0创建-20260612.md](CHG-014-物品系统v1.0创建-20260612.md) | 2026-06-12 | 物品系统v1.0创建：ItemDefId/ItemEntId(u64)、Quality×Rarity、BodyPlan自动派生、Assembly组件树、Enchantment卡槽+直接双模式 | 完成 |
| CHG-015 | [CHG-015-技能系统v1.0创建-20260612.md](CHG-015-技能系统v1.0创建-20260612.md) | 2026-06-12 | 技能系统v1.0创建：SkillId(u64)5分类22子组、SkillEntry稀疏存储、三层天赋、TeachingRisk trait、CrossTraining非递归 | 完成 |
| CHG-016 | [CHG-016-天气与季节系统v1.0创建-20260612.md](CHG-016-天气与季节系统v1.0创建-20260612.md) | 2026-06-12 | 天气与季节系统v1.0创建：WeatherQuery::sample()零事件总线、SeasonClock纯时间函数、双层温度、Markov 6状态+雾 | 完成 |
| CHG-017 | [CHG-017-语言表达系统v1.0创建-20260613.md](CHG-017-语言表达系统v1.0创建-20260613.md) | 2026-06-13 | 语言表达系统v1.0创建：LanguageId/ScriptId、ExpressionRef 8B句柄、ContentResolver trait、片段组合文本(~430片段)、PhaticLayer五类应酬 | 完成 |
| CHG-018 | [CHG-018-语言表达系统v1.1完善-信息传播非语言联动记忆消化-20260613.md](CHG-018-语言表达系统v1.1完善-信息传播非语言联动记忆消化-20260613.md) | 2026-06-13 | 语言表达系统v1.1完善：五传播通道+失真算子、DeceptionIntent四种、CommunicationNorms→文化系统、GestureCultureMapping→文化系统 | 完成 |
| CHG-019 | [CHG-019-LLM增强层与语音输出v2.0创建-20260613.md](CHG-019-LLM增强层与语音输出v2.0创建-20260613.md) | 2026-06-13 | LLM增强层与语音输出v2.0创建：LlmSceneConfig 19场景开关、LlmBackend trait、VoiceProfile、TtsEngine trait、VoicePriority五级 | 完成 |

### 5. CHG-022 ~ CHG-033：业务系统扩展 (Business System Expansion)

| 编号 | 文件名 | 日期 | 描述 | 状态 |
|------|--------|------|------|------|
| CHG-022 | [CHG-022-经济系统v1.0创建-20260613.md](CHG-022-经济系统v1.0创建-20260613.md) | 2026-06-13 | 经济系统v1.0创建：价格从订单簿涌现(禁止直写)、Market≠物理地点、两阶段提交、NpcEconomicState trait、中间商四条件涌现、五大货币稳定器 | 完成 |
| CHG-023 | [CHG-023-权力系统v1.0创建-20260614.md](CHG-023-权力系统v1.0创建-20260614.md) | 2026-06-14 | 权力系统v1.0创建：17 UniversalPowerAtom、PowerTopology有向多重图、Legitimacy 5因子公式、Duty制裁塌缩链、Polity 4条件涌现、外交6因子 | 完成 |
| CHG-024 | [CHG-024-文化系统v1.0创建-20260614.md](CHG-024-文化系统v1.0创建-20260614.md) | 2026-06-14 | 文化系统v1.0创建：CultureCoreParams 10核心参数、CommunicationNorms迁入、障碍Voronoi空间模型、RitualDef统一原子、四路径文化演变 | 完成 |
| CHG-025 | [CHG-025-信仰系统v1.0创建-20260615.md](CHG-025-信仰系统v1.0创建-20260615.md) | 2026-06-15 | 信仰系统v1.0创建：FaithTheology 10参数、实践优先模型(无"神学立场")、FaithQuery 30方法、NPC→NPC接触传染5渠道 | 完成 |
| CHG-026 | [CHG-026-载具系统v1.0设计-20260615.md](CHG-026-载具系统v1.0设计-20260615.md) | 2026-06-15 | 载具系统v1.0设计：魔力驱动火车/帆船、移动参考系、NPC集体建造 | 完成 |
| CHG-027 | [CHG-027-基本需求系统v1.0创建-20260615.md](CHG-027-基本需求系统v1.0创建-20260615.md) | 2026-06-15 | 基本需求系统v1.0创建：ConsumableEffect schema(Life定义)、7维需求统一框架、element_surplus[8]、NeedSensitivity 8字段、GOAP安全网不扩展 | 完成 |
| CHG-028 | [CHG-028-进阶需求系统v1.0创建-20260615.md](CHG-028-进阶需求系统v1.0创建-20260615.md) | 2026-06-15 | 进阶需求系统v1.0创建：三层需求(生存→心理→成长)、esteem_deficit/competence_frustration、survival_suppression() sigmoid、frustration_regression() ERG挫折回归 | 完成 |
| CHG-029 | [CHG-029-审美系统v1.0创建-20260616.md](CHG-029-审美系统v1.0创建-20260616.md) | 2026-06-16 | 审美系统v1.0创建：AestheticSignal 6维、judge()纯函数零副作用、HasAestheticSignal trait 12实现者、4事件原子、FineArts技能大类 | 完成 |
| CHG-030 | [CHG-030-音频系统v1.0创建-20260616.md](CHG-030-音频系统v1.0创建-20260616.md) | 2026-06-16 | 音频系统v1.0创建：SoundFootprint物理模型、AudioQuery 30方法、VoiceProfile迁入、五类声音、传播引擎(衰减/吸收/风/温度/遮挡/多普勒) | 完成 |
| CHG-031 | [CHG-031-感官与知觉系统v1.0创建-20260616.md](CHG-031-感官与知觉系统v1.0创建-20260616.md) | 2026-06-16 | 感官与知觉系统v1.0创建：PerceptBatch统一产出、VisionQuery/ScentQuery/SpatialQuery trait、PerceptualCache LRU、DarkAdaptation指数松弛 | 完成 |
| CHG-032 | [CHG-032-NPC认知与智慧系统v1.0创建-20260617.md](CHG-032-NPC认知与智慧系统v1.0创建-20260617.md) | 2026-06-17 | NPC认知与智慧系统v1.0创建：CognitiveStyle 4维认知风格、CognitiveTide 3维潮汐、MentalModel(≤20条·6路径跨代传递)、ThoughtTrigger 6类触发、创新管线6阶段 | 完成 |
| CHG-033 | [CHG-033-模型动作物理系统v1.0创建-20260617.md](CHG-033-模型动作物理系统v1.0创建-20260617.md) | 2026-06-17 | 模型动作物理系统v1.0创建：TerrainQuery/EntityIndex/SpatialEventBus/VisibilityQuery四trait空间查询、33/35骨骨架、9层动画栈、38模块姿态、仅玩家保留PhysicsServer3D | 完成 |

### 6. CHG-041 ~ CHG-050：新模块 + 架构升级 (New Modules + Architecture Upgrades)

| 编号 | 文件名 | 日期 | 描述 | 状态 |
|------|--------|------|------|------|
| CHG-041 | [CHG-041-NPC生命周期系统v1.0创建-20260618.md](CHG-041-NPC生命周期系统v1.0创建-20260618.md) | 2026-06-18 | NPC生命周期系统v1.0创建：从受孕到死亡+死后痕迹的完整生命历程。AgeClock纯函数+Gompertz衰老+InfantDependency状态机+FertilityPotential曲线+DeathCause 30种6类+玩家双角色死亡继承 | 完成 |
| CHG-042 | [CHG-042-NPC物理原子层v1.0创建-20260618.md](CHG-042-NPC物理原子层v1.0创建-20260618.md) | 2026-06-18 | NPC物理原子层v1.0创建：三层原子架构(35物理基元+~40领域复合原子+~25社会抽象原子)、AgentSnapshot连续能力快照、MaterialProperties数据驱动涌现、IK+碰撞箱战斗管线 | 完成 |
| CHG-043 | [CHG-043-建筑模块v1.0创建-20260619.md](CHG-043-建筑模块v1.0创建-20260619.md) | 2026-06-19 | 建筑模块v1.0创建：ComponentFamily参数化族(9核心族+Mod扩展)、2.5D WFC三阶段求解(BSP→2D WFC→3D组装)、BuildingGenerator trait(8种生成器)、Blueprint TOML、ConstructionScheduler声明式施工 | 完成 |
| CHG-044 | [CHG-044-概念与语言地基系统v1.0创建-20260619.md](CHG-044-概念与语言地基系统v1.0创建-20260619.md) | 2026-06-19 | 概念与语言地基系统v1.0创建：三层模型(PatternSignature→文化概念空间→语言词汇)、classify_pattern()概念识别、Utterance结构化话语、归纳/类比/演绎三推理模式、六条代际传递路径+保真度数学模型 | 完成 |
| CHG-045 | [CHG-045-世界生成v2.0完全重构-20260620.md](CHG-045-世界生成v2.0完全重构-20260620.md) | 2026-06-20 | 世界生成v2.0完全重构：编排器模式·14阶段管线·结构/细节双层分离·Bootstrap哲学·协同生成（★CHG-048编号修复后在接头总览中登记为CHG-045） | 完成 |
| CHG-046 | [CHG-046-植被系统架构升级-20260620.md](CHG-046-植被系统架构升级-20260620.md) | 2026-06-20 | 植被系统架构升级：15阶段管线·VMC双层空间架构(P2.25植被覆盖·75K个1km²植被宏观Chunk·运行时32m TC确定性展开)、VegetationProvider trait(7方法)、木材形式化合同(WoodMaterialContract)、群落层从个体层分离 | 完成 |
| CHG-048 | — (见CLAUDE-INTERFACES.md CHG-048) | 2026-06-20 | ★全模块交叉审计修复Wave A——幽灵trait注册(EconomyQuery 8方法·OceanProvider 6方法·4 DI trait·VisionQuery+ScentQuery)·编号去重(世界生成012/013/014·生命010/013)·缺失README补全(NPC+载具)·CHG-XXX→正确编号·变更追踪影响地图创建 | 完成 |
| CHG-049 | [CHG-049-LOD架构全面深化-20260620.md](CHG-049-LOD架构全面深化-20260620.md) | 2026-06-20 | LOD架构全面深化：场景8层×角色5层双层体系·LODCoordinator 8步冲突解决·7维LodPrescription(audio_lod 5档)·跨维硬约束·级联预算制交互·音频意图驱动·VRAM监控·Building地标映射·分维度过渡时序 | 完成 |
| CHG-050 | [CHG-050-家具与放置物品系统v1.0创建-20260620.md](CHG-050-家具与放置物品系统v1.0创建-20260620.md) | 2026-06-20 | 家具与放置物品系统 v1.0 创建：物品系统子模块的全面重写——9参数化家具族·PlacementStore数据模型(32B热)·七步放置验证·双Pass种子确定性生成·表面链级联物理·AffordanceSet 64-bit供给位集·CultureFurnishProfile文化派生·雕塑标记化目录(~66 mesh)·工作站u16注册制解耦。6篇新建设计文档+13个文件联动修改 | 完成 |
| CHG-051 | [CHG-051-交互配方表系统-20260620.md](CHG-051-交互配方表系统-20260620.md) | 2026-06-20 | 交互配方表系统——物品获取物理统一入口 | 完成 |
| CHG-052 | [CHG-052-玩家游玩内容全貌设计-20260620.md](CHG-052-玩家游玩内容全貌设计-20260620.md) | 2026-06-20 | 玩家游玩内容全貌设计：36问13编34章。不创建"玩家旅程系统"独立模块——全部从已有24个模块交汇涌现。新建5份开发文档(小精灵系统+方阵统计+角色管理+大日志多镜头+复合原子注册)+3份参考文档(法律分析+游玩大纲+遗漏清单) | 完成 |
| CHG-053 | [CHG-053-Godot4.7技术栈升级与设计深化-20260621.md](CHG-053-Godot4.7技术栈升级与设计深化-20260621.md) | 2026-06-21 | Godot 4.7 技术栈升级与设计深化：引擎升级+画面渲染管线+语音合成+氛围系统+wear维护+面部表情+Bark语声——12子系统设计 | 完成 |
| CHG-054 | [CHG-054-世界生成v2.1五Pass重构与RelationshipNorms集成-20260621.md](CHG-054-世界生成v2.1五Pass重构与RelationshipNorms集成-20260621.md) | 2026-06-21 | 世界生成 v2.1：P8 Phase B 五Pass混合构造(祖先创建+配偶池合并+社会关系初始化含同性/多配偶/非婚生+已故子女+DAG验证)。消除7个一致性bug。P9a事件驱动优化(~10x加速)。新增FertilityNorms方法+RelationshipNorms struct(文化系统004)。P13 FamilyTree校验+死因装饰。SettlementData.founding_year。性能总预算5-13s→3-10s | 完成 |
| CHG-055 | [CHG-055-存档系统v1.0创建-20260621.md](CHG-055-存档系统v1.0创建-20260621.md) | 2026-06-21 | 存档系统 v1.0 创建：全量快照+脏增量·SaveableModule trait(8方法·2必覆·6默认)·LMDB单文件多named_db·崩溃恢复三件套(临时文件+原子重命名/覆盖前备份/session.lock)·版本迁移(模块级)·Mod兼容(冻结/替换)·玩家继承(PendingInheritance+灵魂转世)·世界发现(目录扫描)。6篇正式规格+7篇参考/接头文档·9文档联动修改·11轮审计 | 完成 |
| CHG-056 | [CHG-056-存档系统深度审计与修正-20260621.md](CHG-056-存档系统深度审计与修正-20260621.md) | 2026-06-21 | 存档系统深度审计与修正（10轮迭代审计·24问题全修复）——SaveableModule trait 8→14方法(4必覆+10默认)·LoadContext渐进加载上下文·named_dbs键前缀冲突检测·write_dirty流式增量写入·write_initial Initial直写路径·惰性迁移(ConsumableEffect)·死亡存档最高优先级·SaveQueue调度模式·UUID去重·全遍历写入后验证·World.mod_modules Mod预留 | 完成 |
| CHG-057 | [CHG-057-NPC认知系统深度设计-20260622.md](CHG-057-NPC认知系统深度设计-20260622.md) | 2026-06-22 | NPC认知系统 v1.1 深度设计：PatternExpression数学地基（AlgebraicClosure七算子·PatternSignature u64·扩展ConsequenceGraph）·事件记忆12字段·信息密度连续压缩·crowd_suggestibility群体可暗示性·rhetorical_ability修辞能力·domain_sig跨域概念检测 | 完成 |
| CHG-058 | [CHG-058-NPC认知系统自审修正-20260622.md](CHG-058-NPC认知系统自审修正-20260622.md) | 2026-06-22 | NPC认知系统 v1.1 自审修正：MentalModel评估管道·慢性中毒追踪·玩家话语→NPC信念桥接·cognitive_aging三函数派生（crystallized_factor+cognitive_engagement_score+health_burden→pathological_annual_degradation）·PrenatalAccumulator交互·决策点counterfactual_regret·内部一致性审计 | 完成 |
| CHG-059 | [CHG-059-NPC认知v1.1传播审计-20260622.md](CHG-059-NPC认知v1.1传播审计-20260622.md) | 2026-06-22 | NPC认知v1.1全模块传播审计——8大修正·PatternExpression+MentalModel+认知老化+群集心理+认知潮汐+修辞能力+counterfactual_regret+玩家话语认知管道·CLAUDE-INTERFACES.md全面同步 | 完成 |
| CHG-060 | [CHG-060-开发路线图优化-20260622.md](CHG-060-开发路线图优化-20260622.md) | 2026-06-22 | 开发路线图优化（/grill-me 17问审核）——四轨重定义·NPC L4拉入原型·AgentSnapshot归属core·属性注册表升格·孤儿接口修复·CLAUDE-INTERFACES.md同步·Phase 1-4修正 | 完成 |
| CHG-061 | [CHG-061-轨C孤儿接口修复-20260624.md](CHG-061-轨C孤儿接口修复-20260624.md) | 2026-06-24 | 轨C 孤儿接口修复——ProfessionTag所有权理顺（类型→woworld_core·概念Owner=经济）+ 12文档脱节补全（WorldGen/AgeClock+InfantDependency+PlantSpecies·Skills/InnovationPipeline·跨模块进口8项）·CLAUDE-INTERFACES.md同步·接头总览更新 | 完成 |
| CHG-062 | [CHG-062-UI与UX系统创建-20260624.md](CHG-062-UI与UX系统创建-20260624.md) | 2026-06-24 | 轨B UI/UX 系统创建——6篇799行（信息架构+HUD+对话+面板+接口性能预算）。grill-me 4项裁决。L0-L3信息层级。双输入映射。 | 完成 |

---

## 编号缺口 (Numbering Gaps)

以下编号已预留给已规划但尚未创建文档的变更：

| 编号 | 对应变更 | 状态 |
|------|----------|------|
| CHG-020 | 预留给语言表达/LLM增强后续 | 规划中 |
| CHG-021 | 预留给地基模块后续 | 规划中 |
| CHG-034 ~ CHG-037 | 技术栈全量审计 (v3→v4.0: 物理迁移+7模块条目+峰值互斥预算+LOD统一架构)。审计过程记录于多个参考文档和模块变更中，对应 CHG 文档待创建 | 规划中 |
| CHG-038 ~ CHG-039 | TDI扩展 + 模块接头总览填充。成果已体现在模块接头总览目录中，对应 CHG 文档待创建 | 规划中 |
| CHG-040 | 预留给技术栈审计收尾 | 规划中 |

## 相关目录

### `hand/`

用户直接设计反馈目录。修改涉及的设计决策时，应检查此目录是否有相关意见。

### `archived/renumbered-duplicates/`

存放 CHG-001~004 的早期命名变体（编号被重新分配前的版本）：

| 文件 | 说明 |
|------|------|
| `NPC数量目标变更 20260609.md` | 原初变更记录，后被重新编号为 CHG-001 |
| `CHG-001-废止阶段性实现规划-20260609.md` | 重编号前的 CHG-001（现为 CHG-002） |
| `CHG-002-设计哲学深化与风格定义-20260609.md` | 重编号前的 CHG-002（现为 CHG-003） |
| `CHG-003-创建人世哲学与NPC完整性深化-20260609.md` | 重编号前的 CHG-003（现为 CHG-004） |

这些文件保留用于历史追溯，不应作为当前设计的参考依据。

---

> **注意**：`开发阶段/Change/` 文件夹中包含早期变更记录（NPC 数量目标变更），后续所有变更请使用此顶层 Change 文件夹。

> **最后更新**: 2026-06-24 — CHG-062 已登记。CHG-062 轨B UI/UX 系统创建——6篇799行。
