//! SaveableModule trait — 存档系统的唯一跨模块契约
//!
//! v2.0 骨架（CHG-055/056）: 14 方法——4 必覆 + 10 默认。
//! MVP (V6) 仅定义形状，所有方法使用默认空实现。
//! 完整实现延后至存档系统正式 Phase 2。
//!
//! 参见: `WoWorld-Design/开发阶段/存档系统/003-保存流程与模块接口.md` §1.1

/// 脏数据快照——模块序列化输出
///
/// `snapshot_tick` 用于 confirm_snapshot_written 中比较 dirty_since，
/// 防止在快照和 LMDB 提交之间被修改的数据丢失。
#[derive(Debug, Clone, Default)]
pub struct DirtySnapshot {
    /// (key, value) 字节对——opaque to SaveSystem
    pub entries: Vec<(Vec<u8>, Vec<u8>)>,
    /// 快照时刻的游戏 tick
    pub snapshot_tick: u64,
}

impl DirtySnapshot {
    pub fn empty() -> Self {
        Self {
            entries: vec![],
            snapshot_tick: 0,
        }
    }

    pub fn with_tick(tick: u64) -> Self {
        Self {
            entries: vec![],
            snapshot_tick: tick,
        }
    }
}

// ── LoadContext ──────────────────────────────────────────

/// 渐进加载上下文——模块在 `load()` 中接收此引用，不依赖 `&SaveSystem`（零耦合）。
///
/// 设计 doc: 003-保存流程与模块接口 §二
///
/// `create_txn()` 工厂让大模块（物品 ~1.7GB、NPC ~520MB）在 `load()` 中
/// 只读元数据，运行时按 Chunk 创建独立读事务渐进加载。
pub struct LoadContext<'env> {
    /// 当前 LMDB 读事务——用于主加载流程
    pub txn: heed::RoTxn<'env>,
    /// 创建新读事务的工厂——用于按需/渐进加载
    ///
    /// 每次调用创建新的 LMDB 读事务（~µs 级开销）。模块应批量读取
    /// 而非逐实体创建事务。典型用法：每 Chunk/每区域一个事务。
    pub create_txn: Box<dyn Fn() -> Result<heed::RoTxn<'env>, String> + 'env>,
}

// ── SaveableModule ───────────────────────────────────────

/// 可保存模块——存档系统的唯一契约
///
/// # 对象安全
///
/// 所有方法返回 `Sized` 或 `()`，trait 为 object-safe。
/// 可用作 `&dyn SaveableModule` / `&mut dyn SaveableModule`。
///
/// # 必覆方法（4 个）
///
/// - `module_name()`: 模块唯一标识
/// - `current_version()`: 数据格式版本（默认 1）
/// - `named_dbs()`: 声明的 named_db 列表（默认空）
/// - `snapshot_dirty()`: 序列化脏数据（默认空快照）
///
/// # 快照安全协议
///
/// `snapshot_dirty(&self)` 不消费脏标记——只读取。
/// 仅 `confirm_snapshot_written(&mut self, snapshot)` 清除标记，
/// 且仅清除 `dirty_since <= snapshot.snapshot_tick` 的实体。
pub trait SaveableModule: Send + Sync {
    // ── 必覆方法 ──────────────────────────

    /// 模块唯一标识（如 "npc", "player", "terrain"）
    ///
    /// 改变此返回值 = 新身份，旧 LMDB 数据丢失。
    /// 使用 `&'static str` 避免堆分配。
    fn module_name(&self) -> &'static str;

    /// 当前数据格式版本——线性递增（1, 2, 3...）
    fn current_version(&self) -> u32 {
        1
    }

    /// 此模块写入的 named_db 列表
    ///
    /// 返回 `(db_name, key_prefix)` 对。
    /// `(db_name, key_prefix)` 组合必须跨所有已注册模块唯一。
    /// 返回 `&[]` 表示无持久数据。
    fn named_dbs(&self) -> &[(&str, &str)] {
        &[]
    }

