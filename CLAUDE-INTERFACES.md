# CLAUDE-INTERFACES.md — 跨模块接口契约完整参考

> 本文件从 [[CLAUDE.md]] 迁出，包含全部 CHG 接口契约表。修改任何跨模块概念时必须维护这些契约。
>
> **冲突修正原则**：不删除原有设计。通过建立正确的派生/引用/映射关系消除冲突。两个模块定义同一概念的不同抽象层时——建立派生关系而非强制合并。有疑问时先与用户确认，不要从根上削减原有设计。

## CHG-013 基座契约

经 CHG-013 审计确定的关键接口所有权和约定：

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
| 死亡原因 | **Life** `004 §九` | History（墓碑文本映射） | ★ CHG-041: 30种6类（创伤8/匮乏5/侵入5/奥术5/衰亡3/意志4）。类别级亡灵规则。DeathCauseRegistry trait支持mod扩展 |

## CHG-014 新增契约（物品系统 v1.0）

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| 物品标识 (ItemDefId/ItemEntId) | **物品系统** `001`/`002` | 全部模块 | ItemDefId=u64全局恒定(8+56bit), ItemEntId=u64存档内唯一——旧MaterialId/MagicItemId/resource_type通过映射表桥接 |
| 物品属性 (ItemProperties) | **物品系统** `003` | 全部模块 | 核心属性+Quality(4档)×Rarity(5档)+AestheticProps——各模块在此之上叠加 |
| 装备槽位 (EquipmentSlots) | **物品系统** `004` | NPC/Life | BodyPlan自动派生——不预定义物种类型。双套Outfit切换由NPC自主决定。★ v1.2: 肩部槽位(shoulder_drape解剖条件)·戒指10→4命名手指槽位·容器纯数据层·视觉独立开关 |
| 库存与仓储 | **物品系统** `005` | NPC/Player | 30基础槽位+容器五层体系——[TUNING]可调。货币独立于库存(CharacterWallet) |
| 物品装配 (Assembly) | **物品系统** `001` | Combat/Magic | 通用组件树框架+四种JointType(Rigid/Hinge/Chain/Flexible)——Combat/Magic注册slot_type |
| 附魔 (Enchantment) | **物品系统** `008` | Combat/Magic | 卡槽附魔(日常经济,可撤换)+直接附魔(历史锚点,永久)——两者可共存于同一物品 |
| 制造配方 (CraftingRecipe) | **物品系统** `006` | NPC/Magic | TOML数据驱动——不是代码。配方发现=学习/实验/购买/天启 |

## CHG-015 新增契约（技能系统 v1.0）

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

## CHG-016 新增契约（天气与季节系统 v1.0）

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

## CHG-017 新增契约（语言表达系统 v1.0）

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

## CHG-018 新增契约（语言表达系统 v1.1 — 信息传播·非语言联动·记忆消化）

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

## CHG-019 新增契约（LLM增强层 + 语音输出接口 v2.0）

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

## CHG-022 新增契约（经济系统 v1.0）

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
| 职业标签 (ProfessionTag) | **经济系统**(概念Owner——TOML schema/标签目录/incongruity规则) · 类型定义在 **woworld_core** (`ProfessionTagId(u32)`) | 世界生成(P8初始分配)·NPC身份系统(运行时维护)·技能系统(proficiency派生) | ★ CHG-063: ~80-100个原子标签——TOML数据驱动+预留新增接口。任意2-4个排列组合→职业涌现。proficiency从技能系统派生。incongruity标记不寻常组合(不阻止)。类型与其他ID并列woworld_core。 |
| LLM经济增强 | **经济系统** `009` | 语言表达系统 | LLM不参与任何经济计算。结构化数据注入→自然语言包装。模板回退覆盖100%事件类型 |

## CHG-023 新增契约（权力系统 v1.0）

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

## CHG-024 新增契约（文化系统 v1.0）

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| CultureId | **文化系统** `001` | 全部模块 | CultureId=u32 扁平全局标识——替代所有幽灵类型(CultureSeed/CultureStyleId/CultureClusterId)。CultureGenealogy 独立存储谱系关系——不编码在 ID 中 |
| CultureCoreParams | **文化系统** `002` | 全部模块 | 10 个 f32 核心参数(individualism/power_distance/uncertainty_avoidance/competition_orientation/long_term_orientation/indulgence/openness_to_outsiders/religiosity/militarism/artistry)——0-1 连续值。种子生成，代际漂移(σ=0.003/年)。不可再分原子——所有文化特征从此派生 |
| CommunicationNorms | 文化系统 003 | 语言表达, NPC | 8字段 (interruption_tolerance/eye_contact_norm/personal_space_radius_m/directness/silence_tolerance/emotional_expressiveness/honorifics/touch_norms). CHG-024 从语言表达迁移所有权至文化系统 |
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

## CHG-025 新增契约（信仰系统 v1.0）

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
| ReligiousReproductionNorms | 信仰系统 002 | 生命, NPC | 宗教婚姻规则/出生仪式/死亡仪式/信仰继承. CHG-025 从 Life 012 迁移所有权至信仰系统 |
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

## CHG-026 新增契约（载具系统 v1.0）

| # | 概念 | Owner | 消费者 | 关键约定 |
|---|------|-------|--------|---------|
| 1 | VehicleId / VehicleDef | 载具系统 001 | Items, NPC, World Gen | VehicleId=世界实体+可选ItemEntId契书. 载具永不进入库存 |
| 2 | 5种动力类型 | 载具系统 002 | Life, Magic | MusclePowered/AnimalTowed/WindPowered/MagicEngine/Hybrid. 日常航行不受自然力影响, 仅灾害级天气施加硬约束 |
| 3 | 移动参考系 (T_V) | 载具系统 005 | NPC, Godot渲染 | world_pos = T_V × P_local. 载具局部空间查询由 Rust EntityIndex 提供 |
| 4 | Crew GOAP 目标 | 载具系统 003 | NPC | NavigateVehicle/Lookout/MaintainVehicle/AssistPassengers |
| 5 | 载具损伤模型 | 载具系统 004 | Combat, Items | 3通道连续损伤 (Hull/Propulsion/Steering). 镜像战斗损伤模型 |
| 6 | 所有权与契书 | 载具系统 005 | Items, Power | 5获取路径 (口头/契书/征服/建造/继承). 契书可伪造. 争议裁决→权力系统 |
| 7 | 载具货舱 | 载具系统 006 | Items, Economy | ContainerId 通过物品系统标准 Container 接口 |
| 8 | VehicleQuery trait | 载具系统 010 | Combat, NPC, Godot | 30+只读方法, Send+Sync, 零分配. VehicleMut pub(crate) |
| 9 | CrewSlotDef | 载具系统 001 | Skill, Culture | required_skill→SkillId, label_key→文化系统本地化 |
| 10 | 命名系统 | 载具系统 009 | Culture, Language | 载具命名由文化参数+语言表达共同决定 |
| 11 | 载具涌现 | 载具系统 008 | World Gen, Culture | VehicleArchetype×文化参数→载具变体. 非 enum 涌现 |
| 12 | 铁路系统 | 载具系统 007 | World Gen, History | P9 稀有涌现, 非世界生成. NPC 集体建造 |
| 13 | 日常自动导航 | 载具系统 003 | NPC, Weather | L1-L3 半自动控制. NPC+Player 实现同一 VehicleController trait |

## CHG-027 新增契约（基本需求系统 v1.0）

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| ConsumableEffect | **Life 模块** `004 §十三` | 物品系统(ItemProperties存储)、NPC(进食决策消费) | Life 定义 schema——物品模块只存储不解析。对齐已有 EnchantmentSchema/AssemblySchema 模式 |
| element_surplus[8] | **Life.Vitals** | NPC Physiology(派生 element_balance_urgency) | 仅八元素(金木水火土风雷电)——不含血和灵。索引固定。进食累积,主动释放或睡眠被动释放归零 |
| libido (Vitals) | **Life.Vitals** | NPC Physiology(直通)、概率决策器 | bio-signal——非生存生命值。归零不死亡。周期波动由 LibidoType 决定 |
| blood_element_ratio[8] | **Life.RaceTraits** | Life(ingest_food 瓶颈模型)、(预留)Magic | 种族血元素的八元素合成配比。种子生成，Σ≈1.0 |
| libido_type / libido_cycle_days | **Life.RaceTraits** | Life(compute_libido) | 三种周期模式：Continuous(持续型)/Seasonal(季节型)/EventTriggered(触发型) |
| social_deficit | **03-基本需求.NeedColumn** (SoA) | NPC 决策器(need_action_match)、情绪引擎、跨需求耦合 | 心理状态——不是生物性的,不属于 Physiology。累积+0.02/游戏日,社交互动恢复-0.03~0.15。★ v1.1 方案D: 从 NpcData AoS 迁移至独立 SoA 列族 |
| NeedSensitivity | **03-基本需求.NeedColumn** (SoA) | NPC 决策器(need_action_match)、04-进阶需求(frustration_regression) | 大五人格一次性派生——终身不变。★ v2.0: 8 个 f32 字段覆盖 hunger/thirst/fatigue/element_balance/libido/social/esteem/competence。★ v1.1 方案D: 从 NpcData 迁移至 NeedColumn |
| need_action_match | **NPC 决策器** | 概率决策器权重链 | 替代 physiology_modifier——统一 7 维需求的 urgency→行动权重映射。公式: avg_urgency×sensitivity → 权重 1.0~2.0 |
| element_balance_urgency | **NPC.Physiology** | NPC 决策器、情绪引擎 | 从 Vitals.element_surplus 派生：max(surplus)×0.5 + avg(surplus)×0.5 |
| GOAP 安全网边界 | **NPC GOAP** | 全部模块 | 基本需求系统**不修改 GOAP** ——只有 survival 需求(hunger/thirst/fatigue/health/combat)进安全网。新增元素平衡/libido/social 不进 |
| ItemRegistry::get_consumable() | **物品系统** | Life(ingest_food)、NPC(进食选择) | struct 固有方法(非 trait)。查询 ConsumableEffect——None=不可食用 |

## CHG-028 新增契约（进阶需求系统 v1.0）

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| `esteem_deficit` | **04-进阶需求.GrowthColumn** (SoA) | NPC 决策器(need_action_match)、情绪引擎、社交系统(detect_admiration)、SelfNarrative | 心理状态——不入 Life.Vitals。+0.01/游戏日被动累积，社交钦佩事件恢复。**零新跨模块推送**——钦佩信号从已有社交管道检测。★ v2.1 方案D: 从 NpcData AoS 迁移至独立 SoA 列族 |
| `competence_frustration` | **04-进阶需求.GrowthColumn** (SoA) | NPC 决策器、情绪引擎、SelfNarrative、每日更新(update_competence_frustration) | Allostatic 模型——设定点 = aspiration skill gap。每游戏日更新。无 SkillMastery aspiration → 恒为 0。通过 `npc.skills` 内部查询。★ v2.1 方案D: 从 NpcData AoS 迁移至独立 SoA 列族 |
| `competence_frustration_chronic_days` | **04-进阶需求.GrowthColumn** (SoA) | 每日更新(update_competence_frustration)、决策器(chronic factor) | 慢性追踪——gap>0.3的天数。>30天→×1.5放大。|
| `honor_weight_for_domain()` | **文化系统**（CultureQueryExt 新增方法） | NPC 模块（esteem 计算） | ★ **唯一新跨模块方法**。`domain_code: u8` 参数——零类型依赖。从已有 CultureCoreParams 派生——零新存储 |
| `NeedSensitivity::esteem` | **03-基本需求.NeedColumn** (SoA) | NPC 决策器 | `0.2 + E×0.5 + (1-A)×0.3`。终身不变。公式由 04 定义，存储于 NeedColumn——与 NeedSensitivity 其他 6 字段同列 |
| `NeedSensitivity::competence` | **03-基本需求.NeedColumn** (SoA) | NPC 决策器 | `0.2 + C×0.5 + N×0.3`。终身不变。公式由 04 定义，存储于 NeedColumn |
| `survival_suppression()` | **NPC 模块**（概率权重链内部） | 仅概率引擎 | sigmoid `1/(1+e^(10(x-0.7)))`——软衰减。**不影响 GOAP** |
| `frustration_regression()` | **NPC 模块**（决策预处理） | NeedSensitivity 临时调制 | avg_frustration > 0.6 触发——ERG 挫折回归。社交+50%/尊重+40%/饥饿+30%/libido+25%。**不持久化** |
| `intrinsic_motivation_weight()` | **NPC 模块**（权重链） | 仅概率引擎 | `∏(1 + relevance×commitment×0.5)`, clamp [1,3]。relevance 为纯函数——可编译时优化 |
| `NeedTag::Esteem` / `NeedTag::Competence` | **NPC 模块** | ActionType 定义 | 新增 2 个需求标签。SeekRecognition/Compete/TrainSkill/SeekMentor 等行为映射 |
| GOAP 安全网边界 | **NPC GOAP** | 全部模块 | 进阶需求系统**不修改 GOAP**——心理需求不进安全网。只有生存需求+配偶压力进 |

