---
id: {MODULE}-{NNN}
title: <一行症状描述>
type: 🔴回归 | 🟠架构限制 | 🟡反直觉陷阱 | 🔵性能 | 🟢设计债务
module: <模块名>
status: ✅已修复 | ⚠️已知残留 | 🚫不可修复
confidence: ✅确信 | ⚠️工作绕过 | 🤔推测
discovered: YYYY-MM-DD
resolved: YYYY-MM-DD
last_verified: YYYY-MM-DD
grep_keys: [关键词1, 关键词2, 中英文同义词, 缩写]
env:
  godot: "4.7-stable"
  renderer: "Forward+"
  os: [Windows]
  gpu: "<如有关>"
relations:
  - {target: XXX-NNN, type: 引发 | 同根 | 症状相似 | 替代}
---

## 症状识别
- <肉眼/日志看到的特征>
- <触发条件：什么操作/什么场景/什么环境>

## 误诊路径
| 尝试过的方案 | 结果 | 为什么无效 |
|-------------|------|-----------|
| … | … | … |

## 根因
<简洁——为什么发生>

## 解决方案
<具体步骤/代码改动模式——不写 commit hash>

## 验证方法
<一行具体操作：测试命令 / 视觉检查 / 日志确认>

## 代码位置
- `crate::module::function_name` — <说明>
- 文件: `路径/文件.rs`

## 关联 Bug
- [[XXX-NNN]] — 关系：<引发/同根/症状相似/替代>

## 复发记录
| 日期 | 会话 | 症状是否相同 | 原方案是否有效 | 备注 |
|------|------|-------------|---------------|------|
| — | — | — | — | — |
