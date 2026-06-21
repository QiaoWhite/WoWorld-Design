# 存档系统——正式开发规格

> **版本**: v2.0（经 CHG-056 深度审计修正）
> **日期**: 2026-06-21
> **状态**: 开发规格
> **定位**: WoWorld 存档系统的权威实现规格。存档系统是"世界的快照相机"——不参与游戏模拟，只负责状态的持久化与恢复。
> **关联**: [[../技术栈方案/001-WoWorld正式技术栈方案v3|技术栈方案 v4.0]] · [[../../../参考文档/041-存档系统设计大纲-20260621/README|041 设计大纲]]

---

## 定位

存档系统是 WoWorld **世界状态的持久化层**。它不参与游戏模拟，不定义任何业务逻辑——只负责将内存中的世界状态保存到磁盘，并在需要时恢复。

**核心原则**：
- **存档系统不拥有任何模拟数据**——模块数据归各自模块所有。存档系统只提供 trait 契约和 IO 流程
- **一个存档 = 一个 .woworld 文件**——单文件 LMDB，内含多个 named_db。自包含，可复制传输
- **全量快照 + 脏数据增量**——不是"种子零存储"。初始状态全量存，后续只写脏数据
- **种子从空间优化重新定位为容错手段**——存档内包含完整数据，种子在损坏时可重建种子可重现层
- **模块自治老化策略**——不同数据有不同的自然寿命（NPC 记忆遗忘 ≠ 建筑结构老化）。存档系统不强制统一
- **零模块耦合**——SaveSystem 只知道 `SaveableModule` trait。不知道任何模块的具体类型

---

## 文档

| 编号 | 文档 | 内容 | 状态 |
|------|------|------|------|
| **001** | [存档系统总纲](001-存档系统总纲.md) | 根定义、策略分层（6 种数据类型 × 5 种存储策略）、存档槽设计（手动无上限+自动轮转+快存+退出+初始）、存档时机（手动/自动/快速/退出）、玩家死亡与继承（PendingInheritance + 灵魂转世）、游戏世界管理（目录扫描+world_meta.json+UUID去重）、SaveQueue调度（保留低优先级意图） | ✅ v2.0 |
| **002** | [存档格式与文件架构](002-存档格式与文件架构.md) | SaveHeader（magic·UUID·种子·版本·module_versions·mod_manifest·WorldSummary·WorldSnapshot）、named_db 键空间约定（`<module>/<entity>/<id>`）、LMDB mapsize 管理、磁盘布局（`saves/<世界目录>/saves/<存档>.woworld`）、原子写入协议、全遍历写入后验证 | ✅ v2.0 |
| **003** | [保存流程与模块接口](003-保存流程与模块接口.md) | **SaveableModule trait v2.0（14 方法·4 必覆·10 默认）**、LoadContext 渐进加载上下文、DirtySnapshot（NamedDbSnapshot·snapshot_tick·deleted）、三阶段保存（快照·写入·确认）+ Initial 存档直写路径、dirty_since 竞态安全、防重入、请求队列（Manual>Death>Quick>Auto 优先级+调度补执行）、HeaderBuilder、named_dbs 键前缀冲突检测 | ✅ v2.0 |
| **004** | [读档流程与崩溃恢复](004-读档流程与崩溃恢复.md) | 读档流程（校验→版本检查→迁移(如需要)→加载→运行时初始化）、preflight_check 快速预检、崩溃恢复三件套（临时文件+原子重命名 / 覆盖前自动备份 / session.lock 会话检测）、.tmp 清理、渐进式按需加载（LoadContext.txn_factory）、reset_to_default 半加载恢复、validate_after_migration 迁移后验证 | ✅ v2.0 |
| **005** | [版本迁移与Mod兼容](005-版本迁移与Mod兼容.md) | 模块级版本号（线性递增、迁移函数必须覆盖所有中间版本）、迁移调度（只读→读写→只读 LMDB 打开模式）、ConsumableEffect 惰性迁移（load() 中批量转换）、Mod 兼容（增删检测·冻结/替换降级）、悬空引用处理 | ✅ v2.0 |

---

## 关键参数速查

