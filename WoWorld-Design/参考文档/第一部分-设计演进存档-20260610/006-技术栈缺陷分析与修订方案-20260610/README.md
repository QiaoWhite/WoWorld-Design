# 006-技术栈缺陷分析与修订方案

> **日期**：2026-06-10  
> **前提**：`004-技术栈重新设计` 和 `005-技术栈深入讨论` 已提出 Rust+Godot 混合架构  
> **本批次目的**：诚实地批判该方案，探索替代方案，给出最终修订

## 文档索引

| 编号 | 文档 | 核心内容 |
|------|------|---------|
| 001 | [Rust+Godot混合架构的十大缺陷](001-Rust+Godot混合架构的十大缺陷.md) | 动画规模化（致命）、Node架构开销（高危）、物理分裂（高危）、GDExtension脆弱性等 |
| 002 | [替代架构方案探索](002-替代架构方案探索.md) | 方案A：纯Rust+wgpu、方案B：Rust+Bevy、方案C：C+++Godot；对比矩阵 |
| 003 | [最终修订技术栈方案](003-最终修订技术栈方案.md) | Rust核心+Godot仅做UI/音频/输入；动画和体素渲染迁移到Rust侧GPU计算；渐进实施路径 |

## 核心变化

从 **Rust 模拟 + Godot 全渲染** → **Rust 模拟和核心渲染 + Godot 仅 UI/音频/输入/最终合成**

关键决策：
- NPC 动画：Godot AnimationTree → Rust GPU compute shader
- 体素渲染：Godot Voxel Tools → Rust 自建 Transvoxel + wgpu
- 物理：Jolt (NPC侧) → Rapier
- 集成方式：GDExtension 数据序列化 → 共享 GPU 纹理
