# CHG-041: NPC 生命周期系统 v1.0 创建

> **日期**: 2026-06-18
> **状态**: 完成
> **关联**: [[../Happy Game/开发阶段/NPC活人感模块/07-生命周期系统/001-生命周期系统总纲|生命周期系统总纲]] · [[../Happy Game/开发阶段/NPC活人感模块/07-生命周期系统/跨模块审计/001-跨模块依赖与接口全面清单|跨模块审计]] · [[../参考文档/033-NPC生命周期模块大纲-20260618/001-NPC生命周期模块大纲|模块大纲]] · [[CHG-013-跨模块一致性冲突修正-20260612|CHG-013]] · [[CHG-022-经济系统v1.0创建-20260613|CHG-022]] · [[CHG-024-文化系统v1.0创建-20260614|CHG-024]] · [[CHG-025-信仰系统v1.0创建-20260615|CHG-025]] · [[CHG-033-模型动作物理系统v1.0创建-20260617|CHG-033]]

---

## 变更概述

创建 NPC 生命周期系统（NPC 活人感模块第 7 个子模块），覆盖 NPC 从受孕到死亡——以及死后痕迹留存的完整生命历程。

---

## 一、新增文件

### 参考文档
- `参考文档/033-NPC生命周期模块大纲-20260618/001-NPC生命周期模块大纲.md` — 模块大纲（快览用）

### 开发阶段 — NPC 活人感模块/07-生命周期系统/
| # | 文件 | 内容 |
|---|------|------|
| 001 | 001-生命周期系统总纲.md | 模块定位、6 核心原则、全生命周期概览、关键数据结构、事件流架构 |
| 002 | 002-年龄时钟与阶段转换.md | AgeClock 纯函数、阶段边界预计算、LifeStageTransition 事件、物种阶段分布 |
| 003 | 003-受孕、孕期与出生.md | 受孕概率（纯生物学）、GestationState、FetusBlueprint、产前影响（可开关）、流产/死胎/多胞胎 |
| 004 | 004-婴幼儿与养育.md | InfantDependency 三状态机、L1 母亲 GOAP 驱动喂奶、L3/L4 统计近似、孤儿涌现、收养、NutritionMethod trait |
| 005 | 005-童年与教育.md | 教育三层涌现（家庭→社区→专职教师）、4 教学路径兼容、学习率连续曲线、游戏=低风险学习、天才无标签 |
| 006 | 006-青壮年与老年.md | 峰值参数区间、认知老化三路径（Healthy/Pathological/SuperAging）、社会角色过渡、fertility vs libido 分离、更年期连续曲线 |
| 007 | 007-死亡与死后.md | 30 死因 6 类、DeathCauseRegistry trait、Gompertz 闭式积分、DeathEvent 中性事实、三阶段死亡压缩、6 种死后痕迹归属 |
| 008 | 008-玩家生命周期.md | ControlMode 三层、双角色调度、时间跳过、死亡继承（血亲→大日志→结算） |
| 009 | 009-跨模块接口与事件协议.md | 完整事件目录、10 模块订阅响应表、trait 接口清单、EventBus 设计、数据合同 |
| 010 | 010-性能架构与LOD分层.md | 统一时间流速（不可加速）、单次调用成本分析、L1/L3/L4 负载、阶段边界预计算、衰老查表、批处理接口、内存预算 |
| 011 | 011-边界情况与Mod扩展.md | 10 种边界情况、Mod trait 扩展接口（5 个可扩展 + 4 个不可扩展领域） |

### 跨模块审计
- `07-生命周期系统/跨模块审计/001-跨模块依赖与接口全面清单.md` — 全代码库审计结果、10 条冲突修正清单

---

## 二、核心设计决策

1. **涌现优先——零年龄门控**：没有任何 `if age < X { return CantDoThis }`。能力差异从参数自然派生
2. **零系统开关**：GOAP/审美/语言/认知从出生起持续运行
3. **连续模型**：所有能力是连续渐变，无阈值开关
4. **中性事件**：LifeEvent 不携带情感建议/行为期望/人际通知
5. **统一时间流速**：所有 LOD 层共用相同时间流速（不可 L3/L4 加速老化）
6. **平等性**：生育欲望和吃饭欲望走同一条通用认知管线
7. **归属分离**：生命模块拥有生物学事实；NPC 生命周期拥有行为协调；其余模块独立订阅事件
8. **玩家 = NPC + 控制层**：ControlMode 覆盖 GOAP 输出，不是独立实体类型
9. **母乳为默认**：InfantDependency::Nursing。NutritionMethod trait 留接口

