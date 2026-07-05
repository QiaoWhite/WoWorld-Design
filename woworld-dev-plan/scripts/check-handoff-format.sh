#!/usr/bin/env bash
# check-handoff-format.sh — Handoff 必填节完整性检查
# 用途: 冲刺结束时运行，确保 Handoff 包含所有必填节
# 返回: 0 = 完整, 非0 = 缺节

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
HANDOFF_DIR="$REPO_ROOT/woworld-dev-plan/01-核心基础/handoff"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

# 必填节（不含 ## 前缀，匹配时加 ##）
REQUIRED_SECTIONS=(
    "💾 恢复点"
    "🔧 机械门验证"
    "📐 设计门验证"
    "🔍 一致性验证"
)

# 找最新 handoff 文件
LATEST_HANDOFF=$(ls -t "$HANDOFF_DIR"/handoff-*.md 2>/dev/null | head -1)

if [ -z "$LATEST_HANDOFF" ]; then
    echo -e "${RED}❌ 未找到 handoff 文件${NC}"
    exit 1
fi

echo "## 📋 Handoff 格式检查"
echo ""
echo "**文件**: $(basename "$LATEST_HANDOFF")"
echo ""

MISSING=0
for section in "${REQUIRED_SECTIONS[@]}"; do
    if grep -q "^## $section" "$LATEST_HANDOFF"; then
        echo "| $section | ✅ |"
    else
        echo "| $section | ❌ 缺失 |"
        MISSING=1
    fi
done

echo ""
if [ $MISSING -eq 0 ]; then
    echo -e "${GREEN}✅ Handoff 格式完整——所有必填节就位${NC}"
    exit 0
else
    echo -e "${RED}❌ Handoff 格式不完整——缺失 $MISSING 个必填节。冲刺未完成。${NC}"
    echo -e "${YELLOW}💡 旧格式 handoff 兼容: 如缺失「💾 恢复点」，场景 D 将回退到「代码状态」+「启动指令」节${NC}"
    exit 1
fi
