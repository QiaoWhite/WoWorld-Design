# 038-孤儿接口与预留接口清单

> **创建日期**: 2026-06-20
> **来源**: Phase 2 模块接头总览交叉验证审计
> **审计范围**: 全部 25 个模块接头槽位（00-全局基础设施 ~ 24-概念与语言地基）
> **方法论**: 逐对交叉比对——模块 A 的 `001-接口出口.md` 声明的消费者 vs 模块 B 的 `002-接口入口.md` 实际列出的进口。不一致 = 孤儿或预留。

---

## 一、出口孤儿（有提供者无消费者）

接口在提供者模块的出口文件中声明了消费者，但声称的消费者模块的入口文件并未列出该接口。分三类：

- **文档脱节**：消费者模块实际上需要此接口，但入口文件未记录——纯文档缺口
- **未来预留**：消费者模块尚未成型，或接口对应的功能尚未设计——真正的预留接口
- **待定**：无法判断属于哪一类，需与设计者确认

### 1.1 生命模块 → 世界生成（4项）

| 接口 | Life出口声称的消费者 | WorldGen入口是否列出 | 分类 | 说明 |
|------|---------------------|---------------------|------|------|
| **AgeClock** | "世界生成 P9" | 未列出 | 文档脱节 | WorldGen入口仅列出Vitals/Physiology/DeathCause/RaceTraits等，未列出AgeClock。P9历史模拟引擎依赖advance_age()推动NPC年龄，文档缺口 |
| **InfantDependency** | "世界生成 L3/L4 统计近似" | 未列出 | 文档脱节 | WorldGen入口未列出营养依赖状态机。P8人口生成时需为婴幼儿设定初始依赖状态 |
| **PlantSpeciesRegistry** | "世界生成 P2.25（优势种筛选）" | 未列出 | 文档脱节（CHG-046新） | CHG-046新增的植被接口。WorldGen入口尚未更新以反映P2.25植被覆盖对新trait的依赖 |
| **PlantCommunityTemplate** | "世界生成 P2.25（产出）· 运行时查询" | 未列出 | 文档脱节（CHG-046新） | 同上。群落模板(~100B/Chunk)由Life模块定义，WorldGen P2.25产出后供运行时展开 |

### 1.2 NPC活人感模块 → 魔法模块（4项）

| 接口 | NPC出口声称的消费者 | Magic入口是否列出 | 分类 | 说明 |
|------|---------------------|-------------------|------|------|
| **BigFive** | NPC出口列出消费者：战斗/感官/经济/文化/信仰/语言/模型物理——**未列出魔法** | **已列出**（行26-28） | NPC出口文档脱节 | Magic入口正确列出"BigFive / EmotionState"为进口，但NPC出口在BigFive条目中遗漏了魔法模块。NPC应在BigFive消费者列表中添加"魔法 魔法学习偏好" |
| **EmotionState** | NPC出口列出消费者：战斗/感官/音频/模型物理/语言——**未列出魔法** | **已列出**（行26-28） | NPC出口文档脱节 | Magic入口正确列出EmotionState为进口。NPC出口遗漏魔法——情绪影响施法（愤怒→破坏系加成/平静→疗愈系加成） |
| **CognitiveStyle** | NPC出口列出消费者：战斗/感官/语言——**未列出魔法** | **已列出**（行30-33） | NPC出口文档脱节 | Magic入口正确列出"CognitiveStyle / MentalModel"。NPC出口遗漏魔法——魔法学习方法偏好(直觉vs分析/具象vs抽象) |
| **MentalModel** | NPC出口列出消费者：语言/文化——**未列出魔法** | **已列出**（行30-33） | NPC出口文档脱节 | Magic入口正确列出MentalModel。NPC出口遗漏魔法——MentalModel→魔法世界观(元素理解) |