## CHG-029 新增契约（审美与艺术系统 v1.0）

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| `AestheticSignal` (6维) | **审美系统** `05` | 全部模块（通过 HasAestheticSignal trait） | 审美模块只定义 struct——各实体模块在 trait impl 中计算。6维: fluency/novelty/complexity/harmony/expressiveness/virtuosity |
| `AestheticJudgment` (4维) | **审美系统** `05` | NPC/经济/战斗/情绪 | judge() 纯函数输出——零副作用。4维: valence/arousal/interest/respect |
| `AestheticTaste` | **05-审美与艺术.TasteColumn** (SoA) | NPC 决策器/情绪引擎（通过 `judge()` 间接消费——不直接读 taste 字段） | 40B Copy 类型。青春期 derive_taste()，年更新 mature_taste()，Adopt 事件传播。★ v1.1 方案D: 从 NpcData AoS 迁移至独立 SoA 列族 |
| `HasAestheticSignal` trait | **审美系统** `05`（定义）→ 各模块（实现） | 全部 | 12 个实现者覆盖 ItemEntId/BuildingId/CreatureId/NpcId/VehicleId/ScenePosition/PerformanceRef/SkillActionRef/CombatExchangeRef/SpellCastRef/MagicConstructRef/RitualRef。对标 ConsumableEffect schema 模式 |
| `judge()` 纯函数 | **审美系统** `05` | 任何模块 | 零副作用、零 I/O、零分配。确定性 jitter（seed=hash 三元组，存档可复现）。三层门控：注意力层→judge()层(全精度)→效应层(React/Articulate 调制) |
| `AestheticContext` | **审美系统** `05`（定义）→ 调用方（NPC 感知系统组装） | judge() | familiarity/prior_expectation 由调用方从记忆/文化/物品系统查询后填入——审美模块不查询任何模块 |
| React 原子 | **审美系统** `05**（处理） | 情绪引擎/记忆系统/行为树 | 判断→内部状态：情绪 delta + 记忆写入 + somatic_impact + 行为倾向。每次 judge() 后自动执行 |
| Articulate 原子 | **审美系统** `05**（处理） | 语言表达系统/经济系统/行为树 | 判断→外化：5通道(Silent/Exclamation/Social/Critique/Behavioral)。生存压力几乎完全压制语言通道 |
| Adopt 原子 | **审美系统** `05**（处理） | NPC AestheticTaste | 品味传播唯一机制。有效力=credibility×intensity×(1-aesthetic_confidence)。时尚/共识/流派全部从大量 Adopt 统计涌现 |
| Embellish 原子 | **审美系统** `05**（处理）→ **物品系统**（create_item） | 物品/经济/历史 | 审美意图→持久产出：Embellish(NewItem)→ItemRegistry::create_item()。novelty 对技能依赖 0.2（创意≠技术），virtuosity 0.95 |
| 注意力门控 | **NPC 感知系统** | — | 审美模块不管理注意力——调用方决定 judge() 是否被调用。饥饿者不注意到墙上的画 |
| 情欲吸引力 | **NPC 02-性别与吸引力系统** | — | 情欲=审美判断×吸引力×libido×社会——四系统交叉涌现。不归入审美模块 |
| `FineArts` 技能大类 (0x06) | **技能系统** `002` | NPC/Embellish | 与 Artisan(0x03) 分离——工艺≠艺术。4子组(Visual/Musical/Literary/Performing)8技能。跨类交叉训练最高 0.08（低于 Artisan 内部） |
| `CulturalBeautyStandard` | **文化系统** `004`（保持所有权） | NPC 02 / AestheticTaste 初始派生 | 审美模块不拥有 CBS——仅通过 derive_taste() 的参数引用 culture_params |
| `AestheticProps` | **物品系统** `003`（保持所有权） | 物品 HasAestheticSignal impl | 审美模块不引用 StyleTag/AestheticProps——物品系统在自己的 trait impl 中完成 AestheticProps→Signal 映射 |
| `judge_outfit()` | **物品系统** `004`（保留便捷包装）→ 委托给 **审美系统** `judge()` | NPC | v1.1 重写：内部调用 judge(outfit_signal, taste, ctx)——不再独立实现逻辑 |

## CHG-030 新增契约（音频系统 v1.0）

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| `SoundFootprint` (物理声学快照) | **音频模块** `002`（定义 struct）→ 各模块（写入实体） | 全部模块 | 描述物理现实（表面接触+运动+发声+物品交互+沉默）——不描述游戏语义。音频模块拥有映射逻辑。定义在 woworld_types |
| `ActionRingBuffer` (瞬时动作记录) | **音频模块** `002`（定义类型）→ 各模块（写入） | NPC决策器/记忆/大日志/音频 | 64 条×16B 环形记录。各模块本来就要记录"最近做了什么"——音频模块为多个消费者之一 |
| `AudioMaterial` (装备声学材质) | **音频模块** `002`（定义枚举·15变体）→ **物品系统**（存储） | 音频模块（读取+映射声音） | 对标 ConsumableEffect schema 模式——音频定义，物品只存不解。`Option<AudioMaterial>` 单字段 |
| `SurfaceMaterial` (地表声学材质) | **音频模块** `002`（定义枚举·21变体）→ **世界生成**（在Chunk上赋值） | 音频模块（脚步+碰撞映射） | 统一地表声学分类——替代各模块自制的"地面类型" |
| `HasSoundEmitter` trait | **音频模块** `002`（定义）→ 各模块（impl） | 音频模块 | 对标 HasAestheticSignal 模式。active_sounds()→持续声源（引擎/结界/瀑布）。零分配 |
| `HasSoundFootprint` trait | **音频模块** `002`（定义）→ 各模块（impl） | 音频模块 | 暴露 SoundFootprint + ActionRingBuffer。消费方定义 trait，提供方 impl |
| `HearingModel` (听觉参数) | **音频模块** `002`（定义 struct）→ Life/NPC（初始化存储） | NPC感知系统/战斗系统 | 听觉阈值+方向性+频率范围+听力损伤偏移。对标 Physiology 从 Vitals 派生模式 |
| `VoiceProfile` (声学身份) | **★音频模块** `005`（从语言表达 012 迁入） | NPC(存储)/语言表达(TTS消费)/社交系统 | 声学参数(base_pitch/timbre/speed/expressiveness/breathiness/roughness)。生成公式：种族×性别×年龄×大五。TtsEngine trait 保留在语言表达(渲染层) |
| `VoiceEmotionModulation` | **★音频模块** `005`（从语言表达 012 迁入） | 语言表达(TTS渲染) | 情绪→声学调制(pitch_shift/speed/volume/tremor/breathiness)。音频模块从 EmotionState 派生 |
| `VoiceManager` (播放队列) | **★音频模块** `005`（从语言表达 012 迁入） | 语言表达(TTS播放) | 五级 VoicePriority 队列+打断仲裁。Critical 打断一切 |
| `CurrentSpeech` (正在说的话) | **语言表达模块**（写入）→ **音频模块**（轮询） | NPC感知/记忆 | 语言表达写入 NpcData，不知道音频在读。包含 expression_ref+word_count+words_per_second+delivery |
| `SpeechPerception` / `fraction_heard` | **音频模块** `005` | NPC感知→语言表达 resolve_partial() | 对话时间维度：fraction_heard=传到了多少(传播延迟+语速×时间)，clarity=清晰度(掩蔽)。听者整合→部分文本 |
| `CancelReason` (打断原因) | **音频模块** `005` | NPC感知/记忆 | 8种打断原因(SourceDied/Unconscious/Teleported/Submerged/CombatImpact/Preempted/MagicallySilenced/VoluntaryStop) |
| `AudioQuery` trait (30方法) | **音频模块** `008` | 全部模块 | pub trait(Send+Sync)——对标 WeatherQuery。perceived_sounds()/perceived_speech()/ambient_noise_level()/acoustic_space()/speed_of_sound()/audible_radius()等。零分配 |
| `audible_radius()` / `effective_audible_radius()` | **★音频模块**（从语言表达 011 迁入） | 语言表达/战斗/NPC | 六因子公式(距离×环境噪声×地形×天气×文化×个性)迁至音频模块——这是声音传播物理 |
| `AudioRenderPacket` | **音频模块** | Godot 渲染层 | 对标 WeatherVisualPacket。每帧 ~3KB——包含 SoundRenderCommand[]+VoicePacket+MusicState+Notification+AmbientMix |
| 声音五分类 | **音频模块** `001` | 全部模块 | WorldSound(模拟层·3D·NPC可闻)/Voice(模拟+渲染·3D)/Music(渲染层·2D)/Notification(渲染层·2D)/Accessibility(渲染层·2D) |
| 传播引擎（衰减/吸收/风/温度/遮挡/介质边界） | **音频模块** `003` | — | 全部物理公式由音频模块 OWN。三种衰减/掩蔽/混响模型可选(TOML)。风矢量调制所有传播——不是仅背景风声 |
| 掩蔽引擎（频段分割/清晰度/环境噪声） | **音频模块** `004` | NPC记忆(话语清晰度→文本模糊化)/战斗 | 话语清晰度五档(Perfect/Clear/Garbled/Fragment/Inaudible)→决定语言表达 resolve_partial() 返回什么质量 |
| `PlayerNotificationEvent` (提示音·~45变体) | **音频模块** `007`（订阅 world_core 事件总线） | 全部模块（发射事件） | 各模块通过 world_core 发射枚举——不知道音频存在。音频订阅→查 notification_sounds.toml→播放 |
| 音乐三层模型 | **音频模块** `006` | 文化模块(FestivalMusicPreset)/技能(FineArts) | Layer1:系统BGM(情境×分层)/Layer2:世内NPC演奏(覆盖BGM)/Layer3:玩家自定义。文化模块定义节日预设——映射/过渡/分层在音频 |
| `AcousticTag` / `AcousticSpace` / `ReverbProfile` | **音频模块** `004`（定义+展开）→ **世界生成**（建筑上设置AcousticTag） | Godot混响渲染 | 世界生成只设标签(SmallRoom/StoneCathedral)——音频展开为AcousticSpace(叠加人数/天气) |
| `AudioAssetId` (u32) | **音频模块**（分配） | Godot渲染层 | 对标 ItemDefId。集中式 `audio/` 目录 + `audio_asset_map.toml` 映射表。Mod替换=换TOML+文件 |
| `SoundAestheticMetadata` | **音频模块**（填充）→ **审美模块**（消费） | 审美系统 | PerceivedSound 中附带 harmonic_complexity/rhythmic_regularity/frequency_centroid/temporal_variation |
| 身体声音（心跳/呼吸/肚子叫） | **音频模块**（内部）→ 消费 Vitals | — | 直接读 Life 已有 Vitals 查询——不新增接口。心跳:health<0.3加强+fear+exertion。呼吸:stamina<0.3。肚子叫:hunger>0.6概率 |
| 对话内容(语言表达) vs 声音(音频)分离 | **语言表达**（内容层）+ **音频模块**（声音层） | NPC感知 | 对标"书的内容(语言表达) vs 书的物理属性(物品系统)"。语言表达不知音频在读CurrentSpeech |
| 跨L层声音传播 | **音频模块** `001` | 全部 | L1全入缓冲/L2强度门控(can_reach_l1)/L3≥120dB+传播延迟/L2感知用聚落聚合(SettlementAudioAggregate对标SettlementFaithSnapshot) |

