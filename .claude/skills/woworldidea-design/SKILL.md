---
name: woworldidea-design
description: 设计文档修改同步引擎——改后自动同步所有关联文件的体力活。不影响设计理念，只做机械同步。// When user modifies design docs under 开发阶段/ — use sync to update all related interface files; use impact for pre-change preview; use gate for new module scaffolding; use audit to fix stale files.
---

# /woworldidea-design — 设计文档同步引擎

**定位**: 铁匠学徒——你在本模块自由实现设计想法，skill 接管"同步所有关联文档"的体力活。不改设计理念，只做机械同步。

**核心原则**:
- 不改原设计理念——只同步类型签名、字段名、数值范围。因果推理词开头的句子只标注不动
- 不引入新持久化索引——所有分析当场从已有文件聚合，用完即弃
- 被动+手动分档——本模块管理文件被动改，跨模块文档递纸条等你确认

## 命令入口

| 命令 | 用法 | 什么时候用 |
|------|------|-----------|
| **sync** ★主力 | `/woworldidea-design sync [--dry-run]` | 你在本模块自由改完了设计 → skill 自动识别改动 → 同步所有关联文件 → 发现并修冲突 |
| **impact** | `/woworldidea-design impact <设计意图/概念名>` | 改前想看一眼大概影响面，只报数量级不做详细比对 |
| **gate** | `/woworldidea-design gate <模块名> [--level 1\|2]` | 新模块草稿写完 → L1 内部检查 → L2 外部对接 |
| **audit** | `/woworldidea-design audit <文件路径>` | 发现旧文件和当前系统脱节 → 逐条比对 → 报告过时内容 |

**全局选项**: `--dry-run` — 只预览，不改任何文件。

## 工作流模型

```
[可选] impact → 心里有数 → 在本模块不管不顾自由改 → sync 收场 → commit
                                  ↑
                          hook 持续追踪，改中提醒
```

## 铁匠学徒三层权限

详见 [[references/apprentice-rules.md]]。核心边界：

| 被动做（本模块） | 递纸条（跨模块） | 绝不碰 |
|-----------------|-----------------|--------|
| 接头总览4文件 + 变更追踪队列 + 变更日志 + CLAUDE.md + CHG初稿 | 消费方模块文档 + CLAUDE-INTERFACES契约表 | 消费方设计哲学段落（"因为/所以/设计为/选择"起头的句子） |

## 三级依赖分级

| 级别 | 判断标准 | 示例 |
|------|---------|------|
| **Hard** | 消费方代码/数据模型中直接使用了此类型/字段/常量 | Combat 读 Vitals.hp 做伤害计算 |
| **Soft** | 消费方文档提及此概念作为上下文，但不直接依赖具体定义 | 经济系统说"NPC健康影响生产力"但不引用 struct |
| **Ref** | 仅 wikilink 引用，无实质依赖 | `[[Vitals定义]]` 但无使用 |

全部级别都修，按 Hard → Soft → Ref 排序输出。

## 执行方式

sync 的冲突扫描阶段使用**子代理并行**——每个消费方一个独立子代理，上下文隔离，只返回结论+原文证据。子代理失败降级为主代理串行（标注"退路模式"）。

其他命令（impact/gate/audit）在主代理内执行。

## 质量保障

1. sync 第一步自动 `git stash` 保存安全点
2. 子代理必须输出原文引用，不允许只出"一致/不一致"结论
3. 只追踪最后一次 sync/impact/gate 之后的改动，防 transcript 污染
4. 每处建议标"纯技术同步"或"可能涉及设计意图"置信度
5. Hook 同文件去重 + 同批改动只提醒一次
6. sync 随机抽查消费方验证接头总览准确度
7. 模块匹配用目录名而非编号前缀

## 详细流程

各命令的完整执行流程见 references/ 目录：

- `references/sync.md` — sync 从 S0 到 S12 完整流程
- `references/impact.md` — impact 轻量预览流程
- `references/gate.md` — gate L1/L2 流程
- `references/audit.md` — audit 逐条比对流程
- `references/apprentice-rules.md` — 铁匠学徒规则详解 + Hard/Soft/Ref 分级标准

当用户调用具体命令时，**先读本文件的对应段落了解入口行为，再读 references/ 中的对应文件获取完整执行流程。**
