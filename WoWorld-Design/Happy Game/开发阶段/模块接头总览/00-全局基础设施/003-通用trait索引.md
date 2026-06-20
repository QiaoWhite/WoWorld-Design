# 通用 trait 索引 — 全模块

> **最后验证**: 2026-06-18 | **源文档版本**: CHG-013~033 全量审计 | **定位**: 跨模块 trait 目录
>
> 本文档列举 WoWorld 中**所有** trait, 按定义 crate 分组, 标注实现方和所有消费方。修改任何 trait 签名时须检查本表中所有消费者。

---

## 一、woworld_core — 核心基础设施 trait

| Trait | 方法数 | 定义方 | 实现方 | 消费方 | 关联CHG |
|-------|--------|--------|--------|--------|---------|
| **TerrainQuery** | 9 | woworld_core | world_gen | 感官(05) · NPC(02) · 战斗(06) · 动画(17) · 载具(15) · 音频(16) | CHG-033 |
| **EntityIndex** | 6 | woworld_core | woworld_spatial | 全部模块 (通过 SpatialQuery) | CHG-033 |
| **SpatialEventBus** | 3 | woworld_core | woworld_spatial | 全部模块 (写入+查询事件) | CHG-033 |
| **VisibilityQuery** | 2 | woworld_core | woworld_spatial | 感官(05) · 战斗(06) · 大日志 · 音频(16) | CHG-033 |
| **SpatialQuery** | (聚合) | woworld_core | world_gen + woworld_spatial | 全部模块 — 统一入口, 聚合以上四 trait | CHG-033 |
| **VisionQuery** | 4 | woworld_core | 感官 crate | NPC crate (感知决策) | CHG-031 |
| **ScentQuery** | 4 | woworld_core | 感官 crate | NPC crate (嗅觉感知) | CHG-031 |
| **CulturalKnowledgeBase** | — | woworld_core | TOML 数据驱动 | NPC prepare_facts() | CHG-031 |

---

## 二、weather — 天气与季节 trait

| Trait | 方法数 | 定义方 | 实现方 | 消费方 | 关联CHG |
|-------|--------|--------|--------|--------|---------|
| **WeatherQuery** | sample()等 | 天气 crate | 天气 crate | NPC · 战斗 · 载具 · 音频 · 海洋 · 生命 · 全部 | CHG-016 |
| **BiomeMicroclimateQuery** | 3 | 天气 crate | 世界生成(实现) | 天气系统 (冠层/雪面/沙地修正) | CHG-016 |
| **ElevationQuery** | 1 | woworld_core | 世界生成(实现) | 天气·感官·载具 (地形高度查询) | CHG-016 |
| **ClimateParamsQuery** | 1 | woworld_core | 世界生成(实现) | 天气·生命 (气候参数查询) | CHG-016 |
| **OceanCurrentQuery** | 1 | woworld_core | 世界生成(实现) | 天气·载具·世界生成 (洋流查询) | CHG-016 |
| **WorldBoundaryQuery** | 2 | woworld_core | 世界生成(实现) | 天气·载具 (边界距离/内部判定) | CHG-016 |

---

## 三、senses — 感官与知觉 trait

| Trait | 方法数 | 定义方 | 实现方 | 消费方 | 关联CHG |
|-------|--------|--------|--------|--------|---------|
| **VisionQuery** | 4 (visible_entities/line_of_sight/visual_signature/occlusion_query) | woworld_core | 感官 crate | NPC crate (感知决策) | CHG-031 |
| **ScentQuery** | 4 (scent_sources_at/scent_intensity/scent_trail/scent_identity) | woworld_core | 感官 crate | NPC crate (嗅觉感知) | CHG-031 |

---

## 四、npc — NPC 活人感 trait

