---
id: TOOL-002
title: .canvas 文件被 LLM 文本编辑损坏
type: 🟡反直觉陷阱
module: 工具链
status: ✅已修复
confidence: ✅确信
discovered: 2026-06-24
resolved: 2026-06-24
last_verified: 2026-07-07
grep_keys: [canvas, obsidian, 二进制, 损坏, corrupt, json, 文本编辑, 画布, Edit, Write, 文件损坏, 白板]
env:
  godot: "无关"
  renderer: "无关"
  os: [Windows]
  gpu: "无关"
relations: []
---

## 症状识别
- Obsidian 画布（.canvas）文件用 Edit 或 Write 工具编辑后**无法正常打开**
- Obsidian 中画布显示为空白或报 JSON 解析错误
- 损坏前一刻的操作：LLM 对 .canvas 文件执行了文本编辑

## 误诊路径
| 尝试过的方案 | 结果 | 为什么无效 |
|-------------|------|-----------|
| 修复 JSON 语法 | 可能无效 | .canvas 不是纯 JSON——包含特殊二进制编码 |
| 重新创建画布 | 有效但丢失内容 | 如果能从 git 恢复则不需要 |

## 根因
**Obsidian .canvas 文件是二进制 JSON 格式**——不是纯文本 JSON。用 Edit/Write 等文本工具编辑会破坏二进制编码，导致 Obsidian 无法解析。LLM 没有"这是二进制文件"的感知——看到 `.json` 扩展名（.canvas 内部是 JSON 结构）就会用文本工具操作。

## 解决方案
- **CLAUDE.md 中加硬性规则**："`.canvas` 文件 — ⚠️ 禁止文本编辑。Obsidian 画布文件是二进制 JSON——用 Edit/Write 工具编辑会损坏文件。只能通过 Obsidian 修改。"
- LLM 需要 Glob/Grep 时 `**/*.canvas` 结果只读不写
- 如果 .canvas 已损坏：`git checkout -- <file>` 从 git 恢复

## 验证方法
1. 在 Obsidian 中打开 .canvas 文件 → 画布内容正常显示
2. 不要用 Edit/Write 工具操作 .canvas 文件

## 代码位置
- 不适用（非代码 bug，是工作流程陷阱）

## 关联 Bug
无

## 复发记录
| 日期 | 会话 | 症状是否相同 | 原方案是否有效 | 备注 |
|------|------|-------------|---------------|------|
| 2026-06-25 | sprint-proposals 创建时 | 是——LLM 尝试编辑 .canvas sprint 模板 | git checkout 恢复 | 此后规则生效，未再复发 |
