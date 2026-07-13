//! SaveHeader — 存档文件头
//!
//! 满足 002-存档格式与文件架构 §2.1 的最小 MVP 子集。
//! 完整 17-field SaveHeader 延后至存档系统正式 Phase 2。

use serde::{Deserialize, Serialize};

/// 存档文件魔数
pub const SAVE_MAGIC: &[u8; 8] = b"WOWSAVE\x00";

/// 存档格式人类可读名称
pub const FORMAT_NAME: &str = "woworld-save-v1";

/// 当前存档格式版本
pub const FORMAT_VERSION: u32 = 1;

/// 存档文件头——用于识别和验证存档文件
///
/// 当前为 MVP 子集（9/17 字段）。完整 17-field 延后至 Phase 2。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveHeader {
    /// 魔数: b"WOWSAVE\x00"
    pub magic: [u8; 8],
    /// 格式人类可读名称: "woworld-save-v1"（≤64 UTF-8 bytes）
    pub format_name: String,
    /// 存档格式版本（当前 = 1）
    pub global_version: u32,
    /// 存档不可变标识——UUID v4（跨重命名/移动保持身份）
    pub save_uuid: [u8; 16],
    /// 世界种子
    pub world_seed: u64,
    /// 世界名称（≤128 UTF-8 bytes）
    pub world_name: String,
    /// Unix 时间戳（秒）
    pub timestamp_unix: i64,
    /// 游戏 tick（= frame_count）
    pub game_tick: u64,
    /// 存档名称（≤256 UTF-8 bytes）
    pub save_name: String,
}

impl SaveHeader {
    /// 创建新 SaveHeader
    ///
    /// `save_uuid` 从时间戳 + 地址哈希派生（MVP——正式 Phase 2 引入 uuid crate）。
    pub fn new(world_seed: u64, world_name: String, game_tick: u64, save_name: String) -> Self {
        let timestamp_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // 生成伪 UUID——时间戳低 64 位 + 世界种子
        let mut uuid = [0u8; 16];
        uuid[..8].copy_from_slice(&timestamp_unix.to_le_bytes());
        uuid[8..].copy_from_slice(&world_seed.to_le_bytes());
        // UUID v4 变体标记（bits 6-7 of byte 8 = 10）
        uuid[8] = (uuid[8] & 0x3f) | 0x80;
        // UUID v4 版本标记（bits 12-15 of byte 6 = 0100）
        uuid[6] = (uuid[6] & 0x0f) | 0x40;

        // 截断 world_name 到 128 字节
        let mut wname = world_name;
        truncate_to(&mut wname, 128);

        // 截断存档名到 256 字节
        let mut sname = save_name;
        truncate_to(&mut sname, 256);

        Self {
            magic: *SAVE_MAGIC,
            format_name: FORMAT_NAME.into(),
            global_version: FORMAT_VERSION,
            save_uuid: uuid,
            world_seed,
            world_name: wname,
            timestamp_unix,
            game_tick,
            save_name: sname,
        }
    }

    /// 验证魔数、格式名和版本
    pub fn validate(&self) -> Result<(), String> {
        if self.magic != *SAVE_MAGIC {
            return Err(format!(
                "无效的存档文件魔数: {:?} (期望: {:?})",
                self.magic, *SAVE_MAGIC
            ));
        }
        if self.format_name != FORMAT_NAME {
            return Err(format!(
                "不支持的存档格式: '{}' (期望: '{}')",
                self.format_name, FORMAT_NAME
            ));
        }
        if self.global_version != FORMAT_VERSION {
            return Err(format!(
                "不支持的存档版本: {} (当前支持: {})",
                self.global_version, FORMAT_VERSION
            ));
        }
        Ok(())
    }
}

/// UTF-8 安全截断字符串到 max_len 字节
fn truncate_to(s: &mut String, max_len: usize) {
    if s.len() > max_len {
        let mut end = max_len;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        s.truncate(end);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_roundtrip() {
        let header = SaveHeader::new(42, "test_world".into(), 1000, "test_save".into());
        let bytes = bincode::serialize(&header).expect("serialize");
        let restored: SaveHeader = bincode::deserialize(&bytes).expect("deserialize");
        assert_eq!(restored.magic, *SAVE_MAGIC);
        assert_eq!(restored.global_version, FORMAT_VERSION);
        assert_eq!(restored.world_seed, 42);
        assert_eq!(restored.game_tick, 1000);
        assert_eq!(restored.save_name, "test_save");
        restored.validate().expect("validate");
    }

    #[test]
    fn test_header_validate_rejects_bad_magic() {
        let mut header = SaveHeader::new(0, "w".into(), 0, "bad".into());
        header.magic = *b"DEADBEEF";
        assert!(header.validate().is_err());
    }

    #[test]
    fn test_header_validate_rejects_bad_version() {
        let mut header = SaveHeader::new(0, "w".into(), 0, "bad".into());
        header.global_version = 999;
        assert!(header.validate().is_err());
    }

    #[test]
    fn test_long_name_truncated() {
        let long_name = "a".repeat(300);
        let header = SaveHeader::new(0, "w".into(), 0, long_name);
        assert!(header.save_name.len() <= 256);
    }
}
