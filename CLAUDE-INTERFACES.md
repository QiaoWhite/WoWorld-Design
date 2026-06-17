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
| 死亡原因 | **Life** `004 §九` | History（墓碑文本映射） | 25种，五大类：物理伤害7/环境6/生物5/魔法4/时间与特殊3 |

## CHG-014 新增契约（物品系统 v1.0）

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| 物品标识 (ItemDefId/ItemEntId) | **物品系统** `001`/`002` | 全部模块 | ItemDefId=u64全局恒定(8+56bit), ItemEntId=u64存档内唯一——旧MaterialId/MagicItemId/resource_type通过映射表桥接 |
| 物品属性 (ItemProperties) | **物品系统** `003` | 全部模块 | 核心属性+Quality(4档)×Rarity(5档)+AestheticProps——各模块在此之上叠加 |
| 装备槽位 (EquipmentSlots) | **物品系统** `004` | NPC/Life | BodyPlan自动派生——不预定义物种类型。双套Outfit切换由NPC自主决定 |
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
| 职业标签 (ProfessionTag) | **世界生成**(初始分配)/**NPC身份系统**(运行时) | 经济系统(消费收入来源类型) | ~80-100个原子标签——TOML数据驱动+预留新增接口。proficiency从技能系统派生。任意2-4个排列组合→职业涌现。incongruity标记不寻常组合(不阻止) |
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

## CHG-027 新增契约（基本需求系统 v1.0）

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

## CHG-028 新增契约（进阶需求系统 v1.0）

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

## CHG-029 新增契约（审美与艺术系统 v1.0）

| 概念 | 权威 Owner | 消费方（引用权威） | 关键约定 |
|------|-----------|-------------------|---------|
| `AestheticSignal` (6维) | **审美系统** `05` | 全部模块（通过 HasAestheticSignal trait） | 审美模块只定义 struct——各实体模块在 trait impl 中计算。6维: fluency/novelty/complexity/harmony/expressiveness/virtuosity |
| `AestheticJudgment` (4维) | **审美系统** `05` | NPC/经济/战斗/情绪 | judge() 纯函数输出——零副作用。4维: valence/arousal/interest/respect |
| `AestheticTaste` | **审美系统** `05`（定义+派生公式）→ **NPC 模块**（存储于 NpcData） | NPC 决策器/情绪引擎 | 对标 ReligiousPracticeProfile 模式。32B Copy 类型。青春期 derive_taste()，年更新 mature_taste()，Adopt 事件传播 |
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
| `CognitiveStyle` | **认知系统**（定义 struct + 派生逻辑）→ **NPC NpcData**（存储） | 决策引擎/MentalModel消化/ThoughtTrigger | 4维（直觉-分析/冲动-反思/具象-抽象/顽固-灵活），从BigFive+wisdom+经历派生，含阻尼年度更新。对标AestheticTaste模式 |
| `CognitiveTide` | **认知系统**（定义 struct + 每决策周期更新）→ **NPC NpcData**（存储） | 决策引擎/ThinkingCheck | 3维（负载/反刍压力/安静度），对标Physiology从Vitals派生模式。漫游驱动派生不存储 |
| `CognitiveBiases` | **认知系统**（定义纯函数·惰性计算）→ **MentalModel消化**（消费） | 微消化/宏消化/assess_and_integrate | 7种偏误完全从CognitiveStyle+Emotion+Tide派生。零存储——对标Physiology::from_vitals() |
| `MentalModel` | **认知系统**（定义 struct·≤20条上限）→ **NPC NpcData**（存储） | 决策引擎(权重调制)/语言表达(WisdomSharing)/历史系统(MentalModelRecord)/技能系统(DeepMentorship) | ≤20条(~1.6KB)。跨代传递6路径全部通过已有通道。自有归纳/消化/融合/放弃管线。置信度贝叶斯更新 |
| `ThoughtFragment` | **认知系统**（惰性生成·仅玩家15m内）→ **Godot渲染**（已有通道） | 非语言表达/字幕/物品创建/对话 | 3级清晰度。片段组合模型(~200-300模板)。CognitiveStyle选择表达模式。产生SurfacingType→已有API调用 |
| `MindAttribution` | **认知系统**（定义 struct·≤16条上限）→ **NPC NpcData**（存储） | 决策引擎/对话系统/欺骗检测 | ≤16条(~960B)。Theory of Mind归因。4种来源(TargetStated/Observed/ToldByThird/Inferred) |
| `SleepCognitiveProcessing` | **认知系统**（定义纯函数·睡眠结束时调用）→ **NPC**（内存/情绪/模型修改） | 记忆/情绪/mental_models | 对标SelfNarrative::reflect()。过拟合大脑假说框架。经验窄度→自适应梦境荒诞度。睡眠质量6因子派生——零新trait |
| `InnovationPipeline` | **认知系统**（定义6阶段纯函数管线）→ **各领域系统**（消费构造参数） | 战斗/魔法/艺术/建筑/工艺/社会 | formalize_innovation() 返回数据参数——不返回领域crate类型。领域crate已有一切构造API |
| `cognitive_distress` | **认知系统**（定义派生公式·每游戏日更新）→ **NPC NpcData**（存储1f32） | CognitiveBreak检测/决策随机性 | 完全派生(矛盾+反刍+停滞+神经质)。对标SelfNarrative.stagnation_sense |

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

### 空间查询契约

| 概念 | Owner | 消费方 | 关键约定 |
|------|-------|--------|---------|
| TerrainQuery | **world_gen** (密度场) | 感官/导航/动画/战斗 | height_at/normal_at/terrain_raycast/density_at/is_walkable/surface_material_at/medium_at/light_level_at/sample_horizon。纯函数。DDA 射线在密度场上步进，~10µs/射线 |
| EntityIndex | **woworld_spatial** | 所有模块 | register/unregister/update_transform/entities_in_aabb/entity_aabb/acoustic_tag_at。稀疏哈希网格 O(1)。layer_mask 过滤 |
| SpatialEventBus | **woworld_spatial** | 所有模块 | recent_events_in/push_event/scent_sources_in。Chunk ring buffer(64 entry,LRU)。事件自动过期 max(intensity×10s, 5s) |
| VisibilityQuery | **woworld_spatial** (Arc<TerrainQuery> + &EntityIndex) | 感官/战斗/大日志 | line_of_sight/line_of_sight_hit。DDA 同时检查密度场+实体AABB。命中返回 TerrainHit/EntityHit/WaterSurface |

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
| CRITICAL | 技术栈方案 v3.0 | 物理方案改为"仅玩家PhysicsServer3D"。性能预算更新 |
| HIGH | NPC ver2.0 §4 | "物理表达"替换为本模块引用 |
| MEDIUM | 生命/001 | BodyPlan定义→woworld_core |
| MEDIUM | 物品系统/001 | +WeaponPhysicalParams映射表 |
