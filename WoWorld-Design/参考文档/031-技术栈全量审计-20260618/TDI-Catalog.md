# 技术决策项目录 (TDI Catalog)

> **审计**: WoWorld 技术栈全量审计 | **阶段**: 0.1
> **日期**: 2026-06-18 | **源文档**: 技术栈方案/001-WoWorld正式技术栈方案v3.md

## 概述

- **TDI 总数**: 237
- **ACTIVE**: 234
- **SUPERSEDED (by CHG-033)**: 3 (TDI-032, TDI-045, TDI-202)
- **领域分布**: ENGINE, LANGUAGE, PHYSICS, ANIMATION, RENDERING, STORAGE, CONCURRENCY, ARCHITECTURE, VERSION, INTEGRATION, MOD, UI, ASSET_PIPELINE, PERF_BUDGET
- **范围分布**: GLOBAL ~165, LOCAL ~56, CROSS_CUT ~16

## CHG-033 已知取代

| TDI | 原陈述 | 取代为 |
|-----|--------|--------|
| TDI-032 | 物理 = Godot PhysicsServer3D 含载具移动参考系 | 仅玩家 CharacterBody3D 保留 PhysicsServer3D; 其余 Rust 侧空间查询 (TerrainQuery/EntityIndex/SpatialEventBus/VisibilityQuery) |
| TDI-045 | PhysicsServer3D 管理 NPC RID + 载具 RID + 海洋浮力采样 | Rust 侧空间查询架构取代 |
| TDI-202 | Godot 物理预算 ≤1.7ms | 大批物理从 Godot 移至 Rust，预算需重算 |

## 完整 TDI 清单

### 元/过程决策 (TDI-001 ~ TDI-003)

| TDI-ID | 陈述 | 领域 | 范围 | 状态 |
|--------|------|------|------|------|
| TDI-001 | v3.0 为 WoWorld 权威技术蓝图，所有后续技术决策以此为基准 | [ARCHITECTURE] | [GLOBAL] | ACTIVE |
| TDI-002 | 取代 v2.0 (016，已归档) | [VERSION] | [GLOBAL] | ACTIVE |
| TDI-003 | 载具系统详细设计委托给独立参考文档(2026-06-15) | [ARCHITECTURE] | [LOCAL] | ACTIVE |

### v2.0→v3.0 变更摘要 (TDI-004 ~ TDI-012)

| TDI-ID | 陈述 | 领域 | 范围 | 状态 |
|--------|------|------|------|------|
| TDI-004 | 世界大小: 25万 km² + 自然海洋边界 | [ARCHITECTURE] | [GLOBAL] | ACTIVE |
| TDI-005 | 地形高程: 1500m 总垂直范围，700m+ 山脉 | [ENGINE] | [GLOBAL] | ACTIVE |
| TDI-006 | 垂直稀疏 Chunk: 仅地表存储体素 | [ENGINE] | [GLOBAL] | ACTIVE |
| TDI-007 | 海:陆 = 7:3，Gerstner 波 + 大陆/岛屿架构 | [ENGINE] | [GLOBAL] | ACTIVE |
| TDI-008 | 混合体积云: 可穿越 3D 云 + NPC 天空感知 | [RENDERING] | [GLOBAL] | ACTIVE |
| TDI-009 | 半自动战斗: 玩家 AI = NPC AI = 同一套 Rust 代码 | [ARCHITECTURE] | [GLOBAL] | ACTIVE |
| TDI-010 | 城市规划 + WFC 区域填充 + 蓝图三路径建造 | [ARCHITECTURE] | [GLOBAL] | ACTIVE |
| TDI-011 | 五种载具动力类型 + 移动参考系 + NPC 微型社会 | [ARCHITECTURE] | [GLOBAL] | ACTIVE |
| TDI-012 | 目标硬件: GTX 1660 SUPER 6GB | [PERF_BUDGET] | [GLOBAL] | ACTIVE |

### 设计哲学技术映射 (TDI-013 ~ TDI-019)

