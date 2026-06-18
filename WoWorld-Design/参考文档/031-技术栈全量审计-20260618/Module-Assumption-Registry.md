# 模块假设注册表 (Module Assumption Registry) — 摘要

> **审计**: 阶段 2.1-2.2 | **日期**: 2026-06-18
> **扫描**: 22 模块 (家具跳过, 性能优化仅参考) | **总假设数**: ~314
> **完整注册表**: 见 agent extraction files (3 组, ~190KB total)

## 统计

| 状态 | 数量 | 占比 |
|------|------|------|
| CONFIRMED | ~252 | 80% |
| CONFLICT | ~34 | 11% |
| AMBIGUOUS | ~21 | 7% |
| UNVERIFIABLE | ~7 | 2% |

## 模块健康仪表盘

| # | 模块 | 假设数 | 冲突数 | 健康 |
|---|------|--------|--------|------|
| 01 | 游戏概述 | 未详细扫描 | — | 🟢 GREEN |
| 02 | 技术栈方案 | 已作为 TDI 目录 | 3条SUPERSEDED | 🟡 YELLOW |
| 03 | NPC活人感 | ~32 | 1 (物理)+3 AMBIGUOUS | 🟡 YELLOW |
| 04 | 世界生成 | ~26 | 2 (碰撞体/NavMesh)+1 AMBIGUOUS | 🟡 YELLOW |
| 05 | 信仰系统 | ~16 | 0 | 🟢 GREEN |
| 06 | 历史 | ~22 | 0 | 🟢 GREEN |
| 07 | 天气与季节 | ~19 | 0 | 🟢 GREEN |
| 08 | 感官与知觉 | ~20 | 1 (SpatialQuery旧版)+3 AMBIGUOUS | 🟠 ORANGE |
| 09 | 战斗 | ~39 | 0 + 2 AMBIGUOUS | 🟢 GREEN |
| 10 | 技能系统 | ~24 | 0 | 🟢 GREEN |
| 11 | 文化系统 | ~27 | 0 + 1 AMBIGUOUS | 🟢 GREEN |
| 12 | 权力系统 | ~21 | 0 + 1 AMBIGUOUS | 🟢 GREEN |
| 13 | 模型动作与物理 | ~39 | 0 (自洽) | 🟢 GREEN |
| 14 | 物品系统 | ~15 | 2 (BodyPlan引用过期) | 🟠 ORANGE |
| 15 | 生命 | ~15 | 1 (BodyPlan未迁移) | 🟡 YELLOW |
| 16 | 经济系统 | ~15 | 0 + 1 AMBIGUOUS | 🟢 GREEN |
| 17 | 语言表达 | ~14 | 2 (VoiceProfile未标注, VoicePacket歧义) | 🟠 ORANGE |
| 18 | 载具系统 | ~15 | 4 (天气风/技术栈/世界生成/战斗) | 🔴 RED |
| 19 | 音频系统 | ~15 | 3 (Lang012未标注/Items/Perf doc) | 🟠 ORANGE |
| 20 | 魔法 | ~25 | 0 + 2 AMBIGUOUS | 🟢 GREEN |
| 21 | 家具系统 | 0 | — | ⏭️ SKIPPED |
| 22 | 性能优化 | ~12 | 4 (物理/动画骨骼/音频预算) | 🔴 RED |

## 跨模块核心冲突

1. **CHG-033 物理方案级联** (影响 NPC/世界生成/感官/载具/性能优化) — 多个模块仍引用旧 PhysicsServer3D 假设
2. **BodyPlan 所有权迁移未传播** (影响 生命/物品 — 仍引用 Life 而非 woworld_core)
3. **VoiceProfile 所有权迁移未完成** (语言表达 012 仍定义完整 VoiceProfile, CHG-030 要求迁至音频)
4. **音频跨模块注解 69% 未完成** (13 份待修改文档中 9 份未处理)
5. **性能优化文档全面过期** (物理/动画骨骼数/音频预算均与最新模块矛盾)

> **详细数据** (314条完整假设 + 冲突详情): 见 agent extraction files。阶段 4 合并时引用。
