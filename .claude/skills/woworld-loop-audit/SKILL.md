---
name: woworld-loop-audit
description: >
  设计文档自动审计循环系统。手动触发，最多10轮，每轮全量轻扫（硬错误自动修复）
  + 热点模块16维深度审计（软问题报告）。跨轮持久化进度，子代理工厂模式并行审计。
  适用于项目重大变更、里程碑审计、定期保鲜检查。
trigger: |
  /woworld-loop-audit, loop-audit, 审计循环, 设计审计, woworld-loop-audit
---

# woworld-loop-audit — 设计文档自动审计循环

> **定位**: WoWorld-Design 的项目级设计质量守护进程。手动触发，最多 10 轮，穷尽所能发现并修复问题。
> **与现有 skill 关系**: 独立于 `/woworldidea-design`（单文件增量同步）和 `/woworld-tech-stack-audit`（技术栈全量审计）。填补了"全模块自动发现 + 定期健康检查"的空白。
> **使用场景**: 项目重大变更后、累计 5+ CHG 后、里程碑前、或任何"想确认设计文档是否健康"的时刻。

---

## §1 概述

### 1.1 这个 skill 做什么

```
┌──────────────────────────────────────────────────────────┐
│  每轮自动完成：                                            │
│  S1  全量轻扫（24模块/199文件/78K行——硬错误自动修复）       │
│  S2  热点深度审计（1-3模块/16维全穿透——软问题报告）         │
│  S3  进度持久化 + 报告产出                                 │
│                                                          │
│  跨轮保证：                                                │
│  state.json 追踪进度 → 不重复、不遗漏、可恢复               │
│  硬错误 → 立即自动修复（有 revert 日志）                     │
│  软问题 → 结构化报告（附原文引用 + 正反论证）                 │
│  存疑项 → 待人工裁决清单（不入正文避免 noise）               │
│                                                          │
│  最终交接：                                                │
│  FINAL-SUMMARY.md → 你一页看完所有发现                     │
│  待人工裁决清单 → 你逐条确认/驳回                           │
│  建议后续操作清单 → 你按步骤执行                            │
└──────────────────────────────────────────────────────────┘
```

### 1.2 与现有质量保障体系的关系

| 工具 | 触发 | 范围 | 深度 | 自动化 |
|------|------|------|------|--------|
| `/woworldidea-design sync` | 手动/每次修改后 | 修改文件+关联 | 增量同步 | 半自动 |
| `/woworldidea-design audit` | 手动指定文件 | 单文件 | 逐条比对 | 仅报告 |
| `/woworld-tech-stack-audit` | 纯手动 | 全量 | 8阶段3层 | 手动 |
| **`/woworld-loop-audit`** | **手动触发+loop** | **全量+热点** | **16维** | **自动修复+报告** |

### 1.3 何时使用

**触发时机**（满足任一即可）：
- 累计 5 个以上 CHG 后
- 项目里程碑前（如"世界生成 v2.2 完成"）
- 感觉"设计文档是不是有点乱了"的时候
- 长期未碰项目后回来继续开发前

**不适用场景**：
- 日常单文件修改后（用 `/woworldidea-design sync`）
- 技术栈版本升级（用 `/woworld-tech-stack-audit`）
- 新建模块前的脚手架检查（用 `/woworldidea-design gate`）

---

## §2 触发方式

### 2.1 单轮手动（审一轮就停）

```bash
/woworld-loop-audit --effort high
```

跑一轮（S0→S1→S2→S3），审完当前优先级最高的模块，产出报告，告诉你下一轮该审谁。

### 2.2 多轮 loop（自动跑多轮）

```bash
/loop 5m /woworld-loop-audit --effort high --max-rounds 10
```

每 5 分钟自动跑一轮。每轮读 `state.json` 知道自己在哪，审下一个模块，更新进度。10 轮跑完或模块耗尽 → 自动结束并输出 FINAL-SUMMARY.md。

### 2.3 参数

