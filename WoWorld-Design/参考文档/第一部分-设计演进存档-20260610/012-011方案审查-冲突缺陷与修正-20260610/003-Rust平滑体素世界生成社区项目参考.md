# 003-Rust 平滑体素世界生成社区项目参考

> 为 WoWorld 的体素地形技术选型提供社区参考  
> 重点关注：平滑连续体素（Marching Cubes / Transvoxel / Surface Nets / Dual Contouring）、程序化地形生成、GPU 加速

---

## 一、核心算法实现（可直接参考的 crate）

### 1.1 transvoxel-rs — Transvoxel 算法的 Rust 实现

| 仓库 | `github.com/platonvin/Transvoxel-rs` |
|------|--------------------------------------|
| 许可证 | MIT |
| 成熟度 | ⭐⭐⭐⭐ 完整的 Transvoxel 实现 |
| 说明 | Lengyel 论文的 Rust 翻译——包含完整的查找表、过渡单元格逻辑、LOD 接缝处理 |

**对 WoWorld 的价值**：这是将 Transvoxel 从 Voxel Tools 迁移到 Rust 侧时最直接的参考实现。可以直接 fork 或作为依赖使用。

```rust
// 使用示例（来自 crate 文档）
use transvoxel::density::Density;
use transvoxel::transition_sides::TransitionSides;
use transvoxel::voxel_source::VoxelSource;

// 给定密度场 → 提取等值面三角形
let mesh = transvoxel::extract(voxel_source, density_threshold);
```

### 1.2 bevy_meshem — Bevy 的体素网格生成

| 仓库 | `github.com/happyrust/bevy_meshem` |
|------|-------------------------------------|
| 许可证 | MIT / Apache 2.0 |
| 成熟度 | ⭐⭐⭐ 活跃开发中（2024 年更新） |
| 说明 | Bevy 插件，提供多种体素 Meshing 算法——包括平滑/dual-contouring 风格 |

**对 WoWorld 的价值**：如果不使用 Bevy，可以参考其 meshing 算法的实现方式——特别是 dual contouring 的 Hermite 数据结构和 QEF（二次误差函数）求解器。

---

## 二、完整体素引擎项目（架构参考）

### 2.1 vx_bevy — Bevy 体素引擎原型

| 仓库 | `github.com/rewin123/vx_bevy` |
|------|-------------------------------|
| Stars | ~325 |
| 许可证 | MIT |
| 技术栈 | Rust + Bevy 0.12 |
| 说明 | 最成熟的 Bevy 体素引擎。特性：greedy meshing per chunk、异步 compute shader 生成网格、多线程地形生成 |

**对 WoWorld 的参考价值**：
- **异步生成架构**：Chunk 的生成和 Meshing 在后台线程完成——不阻塞主渲染循环。这个架构可以直接借鉴到 WoWorld 的 Rust 模拟核心 + Godot 渲染的管线中
- **Greedy Meshing**：对同质体素做面合并。对于 WoWorld 的大面积平原/海洋区域，这可以显著减少三角形数量
- **Chunk 管理**：加载/卸载/优先级队列的实现

### 2.2 dust — 体素全局光照研究引擎

| 仓库 | `github.com/dust-engine/dust` |
|------|-------------------------------|
| Stars | ~85 |
| 许可证 | MIT |
| 技术栈 | Rust + Vulkan + Bevy |
| 说明 | 研究项目——体素几何 + 实时全局光照。使用 dual contouring 概念 |

**对 WoWorld 的参考价值**：
- **Dual Contouring**：与 Marching Cubes 相比，Dual Contouring 可以更好地保留尖锐特征（如建筑的边缘）。对于 WoWorld 的建筑+地形混合场景，这可能比纯 Marching Cubes 更好
- **全局光照**：虽然不是 WoWorld 的优先级，但 GI 的实现方式可以作为远期参考

### 2.3 voxel-game-rs — WGSL Compute Shader 体素游戏

| 仓库 | `github.com/qhdwight/voxel-game-rs` |
|------|-------------------------------------|
| Stars | ~101 |
| 许可证 | — |
| 技术栈 | Rust + Bevy + WGSL Compute Shader |
| 说明 | 大量使用 WGSL compute shader 的体素游戏，有 FPS 控制器 |

**对 WoWorld 的参考价值**：
- **WGSL compute shader 在体素中的应用**：地形生成、Meshing、光照计算——全部在 GPU 上。对于 WoWorld 的 GPU 动画管线，这个项目的 shader 代码是极好的参考
- **Bevy + WGSL 集成**：展示了如何优雅地在 Rust 中管理 GPU compute pipeline

---

## 三、程序化地形生成

### 3.1 Kosmos — 模块化程序化地形生成器

