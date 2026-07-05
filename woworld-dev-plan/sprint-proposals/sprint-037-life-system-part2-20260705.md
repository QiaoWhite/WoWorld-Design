# Sprint-037: ECS Phase 1 生命系统（下）— 腐败→消失 + 再生

> **提案日期**: 2026-07-05
> **提案状态**: ✅ 已完成
> **所属阶段**: Phase 1 — 核心基础
> **所属里程碑**: 1H — 生命系统（下半·补齐完整生命周期）
> **前置**: Sprint 036 ✅（死亡→掉落链）
> **后续**: 天气系统涌现化 或 NPC 核心 ECS

## 📋 依赖前提

| 前置项 | 状态 |
|--------|------|
| Sprint 036 生命 Component | ✅ Vitals, DeathCause, Corpse, PendingLoot, LootResult |
| Sprint 036 死亡→掉落链 | ✅ DeathWatch → LootRoll → ItemSpawn |
| 126 tests | ✅ |

## 🎯 目标（3 个）

### 目标 1: 补齐 Component + 修正遗漏

- `DecayingRemains` — decay_progress: f32 (0→1)
- `PendingDespawn` — 零字段 tag
- `RegenState` — hp_regen_rate: f32, stamina_regen_rate: f32
- `CorpseLooted` — 零字段 tag（Sprint 036 遗漏——ItemSpawn 移除 LootResult 后应插入此 tag）

### 目标 2: 腐败→消失链（2 System）

| System | 触发 | 操作 |
|--------|------|------|
| `corpse_decay` | Corpse + CorpseLooted, tick 差 > 5min | remove Corpse, insert DecayingRemains{0.0} |
| `cleanup` | DecayingRemains.progress >= 1.0 或 PendingDespawn | cmd.despawn(entity) |

**CorpseLooted 遗漏修正**: ItemSpawnSystem 移除 LootResult → 同时插入 CorpseLooted tag（标记"已被搜刮"）

### 目标 3: RegenSystem

- `(Entity, &mut Vitals, &RegenState)` → hp/stamina 每帧微量恢复，上限 cap 在 max_hp/max_stamina
- 是唯一一个**每帧对活人执行**的系统——为后续 hunger/thirst decay 模式打样

## 🧪 研究事项

| 问题 | 级别 | 状态 |
|------|------|------|
| 腐败计时：帧 tick 差 vs 游戏时间 | 🟡 | 当前用帧 tick 差——Sprint 037 用帧差即可，精确游戏时间留后续 |
| DecayingRemains progress 递增速率 | 🟢 | 线性 `decay_progress += delta_per_frame`——简单正确 |

## 📖 必读文档

| 文档 | 用途 |
|------|------|
| `开发文档/01-世界框架/02-生命系统.md` | CorpseDecay + Cleanup 规格 |
| `开发文档/00-ECS哲学与架构总纲/006-ECS铁律与陷阱.md` | 铁律 4：标记+延迟清理 |

## 📋 任务清单

### Step 2.1: 补齐 Component + 修正
- [ ] `DecayingRemains` — decay_progress: f32
- [ ] `PendingDespawn` — 零字段 tag
- [ ] `RegenState` — hp_regen, stamina_regen (f32×2)
- [ ] `CorpseLooted` — 零字段 tag
- [ ] ItemSpawnSystem 修正：insert CorpseLooted

### Step 2.2: CorpseDecaySystem
- [ ] `corpse_decay.rs` — 查询 (Corpse + CorpseLooted)，帧差>阈值 → remove Corpse, insert DecayingRemains
- [ ] DecayingRemains decay_progress 递增

### Step 2.3: CleanupSystem
- [ ] `cleanup.rs` — 查询 (DecayingRemains{>=1.0} 或 PendingDespawn) → despawn
- [ ] DecayingRemains 也做每帧 progress += delta

### Step 2.4: RegenSystem
- [ ] `regen.rs` — 查询 (&mut Vitals, &RegenState) → hp += rate, stamina += rate, cap 检查

### Step 2.5: WorldDriver 集成 + 集成测试
- [ ] process() 注册新 System
- [ ] 完整"死亡→掉落→腐败→消失"链条测试
- [ ] RegenSystem 测试

### Step 2.6: 验证
- [ ] `cargo test --workspace` 126 回归 + 新增 ≥128+
- [ ] `cargo clippy --workspace -- -D warnings` ✅
- [ ] ECS 铁律 8 条审查

## 预估

- **冲刺数**: 1
- **风险**: 🟢 低——纯粹增量，模式已建立
- **代码量**: ~350 行