    /// 序列化脏数据为 DirtySnapshot（`&self`——只读）
    ///
    /// MVP: 返回完整数据快照（非增量）。
    /// 正式 Phase 2: 仅返回标记为 dirty 的实体。
    fn snapshot_dirty(&self) -> Result<DirtySnapshot, String> {
        Ok(DirtySnapshot::empty())
    }

    // ── 可选方法（均有默认实现）────────────

    /// 此模块是否为关键模块？
    ///
    /// 关键模块（terrain/npc/player）加载失败 → 拒绝加载整个存档。
    /// 默认 `false`。
    fn is_critical(&self) -> bool {
        false
    }

    /// 是否有脏数据待持久化？（tick 边界查询）
    fn has_dirty(&self) -> bool {
        false
    }

    /// 预估下次保存的脏数据字节数（±50% 精度）
    ///
    /// 用于 SaveSystem 在保存前检查 map_size 是否需要增长。
    /// 返回 `None` 表示"无法预估"（SaveSystem 使用保守估计）。
    fn estimate_dirty_bytes(&self) -> Option<u64> {
        None
    }

    /// 流式写入脏数据到 LMDB（避免全量内存收集）
    ///
    /// 大型模块（terrain/npc/items）应覆盖此方法。
    /// 默认委托给 `snapshot_dirty()` 然后逐条写入。
    fn write_dirty(&self, _txn: &mut heed::RwTxn) -> Result<(), String> {
        let snap = self.snapshot_dirty()?;
        // 默认实现不写——由调用方处理
        let _ = snap;
        Ok(())
    }

    /// 流式写入全部初始数据（Initial save 专用）
    ///
    /// Initial save 可能 ~3GB——调用方直接写 LMDB，不经过内存快照。
    fn write_initial(&self, _txn: &mut heed::RwTxn) -> Result<(), String> {
        Ok(())
    }

    /// 确认快照已写入——仅清除 dirty_since <= snapshot_tick 的实体
    fn confirm_snapshot_written(&mut self, _snapshot: &DirtySnapshot) {}

    /// 从 LMDB 加载数据
    ///
    /// `ctx.txn` 提供当前读事务；`ctx.create_txn()` 工厂用于创建
    /// 独立读事务做按需/渐进加载（大模块用，小模块忽略）。
    fn load(&mut self, _ctx: &LoadContext) -> Result<(), String> {
        Ok(())
    }

    /// 重置为干净的默认状态（非关键模块加载失败时调用）
    fn reset_to_default(&mut self) {}

    /// 版本迁移——链式: while current < self.version() { migrate }
    fn migrate(&self, _from_version: u32, _txn: &heed::RwTxn) -> Result<(), String> {
        Ok(())
    }

    /// 迁移后轻度验证（<1s 预算）
    fn validate_after_migration(&self, _txn: &heed::RoTxn) -> Result<Vec<String>, String> {
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// MVP 占位实现——用于验证 trait object-safety
    struct StubModule {
        name: &'static str,
    }

    impl SaveableModule for StubModule {
        fn module_name(&self) -> &'static str {
            self.name
        }
    }

    #[test]
    fn test_trait_is_object_safe() {
        let m = StubModule { name: "stub" };
        let _dyn: &dyn SaveableModule = &m;
        assert_eq!(_dyn.module_name(), "stub");
        assert_eq!(_dyn.current_version(), 1);
        assert!(_dyn.named_dbs().is_empty());
        assert!(!_dyn.is_critical());
    }

    #[test]
    fn test_dirty_snapshot_default() {
        let snap = DirtySnapshot::empty();
        assert_eq!(snap.snapshot_tick, 0);
        assert!(snap.entries.is_empty());
    }

    #[test]
    fn test_dirty_snapshot_with_tick() {
        let snap = DirtySnapshot::with_tick(42);
        assert_eq!(snap.snapshot_tick, 42);
    }
}
