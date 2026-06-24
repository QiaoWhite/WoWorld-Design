# 代码 vs 设计文档 全量偏离/遗漏审计

> **日期**: 2026-06-25
> **范围**: `woworld/` Rust workspace vs `WoWorld-Design/Happy Game/开发阶段/`
> **方法**: 逐模块对照设计规格，标记 ✅已实现 / ⚠️偏离 / ❌缺失

---

## 总体结论

**27 个设计模块中，4 个有部分代码，23 个为零实现。** 代码处于地形可视化原型阶段，仅覆盖世界生成 P0+P2（15 阶段管线中的 2 个）。

---

## 一、已实现模块（4 个，全部为 PARTIAL）

### 1. 世界生成（P0+P2 only）

参考: `开发阶段/世界生成/001`, `007`

| ✅ 已实现 | ⚠️ 偏离 | ❌ 缺失 |
|----------|---------|--------|
| TerrainQuery trait 全部 9 方法 | Chunk 128m（规格 32m）| P1 海洋系统 |
| SurfaceMaterial 21 变体 | MC 替代 Transvoxel | P2.25 植被 |
| 5 群系硬盒分类 | Clipmap 4 级（规格 8 级，最远 10km）| P2.5 文化/信仰种子 |
| DensityField trait | 密度场 1 层（规格 11 层 L0-L10）| P3-P13 全部 |
| 高度场 + MC + Clipmap LOD | u32 seed（规格 u64）| OceanProvider trait |
| EntityIndex/SpatialEventBus/VisibilityQuery trait | 高程 ~630m（规格 ~1500m）| SDF 编辑、采矿系统 |
| ChunkManager + 对象池 + 速率限制 | 海深 -400m（规格 -4000m+）| VoxelData 4-byte 格式 |

### 2. 大气与氛围

参考: `开发阶段/大气与氛围系统/`

| ✅ | ⚠️ | ❌ |
|----|-----|-----|
| 时间曲线合成 (AtmosCurve, 太阳高度角驱动) | 4 层调制中 3 层为 identity stub | 群系/天气/季节 TOML 数据文件 |
| ResolvedAtmosphere 输出 (35 floats) | | 水下/室内深度分层 |
| | | 天空事件 (aurora, eclipse 等) |

### 3. 时间/昼夜

参考: `开发阶段/天气与季节系统/003`

| ✅ | ⚠️ | ❌ |
|----|-----|-----|
| WorldTime + WorldClock + TimeOfDay | 简化为 sin-wave 太阳（无纬度参数）| Season enum, SeasonState struct |
| 120-day year, 4 季, 季节进度 | | 纬度感知太阳模型 (declination + hour-angle) |

### 4. 空间索引 (woworld_spatial)

| ✅ | ❌ |
|----|-----|
| GridEntityIndex, DdaVisibility, RingEventBus | 四叉树空间索引（设计替代方案） |

---

## 二、零实现模块（23 个）

| 模块 | 等级 | 设计行数 | 关键概念 |
|------|------|---------|---------|
| 天气与季节系统 | 🟡 就绪 | ~1,500 | WeatherSample, Markov 6-state |
| 生命 | 🟡 就绪 | ~2,000 | Vitals, Mana, DeathCause(30种), 种群模板 |
| 战斗 | 🟡 就绪 | ~3,000 | 三层模型(本能→节奏→战略), 半自动 |
| 魔法 | 🔴 冻结 | ~2,500 | 零性能预算——性能预算建立前不可编码 |
| 技能系统 | 🟡 就绪 | ~2,000 | SkillId(5分类), XP公式, 天赋三层 |
| 物品系统 | 🟡 就绪 | ~5,000 | ItemDefId, Assembly, Enchantment, CraftingRecipe |
| 经济系统 | 🟡 就绪 | ~2,500 | OrderBook 匹配, Market, NpcEconomicState |
| 权力系统 | 🟡 就绪 | ~2,000 | 17 PowerAtoms, PowerTopology, Legitimacy |
| 文化系统 | 🟡 就绪 | ~1,500 | CultureCoreParams(10), Voronoi 屏障 |
| 信仰系统 | 🟡 就绪 | ~2,000 | FaithTheology(10 params), 实践先于教义 |
| 历史 | 🟡 就绪 | ~1,500 | AetherImprint, KnowledgeSeed |
| 语言表达 | 🟡 就绪 | ~1,500 | ExpressionRef, Conversation, 信息传播 |
| 载具系统 | 🟡 就绪 | ~1,500 | 5 动力类型, L1-L3 半自动控制 |
| 音频系统 | 🟡 就绪 | ~1,500 | SoundFootprint, AudioQuery(30 methods) |
| NPC活人感模块 | 🟡 就绪 | ~8,000+ | 7 子模块: 需求/审美/认知/生命周期/行动涌现 |
| 小精灵系统 | 🟡 就绪 | ~1,000 | 精灵行为/职业/进化 |
| 建筑模块 | 🟡 就绪 | ~2,000 | ComponentFamily(9 core+Mod), WFC 2.5D |
| 存档系统 | 🟡 就绪 | ~3,000 | LMDB, SaveableModule, 全量快照+脏增量 |
| UI与UX系统 | 🟡 就绪 | ~800 | HUD, 对话面板, 信息架构 |
| 概念与语言地基 | 🟡 就绪 | ~1,500 | 3 层模型(PatternSignature→文化概念→语言词汇) |
| 感官与知觉系统 | 🟡 就绪 | ~2,000 | PerceptBatch, 4 查询 trait, PerceptualCache |
| 玩家系统 | 🟡 就绪 | ~1,500 | 玩家=NPC+I/O适配层, 双角色托管, 死亡继承 |
| 模型动作与物理系统 | 🟡 就绪 | ~3,000 | 9 层动画栈, 4 trait, 5 子模块 |