## CHG-031 新增契约（感官与知觉系统 v1.0）

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| `PerceptualCache` | **感官系统**（定义 struct）→ **NPC 模块**（存储于 SensoryState） | NPC决策引擎、记忆系统 | 有界(64实体)，LRU淘汰，不持久化。空缓存→新奇驱动探索行为涌现 |
| `VisionParams` | **感官系统**（定义 struct）→ **Life 模块**（初始化派生）→ **NPC 模块**（存储） | 感官系统（VisionQuery消费） | 对标 HearingModel 模式。从 race + perception + age + personality 派生 |
| `DarkAdaptation` | **感官系统**（定义 struct）→ **NPC 模块**（存储） | 感官系统（VisionQuery消费） | 指数松弛模型。亮→暗:慢(30min)，暗→亮:快(30s)。不持久化 |
| `VisionQuery` trait | **woworld_core**（定义）→ **感官 crate**（实现） | NPC crate | 纯函数——不依赖 NpcData，只接受离散参数。背景/信号二分类→自适应射线→分级细节→运动检测双路径 |
| `ScentQuery` trait | **woworld_core**（定义）→ **感官 crate**（实现） | NPC crate | 懒采样模型——查询时计算浓度，不模拟扩散。对标 WeatherQuery::sample() 模式。气味源由各模块管理，SpatialIndex 存储+过期 |
| `SpatialQuery` trait | **woworld_core**（定义）→ **世界空间管理 crate**（实现） | 感官/战斗/寻路/音频 | 纯几何+索引——不含业务语义。实体查询/射线/光照/材质/事件buffer/气味源/天际线 |
| `SpatialEntity` | **woworld_core**（定义）→ **世界空间管理**（写入） | 全部模块 | id+位置+速度+粗分类+隐蔽度+发光量+透明度。不含业务语义。NPC死亡不注销——velocity→0 |
| `SpatialEvent` | **woworld_core**（定义）→ **各模块**（写入事件buffer） | 感官（L1+L2消费） | 空间区块粒度(16m)，ring buffer(64)，自动过期(max(intensity×10s,5s)) |
| `CulturalKnowledgeBase` trait | **woworld_core**（定义）→ **TOML 数据驱动** | prepare_facts() | 文化基线知识查询——与文化参数查询(CultureQuery)分离。不同目的，不同trait |
| `Knowledge` | **NPC 模块**（定义 struct + 存储于 NpcData） | 决策引擎 prepare_facts() | 仅存超出文化基线的个人知识(~64条)。永不完全归零。三来源：LearnedFromExperience/Taught/Discovered |
| `AestheticFrameworks` | **审美系统**（定义 struct）→ **NPC 模块**（存储于 NpcData） | 审美系统 judge() | native+personal+adopted(无硬上限)。四独立过程：获取(intelligence+openness门槛)/深化(当前openness)/衰减(仅受memory影响,永不完全归零)/激活(当前cognitive_clarity+社会语境) |
| `AestheticFramework` | **审美系统**（定义 struct）→ **NPC 模块**（存储于 AestheticFrameworks） | 审美系统 judge() | 审美维度权重。从文化继承→经历修改。多框架加权混合——不是单选，是所有相关框架在NPC心中竞争 |
| `PerceptBatch` | **感官系统**（定义+产出） | NPC决策引擎/战斗情报/审美系统/大日志 | 统一知觉产出——不区分战斗/日常。包含实体+视觉场景+听觉场景+气味景观+内感受+环境。同一份数据供给所有消费者 |
| `PerceptualModifiers` | **感官系统**（定义 struct）→ **NPC 模块**（从状态效果派生） | VisionQuery/AudioQuery | 状态效果→感知调制。NPC crate 做桥接——状态效果系统不依赖感官 crate。零循环依赖 |
| `SensoryState` | **NPC 模块**（定义 struct 做组合，各子模块拥有字段定义权） | 感官系统/音频系统 | 组合 HearingModel+VisionParams+PerceptualCache+DarkAdaptation+VoiceProfile+ActionRingBuffer+Silence+CurrentSpeech。NPC crate 不做任何字段设计——只做组合 |
| GOAP事实知识源 | **NPC 模块**（prepare_facts() 推导）→ **GOAP**（消费） | GOAP/行为树 | FactSet 每决策周期临时构建——永不持久化。三源：感官缓存+长期记忆+文化基线。GOAP 永不直接查询世界真值 |
| 大日志感官输入 | **感官系统**（perceive_lightweight()）→ **大日志**（历史 005 消费） | 大日志 tick_journal() | 玩家角色跑轻量感官管线——实体发现+分类，不做注意筛选/噪声注入/内感受。大日志条目无上限不过期 |
| 战斗情报层级 | **战斗模块**（combat_intelligence.assess()）→ **战斗 AI**（消费） | 战斗AI | 消费 PerceptBatch → 派生 CombatantAssessment。HpEstimate/threat_level/observed_style 从 PerceptEntry 通用特征派生。战斗不重复感知。信息战机制不变 |
| 感知退化 | **感官系统**（定义连续函数）→ **NPC调度**（消费分桶层级） | NPC 决策循环 | 感知模型是距离的连续函数。LOD层级只是调度分桶——层级数量可随意增减，不改变感知算法 |
| 感官噪声确定性 | **感官系统**（PerceptualSeed 三部分hash） | 全部感官模态 | world_seed × npc_permutation(NpcId hash) × time_quantum(决策周期) × modality(独立salt) → Pcg64。保证同一存档同一时刻知觉完全相同 |

## CHG-032 新增契约（认知与智慧系统 v1.0）

> **完整规格**: [[WoWorld-Design/Happy Game/开发阶段/NPC活人感模块/06-认知与智慧系统/001-认知与智慧系统总纲|认知与智慧系统总纲]] + [[WoWorld-Design/Happy Game/开发阶段/NPC活人感模块/06-认知与智慧系统/007-跨模块依赖与接口契约|接口契约]]

### 核心契约

| 契约 | Owner | 消费方 | 关键约定 |
|------|-------|--------|---------|
| `CognitiveStyle` | **06-认知与智慧.CognitionColumn** (SoA, HOT) | 决策引擎/MentalModel消化/ThoughtTrigger | 4维（直觉-分析/冲动-反思/具象-抽象/顽固-灵活），从BigFive+wisdom+经历派生，含阻尼年度更新。★ v1.1 方案D: 从 NpcData AoS 迁移至 CognitionColumn |
| `CognitiveTide` | **06-认知与智慧.CognitionColumn** (SoA, HOT) | 决策引擎/ThinkingCheck/AgentSnapshot | 3维（负载/反刍压力/安静度），每决策周期重算但持久化以保证连续。★ v1.1 方案D: 从 NpcData AoS 迁移至 CognitionColumn |
| `CognitiveBiases` | **认知系统**（定义纯函数·惰性计算）→ **MentalModel消化**（消费） | 微消化/宏消化/assess_and_integrate | 7种偏误完全从CognitiveStyle+Emotion+Tide派生。零存储——对标Physiology::from_vitals() |
| `MentalModel` | **06-认知与智慧.CognitionColumn** (SoA, COLD) | 决策引擎(权重调制)/语言表达(WisdomSharing)/历史系统(MentalModelRecord)/技能系统(DeepMentorship) | ≤20条(~2.4KB)。跨代传递6路径全部通过已有通道。★ v1.1 方案D: 从 NpcData AoS 迁移至 CognitionColumn |
| `ThoughtFragment` | **认知系统**（惰性生成·仅玩家15m内）→ **Godot渲染**（已有通道） | 非语言表达/字幕/物品创建/对话 | 3级清晰度。片段组合模型(~200-300模板)。CognitiveStyle选择表达模式。产生SurfacingType→已有API调用 |
| `MindAttribution` | **06-认知与智慧.CognitionColumn** (SoA, WARM) | 决策引擎/对话系统/欺骗检测 | ≤16条(~960B)。Theory of Mind归因。4种来源(TargetStated/Observed/ToldByThird/Inferred)。★ v1.1 方案D: 从 NpcData AoS 迁移至 CognitionColumn |
| `BeliefHistory` | **06-认知与智慧.CognitionColumn** (SoA, COLDEST) | SelfNarrative/叙事系统 | ≤16条(~768B)。信念演变里程碑。★ v1.1 方案D: 从 NpcData AoS 迁移至 CognitionColumn |
| `mental_model_creation_count` | **06-认知与智慧.CognitionColumn** (SoA, COLD) | 07-生命周期(cognitive_engagement_score) | u32。try_induce_pattern()/creative_leap() 成功时递增。★ v1.1 方案D: 从 NpcData AoS 迁移至 CognitionColumn |
| `chronic_intoxication_years` | **06-认知与智慧.CognitionColumn** (SoA) ↔ 07-生命周期(写入) | 07-生命周期(health_burden) | f32。年均更新。★ v1.1 方案D: 从 NpcData AoS 迁移至 CognitionColumn。跨模块写入方：07-生命周期 |
| `SleepCognitiveProcessing` | **认知系统**（定义纯函数·睡眠结束时调用）→ **NPC**（内存/情绪/模型修改） | 记忆/情绪/mental_models | 对标SelfNarrative::reflect()。过拟合大脑假说框架。经验窄度→自适应梦境荒诞度。睡眠质量6因子派生——零新trait |
| `InnovationPipeline` | **认知系统**（定义6阶段纯函数管线）→ **各领域系统**（消费构造参数） | 战斗/魔法/艺术/建筑/工艺/社会 | formalize_innovation() 返回数据参数——不返回领域crate类型。领域crate已有一切构造API |
| `cognitive_distress` | **06-认知与智慧.CognitionColumn** (SoA, HOT) | CognitiveBreak检测/决策随机性 | 完全派生(矛盾+反刍+停滞+神经质)。每游戏日更新。★ v1.1 方案D: 从 NpcData AoS 迁移至 CognitionColumn |

### 已有CHG兼容性

零违反CHG-013至CHG-031全部已有契约。详细检查见007-跨模块依赖与接口契约 §六。

### 新增枚举variant（不新增trait）

| 新增variant | 目标枚举 | 目标系统 |
|------------|---------|---------|
| `WisdomSharing` | `DialogueIntentType` | 语言表达系统 |
| `SeekClarification` | `DialogueIntentType` | 语言表达系统 |
| `MentalModelRecord` | `SegmentType` | 历史系统·TextSegment |
| `ThoughtImprint` | `AetherImprint`子类型 | 历史系统·灵元素印记 |

### 设计原则

零新trait·零新调参旋钮·全部已有维度派生·全部惰性求值·全部可见行为涌现。

---

## CHG-033 模型动作与物理系统 v1.0

| TDI-032/045/202 | — | — | ⚠️ CHG-033 已取代: PhysicsServer3D→仅玩家。TDI 保留为历史参考 |

### 空间查询契约

| 概念 | Owner | 消费方 | 关键约定 |
|------|-------|--------|---------|
| TerrainQuery | **world_gen** (密度场) | 感官/导航/动画/战斗 | height_at/normal_at/terrain_raycast/density_at/is_walkable/surface_material_at/medium_at/light_level_at/sample_horizon。纯函数。DDA 射线在密度场上步进，~10µs/射线 |
| DensityProvider | **woworld_core** (trait 定义) | world_gen(基底层)/建筑(地基切削)/NPC(挖掘)/玩家(SDF雕刻) | density_at(WorldPos)→f32 + material_at→u8 + priority→u8 + layer_name→&str。Send+Sync。正值=实体，负值=空。优先级升序叠加——低prio先叠加，高prio后覆盖 |
| DensityStack | **woworld_worldgen** (编排器) | HeightfieldTerrain(内部查询) | 有序层叠容器。push() 插入保持排序，density_at() 累加所有层 |
| EntityIndex | **woworld_spatial** | 所有模块 | register/unregister/update_transform/entities_in_aabb/entity_aabb/acoustic_tag_at/close_relations/position_of。稀疏哈希网格 O(1)。layer_mask 过滤 |
| SpatialEventBus | **woworld_spatial** | 所有模块 | recent_events_in/push_event/scent_sources_in。Chunk ring buffer(64 entry,LRU)。事件自动过期 max(intensity×10s, 5s) |
| VisibilityQuery | **woworld_spatial** (Arc<TerrainQuery> + &EntityIndex) | 感官/战斗/大日志 | line_of_sight/line_of_sight_hit。DDA 同时检查密度场+实体AABB。命中返回 TerrainHit/EntityHit/WaterSurface |
| ReputationQuery | **woworld_spatial** (Arc<MemoryStore> + &SocialGraph) | 战斗/权力/文化/玩家UI | reputation_toward / local_consensus。标量 f32 ∈ [-1,1]。从分散式 NPC 记忆涌现，不建集中式声望数据库。实现侧聚合 MemoryStore + SocialGraph，trait 不预设存储方案 |

### 动画数据契约

| 概念 | Owner | 消费方 | 关键约定 |
|------|-------|--------|---------|
| SkeletonDef/BodyPlan/PhysicalAppearance | **woworld_core** | 动画/生命/战斗/物品 | BodyPlan 从 Life crate 提升到 woworld_core——跨模块基线。33骨(L1)/35骨(L0)。AnimationModules 四分区 |
| ModulePose/PoseDatabase | **woworld_model** (TOML加载) | woworld_animation | 38模块姿态+15基元轨迹。总~40KB。Keyframe 仅存关节角度——不同体型共用 |
| AnimationBodyState | **woworld_animation** | 动画层栈/GDExt | 每NPC 3.8KB。L1→L2降级丢弃不持久化。相位偏移每帧~83全更新 |
| GaitStyle(9连续参数) | **woworld_animation** (从BigFive×EmotionState派生) | NPC/动画 | 零手写数据。纯数学涌现 |
| FacialExpression(6B) | **woworld_animation** (从EmotionState映射) | GPU shader | 512²共享图集。文化默认+个人偏离。6B塞入INSTANCE_CUSTOM |
| WeaponPhysicalParams | **物品系统** → woworld_animation消费 | 战斗轨迹适配器 | 长度/重量/重心/握持位置→reach_scale/speed_scale/body_commitment |