> **注**：这4项是NPC出口文件的文档缺口（消费者列表不完整），而非真正的"无消费者"。Magic入口正确承认了这些进口。修复方向：NPC出口文件补充魔法模块为消费者。

### 1.3 NPC活人感模块 → 技能模块（1项）

| 接口 | NPC出口声称的消费者 | Skills入口是否列出 | 分类 | 说明 |
|------|---------------------|-------------------|------|------|
| **InnovationPipeline** | "技能 技能发现" | **未列出** | 文档脱节 | NPC出口声称InnovationPipeline消费者包括技能系统（"技能发现"），但Skills入口未列出此进口。技能系统消费InnovationPipeline的阶段输出——新技能/新技术从创新管线固化后进入技能系统 |

### 1.4 NPC活人感模块 → 感官与知觉模块（1项）

| 接口 | NPC出口声称的消费者 | Senses入口是否列出 | 分类 | 说明 |
|------|---------------------|-------------------|------|------|
| **MemoryStore** | "感官 familiarity 判断" | **未列出** | 文档脱节 | NPC出口列出MemoryStore消费者包括感官系统（熟悉度判断），但Senses入口未直接列出MemoryStore。Senses入口列出了NpcData.sensory、AttentionState、PerceptualModifiers、BigFive/CognitiveStyle/CognitiveTide等NPC概念，但MemoryStore不在其中。感官系统通过PerceptualCache做熟悉度判断——可能需要MemoryStore作为参考对比 |

### 1.5 NPC活人感模块 → 战斗模块（2项）

| 接口 | NPC出口声称的消费者 | Combat入口是否列出 | 分类 | 说明 |
|------|---------------------|-------------------|------|------|
| **CognitiveTide** | "战斗 战略层决策" | **未列出** | 文档脱节 | NPC出口列出CognitiveTide(3维认知潮汐)消费者包括战斗系统（战略层决策）。Combat入口列出的NPC进口有CognitiveStyle/MindAttribution，但未列出CognitiveTide。战斗三层模型的战略层(5-30s决策周期)应消费CognitiveTide来调制决策质量 |
| **CognitiveAgingPath** | "战斗 老年战术决策质量" | **未列出** | 文档脱节 | NPC出口列出CognitiveAgingPath(健康老化/病理老化/超级老化三路径)消费者包括战斗系统。Combat入口未列出。老年NPC的战斗决策质量应从CognitiveAgingPath派生 |

### 1.6 感官与知觉模块 → 生命模块（1项）

| 接口 | Senses出口声称的消费者 | Life入口是否列出 | 分类 | 说明 |
|------|----------------------|------------------|------|------|
| **ScentQuery** | "生命 动物觅食" | **未列出** | 文档脱节 | Senses出口列出ScentQuery trait消费者包括生命模块（动物觅食——通过嗅觉懒采样追踪猎物/食物源）。Life入口列出了WeatherQuery/SeasonClock/BiomeParameterField等，但未列出ScentQuery。动物NPC(InstinctiveMind层)的觅食行为依赖ScentQuery |

### 1.7 信仰模块 → 文化/经济模块（3项）

| 接口 | Faith出口声称的消费者 | 消费方入口是否列出 | 分类 | 说明 |
|------|----------------------|-------------------|------|------|
| **FaithTheology** | "文化 005 技术派生" | Culture入口未列出FaithTheology为直接进口 | 文档脱节 | Culture入口通过CulturalShift事件间接消费信仰反馈，但FaithTheology作为直接进口未列出。文化005(TechnologyProfile)的"religiosity × 0.5"假设已被MagicReligionRelation覆盖 |
| **MagicReligionRelation** | "文化 005 技术派生" | Culture入口未列出 | 文档脱节 | 信仰系统定义魔法-宗教四种关系(Gift/Blasphemy/Independent/Unified)，文化005技术派生应消费此项。Culture入口未记录 |
| **信仰塑造社会组织** | 列出经济为消费者 | Economy入口未列出 | 文档脱节 | Faith出口行117-120声称Economy消费"信仰塑造社会组织"，但Economy入口未列出对应的Faith进口 |

