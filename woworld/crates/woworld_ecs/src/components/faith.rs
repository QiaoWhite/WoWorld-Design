//! Faith Component — NPC 信仰归属标签
//!
//! 纯 4B tag Component。关联的神学参数和所有派生数据
//! 均存储在 FaithRegistry Resource 中，通过 FaithQuery trait 访问。
//!
//! 参见: woworld_core::faith

use woworld_core::faith::{FaithId, FAITH_ID_NONE};

/// 信仰归属 — NPC 实体的信仰标识
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Faith {
    pub faith_id: FaithId,
}

impl Default for Faith {
    fn default() -> Self {
        Self { faith_id: FAITH_ID_NONE }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_none() {
        assert_eq!(Faith::default().faith_id, FAITH_ID_NONE);
    }

    #[test]
    fn test_size_4_bytes() {
        assert_eq!(std::mem::size_of::<Faith>(), 4);
    }
}