| 参数 | 值 | 说明 |
|------|-----|------|
| **存档文件格式** | 单文件 LMDB + 多 named_db | 扩展名 `.woworld` |
| **序列化约定** | 存档系统不规定格式 | 模块自选（推荐 bincode 或 rkyv） |
| **增量存档目标** | <500ms（SSD） | 流式写入，仅写脏数据 |
| **全量存档目标** | <5s（新游戏 Initial 存档） | 加载画面覆盖 |
| **读档目标** | <5s（含加载画面） | 5 秒够放加载画面 + 世界知识提示 |
| **自动存档间隔** | 默认 10 分钟（玩家可调 1-120） | 挂钟时间 |
| **自动存档轮转** | 3 个槽，覆盖最旧的 | 每槽保留 .bak 备份 |
| **快速存档** | 1 个槽（F5/F9） | 独立，不和自动/手动混淆 |
| **退出存档** | 1 个槽 | 退出时自动覆盖 |
| **Initial 存档** | 1 个槽 | 世界初始状态，永久保留 |
| **手动存档** | 无上限 | 玩家命名，可删除 |
| **LMDB mapsize** | 初始 4GB，翻倍自动扩容 | mmap 稀疏文件，实际占用 = 写入量 |
| **崩溃恢复** | 临时文件 + 原子重命名 + session.lock | 写入中途崩溃不损坏旧存档 |
| **死亡存档优先级** | 死亡 Exit > Manual > Quick > Auto | 死亡存档打断低优先级存档 |
| **磁盘预警** | mapsize 使用率 ≥ 80% 弹出警告 | 建议删除旧存档 |
| **UUID 去重** | 启动时检测，自动为新副本分配 UUID | 防止"继续游戏"歧义 |
| **SaveableModule trait** | 14 方法（4 必覆 + 10 默认） | 简单模块仅需 4 方法 |

---

## 存档系统架构（v2.0）

```
┌─────────────────────────────────────────────────────────────┐
│                      存档系统                               │
│                                                              │
│  SaveSystem                                                 │
│    ├── LMDB 环境管理（mmap·MVCC·读句柄常驻）                 │
│    ├── 保存流程（三阶段：snapshot → write → confirm）       │
│    │   └── Initial 存档直写路径（不经 Phase 1 内存收集）    │
│    ├── 读档流程（preflight → migrate → load → init）       │
│    ├── 版本迁移调度（调用模块 migrate() + 迁移后验证）       │
│    ├── 崩溃恢复（tmp+rename·bak·session.lock）              │
│    ├── 存档枚举（list_saves·delete_save）                   │
│    ├── 连续存档调度（Manual>Death>Quick>Auto + 补执行）     │
│    ├── 世界发现（目录扫描·world_meta.json·UUID去重）        │
│    └── 磁盘空间预警（mapsize 80% → 警告）                   │
│                                                              │
│  SaveableModule trait v2.0（存档系统 ↔ 模块的唯一契约）      │
│    ├── ★ module_name() → &'static str                      │
│    ├── ★ current_version() → u32                            │
│    ├── ★ named_dbs() → &[(&str, &str)]     ← (db_name, prefix)│
│    ├── is_critical() → bool                                │
│    ├── estimate_dirty_bytes() → Option<u64>                 │
│    ├── has_dirty() → bool                                  │
│    ├── snapshot_dirty(&self) → DirtySnapshot               │
│    ├── write_dirty(&self, txn)    ← 流式增量写入            │
│    ├── write_initial(&self, txn)  ← Initial 直写            │
│    ├── confirm_snapshot_written(&mut self, &DirtySnapshot) │
│    ├── load(&mut self, ctx: &LoadContext)                  │
│    ├── reset_to_default(&mut self)                         │
│    ├── migrate(&self, from_version, &RwTransaction)        │
│    └── validate_after_migration(&self, &RoTransaction)     │
│                                                              │
│  LoadContext（渐进加载上下文——零 SaveSystem 耦合）           │
│    ├── txn: &RoTransaction                                 │
│    └── create_txn() → Result<RoTransaction>                │
│                                                              │
│  存档系统不知道任何模块的具体类型                              │
│  存档系统不规定序列化格式                                    │
│  存档系统不参与数据老化策略                                  │
│  存档系统不持有模块数据的所有权                              │
└─────────────────────────────────────────────────────────────┘
```

---

## 与其他模块的关系

| 模块 | 存档系统提供什么 | 存档系统消费什么 |
|------|----------------|----------------|
| **全部 14 模块** | SaveableModule trait v2.0 契约、LMDB 读写、LoadContext | 各模块的 impl（通过 `&mut dyn SaveableModule`） |
| **世界生成** | 首次持久化（Initial 存档·直写路径） | 内存中的 WorldState（各模块已填充） |
| **文件系统** | .woworld 文件管理、目录扫描 | — |
| **Godot UI** | API 函数（new_game·save·load·list_saves·delete_save·pending_save_count） | 用户操作触发存档/读档 |

---

> **关联**: [[../技术栈方案/001-WoWorld正式技术栈方案v3|技术栈方案 v4.0]] · [[../../../参考文档/041-存档系统设计大纲-20260621/README|041 设计大纲]] · [[000-跨模块依赖与接口全面清单|000 跨模块依赖]] · [[../../../Change/CHG-056-存档系统深度审计与修正-20260621|CHG-056 审计修正]]
