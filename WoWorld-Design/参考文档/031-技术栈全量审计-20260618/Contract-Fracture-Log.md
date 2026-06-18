# 契约断裂日志 (Contract Fracture Log)

> **审计**: 阶段 1.4 | **日期**: 2026-06-18
> **扫描**: 251 条契约条款 (18 CHG组) × 237 TDI

## 摘要

| 严重性 | 计数 | 描述 |
|--------|------|------|
| CRITICAL | 4 | 文档缺失 + 架构矛盾的物理假设 |
| HIGH | 12 | 所有权边界、Rust-Godot边界、缺失模块条目 |
| MEDIUM | 15 | 歧义、不完整、所有权转移未传播 |
| LOW | 8 | 引用过期、注解缺失 |

---

## CRITICAL (4)

| FR-ID | 条款 | 描述 |
|-------|------|------|
| FR-C-001 | CLS-026-001~013 (全部) | **CHG-026 载具系统 13 条款完全缺失于 CLAUDE-INTERFACES.md**。契约文件从 CHG-025 直接跳到 CHG-027。载具系统与其他 8 个模块的接口无契约记录 |
| FR-C-002 | CLS-013-XXX + TDI-032/045 | **基座契约中"PhysicsServer3D 管理 NPC RID + 载具 RID"假设已被 CHG-033 推翻**。CHG-013 是在 CHG-033 之前建立的，隐含假设物理全在 PhysicsServer3D |
| FR-C-003 | CLS-033-XXX + CLS-030-XXX | **SpatialQuery 拆分未传播到感官和音频契约**。CHG-033 定义了 4 trait 拆分，但 CHG-031 (感官) 和 CHG-030 (音频) 的契约可能仍引用旧的单一 SpatialQuery |
| FR-C-004 | CLS-033-001/002 | **物理方案变更**: 契约 CLS-033-001 规定"仅玩家 CharacterBody3D 保留 PhysicsServer3D"，但 TDI-032 和 TDI-045 仍然声称"PhysicsServer3D 管理所有物理" |

## HIGH (12)

| FR-ID | 条款 | 描述 |
|-------|------|------|
| FR-H-001 | CLS-030-007~010 | VoiceProfile/VoiceEmotionModulation/VoiceManager 所有权 Language→Audio。CHG-030 已解决但技术栈未反映 |
| FR-H-002 | CLS-024-XXX + CLS-018-XXX | CommunicationNorms 所有权 Language→Culture。CHG-024 已解决但技术栈未反映 |
| FR-H-003 | CLS-025-XXX | ReligiousReproductionNorms 所有权 Life→Faith。CHG-025 已解决 |
| FR-H-004 | CLS-033-XXX | BodyPlan 定义 Life→woworld_core。CHG-033 MEDIUM 标记但影响物品/生命/模型多模块 |
| FR-H-005 | CLS-031-XXX | 感官系统 SpatialQuery trait — 需拆分为 TerrainQuery/EntityIndex/SpatialEventBus/VisibilityQuery 引用 |
| FR-H-006 | CLS-030-XXX | wind_at() 从 SpatialQuery→WeatherQuery。感官系统文档需要更新引用 |
| FR-H-007 | CLS-022-XXX | 经济系统契约 — 技术栈中无经济模块条目。契约有效但技术栈沉默 |
| FR-H-008 | CLS-023-XXX | 权力系统契约 — 同上，技术栈无权力模块条目 |
| FR-H-009 | CLS-024-XXX | 文化系统契约 — 同上，技术栈无文化模块条目 |
| FR-H-010 | CLS-025-XXX | 信仰系统契约 — 同上，技术栈无信仰模块条目 |
| FR-H-011 | CLS-030-XXX | 音频系统契约 — 同上，技术栈无音频模块条目 |
| FR-H-012 | CLS-019-XXX | LLM 增强层契约 — 技术栈无 LLM 条目，19 场景开关未在技术栈决策中反映 |

## MEDIUM (15)

