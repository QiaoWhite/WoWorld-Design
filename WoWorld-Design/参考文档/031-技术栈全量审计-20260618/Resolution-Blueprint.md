# 解决方案蓝图 (Resolution Blueprint)

> **审计**: 阶段 5 | **日期**: 2026-06-18
> **状态**: ⚠️ **待人工审批** — 审批后方可进入阶段 6 (分波实施)
> **总变更令**: 23 MCO (4 波) + LOD 独立倡议

---

## 波次总览

| 波次 | 变更令数 | 范围 | 依赖 |
|------|---------|------|------|
| **Wave 1** | 8 | 技术栈文档更新 + SpatialQuery传播 + BodyPlan迁移 | 无 (基础) |
| **Wave 2** | 6 | 消费模块更新: 性能/语音/载具/wind_at/CHG-026契约 | Wave 1 |
| **Wave 3** | 6 | 契约与注解完整性 | Wave 2 |
| **Wave 4** | 3 | 独立/低优先级 | 无 |
| **LOD** | 4 | LOD统一架构 (KCQ-022) | Wave 1-2 |

---

## Wave 1: 基础层 (8 MCO)

### MCO-1-001: 更新技术栈 §二 物理行
- **目标**: `技术栈方案/001-WoWorld正式技术栈方案v3.md` §二
- **变更**: "Godot PhysicsServer3D" → "仅玩家 CharacterBody3D 保留 PhysicsServer3D; 其余 Rust 侧空间查询四 trait"
- **标记**: TDI-032, TDI-045, TDI-202 → SUPERSEDED by CHG-033

### MCO-1-002: 更新技术栈 §三 架构图
- **目标**: 同上 §三
- **变更**: 架构图 Godot 侧: "PhysicsServer3D: NPC/载具/海洋浮力" → "PhysicsServer3D: 仅玩家"

### MCO-1-003: 更新技术栈 §十一 性能预算
- **目标**: 同上 §十一
- **变更**: Godot物理 1.7ms→0.5ms; 新增空间查询预算 TerrainQuery≤0.3ms + EntityIndex≤0.2ms + SpatialEventBus≤0.15ms + VisibilityQuery≤0.3ms; 动画≤0.5ms; 音频≤0.17ms; 感官≤1.0ms
- ⚠️ **[ESCALATE]**: Rust 模拟核心累计可能突破 7.0ms。需人工决定: (a)提升至11ms (b)削减模块预算 (c)降至50fps

### MCO-1-004: 技术栈 §二 新增7模块行
- **目标**: 同上 §二
- **变更**: 新增音频/感官/经济/权力/文化/信仰/模型动画物理 7 行

### MCO-1-005: 技术栈 新增 §十四~§二十
- **目标**: 同上
- **变更**: 7 个新模块的~300字摘要 + 指针

### MCO-1-006: 更新技术栈 §八 NPC子模块
- **目标**: 同上 §八
- **变更**: 新增 8.5 节: 基本需求/进阶需求/审美/认知 4 个子系统

### MCO-1-007: 感官系统 SpatialQuery→4 trait
- **目标**: `感官与知觉系统/001-感官系统总纲.md`
- **变更**: `&dyn SpatialQuery` → `&dyn TerrainQuery + &dyn EntityIndex + &dyn SpatialEventBus + &dyn VisibilityQuery`

### MCO-1-008: BodyPlan 迁移至 woworld_core
- **目标**: `生命/001-生命总纲.md` + `物品系统/001-物品系统总纲.md`
- **变更**: 添加 CHG-033 注解块: BodyPlan 定义已提升至 woworld_core; 原定义保留为历史参考; 物品系统新增 WeaponPhysicalParams 映射表

---

## Wave 2: 消费模块更新 (6 MCO)

### MCO-2-001: 重写性能优化文档
- **目标**: `性能优化分析 20260603.md`
- **变更**: 追加CHG-033架构预算章节; 标记旧 PhysicsServer3D 预算为"历史参考"; 更新骨骼数33/35; 更新VRAM面部图集; 新增空间查询/音频/感官预算