### 物理契约

| 概念 | Owner | 消费方 | 关键约定 |
|------|-------|--------|---------|
| 玩家 CharacterBody3D | **Godot PhysicsServer3D** | 玩家输入 | 唯一保留 PhysicsServer3D 的东西。其余全部 Rust 侧空间查询 |
| COM 抛物体飞行 | **woworld_animation** | 战斗/环境/物品 | 重力+g·dt。DDA射线投射检测障碍。三级着地响应(致命>20m/s,重伤>8m/s,硬着陆<8m/s) |
| 骨架松弛死亡 | **woworld_animation** | 战斗/环境/生命 | 肌肉刚度 1.0→0.02 指数衰减。关节限制强制。不做布娃娃 |
| 双人交互约束 | **woworld_animation** (TOML模板) | NPC社交/战斗 | 锚点Coincident/Distance/Facing。被动方IK跟随。推搡=顺序中断(非双人IK) |
| 水下/骑乘模式 | **woworld_animation** | 环境/载具 | medium_at()驱动模式切换。骑手骨盆锚定马鞍transform |

### 渲染契约

| 概念 | Owner | 消费方 | 关键约定 |
|------|-------|--------|---------|
| MultiMesh + GPU skinning | **Godot** | woworld_animation输出 | 双骨蒙皮。INSTANCE_CUSTOM 16B vec4。双缓冲共享内存 |
| 面部纹理图集 | **Godot shader** | woworld_animation输出 | 512² RGBA8。16嘴×16眉×8眼×8虹膜。UV偏移暗向视线方向 |
| TABS式cel渲染 | **Godot shader** | 全部角色 | 单diffuse pass。无PBR。vertex color肤色。所有角色纹理VRAM<5MB |

### 跨模块变更

| 优先级 | 文件 | 变更 |
|--------|------|------|
| CRITICAL | 感官与知觉系统/001 | SpatialQuery→拆分为四个trait。wind_at→WeatherQuery |
| CRITICAL | 技术栈方案 v4.0 | 物理方案改为"仅玩家PhysicsServer3D"。性能预算更新 |
| HIGH | NPC ver2.0 §4 | "物理表达"替换为本模块引用 |
| MEDIUM | 生命/001 | BodyPlan定义→woworld_core |
| MEDIUM | 物品系统/001 | +WeaponPhysicalParams映射表 |

---

## CHG-041 NPC 生命周期系统 v1.0

> **完整设计**：[[WoWorld-Design/Happy Game/开发阶段/NPC活人感模块/07-生命周期系统/001-生命周期系统总纲|生命周期系统总纲]]

### 基座契约

| 概念 | 权威 Owner | 消费方 | 关键约定 |
|------|-----------|--------|---------|
| LifeStage (6阶段) | **Life** `004-身体状态与生命过程` | NPC生命周期 / 模型物理 / 技能 / 信仰（葬式） | Juvenile/Adolescent/YoungAdult/Adult/MiddleAge/Elder。属性乘数由LifeStage决定。不同物种有独立阶段百分比分布 |
| AgeClock | **Life** `014-生命周期时钟与事件`（CHG-056 审计新建） | LOD调度器 | 纯函数 advance_age(entity, delta_days, rng) → (LifeEntity, Vec<LifeEvent>)。同输入+同种子→同输出。与调用频率无关 |
| DeathCause (30种6类) | **Life** `004 §九` | 历史 / 信仰 / 战斗 / 权力 | 30种：创伤8/匮乏5/侵入5/奥术5/衰亡3/意志4。类别级亡灵规则。DeathCauseRegistry trait支持mod扩展 |
| DeathEvent / BirthEvent / LifeStageTransition | **Life** `014` | 所有模块独立订阅 | 中性事实——不携带情感建议/行为期望/人际通知。EventBus分发 |
| GestationState + FetusBlueprint | **Life** `004` | NPC生命周期（产前影响） | 受孕时确定种子——外观通过age_factor插值派生。FetusBlueprint编码青壮年标准模型 |
| FertilityPotential | **Life** `004` | 受孕概率计算 | 连续sigmoid曲线。替代旧FertilityDrivers（退役）。libido和fertility是分离信号 |
| InfantDependency | **Life** `004` | NPC GOAP / 世界生成 | Nursing→Weaning→Weaned三状态。L1由母亲GOAP驱动；L3/L4统计近似 |
| DeathSummary | **Life** `004` | 历史（LifeTrace） | 三阶段压缩：T+0全量→T+7 1-3KB→T+30 LMDB冷存档 |

### 行为层契约

| 概念 | 权威 Owner | 消费方 | 关键约定 |
|------|-----------|--------|---------|
| 生育欲望 | **NPC GOAP 通用管线** | 无处——和其他欲望同质 | 无专用通道。libido是身体信号（和hunger同质）。欲望形成走感知→认知→MentalModel→Aspiration→GOAP竞争 |
| 教育 | 技能模块（教学路径）+ NPC GOAP | 无处——涌现模式 | 三层涌现：家庭→社区→专职教师。4条教学路径覆盖。无"教育系统" |
| ControlMode | **NPC 生命周期** `008` | UI层 / GOAP引擎 | Auto/Manual/DomainDelegated。GOAP持续运转。被控角色无感知 |
| CognitiveAgingPath | **NPC 生命周期** `006` | 认知年度更新 / 战斗老年决策 / 语言流畅度 | Healthy/Pathological/SuperAging。由 crystallized_factor() + cognitive_engagement_score() + health_burden() 三函数联合派生（CHG-058）。高负担(>0.3)独立致病通路 pathological_annual_degradation() |
| Widowhood | **NPC 关系** | NPC认知/情绪管线 | 中性事实{deceased_partner_id, date, death_cause}。不携带情感预设 |
| PrenatalAccumulator | **NPC 生命周期** `003` | 新生儿初始化 | 可开关(enabled: bool)。出生时转移至新生儿→初始记忆偏差 |

### 核心设计原则（不可违反）

| 原则 | 内容 |
|------|------|
| **零年龄门控** | 没有任何 `if age < X { return CantDoThis }` |
| **零系统开关** | GOAP/审美/语言/认知从出生起持续运行 |
| **连续模型** | 所有能力是连续渐变，无阈值开关 |
| **中性事件** | LifeEvent不携带情感建议/行为期望 |
| **统一时间流速** | 所有LOD层共用相同时间流速——不可L3/L4加速老化 |
| **平等性** | 生育欲望和吃饭欲望走同一条通用认知管线 |
| **玩家=NPC+控制层** | ControlMode覆盖GOAP输出，非独立实体类型 |

### 已退役

| 退役项 | 原位置 | 替代 |
|--------|--------|------|
| FertilityDrivers struct + fertility_desire() | 生命 012 | FertilityPotential连续曲线 + 通用认知管线 |
| DeathCause逐条亡灵资格枚举 | 生命 004 | 类别级规则（I-V类默认可，IV/VI有例外） |
| competence_frustration=0（儿童硬编码） | NPC 04-进阶需求 | 从认知参数自然派生 |

## CHG-042 NPC 物理原子层 v1.0

> **完整设计**：[[WoWorld-Design/Happy Game/开发阶段/NPC活人感模块/08-NPC行动涌现与分类/001-NPC行动涌现总纲|NPC行动涌现总纲]]

### 基座契约

| 概念 | 权威 Owner | 消费方 | 关键约定 |
|------|-----------|--------|---------|
| **物理基元 (35个)** | **NPC行动涌现** `002` | 全部模块 | 定义在 `woworld_core::atoms`。MOVE/GRASP/STRIKE/ATTACH/IGNITE/OBSERVE... 零领域知识，仅做物理计算。各领域模块的execute()内部消费。不设门控 |
| **AgentSnapshot** | **NPC行动涌现** `004` | 全部模块 | ~108B SoA连续能力快照。26字段(身体7/认知8★v1.1/社交4/生命周期4/环境3)。认知段含CognitiveTide 3字段(cognitive_load/rumination_pressure/mind_quietude)。从Life/Skill/Cognition/Lifecycle/Weather瞬时派生。零年龄门控。不持久化。单cache line友好 |
| **MaterialProperties** | **NPC行动涌现** `008` | 物品系统/Combat/Magic/Physics | MaterialDef TOML注册表(~30字段)。物品系统存储(`Option<MaterialDefId>`)+查询，不解释含义。消费模块各自读取。遵循ConsumableEffect委托模式 |
| **execution_noise()** | **NPC行动涌现** `006` | 全部消费物理原子的模块 | `execution_noise_std(level) = BASE_NOISE × (1-level/100)²`。技能→原子执行精度连续映射。不新增SkillEntry字段。不修改proficiency() |
| **KnowledgeSeed** | **NPC行动涌现** `005` → 技能系统/历史系统 | World Gen P8/P9 | TOML技术时代时间线。时代框架手工设计，传播路径自动生成。generate_starting_skills()消费。不修改TeachingSession/XP公式 |
| **碰撞扫掠** | **NPC行动涌现** `007` | 战斗系统 | 武器capsule×身体15capsule→轨迹扫掠求交→命中判定。替换O(1)几何查表。capsule-capsule~5ns/test。非Rigid武器PBD约束求解 |
| **IK动画管线** | **NPC行动涌现** `007` | 模型/动画系统 | 零预设战斗动画。2-bone analytical IK从武器轨迹生成。躯干/腿/脚/头的完整IK链。比预设动画混合快~9倍。在骨骼≤0.5ms预算内 |
| **三层原子架构** | **NPC行动涌现** `001` | 全部模块 | L1物理基元(35)→L2领域复合(~40)→L3 GOAP候选(~25)。各模块注册ActionCandidate。NPC模块不拥有列表。新模块=新注册=GOAP自动可见 |
| **AtomEvent** | **NPC行动涌现** `002` | 感官/音频/历史 | 所有物理原子自动推送至SpatialEventBus。音频模块轮询SoundFootprint。感官模块消费PerceptEntry。历史模块检查AetherImprint触发 |

### 性能契约

| 度量 | 数值 |
|------|------|
| 碰撞扫掠 / 帧 | ≤0.1ms |
| IK 解算 / 帧 | ≤0.5ms（在现有骨骼预算内，零增量） |
| MaterialProperties 查询 / 帧 | <0.05ms（24KB L2常驻） |
| 非Rigid武器 PBD / 帧 | ≤0.3ms（动态降级） |
| 总 CPU 增量 | ≤0.45ms（6.4% Rust预算） |
| 总内存增量 | ~10.2MB（0.7% NPC数据） |
| VRAM 影响 | 零（碰撞全部CPU侧） |

### 设计原则

| 原则 | 含义 |
|------|------|
| **零年龄门控** | 不设 `if age < X { cannot_do(Y) }`。所有差异从AgentSnapshot连续参数涌现 |
| **零硬编码禁止** | 不写"中毒→不能劳动"。health_penalty→全原子效用↓→GOAP自然选择 |
| **动画是物理的输出** | IK从武器轨迹生成所有动画。零预设战斗动画 |
| **材料属性是数据** | "铁受热变软"不是规则——是MaterialProperties热软化自动计算 |
| **玩家=NPC** | 同一套原子引擎、同一套AgentSnapshot

