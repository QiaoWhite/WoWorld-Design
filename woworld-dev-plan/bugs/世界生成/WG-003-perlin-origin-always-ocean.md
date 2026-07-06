---
id: WG-003
title: Perlin(0,0)=0 全种子原点永远海洋
type: 🟡反直觉陷阱
module: 世界生成
status: ✅已修复
confidence: ✅确信
discovered: 2026-07-03
resolved: 2026-07-03
last_verified: 2026-07-03
grep_keys: [perlin, 原点, origin, ocean, 海洋, 全种子, sea threshold, grid point, 格点, 噪声, noise, 全地形海平面, all ocean]
env:
  godot: "4.7-stable"
  renderer: "Forward+"
  os: [Windows]
  gpu: "无关（纯数学性质）"
relations: []
---

## 症状识别
- **全部地形在海平面以下**——整个地图是海洋，没有陆地
- 切换 seed 后问题依然存在
- 之前 seed=42 正常运行过，但换个 seed 就全海洋

## 误诊路径
| 尝试过的方案 | 结果 | 为什么无效 |
|-------------|------|-----------|
| 以为是 seed 问题，切换 seed | 无效 | 所有 seed 在 (0,0) 都是 0 |
| `cargo test` 确认噪声函数正常 | 测试通过 | 测试用的坐标不是原点 |

## 根因
**`noise` crate 的 `Perlin::get([0.0, 0.0])` 对所有 seed 返回 0.0**——这是 Perlin 噪声的数学性质，不是 bug。格点 `(0,0)` 处所有梯度的加权和恒为 0。

当 `continent_scale × 0 + 0 = 0`、`sea_threshold = 0.3` 时：原点高度 = 0 < 0.3 → 永远海洋。因为大陆噪声以原点为参考，原点低了整个大陆基准面就低了。

## 解决方案
给噪声坐标加**无理数相位偏移**，使原点不再是格点：
```rust
// noise_gen.rs
let phase_x = 1.0 / PHI;        // 1/φ ≈ 0.618
let phase_z = 1.0 - 1.0 / PHI;  // 1 - 1/φ ≈ 0.382
let nx = x * scale + phase_x;
let nz = z * scale + phase_z;
```
- 原点 `Perlin(0.618, 0.382)` → seed-dependent，不再恒为 0
- `PHI`（黄金比例 φ）是最"无理"的无理数——最大化避免格点对齐
- seed 从 42 调至 99（配合调整）

## 验证方法
1. 启动 Godot → 确认地形有海陆混合
2. 换多个 seed 测试——每个 seed 原点都应不同
3. `cargo test -p woworld_worldgen` — 噪声测试通过

## 代码位置
- `woworld_worldgen::noise_gen::generate_continent_noise()` — 大陆噪声生成
- `woworld_worldgen::terrain::HeightfieldTerrain` — height_at() 使用噪声

## 关联 Bug
- [[TOOL-001]] — 症状相似：修复后看似无效因为 .dll 未更新

## 复发记录
| 日期 | 会话 | 症状是否相同 | 原方案是否有效 | 备注 |
|------|------|-------------|---------------|------|
| — | — | — | — | — |