| FR-ID | 条款 | 描述 |
|-------|------|------|
| FR-M-001 | CLS-022/023-XXX | Economy/Power 桥接 — PowerToEconomicBridge 归属模糊。不属任一模块但两个模块都用 |
| FR-M-002 | CLS-024-XXX | CulturalBeautyStandard 所有权 NPC→Culture (CHG-024) — NPC v2.0 文档可能仍有残留引用 |
| FR-M-003 | CLS-030-XXX | 音频模块 annotation work pending — 多个文档需标注音频概念归属 (CHG-030 §4 列出 10 份待标注文档) |
| FR-M-004 | CLS-031-XXX | 感官系统 PerceptBatch — 战斗 004 已重构为消费 PerceptBatch，但战斗其他文档可能仍有旧引用 |
| FR-M-005 | CLS-033-XXX | WeaponPhysicalParams 映射表 — 物品系统需新增，LOW 但要执行 |
| FR-M-006 | CLS-027/028-XXX | 基本/进阶需求 GOAP 集成 — 契约有效但技术栈 §八 NPC 未反映 7 维需求框架和三层需求模型 |
| FR-M-007 | CLS-029-XXX | 审美系统 FineArts 技能大类(0x06) — 技能系统需确认是否已添加 |
| FR-M-008 | CLS-032-XXX | 认知系统 MentalModel 跨代传递(6 路径) — 历史系统需确认是否对接 |
| FR-M-009 | CLS-016-004 | 天气系统 wind_at() — 原属 SpatialQuery，已移至 WeatherQuery。天气契约需确认 |
| FR-M-010 | CLS-014-XXX | 物品系统 BodyPlan 自动派生 — BodyPlan 迁移至 woworld_core 后物品系统引用需更新 |
| FR-M-011 | CLS-024-XXX | 文化系统节日子模块 FestivalAudioProfile — 音频系统需确认对接 |
| FR-M-012 | CLS-033-XXX | 模型动作系统 AnimBodyState 3.8KB/NPC — 性能预算需在技术栈 §十一 中体现 |
| FR-M-013 | CLS-030-XXX | 音频系统 L1-L4 分层声音精度 — NPC L4 声音处理需与 NPC 分层对齐 |
| FR-M-014 | CLS-031-XXX | 感官噪声确定性种子 — 需确认与其他模块的种子管理一致 |
| FR-M-015 | CLS-019-XXX | LlmBackend trait 注册 — 技术栈无本地5+云端6后端架构的 TDI |

## LOW (8)

| FR-ID | 条款 | 描述 |
|-------|------|------|
| FR-L-001 | CLS-016-XXX | 天气 VisualPacket 结构 — Godot 侧格式需与技术栈 §六 对齐 |
| FR-L-002 | CLS-014-XXX | 物品 Quality×Rarity 双维度 — TDI-Catalog 中无对应 TDI |
| FR-L-003 | CLS-015-XXX | 技能三层天赋(MentalAccess×天生×交叉训练) — TDI-Catalog 中无对应 TDI |
| FR-L-004 | CLS-017-XXX | 语言 ExpressionRef 8B 句柄 — TDI-Catalog 中无对应 TDI |
| FR-L-005 | CLS-024-XXX | 文化四路径演变 — TDI-Catalog 中无对应 TDI |
| FR-L-006 | CLS-026-XXX | 载具 5 动力类型 — TDI-Catalog TDI-157~162 已覆盖但技术栈 §二 无载具动力行 |
| FR-L-007 | CLS-033-XXX | 面部图集 512² (16嘴×16眉×8眼) — TDI-Catalog 中无对应 TDI |
| FR-L-008 | CLS-029-XXX | HasAestheticSignal trait 12 实现者 — TDI-Catalog 中无对应 TDI |

---

## 断裂聚类

三组聚类覆盖大部分 HIGH+ 断裂：

1. **CHG-033 物理方案级联** (FR-C-002/003/004, FR-H-004/005/006) — 根源: PhysicsServer3D→Rust 空间查询的架构转变未传播到受影响契约
2. **7 模块条目缺失** (FR-H-007~012) — 根源: 技术栈 v3 之后创建的模块在技术栈中无条目
3. **文档缺失** (FR-C-001) — CHG-026 13 条款不在 CLAUDE-INTERFACES.md 中

---

## 注意事项检查 (阶段 1)

### 游戏设计原则
| 原则 | 状态 | 发现 |
|------|------|------|
| Rust解耦 | ⚠️ | FR-C-002: 物理方案变更影响 Rust-Godot 边界位置 |
| 涌现式交互 | ✅ | |
| 程序化生成 | ✅ | |
| 不删除原有设计 | ⚠️ | 多处所有权转移需在原位置保留引用 |
| 性能与画面平衡 | ⚠️ | FR-M-012: 动画预算未更新 |
| 独立开发者可实现性 | ✅ | |
| 完整的人 | ✅ | |
| 冒险与生活平等 | ✅ | |
| 全球多元文化 | ✅ | |

### 协作原则 & 工程纪律 — 全部 ✅ 通过