---

## 三、修改的已有文件

### 模块接头总览
| 文件 | 修改内容 |
|------|---------|
| `模块接头总览/01-生命/001-接口出口.md` | DeathCause 25→30 种 6 类。新增 AgeClock/LifeEvent/GestationState/FertilityPotential/InfantDependency/DeathSummary/DeathCauseRegistry 条目 |
| `模块接头总览/02-NPC活人感/001-接口出口.md` | 新增 InfantDependency/ControlMode/CognitiveAgingPath/PrenatalAccumulator/Widowhood 条目 |
| `模块接头总览/02-NPC活人感/002-接口入口.md` | 新增 LifeStageTransition/BirthEvent/DeathEvent/AgeClock/FertilityPotential 条目 |

---

## 四、需要联动修改的其他模块（本次未执行，留待后续 CHG）

| 模块 | 需修改内容 | 优先级 |
|------|-----------|:----:|
| 生命 004 | DeathCause 25→30 种 6 类；新增 DeathCauseRegistry trait | 🔴 |
| 生命 012 | FertilityDrivers 退役 → FertilityPotential 连续曲线 | 🔴 |
| 生命模块 | 新增 013-生命周期时钟与事件.md | 🔴 |
| 世界生成 P8 | NPC 初始化增加年龄分布（人口金字塔）；教育设施生成；教师职业 | 🔴 |
| 世界生成 P9 | 历史模拟与个体生命周期衔接 | 🔴 |
| CLAUDE-INTERFACES.md | CHG-013 DeathCause 更新；新增 CHG-041 契约表 | 🟡 |
| 文化模块 | 新增 CulturalElderCareNorms | 🟡 |
| 信仰模块 | FuneralCustoms 增加 LifeStage 维度 | 🟡 |
| NPC 关系 | 新增 Widowhood、RelationshipKind 扩展（StepParent/Adoptive 等） | 🟡 |
| NPC 认知 | 新增 CognitiveAgingPath | 🟡 |
| NPC 04-进阶需求 | competence_frustration=0 修正为从认知参数自然派生 | 🟡 |
| 权力 | 确认 DeathEvent/BirthEvent 订阅接口 | 🟢 |
| 经济 | 确认遗产分配接口 | 🟢 |
| 技能 | 确认教学路径与儿童学习率兼容 | 🟢 |

---

## 五、冲突修正

| # | 冲突 | 修正 |
|---|------|------|
| 1 | DeathCause 数量：生命 004 25 种 → 本模块 30 种 | 扩展至 30 种 6 类 |
| 2 | FertilityDrivers 集中式 struct vs 涌现原则 | 退役。FertilityPotential 连续曲线替代。生育欲望走通用认知管线 |
| 3 | 无 AgeClock | 生命模块新增 013 |
| 4 | 儿童 competence_frustration=0 硬编码 | 改为从认知参数自然派生 |
| 5 | 亲子关系无数据模型 | NPC 关系模块新增 RelationshipKind 变体 |
| 6 | Nursing 标签无机制 | InfantDependency 状态机 + 母亲 GOAP |
| 7 | 丧偶无追踪 | NPC 关系模块新增 Widowhood（中性事实） |
| 8 | 孤儿无定义 | InfantDependency + 社区接管涌现 |
| 9 | 玩家死亡继承悬置（生命 011 推迟） | 本模块 008 定义继承方案 |
| 10 | 衰老公式模糊 | Gompertz 闭式积分 |

---

## 六、接头总览更新

✅ 已完成：
- 02-NPC活人感/001-接口出口：新增 6 条接口条目
- 02-NPC活人感/002-接口入口：新增 2 条接口条目 + 修改 1 条
- 01-生命/001-接口出口：修改 1 条 + 新增 7 条接口条目