### 1.8 文化模块 → 历史/权力模块（2项）

| 接口 | Culture出口声称的消费者 | 消费方入口是否列出 | 分类 | 说明 |
|------|------------------------|-------------------|------|------|
| **honor_weight** | "历史 003 立碑权重" | History入口未列出honor_weight | 文档脱节 | Culture出口列出honor_weight消费者包括历史系统(立碑权重)。History入口列出了DeathCause/Vitals、NPC技能快照、Polity事件等，但未列出Culture的honor_weight。墓碑系统依赖文化荣誉权重决定是否立碑 |
| **GovernmentForm推断** | CultureCoreParams影响GovernmentForm推断 | Power入口未列出CultureCoreParams直接进口（但列出了power_distance/uncertainty_avoidance） | 部分一致 | Power入口确实列出了power_distance和uncertainty_avoidance从Culture进口，这是GovernmentForm推断的关键参数。但Culture出口的GovernmentForm条目与Power入口的列项不完全对齐——Power入口只列了2个参数，而Culture出口声称整体影响 |

### 1.9 权力模块 → 文化/信仰模块（3项）

| 接口 | Power出口声称的消费者 | 消费方入口是否列出 | 分类 | 说明 |
|------|----------------------|-------------------|------|------|
| **UniversalPowerAtom** | "文化 文化规范执行" | Culture入口未列出UniversalPowerAtom | 文档脱节 | Culture入口未列出Power的UniversalPowerAtom为进口。文化规范(NormScope::CulturalNorm)的执行可能需要PowerAtom作为底层机制 |
| **GovernmentForm** | "文化" / "信仰 Theocracy判定" | Culture入口未列出GovernmentForm；Faith入口未列出（但列出了DivineAuthorizationEvent事件） | 文档脱节 | Power出口声称GovernmentForm的消费者包括文化和信仰。Faith入口通过事件订阅间接关联，但Culture入口未直接记录GovernmentForm为进口 |
| **PowerSource** | "信仰 Divine 路径" | Faith入口列出了DivineAuthorizationEvent事件订阅，但未列出PowerSource | 部分一致 | Faith入口通过事件订阅间接接受了PowerSource::Divine的触发，但未将PowerSource本身列为进口 |

### 1.10 语言表达模块 → 文化模块（1项）

| 接口 | Language出口声称的消费者 | Culture入口是否列出 | 分类 | 说明 |
|------|------------------------|-------------------|------|------|
| **NonVerbalSignal** | "文化 GestureCultureMapping ★所有权已迁至文化" | **未列出** | 迁移残留 | Language出口行77-81列出NonVerbalSignal消费者包括文化系统，并注明"★所有权已迁至文化"（CHG-024）。Culture入口未列出NonVerbalSignal为进口——因为所有权已经迁移，Culture现在拥有GestureCultureMapping。Language出口仍将旧NonVerbalSignal列为出口是残留记录 |

### 1.11 载具模块 → 经济/历史模块（1项）

| 接口 | Vehicles出口声称的消费者 | 消费方入口是否列出 | 分类 | 说明 |
|------|------------------------|-------------------|------|------|
| **VehicleQuery** | "经济 · 历史" | Economy入口未列出VehicleQuery；History入口未列出VehicleQuery | 未来预留 | Vehicles出口列出VehicleQuery trait(30+方法)消费者包括经济和历史。Economy入口未列出VehicleQuery——经济系统可能需要查询运输成本/贸易路线载具可用性。History入口未列出——历史系统可能通过VehicleQuery记录航海/运输历史事件。这些消费场景可能尚未在消费方文档中正式设计 |

### 1.12 概念与语言地基模块 → 不存在的模块（2项）