| TDI-ID | 陈述 | 领域 | 范围 |
|--------|------|------|------|
| TDI-013 | 参数驱动涌现引擎: 一切由规则生成 | [ARCHITECTURE] | [GLOBAL] |
| TDI-014 | NPC 密度: 100K+ 跨 25万 km² ≈ 0.4 人/km² | [PERF_BUDGET] | [GLOBAL] |
| TDI-015 | 确定性生成链: 全局种子 → 大陆形状 → 海陆 → 地形 → 文化种子 → 完全涌现 | [ARCHITECTURE] | [GLOBAL] |
| TDI-016 | 大陆/岛屿隔离 → 文化分化; 贸易路线连接 → 文化融合 | [ARCHITECTURE] | [GLOBAL] |
| TDI-017 | 统一底层系统: 铁匠和冒险者共享同一战斗AI/经济/关系网络 | [ARCHITECTURE] | [GLOBAL] |
| TDI-018 | 种子到Chunk无限生成 + 70%海洋 → 航海探索即发现新大陆 | [ENGINE] | [GLOBAL] |
| TDI-019 | 载具为移动微聚落: NPC在火车/船上继续日常生活 | [ARCHITECTURE] | [LOCAL] |

### 总技术栈表 (TDI-020 ~ TDI-037)

| TDI-ID | 陈述 | 领域 | 范围 | 状态 |
|--------|------|------|------|------|
| TDI-020 | 模拟语言: Rust stable 1.80+ | [LANGUAGE] | [GLOBAL] | ACTIVE |
| TDI-021 | 游戏引擎: Godot 4.6 LTS | [ENGINE] | [GLOBAL] | ACTIVE |
| TDI-022 | 集成: GDExtension (godot-rust) 0.2+ | [INTEGRATION] | [GLOBAL] | ACTIVE |
| TDI-023 | 实体管理: Rust SoA 布局 | [ARCHITECTURE] | [GLOBAL] | ACTIVE |
| TDI-024 | 数据库: LMDB 0.8+ 流式 Chunk 存储 | [STORAGE] | [GLOBAL] | ACTIVE |
| TDI-025 | 并发: rayon 1.x | [CONCURRENCY] | [GLOBAL] | ACTIVE |
| TDI-026 | 体素引擎: Transvoxel (Rust 自建) 垂直稀疏Chunk + Clipmap LOD | [ENGINE] | [GLOBAL] | ACTIVE |
| TDI-027 | 噪声/数学: noise 0.9+ 和 glam 0.28+ | [LANGUAGE] | [GLOBAL] | ACTIVE |
| TDI-028 | 海洋: Gerstner 波 (Godot shader) + OceanProvider trait | [RENDERING] | [GLOBAL] | ACTIVE |
| TDI-029 | 天空: 混合体积云 (Rust compute + Godot) | [RENDERING] | [GLOBAL] | ACTIVE |
| TDI-030 | 动画: Rust CPU 骨骼矩阵 → Godot GPU skinning | [ANIMATION] | [GLOBAL] | ACTIVE |
| TDI-031 | NPC 渲染: Godot MultiMeshInstance3D | [RENDERING] | [GLOBAL] | ACTIVE |
| TDI-032 | ~~物理: Godot PhysicsServer3D~~ | [PHYSICS] | [GLOBAL] | **SUPERSEDED by CHG-033** |
| TDI-033 | 载具: Rust 载具逻辑 + Godot 物理 | [ARCHITECTURE] | [LOCAL] | ACTIVE |
| TDI-034 | 建造: Rust 蓝图解析 + NPC 施工调度 | [ARCHITECTURE] | [LOCAL] | ACTIVE |
| TDI-035 | Mod: TOML 格式涌现乘数调优 | [MOD] | [GLOBAL] | ACTIVE |
| TDI-036 | UI: Godot Control 节点 | [UI] | [GLOBAL] | ACTIVE |
| TDI-037 | GLTF 管线: gltf crate + Godot 导入 | [ASSET_PIPELINE] | [GLOBAL] | ACTIVE |

### 更多 TDI 组（详见完整提取）