## CHG-043 新增契约（建筑模块 v1.0）

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| ComponentFamily / ComponentRegistry | **建筑模块** `001` | 全部生成器、Mod 扩展 | 参数化组件族——关联类型 `Params`。Mod 通过 TOML 注册，非运行时 register()。非全局单例——依赖注入 |
| ComponentInstance / Building | **建筑模块** `002` | World Gen, NPC, Combat, Senses | Building=组件图+空间索引+所有权。无 `building_type` 标签——建成后消失。热冷数据分离 |
| BuildingQuery trait | **建筑模块** `003` | Weather, Senses, NPC, Combat, Audio, Economy, Aesthetic, History, Culture（10 消费方） | 建筑世界数据唯一只读接口。Send+Sync。15+方法。热路径 ≤1.8ms/帧 |
| Surface trait | **建筑模块** `003` | Items（PlacedItem 放置消费） | 可放置表面——Floor/Wall/Ceiling/Counter。建筑模块不碰家具管理 |
| BuildContext | **建筑模块** `008`（定义） | World Gen 胶水代码（聚合各模块参数后传入） | 六维参数聚合（RaceBodyProfile+ClimateBuildProfile+MaterialAvailability+CultureBuildProfile+FaithBuildProfile+OwnerBuildPrefs）。建筑模块不 import Culture/Faith/Weather/Life |
| BuildingGenerator trait | **建筑模块** `007` | World Gen P5/P6, 玩家编辑器 | 8 种生成器实现——WfcRectangular(80%)+WfcRadial+Cathedral+Complex+Wall+Bridge+Underground+Shelter |
| WfcBuildingSolver | **建筑模块** `004` | World Gen P5/P6, 玩家编辑器预览 | 2.5D 三阶段（BSP→2D WFC→3D 组装）。≤5ms/栋。城市级 WFC 留在 World Gen |
| ConstructionScheduler / ConstructionTask | **建筑模块** `005` | NPC 行动（CONSTRUCT 原子）、玩家 | 声明式施工——逐组件 Task，阶段级渲染。≤0.2ms/tick |
| MaterialRequirementList | **建筑模块** `005`（产出） | NPC EconomicBehavior（自行规划获取方式） | 声明式——不碰 Market trait。施工负责人（NPC/玩家）自行决定如何获取材料 |
| ConstructionModifier trait | **建筑模块** `005`（定义）→ Magic（实现） | ConstructionScheduler | 魔法辅助建造预留——modifier 从外部传入，Scheduler 不持有实例 |
| Blueprint（TOML 格式） | **建筑模块** `006` | 玩家 DIY, Items（Blueprint 0x52） | 玩家设计文件，跨存档可分享。连接关系几何自推导。`cultural_hint` 偏好提示——跨文化移植自动本地化 |
| BuildingHistory | **建筑模块** `002` | History（AetherImprint 爬取）, NPC 认知 | 双层时间窗口存储——活跃层（10 年完整）+归档层（50 年快照聚合） |
| StructureValueFactors | **建筑模块** `003`（产出） | Economy（房产估值） | 估值因子——不含价格。价格由经济系统自行计算 |
| BuildingMaterialProps | **建筑模块** `003`（定义） | 胶水代码（从 MaterialProperties 转换） | 建筑模块不 import NPC 物理原子层的类型。胶水代码做 `.into()` 转换 |
| BuildingStylePreferences → 建筑模块 | **文化系统** `004`（权威）→ 建筑模块（消费） | 建筑模块（通过 BuildContext） | 文化系统为权威 Owner。建筑模块不重复定义 RoofStyle/WallMaterial——废弃 005 中的 BuildingStyle struct |
| SacredArchitectureParams → 建筑模块 | **信仰系统** `002`（权威）→ 建筑模块 CathedraGenerator（消费） | 建筑模块（通过 BuildContext） | 信仰系统为权威 Owner。7 维（几何/朝向/高度/光线/图像/材质/布局）驱动 CathedralGenerator |
| ClimateParams → 建筑模块 | **天气系统**（权威）→ 建筑模块（消费） | 建筑模块（通过 BuildContext） | 雪荷载→屋顶坡度，温度→墙厚，风速→建筑高度上限，洪水→地基类型 |
| RaceBodyPlan → 建筑模块 | **生命系统**（权威）→ 建筑模块（消费） | 建筑模块（通过 BuildContext） | avg_height→天花板高度/门高/楼梯步高，avg_width→走廊宽 |
| World Gen 005 迁移 | **World Gen 005** → **建筑模块** | BuildingData/RoomData/BuildingStyle/WFC 两阶段等全部 struct/enum/trait——所有权转移至建筑模块 | World Gen P5/P6 保留编排调用，城市级 WFC 保留在 World Gen |
| CONSTRUCT 原子 → 建筑模块 | NPC 行动涌现 `003`（CONSTRUCT）→ 建筑模块（Blueprint 约束+load-bearing 计算） | 建筑模块提供 Blueprint 类型和 load-bearing 验证，CONSTRUCT 的 domain checks 消费 | |

### 建筑模块设计原则

| 原则 | 含义 |
|------|------|
| **建筑模块不碰市场** | MaterialRequirementList 是声明式的——NPC 自行规划材料获取。建筑模块不 import Economy |
| **建筑模块不定义风格** | BuildContext 从外部接收文化/信仰参数——建筑模块只做参数到组件的映射 |
| **房间功能不存储** | 房间功能是涌现标签——从家具分布+NPC 使用行为实时派生，存储于 NPC MentalModel |
| **家具不属于建筑模块** | 家具定义在 Items 模块，放置实例由 Items 管理——建筑模块只提供 Surface trait |
| **Blueprint 是玩家格式** | Blueprint 仅供玩家 DIY——世界生成走 WFC，NPC 翻修走 LocalIncrementalSolver |
| **连接关系几何自推导** | Blueprint 不存储连接关系——从组件位置+ConnectionFace 兼容性自动计算 |

---

## CHG-044 概念与语言地基系统 v1.0

> **完整设计**: [[WoWorld-Design/Happy Game/开发阶段/概念与语言地基/001-概念与语言地基总纲|概念与语言地基总纲]]
> **参考大纲**: [[WoWorld-Design/参考文档/036-概念与语言地基设计探讨-20260619/001-概念与语言地基大纲|036-大纲]]

### 基座契约

| 概念 | 权威 Owner | 消费方 | 关键约定 |
|------|-----------|--------|---------|
| PatternSignature (u64) | **概念与语言地基** `002` | 集成层、信息传播、NPC | 从权力拓扑/经济/关系中确定性哈希的模式指纹——全球客观。compute_pattern_signature()是woworld_core纯函数 |
| ConceptLocalId (u16) | **概念与语言地基** `002` | NPC、语言表达、信息传播 | 文化内的概念标识——必须搭配CultureId才有意义。高4位命名空间(0=核心,1-15=Mod)，低12位文化内序号 |
| CultureConceptDef | **概念与语言地基** `002` | 集成层、woworld_lang | 文化概念空间中的一条概念：coverage(覆盖的模式空间)+granularity(粒度)。TOML数据驱动 |
| CompactConceptEncoding ([u8;64]) | **概念与语言地基** `002` | NPC(EventMemory)、历史(PhysicalBook)、信息传播(InformationPayload) | 64B压缩概念编码：pattern_sig+3个最佳概念ID+置信度+detail_level+flags。自包含展开——不需要外部概念空间 |
| Utterance | **概念与语言地基** `003` | 语言表达、音频、LLM增强 | 结构化话语{concepts,speech_act,language,delivery}——替代裸String作为NPC语言产出的统一格式 |
| UtteranceId (u64) | **概念与语言地基** `003` | 音频(CurrentSpeech) | 瞬时话语标识——与ExpressionRef(持久可读物句柄)分离。仅当次对话有效 |
| TextGenerator 三模式 | **概念与语言地基** `003` → 语言表达模块 | 语言表达、音频 | render_utterance()(对话)+generate()(书籍·已有)+render_thought()(自语)——三入口共享FragmentLibrary |
| translate_concepts() | **概念与语言地基** `003`(woworld_lang) | 集成层 | 跨文化概念翻译——源概念模式覆盖投影到目标文化概念空间，找最大重叠。三种情况：直接翻译(overlap>0.8)/近似翻译+描述补全(>0.3)/描述性短语(无匹配) |
| effective_concept_space() | **概念与语言地基** `002`(woworld_lang) | 集成层 | 纯函数——从birth_culture+residences加权派生混合概念空间。母文化0.6+居住每年0.05(上限0.3) |
| language proficiency | **概念与语言地基** `004` | NPC、语言表达、音频 | 从原子日志LISTEN/SPEAK/READ/WRITE/EXAMINE涌现——S曲线增长。衰减：童年习得地板0.4/青少年0.25/成人0.05。零枚举路径 |
| script proficiency | **概念与语言地基** `004` | NPC、语言表达 | 独立于language proficiency——从READ/WRITE/EXAMINE原子涌现。书籍理解=lang_prof × script_prof × concept_overlap |
| 概念习得 | **概念与语言地基** `004` | NPC | 暴露累积>阈值→习得，初始低置信(0.3)。阈值受认知风格调制(分析型更高，灵活型更低) |
| 演绎推理 deductive_chain() | **概念与语言地基** `005`(woworld_core) | NPC | MentalModel链式应用到事实。浅层(depth≤2)在memory_encode时自动执行。深层在expand_inference时惰性重构 |
| 推理三模式 | **概念与语言地基** `005` | NPC/认知 | 归纳(try_induce_pattern·特殊→一般)+类比(creative_leap·特殊→特殊)+演绎(deductive_chain·一般→特殊)——完整覆盖 |
| wisdom_effective() | **概念与语言地基** `005`(NPC crate) | NPC决策 | 纯函数派生——自省(0.30)+经验内化(0.25)+认知平衡(0.25)+灵活性(0.10)+年龄(0.10)。不存储 |
| life_event_cognitive_shift() | **概念与语言地基** `005`(NPC crate) | NPC认知更新 | 重大事件→CognitiveStyle方向性偏移。冲动失败→反思+，教育→抽象+，创伤→直觉+固执 |
| transmission_fidelity() | **概念与语言地基** `006`(woworld_core) | 集成层 | 六条路径独立保真度衰减率。书面k=0.02(100代后0.13)、口传k=0.15(10代后0.11)、师徒k=0.05-0.08 |
| knowledge_preservation_impulse() | **概念与语言地基** `006`(NPC crate) | NPC WritingImpulse | 从wisdom×年龄×有价值知识×紧迫感派生——知识保存从个体行为涌现为制度 |
| settlement_knowledge_health() | **概念与语言地基** `006`(NPC crate) | 世界生成 | 书籍(0.40)+师徒(0.35)+口传(0.25)加权。检测唯一持有者风险、书籍退化风险 |
| concept_space_drift() | **概念与语言地基** `006`(woworld_lang) | 集成层 | 使用频率→边缘化/强化。外部文化接触→概念覆盖漂移。每代评估 |
| LLM增强位置 | **概念与语言地基** `007` | LLM增强层 | LLM在概念层操作Utterance.concepts——不直接产出字符串。TextGenerator负责所有渲染。失败→回退确定性概念 |
| 概念识别位置 | **概念与语言地基** `002` | 集成层 | classify_pattern()在集成层调用——各业务模块不知道"概念"的存在。NPC只接收已识别的CompactConceptEncoding |
| NpcData新增字段 | **概念与语言地基** `001` | NPC | birth_culture/native_language/native_script/known_scripts/acquired_concepts/residences/atom_execution_log/concept_space_version。L1 NPC ~4.3KB |
| AgentSnapshot新增字段 | **概念与语言地基** `001` | NPC行动涌现 | language_proficiencies(8B)/script_proficiencies(4B)/dominant_concept_culture(2B)/concept_space_version(2B)/literacy_best(4B)。总计20B |
| EntityIndex::close_relations() | **概念与语言地基** `008`(woworld_types trait扩展) | 集成层 | 已有EntityIndex trait新增方法——返回关系强度>threshold的实体。用于社会事件间接感知路由 |
| 文化≠语言(多对多) | **概念与语言地基** `004` | 文化系统、语言表达 | 一种文化可有多语言，一种语言可服务多文化。概念空间附属于文化，词汇附属于语言 |
| 名字渲染 | **概念与语言地基** (woworld_lang) | 全部显示层 | PersonalName{ given_name, family_name, patronymic, epithet }——结构化名字。render_name()根据observer_culture+familiarity渲染 |

### 核心设计原则

| 原则 | 内容 |
|------|------|
| **思维是结构化数据，语言是惰性渲染** | NPC内心以概念思考，自然语言仅在需要外显时生成 |
| **概念先于词语** | 概念是语言无关的，词语是概念在特定语言中的投影 |
| **文化相对概念空间** | 同一客观模式，不同文化可能识别为零个或多个不同概念 |
| **翻译=模式重叠搜索** | 源概念的模式覆盖投影到目标文化概念空间，找最大重叠 |
| **零新门控，零硬编码路径** | 语言学习/概念习得/认知风格演变全部从连续参数数学交互涌现 |
| **概念识别在集成层** | 各业务模块不知道"概念"的存在——它们只产出客观数据 |
| **玩家=NPC（贯彻到语言层）** | 同一套Utterance/proficiency/概念空间。UI语言是独立渲染目标 |

### 数据预算

| 类别 | 数值 |
|------|------|
| TOML数据（启动加载） | ~1.2MB |
| NpcData新增（1000 L1 + 100K L2） | ~34MB |
| EventMemory新增（1000 L1 × 2000条） | ~144MB |
| 总新增（含AgentSnapshot/AtomLog） | ~180MB（占1.4GB预算13%） |
| 帧CPU增量 | <0.5ms（<3%帧预算） |

