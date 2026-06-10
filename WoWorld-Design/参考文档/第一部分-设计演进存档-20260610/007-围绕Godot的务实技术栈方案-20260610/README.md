# 007-围绕 Godot 的务实技术栈方案

> **日期**：2026-06-10  
> **前提**：004-006 从 Godot 全栈 → Rust+Godot 混合 → Rust wgpu 渲染的演化路径  
> **本批次目的**：批判 006 的自建渲染器方案，探索在 Godot 内解决规模化问题的务实路径

## 文档索引

| 编号 | 文档 | 核心内容 |
|------|------|---------|
| 001 | [006修订方案的缺陷：自建渲染器的真实代价](001-006修订方案的缺陷：自建渲染器的真实代价.md) | 自建 wgpu 渲染器需 17~36 人·月；光照/阴影/后处理全部需自建；GPU compute 动画调试困难；共享纹理跨平台兼容噩梦 |
| 002 | [围绕Godot的规模化方案：用对工具而非换工具](002-围绕Godot的规模化方案：用对工具而非换工具.md) | MultiMesh 批量渲染、GPU skinning shader、PhysicsServer3D API 无节点物理、AudioServer API、体素分工（Rust生成+Godot渲染） |
| 003 | [最终务实方案：Godot优先+Rust模拟核心](003-最终务实方案：Godot优先+Rust模拟核心.md) | 完整技术栈、架构图、3 个 Phase 实施路径、风险矩阵、与 006 方案对比 |

## 核心结论

**不要因为 Godot 的 AnimationTree 不能规模化就把渲染器搬到 Rust。** 用 MultiMesh + GPU skinning shader + PhysicsServer API 在 Godot 内解决规模化问题。Rust 负责 CPU 密集计算（模拟+骨骼矩阵+Transvoxel），Godot 负责渲染+工具链。

新增工作量：3~6 人·月（vs 006 的 17~36 人·月）。
