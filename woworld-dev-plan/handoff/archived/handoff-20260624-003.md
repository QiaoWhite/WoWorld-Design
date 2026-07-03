# 会话交接 — 2026-06-24（最终版）

## 当前冲刺

轨A 地形冲刺：昼夜循环 + 群系系统（已完成编码）
后续：用户审查发现了架构偏离，触发了宪法 v1.1→v1.4 重大修订

## 代码交付

### 新增文件
- `crates/woworld_core/src/time.rs` — WorldTime, TimeOfDay 纯数据类型 (6 tests)
- `crates/woworld_worldgen/src/time.rs` — WorldClock 运行时钟 (6 tests)
- `crates/woworld_worldgen/src/biome.rs` — BiomeClassifier, BiomeDef (5 tests)
- `assets/biomes.toml` — 5 群系 TOML
- `godot/scripts/time_manager.gd` — GDScript 昼夜循环（⚠️ 已知偏离：太阳轨道/色板在 GDScript 独立计算，未走 Rust）

### 修改文件（17 个）
Rust crates: Cargo.toml ×3, lib.rs ×2, material.rs, noise_gen.rs, terrain.rs, terrain_chunk.rs
Godot: main.tscn, player.gd (JUMP_VELOCITY 10×)
Docs: CLAUDE.md

### 测试
46 个测试全部通过（core 6 + spatial 12 + worldgen 25 + godot 3）

## 架构偏离（CHG-064 — 需在下个冲刺处理）

| ID | 描述 | 严重级别 |
|----|------|---------|
| D01 | 昼夜渲染 100% 在 GDScript（太阳轨道/色板/季节偏移），Rust `get_sun_position()` 等 4 个 `#[func]` 僵尸 | 🟡 |
| D02 | `woworld_core` 加了 serde 硬依赖，应改为 optional feature gate | 🟡 |
| D03 | `process()`→`advance_time()` 的理由"GDExtension _process 不可靠"基于未验证假设——真正 bug 是忘了 `look_at()` | 🔴 |
| D04 | 群系子材质系统（`SubMaterialWeight` + `sub_material_matches()` + TOML sub_materials）成为死代码 | 🟡 |

## 踩坑记录

1. **DirectionalLight3D 光照方向来自 rotation 不是 position** — 必须调 `look_at()`
2. **godot-rust #[func] 可能不支持 &mut self** — 用 RefCell 内部可变性（未 100% 确认是框架限制还是其他 bug）
3. **GDScript 没有 double()** — float 就是 64 位
4. **GDScript API 幻觉风险极高** — LLM 训练数据中 GDScript 占比远小于 Rust

## 宪法修订（v1.1 → v1.4）

| 版本 | 新增 |
|------|------|
| v1.2 | §14 架构边界合规审计（双权威检测 + 僵尸代码扫描 + 边界穿越审计） |
| v1.2 | §15 偏离根因分析（RCA 模板 + 偏离日志） |
| v1.3 | §4 GDScript API 预验证（写 .gd 前必须 WebFetch 查 Godot 文档） |
| v1.3 | §14.1 GDScript 代码铁律（每行归类三种：Input/读Rust/设节点） |
| v1.4 | §14.5 对抗性重新验证（🟢 "合理"偏离必须假设不合理 + 找反证） |

## Godot 启动

```bash
cd woworld
cargo build --workspace
../tools/godot/Godot_v4.7-stable_win64.exe godot/project.godot
```

## 下一步候选

1. **修复 CHG-064 偏离** — time_manager.gd 计算逻辑迁回 Rust，验证 `set_process(true)` 是否可用
2. **Chunk 分块 + 动态加载** — 32m Chunk 网格，LOD
3. **海洋水面渲染** — Gerstner 波平面 + 浅水过渡
4. **Transvoxel 体素化** — 高度场→真 3D 体素
