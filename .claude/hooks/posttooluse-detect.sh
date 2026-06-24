#!/usr/bin/env bash
# PostToolUse Hook: 检测开发阶段/文件变更
# 当 Write/Edit 工具修改了开发阶段/下的文件时，写入标记文件
# 用于 Stop hook 和 /woworldidea-design sync 的改动识别

set -euo pipefail

# 读取 hook 输入
HOOK_INPUT=$(cat)

# 提取工具名和文件路径
TOOL_NAME=$(echo "$HOOK_INPUT" | jq -r '.tool_name // ""')
FILE_PATH=$(echo "$HOOK_INPUT" | jq -r '.tool_input.file_path // ""')

# 只处理 Write 和 Edit 工具
if [[ "$TOOL_NAME" != "Write" ]] && [[ "$TOOL_NAME" != "Edit" ]]; then
  exit 0
fi

# 检查文件路径是否在 开发阶段/ 下
# 支持 Windows 反斜杠和 Unix 正斜杠
if ! echo "$FILE_PATH" | grep -qE '开发阶段[/\\]'; then
  exit 0
fi

# 标记文件路径 —— 从脚本位置推导项目根目录（与 stop-remind.sh 对齐）
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
MARKER_DIR="$PROJECT_ROOT/.claude/temp"
mkdir -p "$MARKER_DIR"
MARKER_FILE="$MARKER_DIR/modified_files.txt"

# 从 hook 输入中提取 session_id（可选，用于隔离）
SESSION_ID=$(echo "$HOOK_INPUT" | jq -r '.session_id // "unknown"')

# 去重：检查该文件路径是否已在标记文件中
if [[ -f "$MARKER_FILE" ]]; then
  if grep -qF "$FILE_PATH" "$MARKER_FILE" 2>/dev/null; then
    # 已存在，不重复添加
    exit 0
  fi
fi

# 追加文件路径 + session_id + 时间戳
echo "$FILE_PATH|$SESSION_ID|$(date +%s)" >> "$MARKER_FILE"

exit 0