| 参数 | 值 | 默认 | 说明 |
|------|-----|------|------|
| `--effort` | `medium` / `high` / `max` | `high` | 审计深度档位 |
| `--max-rounds` | 1-10 | 10 | 单次 session 最多轮数 |
| `--dry-run` | — | — | 只看本轮会审谁、审什么，不实际跑 |
| `--resume` | — | — | 从上次中断处恢复（读 state.json） |
| `--reset` | — | — | 清空 state.json 重新开始 |

### 2.4 Effort 档位详情

| 档位 | S1 行为 | S2 行为 | 子代理数 | 每轮 ~token |
|------|---------|---------|---------|------------|
| **medium** | 全量 A1-A3 | 每个模块前 8 维（跳过 D 类体验维度） | 3 子代理 | ~50k-80k |
| **high** ⭐ | 全量 A1-A3 | 16 维全穿透 | 4 子代理 | ~140k-220k |
| **max** | 全量 A1-A3 + 消费方全量验证 | 16 维 + 每个 finding spawn 1 skeptic 反驳 | 8 子代理 | ~200k-400k |

---

## §3 单轮执行模型

```
S0 准入检查（~30s）
│
├─ 读 state.json
│   ├─ 不存在 → 初始化新 session（创建目录、state.json、扫描所有模块）
│   ├─ rounds_completed >= max_rounds → STOP，提示用户 --reset 或增加轮数
│   └─ 正常 → 继续
│
├─ 读 modified_files.txt（PostToolUse hook 产物）+ git diff
│   └─ 确定本轮热点模块清单
│
├─ 判断：无热点 且 无待审模块 → STOP（世界干净）
│
└─ 进入 S1
    │
    ▼
S1 全量轻扫（并行，全量 199 文件）
│
├─ A1 结构完整性扫描
│   ├─ 正则提取所有 [[wikilink]]，Glob 验证目标存在
│   ├─ 检查编号重复/跳号
│   ├─ 检查 README 存在 + 最低 60 行
│   ├─ 检查 frontmatter 完整性
│   └─ 检查接头总览时间戳腐化率
│
├─ A2 接口契约三方对齐
│   ├─ 出口 vs CLAUDE-INTERFACES.md
│   ├─ 入口 vs 源模块出口签名
│   └─ trait 索引登记完整性
│
├─ A3 数学正确性
│   ├─ python 验算所有公式
│   ├─ 边界值代入
│   ├─ 量纲一致性
│   └─ 概率范围检查
│
├─ 硬错误 → 按 fix-rules.md 自动修复
│   每个修复写入 auto-fixes.log
│
└─ 产出：S1 全量轻扫报告
    │
    ▼
S2 深度审计（串行，仅热点模块）
│
├─ pick_next_module(state) → 按优先级公式选择
│   │
│   ▼
├─ 启动 4 个子代理（并行）：
│   ├─ Agent-A: 正确性审计 A1+A2+A3 → findings_a[]
│   ├─ Agent-B: 设计质量审计 B1+B2+B3+B4 → findings_b[]
│   ├─ Agent-C: 架构解耦审计 C1+C2+C3+C4 → findings_c[]
│   └─ Agent-D: 体验涌现审计 D1+D2+D3+D4+D5 → findings_d[]
│   │
│   ▼
├─ 收集 4×findings → 合并去重
│   │
│   ▼
├─ 对每条 finding 执行 G1 自我质疑：
│   ├─ 引用原文原句作为证据
│   ├─ 评估置信度：certain / likely / uncertain
│   ├─ certain/likely + 硬错误 → 自动修复
│   ├─ certain/likely + 软问题 → 写入报告
│   └─ uncertain → 写入待人工裁决清单（不入报告正文）
│   │
│   ▼
├─ 写入本轮深度审计报告 R0N-{模块名}-{timestamp}.md
│
└─ 更新 state.json（标记模块为 done）
    │
    ▼
S3 收尾
│
├─ 合并 S1+S2 报告
├─ 更新 state.json（rounds_completed++, 更新各模块状态）
│
├─ 判断终止条件：
│   ├─ rounds_completed >= max_rounds → 生成 FINAL-SUMMARY.md → DONE
│   ├─ 所有热点模块已审完 → 生成 FINAL-SUMMARY.md → DONE
│   ├─ 本轮无新发现且上次轻扫距今 < 1h → 生成 FINAL-SUMMARY.md → DONE
│   └─ 否则 → 标记 CONTINUE，输出进度提示
│
└─ 如果 DONE：输出 FINAL-SUMMARY.md 路径 + 用户待办清单
```