---

## 三、偏离严重度分级

### 🔴 架构级（影响后续模块接入）

| 偏离 | 规格 | 当前 | 后果 |
|------|------|------|------|
| Chunk 尺寸 | 32m×32m 水平, 垂直稀疏 | 128m, y=0 恒定 | 32m 是 LMDB 存储 + Clipmap tile 基础单元 |
| DensityField trait | `material_at(pos) -> u8` + `priority() -> u8` | 只有 `sample() -> f32` | 多层密度叠加需要 priority 排序和 material 查询 |
| Seed 类型 | u64 → `hash(seed, stage, chunk)` | u32 + wrapping_add | 未来确定性生成需要 u64 + 阶段/Chunk hash 派生 |
| Transvoxel | Transvoxel + transition cells | 标准 Marching Cubes | LOD 过渡接缝需要 Transvoxel transition cell 机制 |
| DensityProvider trait | 11 层可插拔密度场 | 1 个 HeightfieldDensity | 洞穴/矿脉/建筑地基/NPC修改/玩家SDF 全依赖此架构 |

### 🟡 参数级（可调，不阻塞架构）

- 高程 ~630m vs 规格 ~1500m
- Clipmap 4 级 vs 8 级，最远 1km vs 10km
- 太阳 sin-wave vs 纬度感知模型
- 群系硬盒 vs 连续参数场
- 海深 -400m vs 规格 -4000m+

### 🟢 合规

- TerrainQuery 9 方法全部实现
- SurfaceMaterial 21 变体全部定义
- SpatialEntity/SpatialEvent/ScentSource/AcousticTag 类型完整
- GDScript 铁律 (宪法 §14.1) 合规
- 74 tests, clippy 零警告

---

## 四、建议优先级

基于宪法 §2 决策矩阵（依赖链 ×3 + 风险验证 ×3 + 长期架构 ×2）：

| 优先级 | 任务 | 理由 |
|--------|------|------|
| **P0** | DensityField trait 补全 (`material_at` + `priority`) | 阻塞所有后续地形特性 |
| **P0** | Seed u64 + stage/chunk hash 派生 | 阻塞确定性生成 |
| **P1** | 植被渲染 (P2.25) | 视觉回报最大，MultiMeshInstance3D |
| **P1** | 海洋系统 (P1) | 70% 地表是水，当前只有着色器平面 |
| **P2** | 天气与季节 | 大气模块骨架已有，自然扩展 |
| **P2** | 存档系统 (LMDB) | 设计完整，可开始搭架子 |
| **P3** | 生命/动物 | 需要先有植被输出初级生产力 |
| **P3+** | NPC/经济/权力/文化/信仰/历史... | 依赖世界生成 P3-P13 全量输出 |

---

> **关联文件**: `CONSTITUTION.md` · `DEVELOPMENT_STATUS.md` · `DEPENDENCY_GRAPH.md`
> **审计员**: Claude Code (Explore subagent + 手动综合)
