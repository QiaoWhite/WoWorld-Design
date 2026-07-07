//! Culture Component — NPC 文化归属标签
//!
//! 纯 4B tag Component。关联的参数（CultureCoreParams）和所有推导数据
//! 均存储在 CultureRegistry Resource 中，通过 CultureQuery trait 访问。
//!
//! 参见: woworld_core::culture

use woworld_core::culture::{CultureId, CULTURE_ID_NONE};

/// 文化归属 — NPC 实体的文化标识
///
/// 纯数据，零方法（ECS 铁律 1）。
/// 4 bytes, Copy——可嵌入高频查询路径。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Culture {
    pub culture_id: CultureId,
}

impl Default for Culture {
    fn default() -> Self {
        Self {
            culture_id: CULTURE_ID_NONE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_none() {
        let c = Culture::default();
        assert_eq!(c.culture_id, CULTURE_ID_NONE);
    }

    #[test]
    fn test_size_4_bytes() {
        assert_eq!(std::mem::size_of::<Culture>(), 4);
    }
}