---

## §4 16 维审计矩阵

> 📘 **详细检查清单见 [`references/16-dimensions.md`](references/16-dimensions.md)**。每维度含：检查项枚举、判定标准、引用要求、置信度指南。

### A 类：正确性（硬 —— 自动检测 + 自动修复）

| 维度 | 核心检查 |
|------|---------|
| **A1 结构完整性** | 死 wikilink、残缺 wikilink、编号重复/跳号、README 缺失、README <60 行、接头总览时间戳过期 >90 天、归档文件引用、frontmatter 缺失 |
| **A2 接口契约一致性** | 出口 vs CLAUDE-INTERFACES 对齐、入口 vs 源出口签名一致、trait 索引登记、所有权唯一性、影响链反查、CHG 接头段落存在 |
| **A3 数学正确性** | python 验算公式、边界值代入、量纲一致性、概率 [0,1] 范围、百分比之和、浮点精度说明 |

### B 类：设计质量（软 —— AI 语义理解 → 报告）

| 维度 | 核心检查 |
|------|---------|
| **B1 内部一致性** | 同概念同定义、跨模块定义不漂移、版本号一致、参数传递一致、命名一致 |
| **B2 边界完整性** | 输入边界声明、极端值行为、失败模式、并发边界、时间边界 |
| **B3 设计原则遵循** | 故事生成器非容器、NPC=完整的人、玩家=NPC 同规则、涌现>门控、种子确定性、零硬编码、技能事后记录、路径平等 |
| **B4 遗漏检测** | 关键概念未定义、引用链断裂、孤立接口、消费方无供应、示例缺失、待完善项追踪 |

### C 类：架构与解耦（软 → 报告）

| 维度 | 核心检查 |
|------|---------|
| **C1 Trait 边界** | 暴露行为非数据、pub(crate) 封装、单一职责、可 mock、异步边界 |
| **C2 零循环依赖** | 直接循环、隐式事件循环、类型循环、文档引用循环 |
| **C3 数据驱动边界** | TOML 可覆盖性、Mod 安全边界、配方表分离（交互 vs 制造）、Registry 扩展点、事件订阅 |
| **C4 事件总线覆盖** | 跨模块通知、事件完整性（生命周期）、事件负载充足、幂等性 |

### D 类：体验与涌现（软 → 报告）

| 维度 | 核心检查 |
|------|---------|
| **D1 玩家接触面** | 首次接触点、学习曲线、掌握深度、反馈清晰度、无障碍性、跨系统关联 |
| **D2 涌现潜力** | 门控 vs 涌现、参数自由度、NPC 自主性、连锁反应、稀有事件 |
| **D3 程序化参数空间** | 种子确定性、参数可调范围、玩家可感知变化、参数交互、增量可扩展 |
| **D4 模块独特性** | 核心差异、与世界观绑定、最小可行复杂度、玩家记忆点 |
| **D5 性能可落地性** | 预算存在+推导、整体预算协调（GTX 1660 SUPER 6GB）、LOD 策略、最坏情况、内存布局 |

---

## §5 子代理工厂

### 5.1 分派规则

