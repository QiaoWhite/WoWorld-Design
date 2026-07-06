---
id: WG-002
title: Transvoxel y-z stride swap 双向抵消
type: 🟡反直觉陷阱
module: 世界生成
status: ✅已修复
confidence: ✅确信
discovered: 2026-06-29
resolved: 2026-06-29
last_verified: 2026-06-29
grep_keys: [stride, swap, y-z, 狼牙棒, spike, 凸起, bump, transvoxel, marching cubes, 密度数组, 索引错误, density array, global_corners, corner_pos, edge cache]
env:
  godot: "4.7-stable"
  renderer: "Forward+"
  os: [Windows]
  gpu: "无关（纯 Rust 计算错误）"
relations:
  - {target: WG-001, type: 同根}  # 同在 clipmap 渲染管线中暴露
---

## 症状识别
- 地形表面出现**山体凸起（bumps）和裂缝（cracks）**
- 绿色多边形交错闪烁（transition cells 的附加症状）
- 伴随 transition cells 错误——大面积绿色伪影
- `cargo test` **全部通过**（因为 MC 和 Transvoxel 有相同的错误，互相抵消）

## 误诊路径
| 尝试过的方案 | 结果 | 为什么无效 |
|-------------|------|-----------|
| 回退新增 LOD 6-7 | 无效 | 基线代码已有此 bug |
| 回退 Part A 代码去重 | 无效 | 去重未引入 bug |
| 全 SH 替换 Transvoxel | 问题消失 | 确认 bug 在 Transvoxel 路径 |
| 移除洞穴密度层 | 无效 | 不是洞穴层问题 |
| 禁用 transition cells | 绿色伪影消失，bumps+cracks 残留 | 确认是两个独立 bug |

## 根因
`marching_cubes.rs` 和 `transvoxel.rs` 的密度数组访问中，**y 和 z 的 stride 被互换**：
- 正确：x stride = 1, y stride = nx, z stride = nx·ny
- 错误（代码）：x stride = 1, y stride = **nx·ny**（应 nx）, z stride = **nx**（应 nx·ny）

两个文件有**相同的**错误 stride swap——密度值从错误位置读取，但 MC 提取的等值面和 Transvoxel 过渡面使用同样的错误坐标，导致测试中互相抵消、不可检测。

## 解决方案
在 `marching_cubes.rs` 和 `transvoxel.rs` 中修正 d 数组索引和 `global_corners` 函数：
- `n_x` 和 `n_y * n_z` 互换
- 两个文件必须**同时**修正——只修一个会暴露另一个的 bug
- 同时修正 edge cache 键值计算（也受 stride 影响）

## 验证方法
1. `cargo test -p woworld_worldgen` 全部通过
2. 启动 Godot → 地形无异常凸起/裂缝
3. 目视确认：山体形状自然，无"狼牙棒"式尖刺

## 代码位置
- `woworld_worldgen::marching_cubes::density_at()` — d 数组索引
- `woworld_worldgen::transvoxel::global_corners()` — 角落坐标计算
- `woworld_worldgen::transvoxel::edge_cache_key()` — 边缓存键值

## 关联 Bug
无

## 复发记录
| 日期 | 会话 | 症状是否相同 | 原方案是否有效 | 备注 |
|------|------|-------------|---------------|------|
| — | — | — | — | — |
