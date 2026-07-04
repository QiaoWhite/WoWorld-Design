# WoWorld 开发工作流程 v1.5

> ⚠️ **已迁移** (2026-07-04): 本文件内容已拆解吸收到新的六阶段开发流程体系中。
> - 流程全景 → [`woworld-dev-plan/00-流程总览.md`](woworld-dev-plan/00-流程总览.md)
> - 文件速查 → [`woworld-dev-plan/README.md`](woworld-dev-plan/README.md)
> - AI 标准作业流程 → [`woworld-dev-plan/00-流程总览.md` §三](woworld-dev-plan/00-流程总览.md)
> - 本文件保留作为历史参考，不再作为 AI 启动入口。
>
> **宪法版本**: v1.5（2026-06-25 用户审批生效）
> **定位**: 流程全景图——所有规则分散在具体文件中，本文件是索引和速查。

---

## 治理文件地图

```
woworld-dev-plan/
├── CONSTITUTION.md          ← 游戏规则（怎么判断下一步做什么）
├── 00-流程总览.md            ← 六阶段全景 + 9 SOP
├── DEVELOPMENT_STATUS.md    ← 当前位置（每个模块 🔴🟡🟢 + WIP）
├── DEPENDENCY_GRAPH.md      ← 地图（层 0→1→2→3→4，先做什么后做什么）
├── ARCHITECTURE_DECISIONS.md ← 为什么选了 A 不选 B（4 条 ADR）
├── 01-核心基础/              ← Phase 1 执行手册 + 9 里程碑 + handoff + devlogs
├── 02-垂直切片/ → 06-持续运营/ ← Phase 2-6 骨架
├── sprint-proposals/        ← 下一站（§8 格式冲刺提案）
└── work-logs/               ← 踩坑记录（按需）
```

---

## 一、新会话启动

```
新 Claude 实例启动
      │
      ├─ 1. 读 CLAUDE.md（项目大局观 + 工作约定）
      ├─ 2. 读 CLAUDE-INTERFACES.md（跨模块契约）
      ├─ 3. 读 woworld-dev-plan/01-核心基础/handoff/ 最新交接摘要
      │
      └─ 4. 激活协议（宪法 §10）:
           ├─ 读 WIP 代码 → 对照交接摘要
           ├─ 对比 DEVELOPMENT_STATUS.md → 查漂移
           └─ 产出: 冲刺提案 或 纠偏报告
```

---

## 二、冲刺生命周期

```
┌─────────────────────────────────────────────────────┐
│                   冲刺生命周期                        │
│                                                      │
│  Claude 写提案 ──→ 用户审批 ──→ Claude 执行        │
│   (§8 格式)        (口头/编辑)    │                  │
│                                   ↓                  │
│                            ┌──────────────┐         │
│                            │ 编码 + 自检   │         │
│                            │ (§4 三层质量门)│         │
│                            └──────┬───────┘         │
│                                   ↓                  │
│   git commit ←── 通过 ←── Claude 自检              │
│   <type>: <描述>           │                         │
│                            │ 不通过 → 修 → 重检     │
│                            │ 3次不过 → 举手         │
│                            │ (§9 失败回滚)           │
│                                   ↓                  │
│   Claude 写交接摘要 ──→ 循环: 下一冲刺提案          │
│   (§10 格式)                                         │
└─────────────────────────────────────────────────────┘
```

**冲刺提案**（宪法 §8）：依赖前提检查 → 全局快照 → ≤3 目标 → 必读文档清单 → 外部 API 预验证 → 决策矩阵 → 需裁决问题 → 预计影响

**交接摘要**（宪法 §10）：冲刺目标回顾 → 当前 WIP 精确状态 → 下一步候选（含决策矩阵） → 已知问题 → 关键设计决策记录

---

## 三、编码执行（冲刺内）

### 质量门（宪法 §4）

```
层 0: 理解声明（编码前）
      └─ "本冲刺要创建的类型/算法/约束 + 不确定的地方"

层 1: 反翻译并排对比（编码后）
      └─ 从代码提取行为描述 vs 原设计文档 → 逐条对照

层 2: 约束检查清单勾销（编码后）
      └─ 设计文档中每个布尔约束 → 逐条打勾

层 3: 对抗性破坏提示（编码后）
      └─ "假设这段代码有至少一个严重 bug，找出 3 个"

层 4: 跨文档追溯矩阵（commit 前）
      └─ 每个概念: 定义者 → 契约 → 各消费方 → 标记一致/不一致
```

### 架构边界合规审计（宪法 §14）

```
层一：双权威检测
  □ GDScript 中无独立维护的时间/颜色/模拟参数
  □ GDScript 中无 sin/cos 用于游戏逻辑计算

层二：僵尸代码检测
  □ 所有 #[func] 方法在 GDScript 有对应调用
  □ 所有 pub 函数有消费方

层三：边界穿越审计
  □ GDScript 每行归类三种: 读Input / 读Rust #[func] / 设Godot节点属性
  □ GDScript 中无数学公式（sin/cos/lerp/clamp）用于游戏逻辑
  □ GDScript 中无条件分支基于游戏状态

检出物: 每项偏离 → 记录交接摘要 + 标注严重级别 + 创建修复任务
例外: 原型豁免需标注 ## ARCH-DEVIATION: <原因> <到期条件>
```