| Trait | 方法数 | 定义方 | 实现方 | 消费方 | 关联CHG |
|-------|--------|--------|--------|--------|---------|
| **MentalAccess** | — | 技能系统 001 | (NPC各实体实现) | 技能系统 (天赋三层模型) | CHG-015 |
| **TeachingRisk** | — | 技能系统 003 | (NPC empty default) | 技能系统 (教学风险) | CHG-015 |
| **FaithAgent** | — | 信仰系统 004 | NPC/Player | 信仰系统 (实践查询) | CHG-025 |
| **HasAestheticSignal** | — | 审美系统 05 | 12 实现者: ItemEntId/BuildingId/CreatureId/NpcId/VehicleId/ScenePosition/PerformanceRef/SkillActionRef/CombatExchangeRef/SpellCastRef/MagicConstructRef/RitualRef | 审美 judge() | CHG-029 |

---

## 五、combat — 战斗 trait

| Trait | 方法数 | 定义方 | 实现方 | 消费方 | 关联CHG |
|-------|--------|--------|--------|--------|---------|
| **LootGenerator** | generate_loot() | 战斗 crate | 战斗 crate | 物品系统 (战后战利品) | CHG-014 |

---

## 六、magic — 魔法 trait

| Trait | 方法数 | 定义方 | 实现方 | 消费方 | 关联CHG |
|-------|--------|--------|--------|--------|---------|
| **ManaConsumer** | channel_mana_to_engine() | 魔法 crate | 载具系统 (MagicEngine) · 法器 | 魔法系统 | CHG-026 |

---

## 七、skills — 技能 trait

| Trait | 方法数 | 定义方 | 实现方 | 消费方 | 关联CHG |
|-------|--------|--------|--------|--------|---------|
| **SettlementTechQuery** | — | 技能系统 001 (定义) | 文化系统 005 (实现 — TechnologyProfile) | 技能系统 003 (技能天花板) | CHG-015, CHG-024 |
| **CrossTraining** | (逻辑非trait) | 技能系统 002 | — | 全部模块 — 非递归, 天花板 min(40, 0.5L) | CHG-015 |

---

## 八、culture — 文化 trait

| Trait | 方法数 | 定义方 | 实现方 | 消费方 | 关联CHG |
|-------|--------|--------|--------|--------|---------|
| **CultureQuery** | — | 文化系统 006 | 文化 crate | 全部模块 — 高频 O(1)缓存: culture_at / core_params / communication_norms | CHG-024 |
| **CultureMut** | — | 文化系统 006 (pub(crate)) | 文化 crate | 世界生成管线 · 历史模拟引擎 — 消费模块不可调用 | CHG-024 |
| **RitualQuery** | last_ritual_at()等 | 文化系统 008 (定义) | 权力系统 004 (实现) | 权力系统 (合法性 ritual 因子) — 对标 MentalAccess 模式 | CHG-024 |
| **FestivalQuery** | 4 高频方法 | 文化系统 008 | 文化 crate | 全部模块 — 节日基础查询 | CHG-024 |
| **FestivalQueryExt** | 6 低频方法 | 文化系统 008 | 文化 crate | NPC (npc_attending等) | CHG-024 |
| **FaithCalendarQuery** | holy_days()/primary_deities()/fasting_rules() | 文化系统 008 (定义) | 信仰系统 006 (实现) | 节日系统 — 对标 MentalAccess 模式 | CHG-025 |

---

## 九、faith — 信仰 trait

| Trait | 方法数 | 定义方 | 实现方 | 消费方 | 关联CHG |
|-------|--------|--------|--------|--------|---------|
| **FaithQuery** | 30 | 信仰系统 010 | 信仰 crate | 全部模块 — 零分配, 高频 O(1)缓存 | CHG-025 |
| **FaithMut** | — | 信仰 system 010 (pub(crate)) | 信仰 crate | 世界生成管线 · 历史模拟引擎 — 消费模块不可调用 | CHG-025 |
| **FaithAgent** | — | 信仰系统 004 (定义) | NPC/Player (实现) | 信仰系统 — 信仰不区分 NPC/Player | CHG-025 |