| 接口 | Concept出口声称的消费者 | 消费方是否存在 | 分类 | 说明 |
|------|------------------------|---------------|------|------|
| **PatternSignature(u64)** | "信息传播、NPC" | "信息传播"不是独立模块 | 未来预留 | Concept出口将"信息传播"列为PatternSignature的消费者。当前模块接头总览中没有"信息传播"模块槽位。可能指：语言表达模块的009-信息传播子文档（五传播通道+失真算子），或未来独立的信息传播模块。NPC消费PatternSignature进行概念识别 |
| **CompactConceptEncoding([u8;64])** | "NPC(EventMemory)、历史(PhysicalBook)、信息传播" | "信息传播"不是独立模块 | 部分确认+未来预留 | NPC(EventMemory)和历史(PhysicalBook)作为消费者已确认。但"信息传播"再次出现——需明确是指语言表达009子文档还是预留模块 |

### 1.13 家具模块 —— 全量v0.1待重写

| 状态 | 说明 |
|------|------|
| **所有出口均为占位** | 家具模块全部接口标记为"v0.1 pending rewrite"。FurnitureDef、FurnitureEnt、功能交互、BuildingScene四项出口均为占位性质。实际消费者（世界生成/建筑模块/NPC）的入口文件也标记了"重写后须对齐" |
| **重写后对接点** | 建筑模块Surface trait（CHG-043）为家具放置预留了PlacedItem接口。家具系统重写后须：实现HasAestheticSignal（审美）、注册CraftingRecipe（物品）、查询BuildingQuery::surfaces_in_region()（建筑） |

---

## 二、入口预留（有消费者无提供者）

接口在消费者模块的入口文件中声明为进口，但声称的提供者模块的出口文件未将其列为出口。分两类：

- **真正的预留接口**：为未成型模块预留的接口——需确认模块状态
- **文档脱节**：提供者确实提供此接口，但出口文件遗漏——纯文档缺口

### 2.1 生命模块 → 模型动作与物理（2项）

| 接口 | Life入口声称的来源 | 模型物理出口是否列出 | 分类 | 说明 |
|------|-------------------|---------------------|------|------|
| **BodyPlan / SkeletonDef** | "模型动作与物理" (⚠️ CHG-033: 从生命提升至woworld_core) | **已列出**（Life为消费者） | 一致（但所有权已迁移） | Life入口正确列出BodyPlan/SkeletonDef来自模型物理。模型物理出口也确实列出Life为消费者。但⚠️ CHG-033已将BodyPlan提升至woworld_core——双方文件虽一致，但共同的"提供者"已是woworld_core而非模型物理 |
| **SpatialQuery四trait** (TerrainQuery/EntityIndex/SpatialEventBus/VisibilityQuery) | "模型动作与物理" (⚠️ CHG-033) | **未列出Life为消费者** | 文档脱节 | Life入口明确列出空间查询四trait来自模型物理（用于生物放置地形查询/实体注册/气味源emit/视线检测）。但模型物理出口的SpatialQuery四trait条目中，消费者仅列出"感官·战斗·NPC·载具·音频"——生命模块不在其中。Life确实需要SpatialQuery（生物放置时的height_at/walkable/register/emit_scent），模型物理出口应补充Life为消费者 |

### 2.2 NPC活人感模块 → 模型动作与物理（1项）

| 接口 | NPC入口声称的来源 | 模型物理出口是否列出 | 分类 | 说明 |
|------|------------------|---------------------|------|------|
| **BodyPlan** | "模型动作与物理" (⚠️ CHG-033:从生命提升) | **已列出**（NPC为消费者） | 一致 | NPC入口和模型物理出口均一致——NPC是BodyPlan的消费者（用于动画骨骼适配/装备槽位） |

### 2.3 NPC活人感模块 → 经济模块（2项）