```
S2 深度审计目标模块确定后 → 并行启动 4 子代理

  Agent-A (正确性)          Agent-B (设计质量)
  ├─ A1 结构完整性           ├─ B1 内部一致性
  ├─ A2 接口契约             ├─ B2 边界完整性
  ├─ A3 数学正确性           ├─ B3 设计原则遵循
  └─ 读取：模块全部 .md       └─ B4 遗漏检测
     + CLAUDE-INTERFACES       读取：模块全部 .md
     + 接头总览出口/入口          + CLAUDE-INTERFACES 相关条目
                                 + CLAUDE.md 核心哲学

  Agent-C (架构解耦)        Agent-D (体验涌现)
  ├─ C1 Trait 边界           ├─ D1 玩家接触面
  ├─ C2 零循环依赖           ├─ D2 涌现潜力
  ├─ C3 数据驱动边界         ├─ D3 程序化参数空间
  ├─ C4 事件总线覆盖         ├─ D4 模块独特性
  └─ 读取：模块全部 .md       └─ D5 性能可落地性
     + 接头总览出口/入口         读取：模块全部 .md
     + trait 索引                + 玩家相关交叉引用
     + 依赖图
```

### 5.2 子代理输入契约

每个子代理接收（通过 prompt 传递）：

```
模块名: <module_name>
模块路径: WoWorld-Design/Happy Game/开发阶段/<module_dir>/
文件清单:
  - 001-xxx.md
  - 002-xxx.md
  ...
审计维度: [<dim1>, <dim2>, ...]  ← 详细检查项见 references/16-dimensions.md
关联契约: CLAUDE-INTERFACES.md §<module_section>
关联接头: 接头总览/<slot>/001-接口出口.md, 002-接口入口.md
effort_level: <medium|high|max>
```

### 5.3 子代理输出契约（结构化 JSON）

```json
{
  "agent": "Agent-A",
  "dimensions_audited": ["A1", "A2", "A3"],
  "stats": {
    "files_read": 10,
    "lines_read": 7103,
    "findings_total": 12,
    "findings_certain": 5,
    "findings_likely": 4,
    "findings_uncertain": 3
  },
  "findings": [
    {
      "id": "A1-001",
      "dimension": "A1",
      "severity": "hard_error",
      "confidence": "certain",
      "title": "死 wikilink: [[过时的路径]]",
      "source": {
        "file": "经济系统/003-xxx.md",
        "line": 42,
        "quote": "...详见 [[过时的路径]]..."
      },
      "analysis": "目标文件已于 CHG-xxx 中重命名为 [[新路径]]",
      "suggested_fix": "更新 wikilink 为 [[新路径]]"
    }
  ]
}
```

### 5.4 子代理工作指令

```
你是一个专项审计代理。你的任务是指定维度的审计，不是修改文件。

规则：
1. 逐条检查 references/16-dimensions.md 中分配给你的检查项
2. 每条发现必须引用原文原句（文件:行号 + 原文引用）
3. 每条发现必须评估置信度：certain / likely / uncertain
4. 不确定的发现也要报告——由主控决定是否入报告正文
5. 返回结构化 JSON——不要 Markdown 正文，不要修改任何文件
6. 你的返回就是最终产出，不是中间对话——直接给结果
```

---

## §6 发现处理流水线

```
发现问题
    │
    ▼
┌──────────────────────────────────────────────┐
│ G1 自我质疑（主控执行，非子代理）                │
│                                               │
│  "这个真的是问题吗？"                           │
│                                               │
│  ① 验证子代理是否引用了原文原句（无引用 → 退回）  │
│  ② 快速检查是否有被忽略的上下文                  │
│  ③ 确认置信度标签合理                           │
│                                               │
│  输出：验证通过的 finding + confidence 标签      │
└──────────────────┬───────────────────────────┘
                   │
        ┌──────────┴──────────┐
        ▼                     ▼
    certain / likely       uncertain
        │                     │
        ▼                     ▼
┌───────────────┐   ┌──────────────────────┐
│ G2 分类处理    │   │ 写入待人工裁决清单      │
│               │   │                      │
│ 硬错误 → 自动修│   │ 不入报告正文           │
│ （按 fix-rules │   │ 附：证据 + 正反论证    │
│  .md 规则表）  │   │ 不自动修改任何文件      │
│               │   │                      │
│ 软问题 → 写入  │   │ 最终报告醒目标注数量    │
│ 报告正文       │   └──────────────────────┘
└───────┬───────┘
        │
        ▼
┌──────────────────────────────────────────────┐
│ G3 修复后自检（仅自动修复项）                    │
│                                               │
│ ① 修改是否引入了新矛盾？（重新扫描修改段落）      │
│ ② 修改后的段落是否仍可解析？                     │
│ ③ 写入 auto-fixes.log（什么改了、为什么、改前值）│
└──────────────────────────────────────────────┘
```