---

## 十、power — 权力 trait

| Trait | 方法数 | 定义方 | 实现方 | 消费方 | 关联CHG |
|-------|--------|--------|--------|--------|---------|
| **PowerTopologyQuery** | 14 | 权力系统 008 | 权力 crate | 全部模块 — 只读方法覆盖所有查询模式 | CHG-023 |

---

## 十一、history — 历史 trait

| Trait | 方法数 | 定义方 | 实现方 | 消费方 | 关联CHG |
|-------|--------|--------|--------|--------|---------|
| **AetherQuery** | — | 历史系统 006 | 历史 crate | 生命 (CachedImprintView) | CHG-013 |

---

## 十二、language — 语言表达 trait

| Trait | 方法数 | 定义方 | 实现方 | 消费方 | 关联CHG |
|-------|--------|--------|--------|--------|---------|
| **ContentResolver** | — | 语言表达 004 | 历史 · 魔法 · 物品 (各模块注册) | 语言表达 ExpressionRegistry | CHG-017 |
| **LlmBackend** | — | 语言表达 011 | 本地5种 + 云端6种 + Mock | 语言表达 (LLM装饰器) — 玩家配置 | CHG-019 |
| **TtsEngine** | — | 语言表达 012 | System/LocalAI/CloudAPI/PreRecorded/None | Godot (TTS渲染) | CHG-019, CHG-030 |

---

## 十三、economy — 经济 trait

| Trait | 方法数 | 定义方 | 实现方 | 消费方 | 关联CHG |
|-------|--------|--------|--------|--------|---------|
| **NpcEconomicState** | — | 经济系统 001 | NPC (trait注入 NpcData) | NPC决策器 · 经济系统 — 不修改 NpcData 本体 | CHG-022 |
| **EconomyQuery** | 8 | woworld_core | 经济 crate | 概念与语言地基(模式签名)·权力(合法性推导)·NPC(消费决策)·世界生成(Bootstrap) | CHG-048 |

---

## 十四、vehicles — 载具 trait

| Trait | 方法数 | 定义方 | 实现方 | 消费方 | 关联CHG |
|-------|--------|--------|--------|--------|---------|
| **VehicleQuery** | 30+ | 载具系统 010 | 载具 crate | NPC · 战斗 · 经济 · 历史 · 天气 · Godot | CHG-026 |
| **VehicleMut** | — | 载具系统 010 (pub(crate)) | 载具 crate | 世界生成 · 天气 · 战斗 · 权力 — 消费模块不可直接调用 | CHG-026 |
| **VehicleController** | — | 载具系统 003 | NPC + Player | 载具系统 — L1-L3 半自动控制 | CHG-026 |

---

## 十五、audio — 音频 trait

| Trait | 方法数 | 定义方 | 实现方 | 消费方 | 关联CHG |
|-------|--------|--------|--------|--------|---------|
| **AudioQuery** | 30 | 音频系统 008 | 音频 crate | 全部模块 — 零分配, 对标 WeatherQuery | CHG-030 |
| **HasSoundEmitter** | active_sounds() | 音频系统 002 (定义) | 各模块 (impl) | 音频模块 — 对标 HasAestheticSignal | CHG-030 |
| **HasSoundFootprint** | — | 音频系统 002 (定义) | 各模块 (impl) | 音频模块 — 暴露 SoundFootprint + ActionRingBuffer | CHG-030 |

---

## 十六、ocean — 海洋 trait

| Trait | 方法数 | 定义方 | 实现方 | 消费方 | 关联CHG |
|-------|--------|--------|--------|--------|---------|
| **OceanProvider** | 6 (wave_height/wave_direction/current_velocity/depth_query/sea_state/is_navigable) | 技术栈方案 (woworld_core定义) | 海洋 crate | Godot 渲染 · 载具 (导航) · 世界生成 (P1海洋系统) — 预留 FFT 升级 | 技术栈 v4.0 |