| 接口 | NPC入口声称的来源 | 经济出口是否列出 | 分类 | 说明 |
|------|------------------|-----------------|------|------|
| **EconomicState** | "经济" | NpcEconomicState trait已列出NPC为消费者 | 一致 | NPC入口列出EconomicState从经济模块进口，经济出口的NpcEconomicState trait确实列出NPC为"trait注入目标" |
| **Market 接口** | "经济" | Market已列出NPC为消费者("NPC 交易行为") | 一致 | 双方一致 |

### 2.4 世界生成模块 → 建筑模块（2项）

| 接口 | WorldGen入口声称的来源 | 建筑出口是否列出 | 分类 | 说明 |
|------|----------------------|-----------------|------|------|
| **BuildingSpec** | "建筑模块" | BuildingGenerator trait已列出World Gen P5/P6为消费者 | 一致 | BuildingSpec是WorldGen P5产出→P6传入建筑模块的中间产物。建筑出口的BuildContext和BuildingGenerator条目均覆盖此流程 |
| **BuildingGenerator / generate_buildings()** | "建筑模块" | BuildingGenerator trait已列出World Gen为消费者 | 一致 | 双方一致。WorldGen P6调用generate_buildings()消费返回的BuildingData |

### 2.5 世界生成模块 → 经济模块（2项）

| 接口 | WorldGen入口声称的来源 | 经济出口是否列出 | 分类 | 说明 |
|------|----------------------|-----------------|------|------|
| **ProfessionTag TOML** | "经济" | ProfessionTag在经济学出口中列出，但Owner标注为"世界生成（初始分配）· NPC 身份系统（运行时）" | **所有权混乱** | WorldGen入口说ProfessionTag来自经济模块，但经济出口却说ProfessionTag的Owner是世界生成和NPC——经济系统只是"消费收入来源类型"。这构成了反向依赖：WorldGen声称消费经济定义的ProfessionTag，但经济声称ProfessionTag由WorldGen和NPC拥有。需明确ProfessionTag TOML规范文件的权威Owner |
| **Market / bootstrap_economy()** | "经济" | Market已列出世界生成为消费者；bootstrap_economy()未作为独立出口列出 | 文档脱节 | Market条目一致。但bootstrap_economy()函数——WorldGen P11调用此函数进行经济Bootstrap——在经济出口中未作为独立条目列出。经济出口的订单簿/市场机制条目描述了机制，但未显式导出bootstrap_economy()函数签名 |

### 2.6 建筑模块 → 物品模块（1项）

| 接口 | Building入口声称的来源 | Items出口是否列出 | 分类 | 说明 |
|------|----------------------|-------------------|------|------|
| **Blueprint TOML** | 建筑出口将Blueprint TOML格式列为其出口（消费者="玩家, Items Blueprint item 0x52"） | Items入口未列出Blueprint TOML从建筑模块进口 | 进口预留 | 建筑出口行5将Blueprint TOML格式规范列为出口，消费者包括物品系统（Blueprint作为物品0x52存在背包中）。但Items入口未列出此进口。物品系统需要知道Blueprint TOML的schema以正确存储/验证Blueprint物品类型。这是Items入口的文档缺口——物品系统确实需要消费建筑模块定义的Blueprint格式 |

---

## 三、所有权冲突

同一概念/接口被多个模块的出口文件声明为Owner，或同一模块的出口和入口对所有权做出矛盾声明。

### 3.1 DeathCause —— 生命模块自相矛盾

| 冲突方 | 声明 | 文件位置 |
|--------|------|---------|
| **Life出口** | DeathCause Owner = "生命 §九"。消费者 = "历史 死亡事件记录 · 权力 继位触发" | `01-生命/001-接口出口.md` 行27-30 |
| **Life入口** | DeathCause 来源模块 = "历史"。用途 = "死亡事件→历史记录。Life定义25种死亡原因→历史消费" | `01-生命/002-接口入口.md` 行80-83 |