### MCO-2-002: 语言表达012 VoiceProfile注解
- **目标**: `语言表达/012-语音输出接口.md`
- **变更**: CHG-030 注解: VoiceProfile所有权→音频模块; 保留旧定义为历史参考

### MCO-2-003: 载具物理级联解决 (5冲突)
- **目标**: `载具系统/001~005`
- **变更**: PhysicsServer3D→Rust空间查询; TerrainQuery地面/导航; EntityIndex避障; 玩家乘坐载具例外

### MCO-2-004: wind_at() 归属 WeatherQuery
- **目标**: `天气与季节系统/001` + 感官 + 音频 + 载具
- **变更**: 定义 wind_at() 于 WeatherQuery trait; 更新所有消费者引用

### MCO-2-005: CHG-026 13条款补入 CLAUDE-INTERFACES.md
- **目标**: `CLAUDE-INTERFACES.md`
- **变更**: 新增 CLS-026-001~013 (VehicleId/动力类型/移动参考系/Crew GOAP...)

### MCO-2-006: 技术栈 §十 载具动力扩展
- **目标**: `技术栈方案/001-WoWorld正式技术栈方案v3.md` §十
- **变更**: 5动力类型; NavMesh→Rust EntityIndex 标注

---

## Wave 3: 契约与注解完整性 (6 MCO)

### MCO-3-001: 音频注解管线完成 (9文档)
- **目标**: CHG-030 §4 列出的9份待标注文档
- **变更**: 每份文档添加 `> [CHG-030 权威注解]` 块

### MCO-3-002: CommunicationNorms 契约
- **目标**: CLAUDE-INTERFACES.md + 语言表达 + 文化系统
- **变更**: 所有权 Language→Culture 注解

### MCO-3-003: ReligiousReproductionNorms 契约
- **目标**: CLAUDE-INTERFACES.md + 生命 + 信仰系统
- **变更**: 所有权 Life→Faith 注解

### MCO-3-004: MEDIUM 契约断裂解决 (FR-M-001~015)
- 关键: FR-M-001 (PowerToEconomicBridge 归属) → ⚠️ **[ESCALATE]** 需人工决定
- FR-M-006~015: 注解/确认/对接

### MCO-3-005: 世界生成物理查询更新
- **目标**: `世界生成/001` + 碰撞体相关文档
- **变更**: 移除 PhysicsServer3D 碰撞烘焙; 替换为 TerrainQuery 密度场

### MCO-3-006: 技能系统 FineArts (0x06) 确认
- **目标**: `技能系统/002-技能分类体系.md`
- **变更**: 确认/新增 FineArts 大类

---

## Wave 4: 低优先级 (3 MCO)

### MCO-4-001: 模块假设缺口系统性解决
### MCO-4-002: NPC物理假设更新
### MCO-4-003: 8 LOW引用过期解决

---

## LOD 统一架构 (KCQ-022)

### 建议方案
- 新建 **TDI-330: LOD统一架构**: "场景LOD与角色LOD分离为两个独立多层体系, 通过 LODCoordinator 协同"
- 新增 **§二十一 LOD统一架构**: 场景LOD(地形/建筑/海洋/云/植被 0-8级) + 角色LOD(骨架/动画/渲染/物理/音频/AI各自独立分层) + LODCoordinator
- 更新 §四/§八/§十一/模块17/模块22 引用统一LOD

### 波次归属: Wave 2-3 (依赖 RC-001/002)

---

## 需人工审批的决定 — ✅ 全部已决

1. ✅ **Rust模拟核心预算**: 保持 7.0ms 上限。新增峰值互斥规则明确标注。最坏重载帧 ≤15ms 为可接受单帧尖峰。VRAM 提升为风险矩阵首要关注。
2. ✅ **PowerToEconomicBridge**: 独立调度层。不在经济也不在权力 crate。~50 行映射代码，两模块零直接依赖。
3. ✅ **LOD统一架构**: 采纳。新增 TDI-330 + §二十一。场景LOD/角色LOD分离, LODCoordinator 统一调度。7 维 LodPrescription + 跨维约束规则。

---

## 实施就绪 — 阶段 6 待启动
