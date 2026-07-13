//! 社交 Component — ECS 铁律合规（纯数据·零方法·无堆）
//!
//! SocialPresence 定义 NPC 的社交影响范围与恢复强度。
//! RelationHandle 标记实体参与关系系统（数据在 RelationStorage Resource）。
//!
//! 参见: `开发文档/02-NPC核心/01-NPC总纲.md` §社会层

use crate::components::bigfive::BigFive;
use serde::{Deserialize, Serialize};

/// 社交存在——决定 NPC 对他人的社交影响力
///
/// 外向者半径大、恢复快；内向者半径小、恢复慢。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SocialPresence {
    /// 社交半径 (m)——在此范围内的其他 NPC 可恢复社交需求
    pub radius: f32,
    /// 社交恢复速率 (/s)——每秒降低对方 social 需求的基础速率
    pub recovery_rate: f32,
}

impl Default for SocialPresence {
    fn default() -> Self {
        Self {
            radius: 4.5,
            recovery_rate: 0.006,
        }
    }
}

impl SocialPresence {
    /// 从 BigFive 外向性派生社交存在
    ///
    /// - radius: 2.0 + E × 5.0 → [2m, 7m]
    /// - recovery_rate: 0.002 + E × 0.008 → [0.002/s, 0.01/s]
    pub fn derive_from_bigfive(b: &BigFive) -> Self {
        Self {
            radius: 2.0 + b.extraversion * 5.0,
            recovery_rate: 0.002 + b.extraversion * 0.008,
        }
    }
}

// ── RelationHandle ─────────────────────

/// 标记实体参与关系系统——零字段 tag Component。
///
/// 实际关系数据存储在 `RelationStorage` Resource 中。
/// 对应文档: `开发文档/02-NPC核心/01-NPC总纲.md` §Handle 类型 —
/// `RelationHandle{storage_key, count}` → `RelationStorage` Resource
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RelationHandle;

impl Default for RelationHandle {
    fn default() -> Self {
        Self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_bigfive_extravert() {
        let b = BigFive {
            extraversion: 1.0,
            ..BigFive::default()
        };
        let sp = SocialPresence::derive_from_bigfive(&b);
        assert!((sp.radius - 7.0).abs() < 0.01);
        assert!((sp.recovery_rate - 0.01).abs() < 0.001);
    }

    #[test]
    fn test_from_bigfive_introvert() {
        let b = BigFive {
            extraversion: 0.0,
            ..BigFive::default()
        };
        let sp = SocialPresence::derive_from_bigfive(&b);
        assert!((sp.radius - 2.0).abs() < 0.01);
        assert!((sp.recovery_rate - 0.002).abs() < 0.001);
    }

    #[test]
    fn test_default_in_range() {
        let sp = SocialPresence::default();
        assert!(sp.radius > 0.0);
        assert!(sp.recovery_rate > 0.0);
    }

    #[test]
    fn test_relation_handle_is_zst() {
        assert_eq!(std::mem::size_of::<RelationHandle>(), 0);
    }
}
