# 审计台账 (Audit Ledger)

> **审计**: WoWorld 技术栈全量审计 | **日期**: 2026-06-18
> **状态**: 全部 6 阶段完成 — 审计闭合

---

## 一、TDI 目录

→ 详见 [[TDI-Catalog.md]] | **237 条 TDI** | 234 ACTIVE | 3 SUPERSEDED by CHG-033

---

## 二、模块假设注册表

→ **阶段 2 完成** | [[Module-Assumption-Registry.md]] | **~314 条假设**: 252 CONFIRMED + 34 CONFLICT + 21 AMBIGUOUS + 7 UNVERIFIABLE

### 模块健康
- 🔴 RED: 载具(4冲突), 性能优化(4冲突)
- 🟠 ORANGE: 感官, 物品, 语言表达, 音频
- 🟡 YELLOW: NPC, 世界生成, 技术栈, 生命
- 🟢 GREEN: 其余12模块

---

## 三、契约断裂日志

→ **阶段 1 完成** | [[Contract-Fracture-Log.md]] | **39 条断裂**: 4 CRITICAL + 12 HIGH + 15 MEDIUM + 8 LOW

### 断裂聚类
1. CHG-033 物理方案级联 (6 条) — PhysicsServer3D→Rust 空间查询未传播
2. 7 模块条目缺失 (6 条) — v3 后创建的模块在技术栈中无条目
3. CHG-026 文档缺失 (1 条) — 载具系统契约不在 CLAUDE-INTERFACES.md 中

---

## 四、冲突矩阵

→ **阶段 3-4 完成** | [[Conflict-Matrix.md]] | **17 根冲突** (去重合并自 39断裂 + 34冲突 + 3取代TDI)

| 优先级 | 数量 | 顶级项 |
|--------|------|--------|
| P0 | 5 | RC-001(物理迁移46分), RC-002(6模块缺失38分), RC-003(SpatialQuery 31分), RC-004(BodyPlan 27分), RC-005(性能过期26分) |
| P1 | 3 | VoiceProfile, 音频注解, 载具物理 |
| P2 | 5 | CommunicationNorms, ReligiousReproductionNorms, wind_at(), CHG-026缺失, 文档缺口 |
| P3 | 3 | 契约注解, 世界生成物理, NPC物理 |
| P4 | 1 | 引用过期 |

### TDI处置
- DEPRECATE: 3 | REPLACE: 2 | MODIFY: ~10 | CLARIFY: ~5 | ADD: ~93 | KEEP: ~124

---

## 五、解决方案队列

→ 阶段 5 完成 | 全部冲突已分派至 4 波变更令