| 仓库 | `github.com/kaylendog/kosmos` |
|------|-------------------------------|
| 许可证 | GPL-3.0 |
| 技术栈 | Rust 后端 + TypeScript 前端 + WebGPU + Bevy |
| 说明 | 类似 Gaea/World Machine 的地形生成工具。GPU 加速的噪声生成（Perlin/Simplex/Worley）、侵蚀模拟、基于图的数据流管线 |

**对 WoWorld 的参考价值**：
- **噪声组合管线**：多噪声叠加 → 生物群系映射 → 侵蚀后处理。这个管线可以直接映射到 WoWorld 的世界生成规格
- **GPU 加速噪声生成**：使用 compute shader 在 GPU 上生成噪声——比 CPU 快 10-100 倍。WoWorld 的"首次世界生成"可以从 CPU 迁移到 GPU
- **侵蚀模拟**：水力侵蚀、热力侵蚀——让程序化生成的地形看起来更自然。Kosmos 的实现可以作为 Rust 侧的初始代码

### 3.2 wgpu-voxel-terrain — WebGPU 体素地形演示

| 仓库 | `github.com/statusfailed/wgpu-voxel-terrain` |
|------|----------------------------------------------|
| 许可证 | — |
| 技术栈 | Rust + wgpu |
| 说明 | 纯 wgpu（无引擎依赖）的体素地形渲染。使用 compute shader 生成地形，带 AO（环境光遮蔽） |

**对 WoWorld 的参考价值**：
- **零引擎依赖**：展示了在没有任何游戏引擎（Godot/Bevy/Unity）的情况下用 Rust + wgpu 渲染体素世界的完整路径。如果未来从 Godot 迁移到纯 Rust 渲染，这是直接的参考
- **AO 实现**：低多边形 + AO 可以显著提升体素世界的视觉质量

---

## 四、大规模体素数据结构

### 4.1 voxelis — 稀疏体素八叉树 DAG

| 仓库 | `github.com/WildPixelGames/voxelis` |
|------|-------------------------------------|
| Stars | ~85 |
| 许可证 | — |
| 技术栈 | 纯 Rust |
| 说明 | 基于 Sparse Voxel Octree DAG 的大规模体素引擎 |

**对 WoWorld 的参考价值**：
- **SVDAG（Sparse Voxel Directed Acyclic Graph）**：极其紧凑的体素存储格式。一个 4km² × 256m 的世界如果使用 SVDAG 可能只需要几十 MB 内存。对于 WoWorld 的远期目标（100K NPC + 大型世界），这种数据结构可能是内存占用的终极优化
- **注意**：SVDAG 是只读的——不适合频繁修改。对于 WoWorld 中 NPC/玩家雕刻地形的需求，需要混合方案：SVDAG 存储"未修改的地形" + patch buffer 存储"被修改的区域"

### 4.2 all-is-cubes — 递归细分方块体素

| 仓库 | `github.com/kpreid/all-is-cubes` |
|------|----------------------------------|
| 许可证 | MIT / Apache 2.0 |
| 技术栈 | Rust + WASM (浏览器可运行) |
| 说明 | 体素引擎，使用递归细分块的概念。方块由更小的方块组成 |

**对 WoWorld 的参考价值**：
- **递归细分**：不是所有地方都需要相同的体素精度。城市区域 0.25m³ 精度，荒野区域 1m³ 精度——递归细分自动处理这种需求
- **WASM 编译**：展示了 Rust 如何编译到浏览器——如果未来 WoWorld 需要一个轻量级的网页版地图查看器或角色展示，这是可行的技术路径

---

## 五、最适合 WoWorld 直接使用的组合

| 组件 | 推荐来源 | 方式 |
|------|---------|------|
| **Transvoxel 等值面提取** | `transvoxel-rs` crate | 直接依赖或 fork |
| **噪声生成** | `noise` crate + Kosmos 的 GPU compute 噪声 | 标准依赖 + 远期参考 |
| **侵蚀模拟** | Kosmos 的侵蚀算法 | 移植代码片段 |
| **Chunk 管理** | vx_bevy 的异步分帧架构 | 架构参考，用 Godot API 重写 |
| **大规模存储** | voxelis 的 SVDAG 概念 | 远期参考，中期用密度数组 |
| **GPU compute 管线** | voxel-game-rs 的 WGSL shader | Shader 代码参考 |

---

## 六、注意事项

- **GPL-3.0 项目（Kosmos）**：GPL 的代码不能直接整合到 WoWorld（如果你的游戏是闭源的）。但可以参考其算法思想，用 MIT/Apache 2.0 代码重新实现
- **Bevy 依赖**：vx_bevy、voxel-game-rs、dust 都依赖 Bevy。WoWorld 使用 Godot——参考它们的算法和架构，但不直接使用代码
- **代码成熟度**：所有列出的项目都是社区维护的。在生产中使用前需要充分测试

---

*搜索日期：2026-06-10*  
*工具：Web Search + crates.io + GitHub Topics*
