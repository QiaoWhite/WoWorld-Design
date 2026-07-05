#!/usr/bin/env bash
# verify-consistency.sh — 文档与代码一致性验证
# 用途: 冲刺结束 / 阶段过渡时运行，检测数字漂移
# 返回: 0 = 一致, 非0 = 发现漂移

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
WOWORLD_DIR="$REPO_ROOT/woworld"
DEVPLAN_DIR="$REPO_ROOT/woworld-dev-plan"

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'
DRIFT=0

echo "## 🔍 一致性验证"
echo ""
echo "| 检查项 | 代码实际 | 文档声称 | 来源 | 状态 |"
echo "|--------|---------|---------|------|------|"

# ── 1. 提取 per-crate 测试计数 ──
# Order is deterministic (workspace Cargo.toml member order):
#   atmosphere → core → godot → worldgen → doc-tests
cd "$WOWORLD_DIR"
TEST_COUNTS=$(cargo test --workspace 2>&1 | grep 'test result: ok' | grep -o '[0-9]\+ passed' | grep -o '[0-9]\+' | head -4)

ATMOS_TESTS=$(echo "$TEST_COUNTS" | sed -n '1p')
CORE_TESTS=$(echo "$TEST_COUNTS" | sed -n '2p')
GODOT_TESTS=$(echo "$TEST_COUNTS" | sed -n '3p')
WORLDGEN_TESTS=$(echo "$TEST_COUNTS" | sed -n '4p')

ATMOS_TESTS=${ATMOS_TESTS:-0}
CORE_TESTS=${CORE_TESTS:-0}
GODOT_TESTS=${GODOT_TESTS:-0}
WORLDGEN_TESTS=${WORLDGEN_TESTS:-0}

TOTAL=$((CORE_TESTS + WORLDGEN_TESTS + ATMOS_TESTS + GODOT_TESTS))

# ── 2. 检查 附录E ──
APPENDIXE="$DEVPLAN_DIR/附录E-开发状态.md"
if [ -f "$APPENDIXE" ]; then
    DOC_TOTAL=$(grep -o '\*\*[0-9]\{2,4\}\s*tests\?\s*全绿\*\*' "$APPENDIXE" | grep -o '[0-9]\{2,4\}' | head -1 || echo "0")
    if [ "$DOC_TOTAL" != "0" ]; then
        if [ "$TOTAL" != "$DOC_TOTAL" ]; then
            echo "| 总测试数 | $TOTAL | $DOC_TOTAL | 附录E-开发状态.md | ❌ 漂移 |"
            DRIFT=1
        else
            echo "| 总测试数 | $TOTAL | $DOC_TOTAL | 附录E-开发状态.md | ✅ |"
        fi
    else
        echo "| 总测试数 | $TOTAL | (未找到) | 附录E-开发状态.md | ⚠️ |"
    fi
fi

# ── 3. 检查 CLAUDE.md ──
CLAUDE_MD="$REPO_ROOT/CLAUDE.md"
if [ -f "$CLAUDE_MD" ]; then
    CLAUDE_TOTAL=$(grep -o '\*\*[0-9]\{2,4\}\s*个测试全部通过\*\*' "$CLAUDE_MD" | grep -o '[0-9]\{2,4\}' | head -1 || echo "0")
    if [ "$CLAUDE_TOTAL" != "0" ] && [ "$TOTAL" != "0" ]; then
        if [ "$TOTAL" != "$CLAUDE_TOTAL" ]; then
            echo "| 总测试数 (CLAUDE.md) | $TOTAL | $CLAUDE_TOTAL | CLAUDE.md | ❌ 漂移 |"
            DRIFT=1
        else
            echo "| 总测试数 (CLAUDE.md) | $TOTAL | $CLAUDE_TOTAL | CLAUDE.md | ✅ |"
        fi
    fi
fi

# ── 4. 检查 1A ──
FILE_1A="$DEVPLAN_DIR/01-核心基础/1A-Layer0核心类型.md"
if [ -f "$FILE_1A" ]; then
    DOC_1A=$(grep -o '[0-9]\{2,4\}\s*tests\?\s*全绿' "$FILE_1A" | grep -o '[0-9]\{2,4\}' | head -1 || echo "0")
    if [ "$DOC_1A" != "0" ] && [ "$CORE_TESTS" != "0" ]; then
        if [ "$CORE_TESTS" != "$DOC_1A" ]; then
            echo "| woworld_core (1A) | $CORE_TESTS | $DOC_1A | 1A-Layer0核心类型.md | ❌ 漂移 |"
            DRIFT=1
        else
            echo "| woworld_core (1A) | $CORE_TESTS | $DOC_1A | 1A-Layer0核心类型.md | ✅ |"
        fi
    fi
fi

# ── 5. 检查 1B ──
FILE_1B="$DEVPLAN_DIR/01-核心基础/1B-世界生成.md"
if [ -f "$FILE_1B" ]; then
    DOC_1B=$(grep -o '[0-9]\{2,4\}\s*tests\?\s*全绿' "$FILE_1B" | grep -o '[0-9]\{2,4\}' | head -1 || echo "0")
    if [ "$DOC_1B" != "0" ] && [ "$WORLDGEN_TESTS" != "0" ]; then
        if [ "$WORLDGEN_TESTS" != "$DOC_1B" ]; then
            echo "| woworld_worldgen (1B) | $WORLDGEN_TESTS | $DOC_1B | 1B-世界生成.md | ❌ 漂移 |"
            DRIFT=1
        else
            echo "| woworld_worldgen (1B) | $WORLDGEN_TESTS | $DOC_1B | 1B-世界生成.md | ✅ |"
        fi
    fi
fi

# ── 6. 明细 ──
echo "| per-crate | core=$CORE_TESTS worldgen=$WORLDGEN_TESTS atmos=$ATMOS_TESTS godot=$GODOT_TESTS | total=$TOTAL | — | — |"

echo ""
if [ $DRIFT -eq 0 ]; then
    echo -e "${GREEN}✅ 一致性验证通过——文档与代码一致${NC}"
else
    echo -e "${RED}❌ 一致性验证失败——发现文档漂移，请修复后重新运行${NC}"
fi

exit $DRIFT