完整 237 条 TDI 涵盖: 架构总图 (TDI-038~048) · 世界生成全局噪声 (TDI-049~054) · 分层密度场 (TDI-055~059) · 噪声基准参数 (TDI-060~066) · 垂直稀疏Chunk (TDI-067~072) · LOD系统 (TDI-073~077) · 水文 (TDI-078~080) · 海洋/Gerstner波 (TDI-081~086) · 空间语义标签 (TDI-087) · 世界边界 (TDI-088~090) · 天空三架构 (TDI-091~099) · NPC天空感知 (TDI-100~108) · 时间/季节/天气 (TDI-109~113) · 战斗半自动 (TDI-114~136) · NPC分层模拟 (TDI-137~140) · 载具NPC行为 (TDI-141~143) · 长途旅行NPC (TDI-144~145) · 城市规划 (TDI-146~152) · 蓝图建造 (TDI-153~156) · 载具类型 (TDI-157~164) · 移动参考系 (TDI-165~168) · 载具NPC行为 (TDI-169~170) · 旅行节奏 (TDI-171~174) · 性能预算 (TDI-175~220) · 开发路线 (TDI-221~227) · 风险矩阵 (TDI-228~236) · 溯源 (TDI-237)

> **完整 TDI 提取见文件**: agent extraction (237 items, 51KB). 此摘要包含前 37 条 + 分类索引。阶段 3 逐条审视时参考完整提取。

---

## TDI 缺口分析

**当前覆盖**: 237 条 TDI 全部提取自 v3 文档（2026-06-10）。**v3 之后 CHG-022~033 创建了 8 个新模块，其技术决策完全未收录。**

### 预估缺失 TDI（~93 条）

| 缺失域 | 来源模块 | 预估 TDI |
|--------|---------|----------|
| 模型/动画/物理细化 | CHG-033 | ~20 (38姿态/15轨迹/9层栈/面部图集/空间查询4trait/骨架33+35骨…) |
| 音频架构 | CHG-030 | ~15 (双层模型/5分类/传播引擎/掩蔽/语音管道/音乐/提示音…) |
| 感官架构 | CHG-031 | ~12 (VisionQuery/ScentQuery/PerceptBatch/显著性/噪声/注意切换…) |
| 经济架构 | CHG-022 | ~12 (订单簿/价格涌现/两阶段提交/稳定器/中间商涌现…) |
| 文化架构 | CHG-024 | ~10 (CultureCoreParams/障碍Voronoi/四路径演化/CommunicationNorms…) |
| 信仰架构 | CHG-025 | ~8 (FaithTheology/实践优先/接触传染/虔诚度…) |
| 权力架构 | CHG-023 | ~8 (17 PowerAtom/PowerTopology/Legitimacy/Polity涌现…) |
| 认知架构 | CHG-032 | ~8 (CognitiveStyle/MentalModel/InnovationPipeline…) |
| **合计** | | **~93** |

**实际 TDI 总量预估**: 237 (当前) + 93 (缺失) ≈ **330 条**

### 状态标签补充

除 ACTIVE / SUPERSEDED，新增:
- **EXPANDED** — 原始 TDI 正确但已被后续 CHG 大规模扩展（如 TDI-030 "动画" → 完整模型动作物理系统），阶段 3 拆分为多条
- **TO_EXTRACT** — TDI 存在于新模块文档但尚未提取到目录（阶段 2 处理）

### 领域标签细化

阶段 3 前将粗标签拆分：RENDERING→RENDER_TERRAIN/RENDER_NPC/RENDER_OCEAN/RENDER_SKY/RENDER_SHADOW/RENDER_POST; ANIMATION→ANIM_SKELETON/ANIM_GAIT/ANIM_FACIAL/ANIM_STACK; ARCHITECTURE→ARCH_CRATE/ARCH_EVENT/ARCH_SIM/ARCH_DATAFLOW; 新增 LOD/SPATIAL/SIMULATION/SAVE/PROCEDURAL/INPUT/CULTURE/ECONOMY/POWER/FAITH