**分析**：Life出口明确声称DeathCause的所有权（"Owner: 生命 §九"），但Life入口却说DeathCause来自历史模块。这构成了同一模块内的自相矛盾。实际设计意图（根据CHG-013基座契约）：DeathCause由Life定义（30种6大类），History消费。Life入口中"来源模块: 历史"是错误的——应为"来源模块: 生命（本模块定义）"。Life入口还在行80描述"Life定义25种死亡原因→历史消费"——这与自身出口的30种6类也不一致（CHG-041已扩展至30种）。

**修复方向**：Life入口的DeathCause条目应修正为：来源模块=生命（本模块定义），而非历史。历史模块是消费者而非提供者。

### 3.2 VoiceProfile / VoiceEmotionModulation —— 语言表达 vs 音频（CHG-030迁移未完成）

| 冲突方 | VoiceProfile声明 | VoiceEmotionModulation声明 |
|--------|-----------------|--------------------------|
| **Language出口** | Owner = "语言表达 012"。消费者 = "音频 TTS 消费 · NPC 语音身份" | Owner = "语言表达 012"。消费者 = "音频 TtsEngine · NPC 情绪→语音" |
| **Audio出口** | Owner = "音频 005"。标注"⚠️ CHG-030: 所有权从语言表达012迁入音频模块"。消费者 = "NPC (存储于 SensoryState) · 语言表达 (TTS消费)" | Owner = "音频 005"。标注"⚠️ CHG-030: 从语言表达012迁入"。消费者 = "语言表达 (TTS渲染)" |

**分析**：CHG-030明确将VoiceProfile和VoiceEmotionModulation的所有权从语言表达模块迁移至音频模块。Audio出口已更新以反映新所有权，但Language出口**未更新**——仍将这两个概念列在出口文件中，声称自己是Owner。这是典型的迁移残留：迁移执行方（Audio）已完成文件更新，但被迁移方（Language）的文件未清理。

**修复方向**：Language出口的VoiceProfile和VoiceEmotionModulation条目应从"出口"改为"已迁出"或删除，保留交叉引用指向Audio出口。

### 3.3 WeaponPhysicalParams —— 物品入口 vs 物品出口

| 冲突方 | 声明 | 文件位置 |
|--------|------|---------|
| **Items入口** | WeaponPhysicalParams来源模块 = "模型动作与物理"。用途 = "武器物理参数 → 动画轨迹适配" | `18-物品/002-接口入口.md` 行43 |
| **Items出口** | WeaponPhysicalParams Owner = "物品 001"。消费者 = "模型物理 (战斗轨迹适配器)"。标注"⚠️ CHG-033 新增" | `18-物品/001-接口出口.md` 行46-49 |

**分析**：Items入口说WeaponPhysicalParams是从模型物理模块进口的，但Items出口却声称自己是WeaponPhysicalParams的Owner。一个物品模块不可能同时进口和出口同一个概念。实际设计意图（根据CHG-033）：WeaponPhysicalParams映射表（长度/重量/重心/握持位置→reach_scale/speed_scale/body_commitment）由物品系统定义并存储（因为物品知道武器的物理属性），模型物理系统消费来做战斗轨迹适配。Items出口的说法是正确的——Items是Owner。Items入口错误地将此列为了进口——应修正为"本模块定义并导出至模型物理"。

**修复方向**：Items入口的WeaponPhysicalParams条目应移除（它不是进口，而是本模块的出口）。或改为注释说明"以下接口由本模块定义，供模型物理消费"。

### 3.4 InfantDependency —— 生命模块 + NPC模块双出口

| 冲突方 | 声明 | 文件位置 |
|--------|------|---------|
| **Life出口** | Owner = "生命 004"。消费者 = "NPC L1 母亲 GOAP · 世界生成 L3/L4 统计近似" | `01-生命/001-接口出口.md` 行58-62 |
| **NPC出口** | Owner = "生命 004（生物学事实）· NPC活人感模块/07-生命周期系统/004-婴幼儿与养育（行为编排）"。消费者 = "生命 L1 母亲 GOAP 喂奶 · 世界生成 L3/L4 统计近似" | `02-NPC活人感/001-接口出口.md` 行143-148 |