### 6.1 硬规则（不可违背）

| # | 规则 | 原因 |
|---|------|------|
| 1 | **不引用原文 → 不算发现问题** | 防止 AI 幻觉 |
| 2 | **uncertain → 不入报告正文** | 避免 noise 淹没 signal |
| 3 | **自动修复 → 必须写 revert 日志** | 错了能回退 |
| 4 | **不碰设计意图** | 不修改数值、算法、设计决策——只修结构性错误 |
| 5 | **不跨模块自动修改设计文档正文** | 跨模块修只改接头总览入口/出口文件 |
| 6 | **不确定就标记** | 宁可标记 uncertain 让人看，也不强行分类 |

---

## §7 进度状态机

### 7.1 state.json 位置与格式

```
.claude/audit-reports/state.json
```

```json
{
  "session_id": "20260621-001",
  "started_at": "2026-06-21T14:00:00",
  "status": "in_progress",
  "max_rounds": 10,
  "rounds_completed": 3,
  "effort_level": "high",
  "modules": {
    "经济系统": {
      "status": "done",
      "path": "WoWorld-Design/Happy Game/开发阶段/经济系统/",
      "file_count": 11,
      "approx_lines": 7103,
      "impact_rank": 4,
      "round": 1,
      "hard_errors_fixed": 5,
      "soft_findings": 7,
      "pending_human": 2,
      "health_score": 7.2,
      "last_audit": "2026-06-21T14:23:00"
    },
    "生命": {
      "status": "in_progress",
      "path": "WoWorld-Design/Happy Game/开发阶段/生命/",
      "file_count": 15,
      "approx_lines": 6305,
      "impact_rank": 1,
      "round": 3
    },
    "魔法": {
      "status": "pending",
      "path": "WoWorld-Design/Happy Game/开发阶段/魔法/",
      "file_count": 2,
      "approx_lines": 129,
      "impact_rank": 22
    }
  },
  "quick_scan_total_fixes": 23,
  "global_pending_human_decisions": 5,
  "next_recommended_trigger": "建议在下次 CHG 后运行，或 2026-07-01 前"
}
```

### 7.2 状态转换

```
pending → in_progress → done
                      ↘ partial (大模块未审完，下轮续审)
                      ↘ skipped (轮次耗尽未审到)
```

### 7.3 跨轮逻辑

```
每轮开始:
  1. 读 state.json
  2. 找 status == "in_progress" 且标记了 partial 的模块 → 优先续审
  3. 否则按优先级公式选择下一个 pending 模块

每轮结束:
  1. 更新当前模块状态: done / partial
  2. rounds_completed += 1
  3. 写回 state.json

如果 state.json 损坏或不存在:
  → 初始化新 session，扫描所有模块目录，填充 modules 字段
```

### 7.4 模块优先级公式

```
priority_score = 
    impact_matrix_risk × 0.3
  + (git_changes_last_30d / max_changes) × 0.4
  + (days_since_last_audit / 90) × 0.3

首次运行（所有模块无 last_audit）:
  → 按 impact_rank 从高到低纯排序
  → 前 3 轮：生命 → 世界生成 → NPC活人感模块
  → 第 4 轮起：切换为混合加权公式
```

### 7.5 模块大小分类