---

## CHG-046 植被系统架构升级

> **完整设计**: [[WoWorld-Design/Happy Game/开发阶段/生命/010-植被群落与覆盖|生命 010]] · [[WoWorld-Design/Happy Game/开发阶段/世界生成/012-植被覆盖生成|世界生成 012]]
> **CHG文档**: [[Change/CHG-046-植被系统架构升级-20260620|CHG-046]]

### 基座契约

| 概念 | 权威 Owner | 消费方 | 关键约定 |
|------|-----------|--------|---------|
| VegetationProvider trait | **woworld_core** 定义 → **Life 010** (woworld_vegetation) 实现 | ✅ 天气 · 🔗 建筑/NPC/音频/经济/文化 · ⚠️ 战斗(接口预留) | Pattern D (trait inversion)。7方法。消费方三级：✅已验证(天气) · 🔗设计合理接收端已补(建筑/经济/文化/音频/NPC) · ⚠️接口预留(战斗掩体/NPC森林恐惧/历史读树) |
| TimberAvailability | **woworld_core** (共享类型) | 建筑(BuildingContext.materials)/经济(市场供给) | `available: bool, quality: TimberQuality, abundance: f32, harvest_difficulty: f32, dominant_species: Vec<SpeciesId>`。单一事实来源——Building不import植物类型 |
| TimberQuality | **woworld_core** (共享类型) | 建筑/经济/物品 | `Softwood \| Hardwood \| TropicalHardwood \| GiantWood \| MagicWood`。从PlantSpecies形态参数推导——非手工标注 |
| GroundCoverMap | **woworld_core** (共享类型) | 音频(脚步声)/NPC(移动速度) | `grass_density/moss_density/leaf_litter/bare_soil` 四通道，和=1.0 |
| WoodMaterialContract | **Life 010** §五 | 建筑/经济/物品/NPC物理原子 | 木材从PlantSpecies→TimberAvailability的完整推导链。5模块隐式链路收敛为VegetationProvider单一trait枢纽。Building crate的Cargo.toml不含woworld_life依赖 |
| P2.25 植被覆盖 | **World Gen 012** | P2.5(文化)/P3(资源)/P3.5(动物) | 植被是自然基底(P2.25位于P2和P2.5之间)，非生命层(P3.5)。L6.5密度场四通道在此阶段正式生成。P3木材zone从VegetationCover推导(非独立噪声)。P3.5动物初级生产力从VegetationCoverMap查询 |
| 演替阶段 | **Life 010** §四 | World Gen P2.25/运行时查询 | 纯函数 `f(years_since_disturbance, soil_fertility, climate)` ——不存枚举标签。同一片森林50年后重访→自动过渡到下一阶段 |
| 群落涌现 | **Life 010** §三/§六 | World Gen P2.25 | 香农熵加权优势种筛选(丰富度非硬编码整数)。Voronoi tessellation林窗检测。并查集连通分量→森林斑块从连通性涌现(无预定义ForestRegion) |

### 核心设计原则

| 原则 | 内容 |
|------|------|
| **植被是自然基底，不是生命层** | P2.25位于地形(P2)和文化(P2.5)之间——和地形一样是画布 |
| **trait隔离** | 消费方依赖woworld_core trait，不依赖Life crate植物类型。Building完全不知道PlantSpecies的存在 |
| **木材品质涌现** | TimberQuality = f(wood_type, max_trunk_diameter, spirit_capacity)。新增金属木植物→只需woworld_vegetation内添加映射规则 |
| **群落从数学涌现** | 香农熵、Voronoi、并查集——从连续参数场自然推导，零硬编码分类 |
| **演替是纯函数** | 不存succession_stage标签。同一片林随years_since_disturbance流逝自动演进 |
| **种子确定性** | 每Chunk RNG从hash(seed, "vegetation", cx, cy)派生。T0实例零存储，PlantInstanceId从种子推导 |

### 数据预算

| 类别 | 数值 |
|------|------|
| VegetationCover LMDB（全量75K Chunk） | ~50MB（密度场降采样策略） |
| PlantCommunityTemplate（每Chunk ~100B） | ~7.5MB |
| SurfaceMaterialMap | ~19MB |
| TimberAvailabilityMap | ~5MB |
| P2.25世界生成耗时（16线程rayon） | ~16s（全量VMC模板~3s + 近场预展开~13s） |
| 运行时查询（canopy_closure × 100 Chunk/帧） | ~0.1ms |

## CHG-048 新增契约（全模块交叉审计修复——Wave A 基础设施清理）

> **来源**: CHG-047 全模块系统性交叉审计 Phase 2-5 发现，CHG-048 修复执行（2026-06-20）
> **影响范围**: 模块接头总览·全局基础设施·世界生成编号·生命编号·CHG文书

### 幽灵 Trait 注册

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| EconomyQuery trait | **woworld_core** (定义) → **经济 crate** (实现) | 概念与语言地基(compute_pattern_signature)·权力(legitimacy推导)·NPC(消费决策)·世界生成(Bootstrap) | 8方法(query_price/query_market_volume/query_wealth_distribution/query_trade_routes/query_production_capacity/query_consumption_demand/query_labor_market/query_economic_health)。标准Query模式——对标CultureQuery |
| OceanProvider trait | **woworld_core** (定义) → **海洋 crate** (实现) | Godot渲染·载具(导航)·世界生成(P1海洋系统) | 6方法(wave_height/wave_direction/current_velocity/depth_query/sea_state/is_navigable)——预留FFT升级 |
| ElevationQuery trait | **woworld_core** (定义) → **世界生成** (实现) | 天气·感官·载具 | 1方法(elevation_at)——地形高度查询 |
| ClimateParamsQuery trait | **woworld_core** (定义) → **世界生成** (实现) | 天气·生命 | 1方法(climate_params_at)——气候参数查询 |
| OceanCurrentQuery trait | **woworld_core** (定义) → **世界生成** (实现) | 天气·载具·世界生成 | 1方法(current_at)——洋流查询 |
| WorldBoundaryQuery trait | **woworld_core** (定义) → **世界生成** (实现) | 天气·载具 | 2方法(boundary_distance/is_inside)——边界距离/内部判定 |
| VisionQuery trait | **woworld_core** (定义) → **感官 crate** (实现) | NPC crate | 4方法(visible_entities/line_of_sight/visual_signature/occlusion_query) |
| ScentQuery trait | **woworld_core** (定义) → **感官 crate** (实现) | NPC crate | 4方法(scent_sources_at/scent_intensity/scent_trail/scent_identity) |

### 编号体系修复

| 修复项 | 变更 | 影响文件 |
|--------|------|---------|
| 世界生成 012/013/014 去重 | 012-权力→013-权力·013-校验→014-校验。全部交叉引用同步更新。005 确认为已归档——README 已记录。008-旧版(NPC初始化)自声明已退役——009引用修复。 | 世界生成/ 11文件 + 接口出口 1文件 |
| 生命 010 去重 | 010-植被群落与覆盖(保留010)·010-神明→013-神明。README更新——新增010植被条目。信仰/文化模块 6文件引用同步。 | 生命/ 5文件 + 信仰/ 4文件 + 文化/ 2文件 |
| CHG-XXX 占位符替换 | 15处 → CHG-041(生命周期·DeathCause/FertilityDrivers/FertilityPotential等)·CHG-045(世界生成v2.0重构)。涉及生命/NPC/世界生成接口出口入口和变更日志。 | 5文件 |

### 缺失基础设施

| 修复项 | 新建文件 | 说明 |
|--------|---------|------|
| NPC活人感模块根 README | `NPC活人感模块/README.md` | 7子模块索引+六层模型速览+关键参数表 |
| 载具系统 README | `载具系统/README.md` | 10文档索引+载具分类+关键参数表 |
| 变更追踪/影响地图 | `变更追踪/002-影响地图.md` | 24模块影响矩阵+高风险Top10+消费方反查 |

### CHG-048 设计原则

| 原则 | 说明 |
|------|------|
| **trait 必须注册** | 任何被跨模块消费的 trait 必须在 00-全局基础设施/003-通用trait索引 中注册完整签名。经济模块的 EconomyQuery 对标 CultureQuery/FaithQuery/AudioQuery 标准 Query 模式 |
| **编号必须唯一** | 每模块内编号前缀不得重复。退役文档在 README 中标记并归档——不占编号。旧版文档自声明退役+指向权威替代 |
| **CHG 必须编号** | 任何接口变更必须分配 CHG 编号并同步至模块接头。禁止遗留 CHG-XXX 占位符 |
| **README 必须存在** | 每个模块根目录必须有 README.md（≥60行）——含模块元数据+文档索引+架构速览+关键参数表 |

---
## CHG-049 新增契约（LOD 架构全面深化）

> 📘 **权威规格**: [[WoWorld-Design/Change/CHG-049-LOD架构全面深化-20260620|CHG-049]]。LODCoordinator 所有权归**技术栈方案 v4.0 §二十一**。

### LODCoordinator 接口

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| LODCoordinator | **技术栈方案** §二十一 | 全部模块 | Rust 侧纯函数，每帧一次。输入: CameraState+PlayerAttention+FrameBudget+VramPressure+Entities+Broadcasts+Interactions。输出: `HashMap<EntityId, LodPrescription>`。自身 <0.05ms |
| LodPrescription (7维) | **技术栈方案** §二十一 | 全部模块 | `scene_lod(0-7)/skeleton_lod(0-4)/animation_lod(0-4)/render_lod(0-4)/physics_lod(0-4)/audio_lod(0-4)/ai_lod(0-4)`。~8B/实体。各维度对外契约见 CHG-049 §三 |
| VramPressure | **技术栈方案** §二十一 → woworld_core `VramLedger` | LODCoordinator | current_ratio+predicted_ratio_10fr。Rust 侧记账（非 GPU 查询），±15% 精度。3 级阈值 70/85/95% |
| InteractionIntent | **NPC 活人感/GOAP** → LODCoordinator | LODCoordinator | 7 种类型(PhysicalContact/Combat/Trade/Conversation/PublicSpeech/CasualAcknowledgment/Theft)。级联拉升预算制(≤0.3ms) |

### 统一距离带（所有模块以此为权威）

| 概念 | 权威 Owner | 消费方 | 关键约定 |
|------|-----------|--------|---------|
| 场景距离带 (8层) | **技术栈方案** §二十一 | 世界生成/建筑/植被/海洋/云 | 0-30-80-200-500-1500-4000-10000-inf (m)。体素 0.5→1→2→4→8→16→32→64m。LOD7=Billboard+大气 |
| 角色距离带 (5层) | **技术栈方案** §二十一 | 模型/动画/渲染/物理/AI/音频 | 0-15-60-200-800-inf (m)。骨骼 35→33→28→15→0。基于 Hall 人际距离学 |
| 音频距离带 (5层) | **技术栈方案** §二十一 | 音频系统 | 0-30-100-300-1000-inf (m)。可被 CommunicationIntent+AcousticProjection 覆盖 |

### 跨维硬约束（不可违反）

| 约束 | 权威 Owner | 消费方 | 说明 |
|------|-----------|--------|------|
| skeleton_lod=4 → anim=4, render=4, physics=4 | **技术栈方案** §二十一 | 模型/动画/物理 | 0骨不可逆 |
| scene_lod=N → skeleton_lod≥max(0,N-2) | **技术栈方案** §二十一 | 世界生成/模型 | 场景降了骨架必须跟 |
| ai_lod≥3 → physics≥3, animation≥3 | **技术栈方案** §二十一 | AI/物理/动画 | 统计NPC不需碰撞/动画 |
| skeleton_lod≥2 → physics_lod≥2 | CHG-042 | 模型/物理 | 骨骼粗→物理粗 |
| animation_lod≥skeleton_lod | CHG-033（动画模块内部） | 动画 | 模块内部 clamp |

### 各维度跨模块约定

| 概念 | 权威 Owner | 消费方 | 关键约定 |
|------|-----------|--------|---------|
| scene_lod → Building 映射 | **技术栈方案** §二十一 | 建筑模块 | LOD0(0-30m全WFC)→LOD1(30-200m外壳)→LOD2(200m+统计)。Landmark +1 修正。仿真 LOD 独立 |
| scene_lod → 植被 VMC | **技术栈方案** §二十一 | 植被 CHG-046 | VegetationProvider 消费 scene_lod，内部映射 4 级。Pattern D trait inversion |
| ai_lod ↔ 生命周期 | **技术栈方案** §二十一 | 生命周期 CHG-041 | L1↔ai0-1, L3↔ai2-3, L4↔ai4。统一时间流速。advance_age 对 LOD 零耦合 |
| animation_lod 契约 | CHG-033 动画模块 | 渲染/感官/审美 | CHG-049 定义 5 档对外契约。内部 9 层退化由 CHG-033 自行实现 |
| audio_lod 意图覆盖 | **技术栈方案** §二十一 | 音频 CHG-030 / 语言 CHG-018 | CommunicationIntent 5 级(whisper/normal/loud/shout/declamation) + AcousticProjection(非语言声源)。AudioBroadcast 广播优化。玩家专注聆听(Alt键) |
| 级联交互 | **技术栈方案** §二十一 | NPC/战斗/经济 | 单向·被动·按需拉升。InteractionIntent 指定最低 LOD。预算制并发上限 |