**分析**：两个模块都将InfantDependency列在自己的出口文件中。NPC出口承认Life是生物学事实的Owner（"存储在 LifeEntity 上，由生命模块拥有"），但NPC仍将InfantDependency作为自己的出口——从行为编排的角度。这构成了"双层导出"：Life导出生物学状态机(Nursing→Weaning→Weaned)，NPC导出行为编排逻辑。

这也是消费者列表交叉的体现：Life出口说消费者是"NPC L1 母亲 GOAP · 世界生成"，NPC出口说消费者是"生命 L1 母亲 GOAP 喂奶 · 世界生成"——两个模块都声称对方消费自己的InfantDependency。

**实际设计意图**（根据CHG-041）：InfantDependency是一个生物学事实，存储在LifeEntity上，由Life模块拥有。NPC模块读取InfantDependency状态来编排母亲和婴儿的行为（喂奶/断奶）。NPC不应将InfantDependency列为自己的出口——它是Life出口的消费者（通过行为编排消费生物学事实）。

**修复方向**：NPC出口的InfantDependency条目应从出口移至入口（作为从Life进口的概念），或改为仅描述行为编排层如何消费Life的InfantDependency状态。

---

## 四、汇总统计

### 4.1 按分类统计

| 分类 | 数量 | 说明 |
|------|------|------|
| **出口孤儿——文档脱节** | 20项 | 提供者出口文件列出消费者，但消费者入口文件未列出。消费者确实需要此接口，纯文档缺口 |
| **出口孤儿——未来预留** | 4项 | VehicleQuery→经济/历史(2)、PatternSignature/CompactConceptEncoding→信息传播(2) |
| **出口孤儿——迁移残留** | 1项 | NonVerbalSignal→文化（所有权已迁至文化，Language出口未清理） |
| **出口孤儿——全量待重写** | 4项 | 家具模块全部v0.1占位出口 |
| **入口预留——文档脱节** | 2项 | SpatialQuery四trait→Life(模型物理出口遗漏)、bootstrap_economy()→WorldGen(经济出口遗漏) |
| **入口预留——所有权混乱** | 1项 | ProfessionTag TOML（WorldGen入口说来自经济，经济出口说Owner是WorldGen） |
| **入口预留——已一致** | 6项 | BodyPlan(双向确认)、EconomicState/Market(双向确认)、BuildingSpec/BuildingGenerator(双向确认) |
| **所有权冲突** | 5项 | DeathCause(Life自相矛盾)、VoiceProfile/VoiceEmotionModulation(迁移未完成)、WeaponPhysicalParams(入口vs出口矛盾)、InfantDependency(双模块出口) |
| **总计不重复项** | ~37项 | — |

### 4.2 按涉及模块统计

| 模块 | 出口孤儿数 | 入口预留数 | 所有权冲突数 |
|------|-----------|-----------|-------------|
| 01-生命 | 4 (→WorldGen) | 1 (←模型物理SpatialQuery) | 2 (DeathCause自矛盾、InfantDependency双出口) |
| 02-NPC活人感 | 8 (→魔法4+技能1+感官1+战斗2) | 0 | 1 (InfantDependency双出口) |
| 03-世界生成 | 0 | 0 | 0 |
| 05-感官与知觉 | 1 (→生命) | 0 | 0 |
| 09-文化 | 2 (→历史+权力) | 0 | 0 |
| 10-信仰 | 3 (→文化+经济) | 0 | 0 |
| 11-权力 | 3 (→文化+信仰) | 0 | 0 |
| 13-语言表达 | 1 (→文化NonVerbalSignal) | 0 | 2 (VoiceProfile/VoiceEmotionModulation) |
| 15-载具 | 1 (→经济+历史VehicleQuery) | 0 | 0 |
| 16-音频 | 0 | 0 | 2 (VoiceProfile/VoiceEmotionModulation) |
| 17-模型动作与物理 | 0 | 1 (←Life SpatialQuery遗漏) | 0 |
| 18-物品 | 0 | 0 | 1 (WeaponPhysicalParams入口vs出口矛盾) |
| 21-家具 | 4 (全量v0.1) | 0 | 0 |
| 24-概念与语言地基 | 2 (→信息传播) | 0 | 0 |

