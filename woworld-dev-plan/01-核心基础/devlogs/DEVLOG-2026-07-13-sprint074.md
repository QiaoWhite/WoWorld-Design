# DEVLOG: 2026-07-13 — Sprint-074 V5 旁观工具

> **冲刺**: Sprint-074 — V5 旁观工具·时间加速+实体检视·控制台命令
> **日期**: 2026-07-13
> **阶段**: Phase 2 探针切片「活着的村庄」·进度 `▓▓▓▓▓▓▓▓▓░ 9/10`

## 做了什么

将时间加速和实体检视功能接入 F3 控制台，完成旁观工具的闭环：

1. **模拟速度控制（控制台 `speed` 命令）**
   - `WorldClock` 新增 `time_scale: f32` 字段（默认 1.0，范围 [0.0, 100.0]）+ `set_time_scale()` 自动 clamp
   - `advance()` 内部乘以 `time_scale`——时钟推进自动跟随速度
   - `WorldDriver` ECS 块内计算 `sim_delta = delta * time_scale`，天气/动画/ECS System 统一加速
   - F3 控制台 `speed` 命令：`speed 60`（60倍）/ `speed 1`（正常）/ `speed 0`（暂停）
   - 速度档位常量 `[1.0, 10.0, 60.0]` 集中在 `cmd_speed` 函数内管理

2. **实体检视快照扩展（`entity_debug_system`）**
   - 新增 5 个 section：**Action**（ActionIntent·category+weight）、**Wallet**（铜/银/金+总铜币）、**Economy**（EconomicCognition 5维）、**Growth**（GrowthNeeds·尊重/胜任/慢性）、**Inventory**（InventoryRegistry 查询·持有物品数+装备有无）
   - 函数签名扩展：`entity_debug_system(world, entity, inventory: Option<&InventoryRegistry>)` ——类型系统强制调用者显式处理"无 registry"
   - console `info` 命令自动受益——现输出 12 个 section

3. **旁观者点击选中**
   - 非控制台模式下左键点击 NPC → AABB raycast 选中 + 金色高亮
   - 点击空白 → 取消选中
   - 控制台模式继续使用独立选中状态
   - 新增 `#[func] get_selected_entity_bits()` + `#[func] get_inspect_data(entity_bits)`（JSON 格式·供未来 Godot 面板）

4. **设计修正**
   - 时间加速从"全局热键"改为"F3 控制台命令"（用户裁决——调试功能集中管理，不占用未来快捷栏）
   - 需求满足点可视化降级为"控制台可查"（植被无 mesh 渲染管线，标记需先有渲染系统）
   - Godot 检视面板 `.tscn` 延后——控制台 `info` 提供同等数据

## 机械门

- **1136 tests 全绿**（core 412 + worldgen 75 + atmosphere 26 + ecs 614 + integration 9）
- clippy 零警告（`-- -D warnings`）
- fmt 通过
- build 通过
- 实机验证：`speed 60` 太阳+天气+NPC 统一加速，`info` 显示完整 12 section

## 关键文件

| 文件 | 改动 |
|------|------|
| `woworld_core/src/time.rs` | +time_scale +set_time_scale +advance内部乘法 +7 tests |
| `woworld_godot/src/debug_console.rs` | +pending_time_scale +cmd_speed +注册 |
| `woworld_godot/src/terrain_chunk.rs` | sim_delta(ECS块内)+spectator click/select/highlight+pending处理+2 #[func] |
| `woworld_ecs/src/systems/entity_visual.rs` | +5 section(Action/Wallet/Economy/Growth/Inventory)+签名扩展+2 tests |

## 诚实边界

- 速度档位 `[1.0, 10.0, 60.0]` 硬编码在 debug_console——未来可 TOML 化
- `get_inspect_data` 手写 JSON——未引入 serde_json 依赖
- 无独立 Godot 检视面板——控制台 `info` 提供同等数据
- 采集点 debug 标记未实现——植被无 mesh 渲染管线
- Godot 4.7 已知行为：鼠标环顾与点击选中无冲突——旁观者点击在 process() 处理、相机拖拽在 _input() 处理，天然分层

## 下一步

**V6 快照存档**（第 10/10 步）——bincode 序列化快照 → LMDB → 重载重建。存档零代码、首次引入 LMDB、MVP 级别（不做 SaveableModule 14 方法）。