| MCO-ID | Wave | RC | 处置 | 技术栈变更 | 模块变更 | 状态 |
|--------|------|-----|------|-----------|----------|------|
| MCO-1-001 | Wave 1 | RC-002 | 6模块条目新增 | 技术栈方案 v3.0 — §模块清单 +6条 | Life/Combat/Items(WeaponPhysicalParams)/Skills/Economy/Belief 条目 | ✅ CHG-034 |
| MCO-1-002 | Wave 1 | RC-005 | 性能预算更新 | 帧预算重分配表、VRAM重分配表 | 技术栈方案 v3.0 §性能预算 | ✅ CHG-034 |
| MCO-1-003 | Wave 1 | RC-003 | SpatialQuery拆分+WindAt迁移 | 技术栈方案 — 空间查询条目更新 | 感官系统/技术栈方案 — SpatialQuery→4 trait + wind_at→WeatherQuery | ✅ CHG-034 |
| MCO-1-004 | Wave 1 | RC-004 | BodyPlan提升 | 技术栈方案 — 模型定义条目更新 | woworld_core — BodyPlan定义迁入；生命/001 消费 | ✅ CHG-034 |
| MCO-2-001 | Wave 2 | RC-007 | VoiceProfile迁移 | 技术栈方案 — 音频条目更新 | 音频系统：VoiceProfile迁入；语言表达：VoiceProfile迁出 | ✅ CHG-035 |
| MCO-2-002 | Wave 2 | RC-008 | 音频注解补全 | 无 | 音频系统模块条目补全(5条缺失) + 战斗/魔法模块注解更新 | ✅ CHG-035 |
| MCO-2-003 | Wave 2 | RC-009 | 载具物理条款 | 技术栈方案 — 载具条目更新 | 载具系统 — 物理方案注解 | ✅ CHG-035 |
| MCO-2-004 | Wave 2 | RC-011 | CommunicationNorms+GestureCultureMapping补注 | 无 | 语言表达 + 文化系统 — 契约注解 | ✅ CHG-035 |
| MCO-2-005 | Wave 2 | RC-012 | ReligiousReproductionNorms补注 | 无 | 信仰系统 + 生命 — 契约注解 | ✅ CHG-035 |
| MCO-2-006 | Wave 2 | RC-010 | CHG-026契约表补全 | 无 | CLAUDE-INTERFACES.md — CHG-026 完整契约表 | ✅ CHG-035 |
| MCO-3-001 | Wave 3 | RC-006 | 契约注解补全 | 无 | CHG-027/028/029/030/031 — 技术栈方案条目注解 | ✅ CHG-036 |
| MCO-3-002 | Wave 3 | RC-013 | 世界生成物理注解 | 无 | 世界生成 — 地表物理注解(SurfaceMaterial) | ✅ CHG-036 |
| MCO-3-003 | Wave 3 | RC-016 | 引用过期更新 | 无 | 多项 — v2→v3 引用更新、CHG-033→技术栈方案交叉引用 | ✅ CHG-036 |
| MCO-3-004 | Wave 3 | RC-001 | 物理迁移文档同步 | 无 | CLAUDE-INTERFACES.md — TDI取代注解 | ✅ CHG-036 |
| MCO-3-005 | Wave 3 | RC-014 | NPC物理注解 | 无 | NPC ver2.0 — 物理表达 §4 注解(1条待Wave 4) | ✅ CHG-036 |
| MCO-4-001 | Wave 4 | RC-015 | 文档缺口低优 | 无 | 多项文档注解补全 | ✅ CHG-037 |
| MCO-4-002 | Wave 4 | RC-014 | NPC物理假设更新 | 无 | NPC ver2.0 — Part 4 物理表达 CHG-033 架构变更注解 | ✅ CHG-037 |
| MCO-4-003 | Wave 4 | 残留LOW | 残留LOW确认+台账闭合 | 无 | CLAUDE-INTERFACES.md 确认 + Audit-Ledger.md 闭合 | ✅ CHG-037 |

---

## 六、变更实施日志

→ 阶段 6 完成 | 全部 4 波变更令已执行，审计闭合

### Wave 1 (CHG-034) — P0 关键冲突

| MCO-ID | 处置 | 目标文件 | 状态 |
|--------|------|---------|------|
| MCO-1-001 | 6模块条目新增 | 技术栈方案 v3.0 §模块清单 | ✅ |
| MCO-1-002 | 性能预算更新 | 技术栈方案 v3.0 §性能预算 | ✅ |
| MCO-1-003 | SpatialQuery拆分+WindAt迁移 | 感官系统 001 + 技术栈方案 | ✅ |
| MCO-1-004 | BodyPlan提升至woworld_core | 生命/001 + 技术栈方案 | ✅ |

**Wave 1 完成日期**: 2026-06-18 | **CHG**: [[../../../WoWorld-Design/Change/CHG-034-技术栈方案v3.0审计Wave1修正-20260618|CHG-034]]

### Wave 2 (CHG-035) — P1 高优先级 + P2 文档补全

| MCO-ID | 处置 | 目标文件 | 状态 |
|--------|------|---------|------|
| MCO-2-001 | VoiceProfile迁移 | 音频系统 + 语言表达 | ✅ |
| MCO-2-002 | 音频注解补全(5条) | 音频系统 + 战斗/魔法模块 | ✅ |
| MCO-2-003 | 载具物理条款 | 载具系统 + 技术栈方案 | ✅ |
| MCO-2-004 | CommunicationNorms补注 | 语言表达 + 文化系统 | ✅ |
| MCO-2-005 | ReligiousReproductionNorms补注 | 信仰系统 + 生命 | ✅ |
| MCO-2-006 | CHG-026契约表补全 | CLAUDE-INTERFACES.md | ✅ |

**Wave 2 完成日期**: 2026-06-18 | **CHG**: [[../../../WoWorld-Design/Change/CHG-035-技术栈方案v3.0审计Wave2修正-20260618|CHG-035]]

### Wave 3 (CHG-036) — P2-P3 契约注解 + 文档同步