---

## 十七、trait 定义模式速查

| 模式 | 说明 | 例子 |
|------|------|------|
| **标准 Query** | Owner 定义并实现, 所有模块消费 | WeatherQuery / CultureQuery / FaithQuery / AudioQuery / VehicleQuery / PowerTopologyQuery |
| **消费方定义** | 消费方模块定义 trait, Owner 实现 | MentalAccess(技能定义→NPC实现) / SettlementTechQuery(技能定义→文化实现) / RitualQuery(文化定义→权力实现) / FaithCalendarQuery(文化定义→信仰实现) / FaithAgent(信仰定义→NPC实现) |
| **Schema 实现** | Owner 定义 trait, 各模块在自己的实体上 impl | HasAestheticSignal / HasSoundEmitter / HasSoundFootprint |
| **注册模式** | Owner 定义 trait, 各模块注册自己的实现 | ContentResolver |
| **pub(crate) Mut** | Owner 实现但仅授权内部修改 | CultureMut / FaithMut / VehicleMut |
| **聚合 trait** | 组合多个子 trait 提供统一入口 | SpatialQuery = TerrainQuery + EntityIndex + SpatialEventBus + VisibilityQuery |
| **注入 trait** | 扩展已有 struct 的行为, 不修改本体 | NpcEconomicState (trait 注入 NpcData) |

---

## 十八、按消费方反查 (高频消费)

| 消费方模块 | 消费的 trait |
|-----------|-------------|
| **NPC (02)** | WeatherQuery · TerrainQuery · EntityIndex · SpatialEventBus · VisibilityQuery · VisionQuery · ScentQuery · CultureQuery · FaithQuery · AudioQuery · VehicleQuery · PowerTopologyQuery · NpcEconomicState · CulturalKnowledgeBase |
| **战斗 (06)** | WeatherQuery · TerrainQuery · EntityIndex · SpatialEventBus · VisibilityQuery · VehicleQuery · AudioQuery · LootGenerator |
| **世界生成 (03)** | CultureMut · FaithMut · VehicleMut · BiomeMicroclimateQuery (实现) · OceanProvider |
| **经济 (14)** | CultureQuery · VehicleQuery · NpcEconomicState · PowerTopologyQuery |
| **感官 (05)** | TerrainQuery · EntityIndex · SpatialEventBus · VisibilityQuery · WeatherQuery · AudioQuery |
| **载具 (15)** | TerrainQuery · EntityIndex · WeatherQuery · ManaConsumer · VehicleController |
| **音频 (16)** | WeatherQuery · EntityIndex (acoustic_tag_at) · VisibilityQuery · HasSoundEmitter · HasSoundFootprint |
| **权力 (11)** | PowerTopologyQuery (实现) · CultureQuery · RitualQuery |
| **语言表达 (13)** | ContentResolver · LlmBackend · TtsEngine · CultureQuery (CommunicationNorms) · AudioQuery (VoiceProfile) |
| **历史 (12)** | AetherQuery (实现) · VehicleQuery · FaithQuery · CultureQuery |
| **技能 (08)** | SettlementTechQuery (定义) · MentalAccess (定义) · TeachingRisk (定义) · CrossTraining |
| **物品 (18)** | LootGenerator · HasAestheticSignal (实现) · HasSoundEmitter (实现) |
| **模型/动画 (17)** | TerrainQuery · EntityIndex · SpatialEventBus · WeatherQuery |
| **Godot 渲染** | OceanProvider · AudioQuery → AudioRenderPacket · WeatherQuery → WeatherVisualPacket · VehicleQuery → 共享内存直读 |

---

> **关联文档**: [[001-核心类型注册表|核心类型注册表]] · [[002-空间查询四trait|空间查询四trait 详细签名]] · [[../../../../CLAUDE-INTERFACES.md|CLAUDE-INTERFACES 全部CHG]]