```
大模块（>3,000 行）→ 每轮审 1 个
中型模块（1,000-3,000 行）→ 每轮审 1 个
小型模块（<1,000 行）→ 每轮审 2-3 个
每轮只取一个大小类别，不混搭
```

---

## §8 硬错误自动修复规则

> 📘 **完整规则表见 [`references/fix-rules.md`](references/fix-rules.md)**。以下是概要。

| 错误类型 | 自动修复 | 不自动修复 |
|---------|---------|-----------|
| **死 wikilink** | 模糊匹配距离 ≤3 或 git rename 检测 → 更新链接 | 距离 >3 → 待人工裁决 |
| **编号重复** | 保留优先文件编号，后创文件追加后缀，更新引用 | 如果冲突涉及章节号（中文数字混用）→ 待人工裁决 |
| **README 缺失** | 自动生成骨架（文档索引 + 架构概述） | README 行数不足 → 只报告 |
| **frontmatter 缺失** | 从模块 README 复制模板自动补全 | — |
| **契约签名不一致** | 以 CLAUDE-INTERFACES.md 为权威源修正接头总览 | 涉及设计文档正文 → 不修 |
| **数学计算错误** | 数字错公式对 → 自动修正数字 | 公式本身错 → 待人工裁决 |
| **概率超出 [0,1]** | 明显百分比（如 50）→ 自动 /100 | 负值或上下文不明 → 待人工裁决 |
| **时间戳过期** | 统计腐化率，不自动改时间戳 | — |

### 8.1 自动修复的 revert 路径

每次自动修复在 `auto-fixes.log` 中记录：

```
时间戳 | 错误类型 | 文件:行号 | 改前值 | 改后值 | 修复方法
```

需要回退时，从 log 反向查找即可。

---

## §9 报告产出

> 📘 **完整模板见 [`references/report-template.md`](references/report-template.md)**。

### 9.1 目录结构

```
.claude/audit-reports/
├── README.md                ← 目录用途说明
├── state.json               ← 运行时状态机
│
├── {session_id}/            ← 每次 session 一个子目录
│   ├── S1-R{N}-{timestamp}.md     ← 每轮全量轻扫报告
│   ├── R{N}-{模块名}-{timestamp}.md ← 每轮深度审计报告
│   ├── auto-fixes.log             ← 自动修复日志（追加）
│   └── FINAL-SUMMARY.md           ← Session 最终报告
│
└── archive/                 ← 历史 session 归档
```

### 9.2 三类报告

| 报告 | 频率 | 内容 |
|------|------|------|
| **S1 全量轻扫** | 每轮 | A1/A2/A3 硬错误统计 + 修复日志 |
| **R0N 深度审计** | 每轮（S2） | 目标模块 16 维全量发现 + 健康评分 |
| **FINAL-SUMMARY** | session 结束 | 全部发现汇总 + 待人工裁决清单 + 全局建议 |

### 9.3 报告文件名约定

```
S1-R{N}-{YYYYMMDD-HHmm}.md     ← 全量轻扫
R{N}-{模块名}-{YYYYMMDD-HHmm}.md ← 深度审计
FINAL-SUMMARY-{session_id}.md   ← 最终报告
auto-fixes.log                   ← 修复日志
```

---

## §10 收尾与交接

### 10.1 Session 结束条件

满足任一即结束：
- `rounds_completed >= max_rounds`（默认 10）
- 所有模块 `status == done`（没有更多模块可审）
- 连续 2 轮无新发现 且 距离上次全量轻扫 < 1 小时（世界干净）

### 10.2 用户收到的交接物

Skill 在最后一轮结束时输出：

