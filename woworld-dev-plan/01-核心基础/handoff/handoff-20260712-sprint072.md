# Handoff: 2026-07-12 — Sprint-072 V3b 市场接真

> **会话类型**: 冲刺执行（场景 B）· **日期**: 2026-07-12
> **阶段**: Phase 2 探针切片「活着的村庄」·进度 `▓▓▓▓▓▓▓░░░ 7/10`
> **状态**: ✅ Sprint-072 完成·1116 tests 全绿·clippy/fmt 零警告·**待提交**

## 📊 本会话做了什么

1. **V3b 审计**——对着代码发现三个假数据源：`daily_need` 种子随机、`EconomyRegistry.item_holdings` 假账、Wallet 双账脱节
2. **V3b 编码**——`order_creation_system` 三大改造（Needs.hunger→daily_need、InventoryRegistry→库存、registry→钱包）+ `wallet_sync_system` + `WalletSnapshot→Wallet` From trait + WorldDriver 接线
3. **测试**——新增 5 个 V3b 专项测试（daily_need_from_real_hunger·reads_inventory_registry·no_seed_on_first_tick·wallet_sync_after_trade·wallet_read_from_registry）

### 架构决策

**`EconomyRegistry.item_holdings` 保留不退场**：`execute_trade` 内部仍通过 `add_items`/`remove_items` 操作物品转移——撮合引擎的原子性依赖它。仅 `order_creation_system` 的**读取侧**切到 `InventoryRegistry`。Phase 3 完整迁移时再彻底移除。

**`daily_need = 0.2 + hunger * 1.5`**：简单线性映射。未接入 `assess_physiological_needs()`（它读 Vitals 非 Needs）。Phase 3 统一 hunger 方向后合并。

**钱包从 registry 读**：`order_creation_system` 优先 `registry.get_wallet()`，fallback 到 ECS component。成交后 `wallet_sync_system` 回写 ECS。

## 📦 产物

**代码**（`woworld/`）：ecs (economy/mod.rs + components/economy.rs) + godot (terrain_chunk.rs)。3 文件，+~200 行，+5 tests。

**文档**（`woworld-dev-plan/`）：DEVLOG、本 Handoff、冲刺提案。

## 💾 恢复点（下一会话 AI 必读）

> ⚠️ 下一会话按 `00-流程总览` **场景 B（冲刺执行）** 启动——V3b 已完成，下一步 **V4b 交易气泡**。

- **当前阶段**: Phase 2 切片·`7/10`（V0+V1+V4a+Vf+V2+V3a+V3b ✅）·10 步序列见 [[../../02-垂直切片/README]] §3
- **下一步**: **V4b 交易气泡**（第 8/10 步）——交易吆喝气泡（接 V3 成交事件出口）。依赖：V3b ✅（市场接真·真实成交）+ V4a ✅（问候/情绪气泡框架）
- **机械门状态**: `1116 tests 全绿`（core 401 + worldgen 75 + atmosphere 26 + ecs 605 + integration 9），clippy/fmt 零警告，build 通过。
- **提交状态**: ⚠️ **未提交**。基线 = `646404b`（Sprint-071）。本会话改动待提交。
- **A1 铁律**: 纯涌现，禁脚手架/假坐标/占位驱动/平行 trait/语音决策 silo。
- **关键新增类型/函数**:
  - `wallet_sync_system(world, cmd, registry)` — 成交后 registry→ECS 回写
  - `From<WalletSnapshot> for Wallet` — 反向转换
  - `fake_item_registry_with_food()` — 含 Edible+ConsumableEffect 的测试 registry
- **`order_creation_system` 新签名**:
  ```rust
  pub fn order_creation_system(
      world: &World,
      registry: &mut EconomyRegistry,
      inventory_registry: &InventoryRegistry,  // ★ 新增
      item_registry: &dyn ItemQuery,
      tick: u64,
  )
  ```
- **数据流改变**:
  - `daily_need` = `Needs.hunger` × 1.5 + 0.2（曾是 `seed*17%3 * 0.2`）
  - 库存 → `InventoryRegistry::get_holdings()`（曾是 `EconomyRegistry::get_holdings()`）
  - 钱包 → `registry.get_wallet()`（曾是 `wallet.total_copper()`）
  - 首帧不再调 `seed_npc_items`
- **系统调度**: Block A5 后接 `wallet_sync_system`（独立 cmd 立即 flush）。

## ⚠️ 遗留 / 诚实边界

- **`EconomyRegistry.item_holdings` 保留**：撮合引擎 `execute_trade` 仍用它。标注 `Phase 3 迁移`。
- **`assess_physiological_needs()` 未接入**：它读 Vitals（非 Needs），且不涉及库存。Phase 3 统一后合并。
- **仅 hunger 驱动订单**：thirst 仍走魔法路径。V4 阶段做 drink 物品后接入。
- **商品类别窄**：现有物品池仅 food/mineral/leather/weapon/wood。市场多样性受限于此。
- **无生产端**：NPC 只采集不吃就卖，不会"制造"物品。Phase 3 经济驱动。

## 🔗 关联

- **提案**: [[../../sprint-proposals/sprint-072-V3b-市场接真-20260712]]
- **DEVLOG**: [[../devlogs/DEVLOG-2026-07-12-sprint072]]
- **上游**: [[handoff-20260712-sprint071]]（V3a 代谢闭环）
- **路线图**: [[../../02-垂直切片/README]] · [[../../附录E-开发状态]]
- **下游**: V4b 交易气泡（成交事件→吆喝气泡·薄出口）