| MCO-ID | 处置 | 目标文件 | 状态 |
|--------|------|---------|------|
| MCO-3-001 | 契约注解补全(5个CHG) | 技术栈方案 — CHG-027/028/029/030/031 | ✅ |
| MCO-3-002 | 世界生成物理注解(SurfaceMaterial) | 世界生成 | ✅ |
| MCO-3-003 | v2→v3引用过期更新 | 多项文档 | ✅ |
| MCO-3-004 | TDI取代注解 | CLAUDE-INTERFACES.md | ✅ |
| MCO-3-005 | NPC物理注解(初步) | NPC ver2.0 | ✅ |

**Wave 3 完成日期**: 2026-06-18 | **CHG**: [[../../../WoWorld-Design/Change/CHG-036-技术栈方案v3.0审计Wave3修正-20260618|CHG-036]]

### Wave 4 (CHG-037) — P3-P4 残余清理 + 审计闭合

| MCO-ID | 处置 | 目标文件 | 状态 |
|--------|------|---------|------|
| MCO-4-001 | 文档缺口低优补全 | 多项 | ✅ |
| MCO-4-002 | NPC物理假设 CHG-033 架构变更注解 | NPC ver2.0 Part 4 | ✅ |
| MCO-4-003 | 残留LOW确认 + 台账闭合 | CLAUDE-INTERFACES.md + Audit-Ledger.md | ✅ |

**Wave 4 完成日期**: 2026-06-18 | **CHG**: [[../../../WoWorld-Design/Change/CHG-037-技术栈方案v3.0审计Wave4修正-20260618|CHG-037]]

### 审计闭合摘要

| 指标 | 初始 | 最终 |
|------|------|------|
| TDI | 237 (234 ACTIVE + 3 SUPERSEDED) | 237 (234 ACTIVE + 3 SUPERSEDED by CHG-033) |
| 假设 | 314 (252 CONFIRMED + 34 CONFLICT + 21 AMBIGUOUS + 7 UNVERIFIABLE) | 314 (已审计,冲突已处置) |
| 契约断裂 | 39 (4 CRITICAL + 12 HIGH + 15 MEDIUM + 8 LOW) | 0 残余断裂 |
| 根冲突 | 17 (5 P0 + 3 P1 + 5 P2 + 3 P3 + 1 P4) | 0 未处置冲突 |
| 波次 | — | 4 波 (CHG-034→035→036→037) |
| 模块健康 | 4 RED / 4 ORANGE / 4 YELLOW / 12 GREEN | 22 GREEN |

---

## 注意事项检查 (阶段 0)

### 游戏设计原则
| 原则 | 状态 | 发现 |
|------|------|------|
| Rust解耦 | ✅ | 阶段0纯基础设施建设，未涉及模块间依赖 |
| 涌现式交互 | ✅ | — |
| 程序化生成 | ✅ | — |
| 不删除原有设计 | ✅ | 所有索引为增量，不修改源文档 |
| 性能与画面平衡 | ⚠️ | KCQ已识别性能预算可能超支 |
| 独立开发者可实现性 | ✅ | — |
| 完整的人 | ✅ | — |
| 冒险与生活平等 | ✅ | — |
| 全球多元文化 | ✅ | — |

### 协作原则
| 原则 | 状态 |
|------|------|
| 穷尽理解再判断 | ✅ |
| 证据优先于直觉 | ✅ |
| 区分不可能和很难 | ✅ |
| 保留创意调整路径 | ✅ |
| 不确定时升级而非消灭 | ✅ |
| 沉默不是同意 | ✅ |

### 工程纪律
| 纪律 | 状态 | 发现 |
|------|------|------|
| 接口契约不可绕过 | ✅ | 交叉引用矩阵基于契约表 |
| Owner-Consumer方向性 | ✅ | — |
| CHG是权威记录 | ✅ | KCQ已扫描全部CHG |
| 性能预算不可超支 | ⚠️ | KCQ-018: 累计新模块预算可能突破7ms |
| 技术栈是派生文档 | ✅ | 审计以此为出发点 |
| 根源追溯 | ✅ | — |
| 每次修改后自检 | ✅ | 阶段0无修改 |
| TOML驱动优于硬编码 | ✅ | — |
| 种子确定性 | ✅ | — |
| 分层模拟L1-L4 | ✅ | — |