### 4.3 按紧急程度

| 紧急程度 | 项目 | 说明 |
|---------|------|------|
| **HIGH** | DeathCause 自相矛盾 | Life模块出口和入口给出相反的Owner声明。影响死亡系统的权威来源 |
| **HIGH** | VoiceProfile/VoiceEmotionModulation 迁移残留 | CHG-030迁移已完成但Language出口未清理。两套文件冲突可能导致实现时选错权威 |
| **HIGH** | WeaponPhysicalParams 入口vs出口矛盾 | Items模块同时声称进口和出口同一概念。CHG-033新增，源文件自相矛盾 |
| **HIGH** | InfantDependency 双出口 | Life和NPC同时导出同一概念。消费者列表交叉循环 |
| **MEDIUM** | NPC出口遗漏魔法模块为消费者（4项） | NPC出口文件消费者列表不完整。修复简单——补充4行 |
| **MEDIUM** | Life→WorldGen 4项文档脱节 | WorldGen入口需补充CHG-041/CHG-046新增接口的进口记录 |
| **MEDIUM** | ProfessionTag 所有权混乱 | 三方（WorldGen/NPC/经济）对同一概念的Owner声明不一致 |
| **LOW** | VehicleQuery→经济/历史 | 未来预留接口。当前无紧急影响，但需跟踪载具模块扩展时更新 |
| **LOW** | 概念与语言地基→信息传播 | "信息传播"不是独立模块。需明确究竟是指语言表达009子文档还是预留模块 |
| **LOW** | 家具全量v0.1 | 已知待重写状态。重写时须一次性对齐所有接口 |

---

## 五、建议修复优先级

### 第一批（本周——致命冲突）
1. 修复 Life入口 DeathCause 的来源声明（改为"本模块定义"）
2. 清理 Language出口的 VoiceProfile/VoiceEmotionModulation（标注已迁出或删除）
3. 修复 Items入口的 WeaponPhysicalParams（移除进口条目，改为注明本模块定义导出）
4. 将 NPC出口的 InfantDependency 移至入口（仅作消费方）

### 第二批（下次保鲜——文档脱节）
5. WorldGen入口补充：AgeClock、InfantDependency、PlantSpeciesRegistry、PlantCommunityTemplate
6. NPC出口补充：BigFive/EmotionState/CognitiveStyle/MentalModel的魔法消费者
7. 模型物理出口补充：SpatialQuery四trait的Life消费者
8. Skills入口补充：InnovationPipeline进口
9. Senses入口补充：MemoryStore进口
10. Combat入口补充：CognitiveTide、CognitiveAgingPath进口
11. Life入口补充：ScentQuery进口
12. 经济出口补充：bootstrap_economy()函数签名

### 第三批（可延后——未来预留跟踪）
13. 明确"信息传播"模块状态——是语言表达009子文档还是预留独立模块
14. VehicleQuery→经济/历史的消费方入口更新（载具模块扩展时）
15. ProfessionTag TOML的权威Owner确认（WorldGen or Economy？）
16. 家具系统重写时一次性对齐所有接口

---

> **审计方法**: 逐对交叉比对25个模块槽位的001-接口出口.md和002-接口入口.md文件
> **审计日期**: 2026-06-20
> **下一步**: CHG-047 审计报告 + 第一批修复执行