### 过渡时序

| 维度 | 升级 | 降级 | 权威 |
|------|------|------|------|
| scene | 0ms | 500ms迟滞 | 技术栈方案 §二十一 |
| skeleton | ≤200ms | 0ms | CHG-033 |
| animation | 200-500ms | 0ms | CHG-033 |
| render | 0ms(触发dither) | 200ms dither | CHG-033 |
| physics | 0ms | 0ms | CHG-042 |
| audio | 0ms | 0ms | CHG-030 |
| ai | 1-3帧(<50ms) | 0ms | CHG-032/042 |

### 设计原则（不可违反）

1. **分离关注点**: LODCoordinator 管资源分配。各模块管实现策略。消费者管信息后果
2. **零领域知识**: LODCoordinator 不直接理解 RitualDef/CombatStyle/ProfessionSchedule。重要性信号通过 ai_lod/InteractionIntent/relation_importance 上传
3. **确定性**: 同一输入→同一输出。VRAM/帧预算预测使用上一帧快照
4. **玩家=NPC**: 玩家永远 LOD0，但玩家感知也受 LOD 约束

## CHG-050 新增契约（家具与放置物品系统 v1.0）

> 📘 **权威规格**: [[WoWorld-Design/Change/CHG-050-家具与放置物品系统v1.0创建-20260620|CHG-050]]。家具与放置物品系统是**物品系统的子模块**——所有权归物品系统。

### 核心所有权

| 概念 | Owner | 消费方 | 关键约定 |
|------|-------|--------|---------|
| PlacementStore | **物品系统 (家具与放置物品)** | 世界生成 / NPC 行动 / History | 物品系统内部数据结构——外部模块永不直接碰。通过 EntityIndex trait 查询 |
| PlacedEntry/PlacedCold | **物品系统** | PlacementStore | 热 32B Copy type，冷 40B。热冷分离——与建筑模块 ComponentInstance 一致 |
| ItemPlacementProps | **物品系统** | PlacementStore / 世界生成 Pass B / 制造系统 | 附加在 ItemProperties.placement 上。None=不可摆放 |
| AffordanceSet (u64) | **woworld_core** | 物品系统填充 → NPC GOAP 消费 | 64 标记位。物理供给标记在 AffordanceSet，语义标记(IS_SACRED)在 ItemTags |
| SurfaceId (4变体) | **woworld_core** | 建筑模块 / 物品系统 / 世界生成 | Building/Terrain/Feature/Furniture——4 变体紧凑编码 |
| Surface trait | **woworld_core** — 建筑模块首先实现 | 物品系统 (PlacementStore 放置验证) | orientation/load_bearing_kpa/contains_footprint |
| CultureFurnishProfile | **文化系统** (定义 struct) | 世界生成 Pass B / FurnishEnsemble | 从 CultureCoreParams 显式派生——14 字段，各有确定性公式 |
| WorkstationRegistry (u16) | **启动胶水代码** — 各领域模块注册 | NPC GOAP | 物品系统只存 u16——不知道"这是灶台" |
| Chimney ComponentFamily (0x000A) | **建筑模块** | 世界生成 P6 WFC / Pass A | 烟道组件——提供 PassThrough 连接面 + usage_tag="chimney_opening" |
| ComponentInstance.item_id | **建筑模块** | 物品系统 / History | WFC 生成的内置家具的物品身份——可选字段 |
| 0x44 (FurnitureChest) | **废弃** — 归档标签 | — | 统一为 0x54_02（FurnitureItem 子类别）。旧存档迁移：0x44→0x54_02 |

### 跨模块数据流

| 流 | 提供方 | 消费方 | 说明 |
|-----|--------|--------|------|
| Surface → 放置查询 | 建筑模块 (BuildingQuery) | 物品系统 (validate_furniture_placement) | 家具放置时查询可用表面 |
| CultureCoreParams → 家具风格 | 文化系统 | 物品系统 (CultureFurnishProfile 公式) | P2.5 生成 → P11.5 消费 |
| EntityId → 物理原子 | 物品系统 (PlacementStore via EntityIndex) | NPC 行动 (35 物理原子) | 原子不区分家具/建筑组件——只读 Graspability |
| elemental_affinity → 魔法 | 物品系统 (MaterialDefRegistry) | 魔法系统 | 家具的 MaterialProperties.elemental_affinity——通过 EntityIndex::material_of() |
| 家具→音频表面 | 物品系统 (PlacementStore) | 音频系统 | 地毯覆盖地板 → footstep_material 改变——查询链自顶向下 |
| 家具→天气暴露 | 物品系统 (PlacementStore::weather_exposure) | 天气系统 | 户外家具的 WeatherExposure——风化系统消费 |
| PlacedItem→provenance | 物品系统 (ItemEntId) | 历史系统 | 家具的 provenance 链——History 已有机制，无需新接口 |

### 物料谱系链

```
PlantSpecies → PlantMaterialDef → harvested ItemEntId 
  → crafting (determines_material=true) → FurnitureItem ItemEntId
    → material_instance = 输入材料的 MaterialDefId
      → MaterialProperties { elemental_affinity, density, combustibility, ... }
        → 物理原子 / 魔法 / 风化 消费
```

### 设计原则（不可违反）

1. **不区分组件与家具**: 差异在 Graspability 梯度（Immovable/Attached/Free），不在类型
2. **物品系统不知道 workstation 语义**: 只存 u16 标签，领域模块注册并解释——与 slot_type 同模式
3. **参数化不穷举**: 新文化风格只需调整参数映射表 (TOML)，零 Rust 代码改动
4. **种子确定性**: 世界生成的家具 ID 用 hash(seed, path) 派生——非 AtomicU64 递增
5. **渐进的是渲染/物理 LOD**: 不是家具的存在性——家具全量预生成写入 LMDB

---

## CHG-055 新增契约（存档系统 v1.0）→ CHG-056 修订（v2.0）

> 📘 **权威规格**: [[WoWorld-Design/Change/CHG-055-存档系统v1.0创建-20260621|CHG-055]] · [[WoWorld-Design/Change/CHG-056-存档系统深度审计与修正-20260621|CHG-056 v2.0 修订]]。存档系统是"世界的快照相机"——不参与游戏模拟，只负责状态的持久化与恢复。v2.0 经 10 轮迭代审计修正。

### 核心所有权

| 概念 | Owner | 消费方 | 关键约定 |
|------|-------|--------|---------|
| SaveableModule trait (14 方法) | **存档系统** | 全部 14 持久化模块 | 4 必覆（module_name/current_version/named_dbs） + 10 默认。object-safe——`&mut dyn SaveableModule` |
| named_dbs + key_prefix | **存档系统**（注册时验证）→ 各模块声明 | SaveSystem | `&[(&str, &str)]` — (db_name, key_prefix) 组合全模块唯一 |
| LoadContext | **存档系统** | 各模块 load() | `txn` + `create_txn()` 工厂——模块渐进加载零 SaveSystem 耦合 |
| LMDB 环境 | **存档系统**（SaveSystem 唯一持有） | 无——模块只通过 trait 方法/LoadContext 访问 | 模块不持有 LMDB env 引用。只通过 `snapshot_dirty`/`write_dirty`/`write_initial`/`load`/`migrate` 间接操作 |
| 存档文件格式（.woworld） | **存档系统** | 文件系统 | 单文件 LMDB + 多 named_db。原子写入协议（临时文件 + rename） |
| 崩溃恢复三件套 | **存档系统** | 全部存档操作 | 临时文件+原子重命名 / 覆盖前自动备份(.bak) / session.lock |
| 版本迁移调度 | **存档系统**（编排）→ 各模块 migrate() | 各模块 | from_version 到 current_version 差可 >1——模块内部链式迁移。惰性迁移解决 ConsumableEffect 跨模块协调 |
| 键空间命名约定 | **存档系统**（定义 `module/entity/id` 格式） | 各模块 | 约定——存档系统不验证键内容 |
| 世界发现（目录扫描） | **存档系统** | 主菜单 UI | 文件系统为唯一权威源——无注册表。UUID 去重处理复制目录 |
| 连续存档调度 | **存档系统**（SaveQueue） | 游戏主循环 | Manual > DeathExit > Quick > Auto 优先级 + 补执行。死亡存档不拒绝（等待完成） |

### 跨模块数据流

| 流 | 提供方 | 消费方 | 说明 |
|-----|--------|--------|------|
| WorldState（P13 完成）→ Initial 存档 | 世界生成 | 存档系统（create_initial_save） | P13 输出纯内存 ValidatedWorldState → write_initial 逐模块流式写入。世界生成不再持有 LMDB |
| 模块 ↔ named_db | 各模块（impl SaveableModule） | 存档系统（SaveSystem） | 存档系统只传 bytes——不解析内容。模块自选序列化格式 |
| Schema 版本转换 | Schema 所有者（Audio/Life）→ 存储方（NPC/Items） | load() 流程 | 惰性迁移——load() 中批量转换，加载画面预算内完成 |
| Mod 清单 | Mod 管理器 → HeaderBuilder | SaveHeader.mod_manifest | 存档时记录——加载时比对检测增删 |
| 磁盘空间预警 | SaveSystem（estimate_dirty_bytes 汇总） | UI | mapsize 使用率 ≥ 80% → 弹出警告 |
| Initial 存档直写 | 存档系统 | 各模块 write_initial() | 不经 Phase 1 内存收集——流式写入避免 3GB OOM |

### 设计原则（不可违反）

1. **存档系统不拥有任何模拟数据**: 模块数据归各自模块——存档系统只提供 trait 契约 + IO 流程
2. **存档系统不规定序列化格式**: 模块自选 bincode/rkyv/其他
3. **存档系统不参与数据老化策略**: 模块自治——各模块自己决定记忆压缩、建筑归档等
4. **存档系统不解析键/值内容**: 键和值都是不透明 bytes——存档系统只传递
5. **单一写路径**: 所有 LMDB 写入都经过存档系统——不会出现"P13 写的格式和存档系统写的格式不同"
6. **模块不持有 LMDB 环境**: 模块只通过 trait 方法访问 LMDB——不直接 open/create LMDB
7. **跨模块原子性**: 单文件 + 单写事务保证所有 named_db 同步写入或全部回滚
8. **线程安全由实现者保证**: `snapshot_dirty(&self)` 在 tick 边界调用——`Send + Sync` 约束 + 内部同步

---

## CHG-057 — NPC 认知系统 v1.1 (2026-06-22)

### 新增核心类型

| 概念 | Owner | 消费方 | 关键约定 |
|------|-------|--------|---------|
| PatternExpression | **woworld_core** | NPC认知/概念与语言/存档 | ~120B。PatternStep[] 编码因果步骤。structural_lsh 用于 Creative Leap 结构匹配（Hamming 距离）。domain_signature 用于领域分类/formalize 路由/学科聚类 |
| DomainSignature(u64) | **woworld_core** | NPC认知/概念与语言(修辞) | 替代 MentalModelDomain 枚举。hash(atom_class完整u16, context_hashes)。领域从原子类型涌现 |
| AtomClass(u16) | **woworld_core** | 全领域crate | 统一原子分类=高4位命名空间+低12位索引。IntoAtomClass trait——域crate各自impl |
| EmotionalCharge | **woworld_core** | 感官系统(生产)/NPC认知(消费) | valence/arousal/dominance+primary_emotion+confidence。外部可观察的情绪信号——不包括内感受 |
| GazeEstimate | **woworld_core** | 感官系统(生产)/NPC认知(消费·Theory of Mind) | gaze_direction+is_looking_at_observer+confidence。纯几何——不可靠的 |
| EnrichedPerceptBatch | **woworld_core** | NPC认知/Combat/Economy/Power | PerceptBatch::enrich() 的产出——跨模态绑定+视线估计+情绪读取 |

### 关键架构裁决

