# 已知变更队列 (Known Change Queue)

> **审计**: WoWorld 技术栈全量审计 | **阶段**: 0.4 | **日期**: 2026-06-18
> **扫描源**: CHG-001~033, Change/hand/, CLAUDE.md, CLAUDE-INTERFACES.md, 技术栈 v3

## 摘要

- **总计**: 22 项 | **CRITICAL**: 3 | **HIGH**: 9 | **MEDIUM**: 7 | **LOW**: 2
- 技术栈文档最后撰写于 2026-06-10。CHG-022~033（2026-06-13~17）创建的所有模块均未反映在技术栈中
- `Change/hand/007 技术栈修改.md` 是用户的直接指令，要求全面技术栈修订计划

---

## CRITICAL (3)

| KCQ | 来源 | 描述 | 影响模块 |
|-----|------|------|----------|
| KCQ-001 | CHG-033 §5 (CRITICAL) | §二 物理行："PhysicsServer3D" → "仅玩家 CharacterBody3D; 其余 Rust 空间查询" | 模型物理, 世界生成, 感官, 战斗, 载具 |
| KCQ-002 | CHG-033 §5 (HIGH→CRITICAL) | §十一 性能预算: 动画/物理/空间查询预算需更新 | 模型物理, woworld_spatial |
| KCQ-013 | hand/007 技术栈修改.md | **用户指令**: 全面的技术栈修改计划 | 全22模块 |

## HIGH (9)

| KCQ | 来源 | 描述 |
|-----|------|------|
| KCQ-003 | CHG-033 推断 | §二 动画行 → 完整模型动作物理系统替代片段 |
| KCQ-004 | CHG-033 推断 | §二 缺少独立的 模型/动画/物理 模块条目 |
| KCQ-005 | CHG-030 §4 (显式) | §二 缺少音频模块条目 |
| KCQ-006 | CHG-030 §4 (显式) | §十一 缺少音频性能预算 |
| KCQ-008 | CHG-026 §2 (显式) | §十 载具动力类型不完整(5种) + NavMesh 风险更新 |
| KCQ-010 | CHG-031 推断 | §二 缺少感官模块条目 |
| KCQ-011 | CHG-031, CHG-033 (显式) | SpatialQuery 拆分为 4 trait, wind_at()→WeatherQuery |
| KCQ-016 | CLAUDE-INTERFACES CHG-033 (CRITICAL) | wind_at() 迁移至 WeatherQuery 需反映在技术栈 |
| KCQ-022 | **用户审阅阶段0 (2026-06-18)** | **LOD架构大改**: 场景LOD与角色LOD分开分层, 各自多层体系, 协同配合——最大化性能+画面+游戏性。当前 LOD 分散在 §四(地形8级)、§八(NPC L1-L4)、§十一(预算)、模块动作物理(骨骼LOD)中, 缺乏统一策略 |

## MEDIUM (7)

| KCQ | 来源 | 描述 |
|-----|------|------|
| KCQ-007 | CHG-030 §5 (显式) | VoiceProfile 所有权: Language→Audio 未反映 |
| KCQ-009 | CHG-024 (显式) | CommunicationNorms 所有权: Language→Culture 未反映 |
| KCQ-012 | CHG-032 推断 | NPC 认知与智慧系统未在 §八 中 |
| KCQ-014 | CHG-033 §5 (MEDIUM) | BodyPlan 定义: Life→woworld_core 未反映 |
| KCQ-019 | CHG-027~029,032 推断 | §八 NPC 子系统不完整(缺少4个子系统) |
| KCQ-020 | CHG-022~025 推断 | §二 缺少 经济/权力/文化/信仰 条目 |
| KCQ-021 | CLAUDE.md vs 技术栈 | 22模块 vs ~15行: 7模块缺失 |

## LOW (2)

| KCQ | 来源 | 描述 |
|-----|------|------|
| KCQ-015 | CHG-033 §5 (LOW) | 物品系统需 WeaponPhysicalParams 映射表 |
| KCQ-017 | CHG-033 推断 | §三 架构图物理描述过期(PhysicsServer3D 描述) |

---

## 关键发现

1. **性能预算冲突**: §十一 Rust 模拟核心预算 ≤7.0ms。新增模块累计: 音频0.17ms + 感官~1ms + 认知<0.15ms + 动画0.5ms + 文化<0.05ms + 经济<2ms … 可能突破 7.0ms 上限
2. **7模块完全缺失**: 经济、权力、文化、信仰、音频、感官、模型动作物理在技术栈中无独立条目
3. **3条TDI已被 CHG-033 取代但未更新**: TDI-032 (物理), TDI-045 (PhysicsServer3D管理), TDI-202 (Godot物理预算)
