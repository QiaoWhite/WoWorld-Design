#!/usr/bin/env bash
# Stop Hook: 回合结束时提醒用户同步设计文档
# 检查是否存在待同步的标记文件，如果有则注入 systemMessage

set -euo pipefail

# 读取 hook 输入
HOOK_INPUT=$(cat)

# 标记文件路径 —— 从脚本位置推导项目根目录（与 posttooluse-detect.sh 保持一致）
# $(dirname "$0") = .claude/hooks/ → ../.claude/temp = .claude/temp/
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
MARKER_FILE="$PROJECT_ROOT/.claude/temp/modified_files.txt"

# 备用: 如果项目根路径推导失败，尝试从 hook 输入的 cwd 获取
if [[ ! -d "$(dirname "$MARKER_FILE")" ]]; then
  FALLBACK_CWD=$(echo "$HOOK_INPUT" | jq -r '.cwd // ""')
  if [[ -n "$FALLBACK_CWD" ]]; then
    MARKER_FILE="$FALLBACK_CWD/.claude/temp/modified_files.txt"
  fi
fi

# 检查标记文件是否存在且非空
if [[ ! -f "$MARKER_FILE" ]] || [[ ! -s "$MARKER_FILE" ]]; then
  exit 0
fi

# 检查时间戳——只提醒30分钟内的改动（防骚扰）
CURRENT_TIME=$(date +%s)
RECENT=false
while IFS='|' read -r file_path session_id timestamp; do
  if [[ -n "$timestamp" ]] && [[ "$timestamp" =~ ^[0-9]+$ ]]; then
    AGE=$((CURRENT_TIME - timestamp))
    if [[ $AGE -lt 1800 ]]; then  # 30分钟 = 1800秒
      RECENT=true
      break
    fi
  fi
done < "$MARKER_FILE"

# 如果30分钟内没有改动，清理标记文件（过期标记）
if [[ "$RECENT" != "true" ]]; then
  rm -f "$MARKER_FILE"
  exit 0
fi

# 统计修改文件数量
FILE_COUNT=$(wc -l < "$MARKER_FILE" | tr -d ' ')

# 注入提醒
SYSTEM_MSG="⚠️ 检测到本轮修改了 ${FILE_COUNT} 个开发阶段/下的设计文档。建议运行 /woworldidea-design sync 进行同步检查，确保所有关联文档已更新。"

# 输出 JSON —— systemMessage 将在下一轮对话中注入
jq -n --arg msg "$SYSTEM_MSG" '{
  "systemMessage": $msg,
  "decision": "approve"
}'

exit 0
