# ONBOARDING.md — 开发者认知接入

> 5 分钟了解 WoWorld 是什么、仓库怎么组织、怎么开始工作。

---

## 项目鸟瞰（5 句话）

1. **WoWorld = 故事生成器**——NPC 互动以概率统计自然涌现故事，每个存档是完全不同的世界。没有"通关"。
2. **玩家 = NPC 夺舍**——两者用同一套底层代码。法律、名声从原子系统涌现，不建独立模块。
3. **25 万 km² 体素世界**（≈ 英国面积，海:陆≈7:3）——程序化生成地形、生态、文明、历史。
4. **~25 个独立设计模块**（107,000 行中文文档）→ 正在向 Rust 代码过渡。
5. **Godot 4.7（渲染）+ Rust（模拟核心）**——通过 GDExtension 桥接。无 Bevy。

---

## 仓库地图

```
<repo-root>/
├── woworld-dev-plan/              ← [开发治理] 宪法 + 状态追踪 + 冲刺系统
│   ├── CONSTITUTION.md            ← 元规则层——怎么判断下一步做什么
│   ├── DEVELOPMENT_STATUS.md      ← 全局状态——每模块准入等级 + 当前 WIP
│   ├── GLOSSARY.md                ← 中英术语映射登记册
│   ├── DEPENDENCY_GRAPH.md        ← 模块实现依赖图（层 0→1→2→…）
│   ├── ARCHITECTURE_DECISIONS.md  ← 架构决策记录（ADR 日志）
│   ├── PERFORMANCE_BUDGET.md      ← GPU/CPU/VRAM 性能预算表
│   ├── ONBOARDING.md              ← 本文件
│   ├── sprint-proposals/          ← 冲刺提案归档
│   └── handoff/                   ← 交接摘要归档
├── WoWorld-Design/                ← [设计文档] Obsidian 编辑，中文
│   ├── Happy Game/开发阶段/       ← 权威设计规格（~25 模块）
│   │   └── <模块名>/README.md     ← 模块根文档（≥60 行约定）
│   ├── 开发路线图/                ← 四轨路线图（被宪法包裹）
│   └── Change/                    ← 设计变更追踪（CHG-001 至 CHG-060）
├── woworld/                       ← [Rust workspace] 游戏代码
│   ├── crates/woworld_core/       ← 零依赖——ID 类型 + trait 签名
│   ├── crates/woworld_godot/      ← GDExtension 桥接（cdylib）
│   ├── godot/                     ← Godot 4.7 项目（project.godot + .gdextension）
│   └── assets/                    ← TOML 数据文件（配方表、群系参数等）
├── CLAUDE.md                      ← Claude Code 入口指令
└── CLAUDE-INTERFACES.md           ← 跨模块接口契约完整参考（27 个契约段）
```

---

## 代码架构速览

```
TOML 数据文件 → woworld_core（加载 & 验证）→ woworld_godot（序列化桥接）→ Godot 场景树
```

- **`woworld_core`** — 零依赖。所有 ID 类型、空间查询 trait、共享数据结构。引擎无关。
- **`woworld_godot`** — 薄桥接层。Rust 类型 ↔ Godot GDExtension API。不含游戏逻辑。
- **Godot 项目** — 纯表现层。渲染、UI、音频、输入、玩家物理（仅玩家保留 PhysicsServer3D）。
- **数据流**：Rust 模拟核心输出 → GDExtension 桥 → Godot 节点更新（ArrayMesh、Skeleton3D 等）。

---

## 开发工作流（冲刺循环）

```
Claude 自主提出冲刺提案（宪法 §8 格式）
  → 用户审批
  → Claude 编码（贴设计文档 + 五层防御自检）
  → 用户审核代码 + 运行行为
  → 交接摘要（宪法 §10 格式）
  → 下一个冲刺
```

**关键规则**：
- Claude 不替设计做决定（遇到问题举手，不等不理）
- 用户手动控制 Git 节奏（Claude 不自主 merge/push）
- 模块准入三级制：🔴 冻结 / 🟡 就绪 / 🟢 稳定

---

## 关键约定

| 约定 | 位置 |
|------|------|
| 所有 ID 类型定义在 `woworld_core` | CLAUDE-INTERFACES.md |
| 空间查询走 Rust 四 trait（仅玩家用 Godot 物理） | CLAUDE-INTERFACES.md |
| 持久化：`SaveableModule::snapshot_dirty() → SaveSystem → LMDB` | CLAUDE-INTERFACES.md |
| 确定性：无 `HashMap` 默认哈希 / 无 `SystemTime` / 无非确定归约 | CONSTITUTION.md §4 |
| 错误哲学：Fatal → `panic!` / Recoverable → `Result` + 降级 / Expected → 控制流 | CONSTITUTION.md §4 |
| 所有公开枚举实现 `strum::EnumIter` + `EnumString` | CONSTITUTION.md 附录A L7 |
| Commit 格式：`<类型>(<模块>): <描述> — Sprint-NNN` | CONSTITUTION.md §6 |
| 五层防御自检 SOP | CONSTITUTION.md §4 |

---

## 快速启动

```bash
# 克隆
git clone git@github.com:QiaoWhite/WoWorld-Design.git
cd WoWorld-Design  # 仓库根目录

# 编译
cd woworld
cargo build --workspace

# 启动 Godot 编辑器
../tools/godot/Godot_v4.7-stable_win64.exe godot/project.godot

# 你应该看到：一个由 Rust 构造的 MeshInstance3D 出现在 Godot 视口中
```
