# 开发路线图

> **开发代号**: WoWorld (Wonder World)
> **引擎**: Godot 4.7
> **更新日期**: 2026-06-25

---

## 路线图已迁移

旧版"四轨并行 Week 1-9+"路线图（2026-06-22）已于 2026-06-25 归档至 [[../参考文档/044-开发路线图归档-v1-20260625/README|044-开发路线图归档 v1]]。

**归档原因**：编码速度远超 Week 级规划的更新频率（3 天完成 4 个冲刺）。时间线是涌现的，不能预先固定。

## 当前体系

路线图功能由以下三个文件共同承担（宪法 §7/§8/§13 原生机制）：

| 文件 | 角色 | 位置 |
|------|------|------|
| **`DEPENDENCY_GRAPH.md`** | 地图——层 0→1→2→... 实现顺序 | `woworld-dev-plan/` |
| **`DEVELOPMENT_STATUS.md`** | 当前位置——每个模块 🔴🟡🟢 + WIP | `woworld-dev-plan/` |
| **冲刺提案** (§8 格式) | 下一步——每轮产出，审批后执行 | `woworld-dev-plan/sprint-proposals/` |

## 当前全局快照

- **轨 A (代码)**: 5 crates, 74 tests 全绿 — MC 体素 + Clipmap LOD + 海洋 + 大气 + 昼夜循环。4/27 模块部分实现。
- **5 个红色架构偏离**（审计报告 [[../../../woworld-dev-plan/audit-reports/20250625-code-vs-design/README|20250625-code-vs-design]]）优先修复。
- **轨 B/C (设计)**: 名声系统 + 法律接口 + 魔法性能预算 为下一批设计补全目标。

→ 详细状态见 `DEVELOPMENT_STATUS.md`
→ 下一冲刺见 `sprint-proposals/`

---

> **关联**: [[../../woworld-dev-plan/CONSTITUTION|宪法]] · [[../../woworld-dev-plan/DEVELOPMENT_STATUS|全局状态]] · [[../../woworld-dev-plan/DEPENDENCY_GRAPH|依赖图]]