### Commit 前检查清单

```
□ cargo build --workspace 通过
□ cargo clippy -- -W clippy::all -W clippy::pedantic 零警告
□ cargo test 全部通过
□ cargo fmt --check 通过
□ 附录A Rust 病检查清单通过（12类 65 项）
□ CLAUDE-INTERFACES.md 接口签名核验
□ 开发阶段/<模块>/ 设计文档核验（五层防御 SOP 完整执行）
□ 外部 API 已验证（GDScript 侧必须 WebFetch 查 Godot 4.7 文档）
□ 确定性: 无 HashMap 默认哈希 / SystemTime::now / rayon 非确定归约
□ 新增公开类型登记到 GLOSSARY.md（如有）
□ 架构决策记录到 ARCHITECTURE_DECISIONS.md（如有）
□ §14 架构边界合规审计三层扫描通过

全部打勾 → git commit -m "<type>: <中文描述>"
```

### 错误处理（宪法 §4 三级策略）

| 级别 | 策略 | 适用 |
|------|------|------|
| Fatal | `panic!` / `expect()` | 数据完整性、硬件不可用、世界生成逻辑矛盾 |
| Recoverable | `Result<T, E>` + 降级 | 资源缺失、网络数据校验失败 |
| Expected | `Option` 或默认行为 | 寻路失败、NPC 行为冲突——正常游戏状态 |

---

## 四、用户反馈处理（分诊协议）

```
用户报告问题
      │
      ▼
Claude 分诊（先分类，再行动）
      │
      ├─ 🔴 回归 ──→ 立刻修
      │   (之前工作、现在坏了。测试失败/编译错误/crash)
      │
      ├─ 🟡 已知偏离 ──→ 引用追踪ID，不重复调查
      │   (已在 DEVELOPMENT_STATUS 或冲刺提案中追踪)
      │
      ├─ 🟢 参数调优 ──→ 附成本估算，用户决定现在修或延后
      │   (视觉效果不满意，非正确性错误，根因不依赖未完成重构)
      │
      └─ 🔵 结构性依赖 ──→ 不现在修，标注 Sprint-N 会解决
          (根因依赖未来冲刺的重构/新模块)
```

**回复格式**：
```
> **分诊**: [🔴🟡🟢🔵] — [一句话判定]
> **如果现在修**: 影响 ~N 文件, 预计 ~X-Y 分钟, 阻塞风险: [无 / 依赖 Sprint-N]
> **建议**: [现在修 / 记入已知问题 / 等待 Sprint-N 自然解决]
```

用户有权在任何分诊后说"不，现在修"——分诊是建议，不是否决。

---

## 五、Phase 与 Sprint 的关系

Phase 是**回顾性**概念——当 DEPENDENCY_GRAPH 中一层所有模块达到 🟢 时自然标记完成。不预先规划时间线。

```
DEPENDENCY_GRAPH 层 0: woworld_core + woworld_spatial
  ├─ Sprint-002~005: 核心类型 + 空间索引 + 世界生成 + 大气 + Godot
  └─ ✅ 完成

DEPENDENCY_GRAPH 层 1: 世界生成 + 存档 + 物品 + 技能 + 生命 + 天气
  ├─ 世界生成: 🟡 部分（5 红色偏离待修）
  ├─ 大气氛围: 🟡 部分（3/4 调制层存根）
  ├─ 其余: — 未开始
  └─ Sprint-006: 🔴1 DensityField trait + 🔴2 Seed u64（待用户审批启动）

层 2-4: ⏳ 阻塞于层 1 完成
```

---

## 六、文件角色速查

| 我想… | 去… |
|------|------|
| 知道现在该做什么 | `DEVELOPMENT_STATUS.md` 第一段 + 当前冲刺段 |
| 知道下一步候选 | 最新交接摘要的决策矩阵 |
| 知道先做什么后做什么 | `DEPENDENCY_GRAPH.md` 层图 |
| 知道为什么选了 A 不选 B | `ARCHITECTURE_DECISIONS.md` |
| 知道怎么判断/审批/提交 | `CONSTITUTION.md` |
| 知道跨模块契约 | `CLAUDE-INTERFACES.md` |
| 知道项目大局观 + 工作约定 | `CLAUDE.md` |
| 启动新会话 | 最新 `01-核心基础/handoff/` 文件 |
| 报告问题 | 分诊协议（`CLAUDE.md` §🏥 或本文件 §四） |
| 看流程全景图 | 本文件 |

---

> **关联**: [CLAUDE.md](CLAUDE.md) · [CLAUDE-INTERFACES.md](CLAUDE-INTERFACES.md) · [CONSTITUTION](woworld-dev-plan/CONSTITUTION.md) · [DEVELOPMENT_STATUS](woworld-dev-plan/DEVELOPMENT_STATUS.md) · [DEPENDENCY_GRAPH](woworld-dev-plan/DEPENDENCY_GRAPH.md)