| 裁决 | 内容 | 影响的模块 |
|------|------|-----------|
| 回顾性原则 | NPC 心智是回顾性的记忆加工引擎——不模拟假想世界。预测=模式应用，不是仿真 | NPC认知 |
| MentalModelDomain 移除 | 17个硬编码枚举 → DomainSignature 涌现。学科从聚类涌现 | NPC认知/概念与语言/创新管线 |
| 四层记忆压缩 | L0 Hot ≤2000→L1 Cold ≤500→L2 Era ≤20→L3 Life ≤5。~617KB/NPC | NPC/存档 |
| 感官→认知桥梁 | PerceptBatch::enrich()——woworld_core extension methods。零新crate | 感官/NPC/Combat/Economy |
| 深思熟虑连续涌现 | deliberation_depth=trait×state×stakes。不硬编码 reflective>0.4 | NPC/GOAP |
| 记忆源混淆 | source_confidence 衰减→<0.3 概率性 misattribution。虚假记忆涌现 | NPC |
| formalize 注册制 | 领域crate 注册 ATOM_MASK + consumer_fn。未注册→AcademicWork | NPC/全领域crate |
| 修辞化渲染 | Creative Leap（类比）+ TextGenerator（修辞化渲染）= 比喻 | NPC认知/概念与语言 |
| crowd_emotional_field | EnvironmentPerception 新增字段。调制 trust evaluation 和 source_confidence 编码 | 感官/NPC认知 |

### 感官系统新输出

| 输出 | 类型 | 消费方 |
|------|------|--------|
| PerceptEntry.emotional_charge | EmotionalCharge | NPC(情绪传染/思维触发/信念评估) |
| PerceptEntry.gaze_estimate | Option\<GazeEstimate\> | NPC(Theory of Mind) |
| EnvironmentPerception.crowd_emotional_field | f32 | NPC(群体可暗示性/记忆编码) |
| EnvironmentPerception.crowd_dominant_emotion | Option\<BasicEmotion\> | NPC(群体情绪识别) |

### 概念与语言系统新输入

| 输入 | 来源 | 用途 |
|------|------|------|
| UtteranceConcept.domain_sig | NPC认知系统(写入) | 修辞检测——TextGenerator查相邻概念的domain_similarity |

### 技能系统新需求

| 需求 | 内容 |
|------|------|
| Mathematics 子类(0405) | arithmetic/geometry/algebra/statistics/logic —— 5个技能。支撑Mathematician/TaxCollector/Architect职业 |
| 认知原子 | COUNT/MEASURE/CALCULATE/DERIVE —— 4个。走标准三层(物理原子→复合→GOAP) |

---

## CHG-059 — NPC 认知 v1.1 全模块传播审计 (2026-06-22)

> **完整 CHG 文档**: [[WoWorld-Design/Change/CHG-059-NPC认知v1.1传播审计-20260622|CHG-059]]
> **审计文件目录**: [[WoWorld-Design/参考文档/039-NPC认知传播审计-20260622/README|039-传播审计]]

### 概要

CHG-057/058 在 06-认知与智慧系统中引入根本性架构变更后，CHG-059 系统地将 v1.1 认知变更传播到全部 16 个消费模块——审计每个模块的文档过时引用、缺失概念、接口不一致和级联需求，编辑所有受影响文件。

| 维度 | 内容 |
|------|------|
| **影响模块** | 05-感官/08-行动涌现/24-概念与语言/07-生命周期/02-NPC活人感/13-语言表达/06-战斗/07-魔法/08-技能/09-文化/12-历史/14-经济/11-权力/23-建筑/26-存档/00-全局基础设施 |
| **编辑规模** | 16模块·~80文件编辑·14审计文件 |
| **审计协议** | 4维度——过时引用修复·缺失概念补全·接口不一致对齐·新上游需求记录 |

### 关键修复

| 修复项 | 影响范围 | 说明 |
|--------|---------|------|
| AgentSnapshot v1.1 加入出口 | 08-行动涌现 → 全模块 | CognitiveTide 3字段(cognitive_load/rumination_pressure/mind_quietude)正式加入 001-接口出口。所有消费模块 002-接口入口同步 |
| RenderContext 补齐字段 | 05-感官与知觉 | EnrichedPerceptBatch 产出需 RenderContext(含 EmotionalCharge/GazeEstimate/crowd_emotional_field) |
| CognitiveAgingPath 所有者更正 | 07-生命周期 ← 06-认知 | 从认知系统更正为生命周期系统 OWN——三函数联合派生(crystallized_factor/cognitive_engagement_score/health_burden) |
| MemoryStore 四层升级 | 02-NPC活人感/26-存档 | L0 Hot(≤2000)→L1 Cold(≤500)→L2 Era(≤20)→L3 Life(≤5)。旧"2000条上限"全量更新。存档键空间扩展 |
| MentalModelDomain→DomainSignature | 24-概念与语言/06-认知/全领域 | 17个硬编码枚举 → DomainSignature(u64)涌现。学科从聚类涌现 |
| EnrichedPerceptBatch 消费全量声明 | 06-战斗/07-魔法/14-经济/11-权力 | 战斗 combat_intelligence/魔法 spell_selection/经济 EconomicCognition/权力 legitimacy——全部声明消费 EnrichedPerceptBatch |
| deliberation_depth 连续涌现 | 02-NPC活人感 GOAP | 不硬编码阈值。deliberation_depth = trait×state×stakes 连续值 |
| counterfactual_regret 集成 | 02-NPC活人感 情绪引擎 | 决策后反事实思考影响 regret 强度和后续决策权重 |
| source_confidence 源混淆 | 02-NPC活人感 记忆系统 | 记忆源置信度衰减→<0.3 概率性 misattribution。虚假记忆涌现 |
| formalize_innovation() 注册制 | 全领域crate | 领域crate注册 ATOM_MASK+consumer_fn。未注册→AcademicWork |

### 新上游需求 (级联)

| 需求 | 提出模块 | 目标模块 |
|------|---------|---------|
| Mathematics 子类 (0405) | 06-认知 | 08-技能系统 |
| 认知原子 (COUNT/MEASURE/CALCULATE/DERIVE) | 06-认知 | 08-NPC行动涌现 |
| PatternExpression 序列化 | 06-认知 | 26-存档系统 |
| DomainSignature 查询 | 24-概念与语言 | 00-全局基础设施 |
| crowd_emotional_field 生产 | 06-认知 | 05-感官与知觉 |
| EmotionalCharge 生产 | 06-认知 | 05-感官与知觉 |
| GazeEstimate 生产 | 06-认知 | 05-感官与知觉 |

---

## CHG-063 — 玩家系统新建 (2026-06-24)

> **完整 CHG 文档**: [[WoWorld-Design/Change/CHG-063-玩家系统新建-20260624|CHG-063]]

### 概要

新建独立一级模块 `玩家系统/`（`开发阶段/玩家系统/`），6 篇设计规格（~1,448 行）。原分散在生命模块（011/015）和 NPC 生命周期模块（008）的玩家内容整合、深化、统一入口。

### 关键契约

| 契约 | Owner | Consumer | 说明 |
|------|-------|----------|------|
| **Player = SapientMind + ControlMode** | 玩家系统 001 | 全模块 | Player 不是独立实体类型。差异仅在 `SapientMind.control_mode: Option<ControlMode>` |
| ControlMode 三种 | 玩家系统 003 | NPC生命周期 008 | Auto / Manual / DomainDelegated。6 ActionDomain: Movement/Combat/Speech/ItemUse/Interaction/MagicUse |
| 玩家操控 = 人格塑造 | 玩家系统 001/003 | NPC生命周期 008 | GOAP假设动作 vs 实际动作差异→自我叙事倾向**合理化**，非认知张力。CHG-063 修订 008 |
| PlayerGoal → GOAP 长期计划槽 | 玩家系统 004 | NPC活人感 GOAP | PlayerGoal 进入 LongTermGoalSlot，GOAP 自动展开 sub_goals。不同角色不同展开 |
| 双角色系统 | 玩家系统 003 | 生命/015（stub） | 1-2 可控角色、等权选择、无距离上限、随时切换、远处加载时游戏暂停 |
| 两种进入模式 | 玩家系统 001/002 | — | 原住民（接管 NPC）/ 穿越者（7步向导创建） |
| 信息展示边界 | 玩家系统 006 | UI/UX 系统 | 自身数据✅ / 他人内心❌ / 头衔标签❌。认知仅从 NPC 言行涌现 |
| 死亡继承链 | 玩家系统 005 | 生命/004 · NPC生命周期 007 | 血亲→大日志选择→全新角色。技能/关系不可继承 |
| 孩子 = Auto NPC | 玩家系统 005 | 生命繁衍系统 | 不因"父母是玩家的角色"而自动成为可控角色 |
| 引导体系 | 玩家系统 004 | 小精灵系统 | 小精灵不给目标不给任务——仅生存提醒+问答+百科。Minecraft 式引导 |
| MentalAccess = 1.0 | 玩家系统 006 | 技能系统 | 玩家手动操作不受角色智力属性限制。PhysicalAccess 同理 |
| **仅玩家 PhysicsServer3D** | 技术栈方案 v4.0 | 玩家系统 006 | 唯一保留 Godot 物理引擎的实体 |
| 玩家永远 LOD0 | LOD 架构 | 玩家系统 006 | 非操控角色退正常 LOD 层级 |
| 非操控角色远距离退回 L2/L3 | 玩家系统 006 | LODCoordinator | 和其他 NPC 相同 |

### 迁移影响

| 原文件 | 操作 |
|--------|------|
| 生命/011-玩家.md | → stub，指向玩家系统 |
| 生命/015-玩家角色管理与继承.md | → stub，指向玩家系统 |
| NPC生命周期/008-玩家生命周期.md | §一/§四 修订：认知张力→合理化 |

### 跨模块影响

| 受影响模块 | 影响 |
|-----------|------|
| 生命系统 (011/015) | stub |
| NPC 生命周期 (008) | ControlMode 对齐 + 认知张力修订 |
| UI/UX 系统 | 信息展示边界 + 输入动作清单作为设计输入 |
| 技能系统 | MentalAccess/PhysicalAccess 均 1.0 确认 |
| 小精灵系统 | 引导体系主入口——被 004 消费 |
| 模块接头总览 | 新建 28-玩家系统 |

---

## CHG-067 — 物理运动学地基（2026-07-09·仅文档）

在 COM 抛物体（CHG-033）之上补"质量→冲量→单体运动学"涌现地基。不引入通用物理引擎，扩展 Rust 空间查询。详见 `模型动作与物理系统/运动学地基/001`。

### 契约

| 契约 | Owner | Consumer | 说明 |
|------|-------|----------|------|
| **Mass(kg)** | 运动学地基 (woworld_ecs) | 生命/物品/载具(spawn填) · 战斗/魔法(读) | 击退/碰撞分母。生物 size×密度涌现 / 物品 weight_grams |
| **ImpulseQueue** | 运动学地基 | 战斗/魔法(push定向冲量) | 定向一次性冲量, 确定性 drain。**非 SpatialEventBus**——后者只承载 WeaponImpact/Explosion 感知通知(标量力+位置, 无目标无向量) |
| **冲量分流** | 运动学地基 (woworld_core) | 战斗(系数) | momentum → 穿透/击退双系数(TOML)。一次读数喂伤害+击退 |
| **LocomotionMode 三态机** | 运动学地基 | — | Grounded / PhysicsBody / Attached |
| **SurfaceAnchor** trait | woworld_core | 载具(船)/生命(坐骑·巨兽)实现 | 移动锚。AttachedTo/WalkableAnchor 组件。不触"载具≠坐骑"意志分界 |
| **投射物内核** | 运动学地基 | 战斗/魔法(payload) | Projectile SoA + CCD。填投射物/火球AOE/掉落物三空白 |
| **Climbing 能力** | 运动学地基 | 魔法/战斗(临时改写) · 导航/NPC行动(可攀爬边·待立项) | movement_system 泛化到任意法线。max_grip_angle/gravity_frame |
| **着地阈值速度制** | 运动学地基 (Q-A1) | 战斗/009 | 坠落伤害以速度制为单一真相源, 战斗009距离阈值降级为涌现推导 |

### 跨模块影响

| 受影响模块 | 影响 |
|-----------|------|
| 战斗系统 (005/009) | launch_power 重解释为传递系数; 009 坠落阈值降级(Q-A1); 消费 ImpulseQueue/分流 |
| 魔法系统 | 火球→ImpulseQueue AOE击退; 攀爬buff(provisional·未成熟) |
| 载具系统 (001) | 实现 SurfaceAnchor(world_transform/walkable_area 已在) |
| 生命系统 (005) | 实现 SurfaceAnchor(坐骑/巨兽); 填 Mass |
| 物品系统 | 填 Mass(weight_grams); DroppedItem = 会静止的投射物 |
| 导航/NPC行动系统 | 可攀爬边 + A* 握力掩码(待立项·Q-A4) |