```
🎉 Loop Audit Session 完成 — {session_id}

📊 概况
  - 完成 {n}/10 轮
  - 深度审计 {n} 个模块
  - 自动修复 {n} 个硬错误
  - 发现 {n} 个软问题（其中 {n} 个待你裁决）

📄 最终报告: .claude/audit-reports/{session_id}/FINAL-SUMMARY.md

⚠️ 待人工裁决: {n} 条 ← 你需要逐条确认

🔧 建议后续操作:
  1. 阅读 FINAL-SUMMARY.md（5 分钟）
  2. 逐条处理"待人工裁决清单"
  3. git diff 检查自动修复内容
  4. 如果自动修复涉及接头总览 → 运行 /woworldidea-design sync
  5. 确认无误后 git commit

💡 下次建议触发: {条件}（{未审模块数} 个模块待审）
```

### 10.3 用户角色

你的工作只在这三步：
1. **读** FINAL-SUMMARY.md（一页概览）
2. **裁决** 待人工清单（确认/驳回/标记后续）
3. **决定** 是否继续（`--resume` 续审未覆盖模块）

体力活（扫描、比对、修复、写报告）都由 skill 完成。

---

## §11 约束与红线

### 11.1 绝对不做的

| 红线 | 说明 |
|------|------|
| ❌ **不修改设计意图** | 不改数值、算法、设计决策、游戏机制——只修结构性错误 |
| ❌ **不删除原有设计** | 冲突时建立正确的派生/引用/映射关系，不从根上削减原有设计 |
| ❌ **不跨模块自动修改设计文档正文** | 跨模块修改只改接头总览入口/出口文件。正文修改只限本模块内 |
| ❌ **不碰 Rust+Godot 4.7 底层** | 技术栈不可改变 |
| ❌ **不翻新游戏创意** | 维护现有异世界开放世界冒险设计 |
| ❌ **不自动执行 sync/gate/audit** | 独立于 `/woworldidea-design`，不自动触发其他 skill |

### 11.2 必须做的

| 铁律 | 说明 |
|------|------|
| ✅ **引用原文** | 每条 finding 必须有源文件:行号 + 原文引用 |
| ✅ **revert 日志** | 每次自动修复有改前/改后/方法记录 |
| ✅ **uncertain 不入正文** | 存疑项进待人工裁决清单，不污染报告 |
| ✅ **不重复审计** | state.json 追踪，审计过的模块不重复 |
| ✅ **数学用 python** | 公式验证必须走 `python -c`，禁止 AI 心算 |
| ✅ **git 友好** | 自动修复尽量小范围、可独立 review |

### 11.3 必须遵守的设计原则（不可违背）

这些原则在审计 B3 维度中作为判定基准，但同时也是 skill 自身操作的红线：

1. **故事生成器非故事容器** — 不定义硬编码剧情
2. **NPC=完整的人** — 记忆/情感/思想/欲望/习惯/人际关系
3. **玩家=NPC 同套规则** — 无玩家专用硬编码
4. **涌现>门控** — 参数×概率，非 if-then 门槛
5. **种子确定性** — 所有生成可重现
6. **技能是事后记录** — 非事前门控
7. **零硬编码禁止** — 数值走 TOML
8. **玩家路径平等** — 冒险=生活=政治
9. **Rust 解耦** — trait 边界、零循环依赖、事件总线

---

## §12 故障与恢复

### 12.1 子代理失败

如果某个子代理出错或无返回：
- 主控记录该维度类别的 findings 为空
- 在报告中标注 "Agent-X 故障，维度 {dim_list} 未审计"
- 对应模块标记为 `partial`，等待下轮续审

### 12.2 state.json 损坏

- 尝试读取 → 失败 → 询问用户：恢复还是重建
- 恢复：从最近的 session 报告反向重建 state
- 重建：全量重新扫描，覆盖写入

### 12.3 磁盘空间

- 每个 session 报告目录 ~200-500KB
- 超过 20 个历史 session → 提示归档
- archive/ 下旧报告可手动压缩或删除

### 12.4 中断恢复

- `/loop` 被用户 Ctrl+C 中断 → state.json 保留最近一轮已完成状态
- 下次 `/woworld-loop-audit --resume` 从断点继续
- `auto-fixes.log` 保留已执行的修复，不会重复修复
