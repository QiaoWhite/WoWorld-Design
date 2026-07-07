//! 经济 Bootstrap
//!
//! 新存档首次启动时注入初始货币供给和市场流动性。
//! Phase 3 简化: GDP 估算用 NPC 数量 × 日均基础产出。
//! Phase 4: 从 ProfessionTag TOML 的 produces/consumes 字段推导。
//!
//! 参见: `WoWorld-Design/Happy Game/开发阶段/经济系统/001-经济系统总纲.md §5-B`

use crate::id::ItemDefId;

// ── BootstrapParams ───────────────────────────────────

/// 经济 Bootstrap 参数
#[derive(Debug, Clone, PartialEq)]
pub struct BootstrapParams {
    /// 经济体总 NPC 数量
    pub total_npc_count: u32,
    /// GDP 估算（铜币/年）
    /// Phase 3: `total_npc_count * daily_base * 365`
    /// Phase 4: 从 ProfessionTag TOML produces 字段推导
    pub gdp_estimate: u64,
    /// 货币流通速度（默认 3——年度周转 3 次）
    pub velocity: f32,
    /// 初始价格乘数——base_value × 此系数 = 初始市场参考价
    pub initial_price_multiplier: f32,
}

impl Default for BootstrapParams {
    fn default() -> Self {
        Self {
            total_npc_count: 0,
            gdp_estimate: 0,
            velocity: 3.0,
            initial_price_multiplier: 1.0,
        }
    }
}

impl BootstrapParams {
    /// 从 NPC 数量快速估算——每个 NPC 日均消费 ~50 铜币。
    pub fn from_npc_count(count: u32) -> Self {
        let daily_per_npc: u64 = 50;
        let gdp = count as u64 * daily_per_npc * 365;
        Self {
            total_npc_count: count,
            gdp_estimate: gdp,
            velocity: 3.0,
            initial_price_multiplier: 1.0,
        }
    }
}

// ── initial_money_supply ──────────────────────────────

/// 计算初始货币供给 M0（设计 001 §5-B）
///
/// M0 = GDP / velocity
///
/// velocity=0 时返回 0（防御性）。
pub fn initial_money_supply(params: &BootstrapParams) -> u64 {
    if params.velocity <= 0.0 {
        return 0;
    }
    (params.gdp_estimate as f64 / params.velocity as f64) as u64
}

// ── inject_liquidity ──────────────────────────────────

/// 为经济体注入初始市场参考价——保证 bootstrap 后至少存在可撮合交易。
///
/// 对每种物品生成一对 bid-ask 价差。Phase 3 不创建实体 NPC 订单——直接注入 PriceSnapshot。
/// 返回 (item_id, bid_price, ask_price) 列表供 registry 写入初始 EMA 价格。
pub fn inject_liquidity(
    item_ids: &[ItemDefId],
    base_values: &dyn Fn(ItemDefId) -> Option<u32>,
    multiplier: f32,
    seed: u64,
) -> Vec<LiquidityInjection> {
    item_ids
        .iter()
        .filter_map(|&id| {
            let base = base_values(id)? as f32;
            let hash = seed.wrapping_mul(id.0).wrapping_add(0x9E37_79B9);
            let noise = ((hash & 0xFF) as f32) / 256.0 * 0.1; // ±5%
            let bid = (base * multiplier * (0.85 + noise)).round() as u64;
            let ask = (base * multiplier * (1.15 + noise)).round() as u64;
            Some(LiquidityInjection {
                item_id: id,
                bid_price: bid.max(1),
                ask_price: ask.max(1).max(bid + 1), // ask > bid
            })
        })
        .collect()
}

/// 流动性注入条目
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiquidityInjection {
    pub item_id: ItemDefId,
    pub bid_price: u64,
    pub ask_price: u64,
}

// ── tests ────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── BootstrapParams ────────────────────────────────

    #[test]
    fn test_default_zero() {
        let p = BootstrapParams::default();
        assert_eq!(p.total_npc_count, 0);
        assert_eq!(p.gdp_estimate, 0);
    }

    #[test]
    fn test_from_npc_count() {
        let p = BootstrapParams::from_npc_count(100);
        assert_eq!(p.total_npc_count, 100);
        assert!(p.gdp_estimate > 0);
    }

    // ── initial_money_supply ───────────────────────────

    #[test]
    fn test_money_supply_basic() {
        let p = BootstrapParams {
            gdp_estimate: 300_000,
            velocity: 3.0,
            ..Default::default()
        };
        let m0 = initial_money_supply(&p);
        assert_eq!(m0, 100_000); // 300k / 3 = 100k
    }

    #[test]
    fn test_money_supply_zero_gdp() {
        let p = BootstrapParams::default();
        assert_eq!(initial_money_supply(&p), 0);
    }

    #[test]
    fn test_money_supply_zero_velocity() {
        let p = BootstrapParams {
            gdp_estimate: 100_000,
            velocity: 0.0,
            ..Default::default()
        };
        assert_eq!(initial_money_supply(&p), 0);
    }

    // ── inject_liquidity ───────────────────────────────

    #[test]
    fn test_inject_empty() {
        let result = inject_liquidity(&[], &|_| Some(100), 1.0, 42);
        assert!(result.is_empty());
    }

    #[test]
    fn test_inject_generates_valid_spread() {
        let ids = vec![ItemDefId(1), ItemDefId(2)];
        let result = inject_liquidity(&ids, &|id| if id.0 == 1 { Some(100) } else { None }, 1.0, 42);
        // 只有 id=1 有 base_value，id=2 返回 None 被过滤
        assert_eq!(result.len(), 1);
        assert!(result[0].bid_price > 0);
        assert!(result[0].ask_price > result[0].bid_price);
    }

    #[test]
    fn test_inject_deterministic() {
        let ids = vec![ItemDefId(1)];
        let a = inject_liquidity(&ids, &|_| Some(100), 1.0, 42);
        let b = inject_liquidity(&ids, &|_| Some(100), 1.0, 42);
        assert_eq!(a[0].bid_price, b[0].bid_price);
        assert_eq!(a[0].ask_price, b[0].ask_price);
    }

    #[test]
    fn test_inject_different_seeds_different() {
        let ids = vec![ItemDefId(1), ItemDefId(2), ItemDefId(3), ItemDefId(4)];
        let a = inject_liquidity(&ids, &|_| Some(100), 1.0, 42);
        let b = inject_liquidity(&ids, &|_| Some(100), 1.0, 999);
        // 不同 seed，多物品时至少有一个价格不同
        let any_diff = a.iter().zip(b.iter()).any(|(ai, bi)| {
            ai.bid_price != bi.bid_price || ai.ask_price != bi.ask_price
        });
        assert!(any_diff, "different seeds should produce different noise for at least one item");
    }
}
