# DEVLOG: 2026-07-12 — Sprint-072 V3b 市场接真

> **冲刺**: Sprint-072 — V3b 市场接真·弃假数据源·双账统一
> **日期**: 2026-07-12
> **阶段**: Phase 2 探针切片「活着的村庄」·进度 `▓▓▓▓▓▓▓░░░ 6→7/10`

## 做了什么

将 `order_creation_system` 的三个假数据源替换为真实数据：

1. **`daily_need` 真 → Needs.hunger**（弃种子随机）
   - `daily_need = 0.2 + hunger * 1.5`（[0.2, 1.7]——饱→低需求，饿→高需求）
   - 饿的 NPC 产生更高 reserve_need → 更多买单

2. **库存真 → InventoryRegistry**（弃 EconomyRegistry.item_holdings 假账）
   - `order_creation_system` 新增 `inventory_registry` 参数
   - `inventory_registry.get_holdings()` 取代 `registry.get_holdings()`
   - 移除首帧 `EconomyRegistry.seed_npc_items`（V3a NPC 自己采集）
   - `EconomyRegistry.item_holdings` 保留不动（撮合引擎仍需，仅读取侧迁移）

3. **钱包真 → 统一双账**
   - `order_creation_system` 从 registry 读钱包（权威源）
   - 新增 `wallet_sync_system`：成交后 registry → ECS Wallet 回写
   - 新增 `Wallet::from(WalletSnapshot)` 反向转换

## 机械门

- **1116 tests 全绿** (+5 V3b 专项: daily_need·InventoryRegistry·no seed·wallet sync·wallet read)
- clippy 零警告
- fmt 通过
- build 通过

## 关键文件

| 文件 | 改动 |
|------|------|
| `woworld_ecs/src/systems/economy/mod.rs` | order_creation_system 三大改造 + wallet_sync_system + 5 tests |
| `woworld_ecs/src/components/economy.rs` | `From<WalletSnapshot> for Wallet` |
| `woworld_godot/src/terrain_chunk.rs` | 传 InventoryRegistry + wallet_sync 接线 |

## 诚实边界

- `EconomyRegistry.item_holdings` 未删除（撮合引擎 `execute_trade` 仍需）
- `assess_physiological_needs()` 未接入（现有 needs.rs 读 Vitals 非 Needs；待 Phase 3 统一）
- 仅 hunger 驱动 daily_need（thirst 仍走魔法路径）
- 未实现 profession/family_size 需求估计（Phase 3）
