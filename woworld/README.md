# WoWorld — Rust Workspace 项目

> **开发代号**: WoWorld (Wonder World)
> **引擎**: Godot 4.7 | **模拟语言**: Rust 1.80+
> **当前阶段**: 轨A 阶段一 — 项目脚手架

## 目录结构

```
woworld/
├── Cargo.toml                  # workspace 清单
├── crates/
│   ├── woworld_core/           # 核心类型 + trait 定义（零依赖）
│   └── woworld_godot/          # GDExtension 桥接（cdylib → Godot）
├── godot/                      # Godot 4.7 项目
│   ├── project.godot
│   ├── WoWorld.gdextension     # GDExtension 配置文件
│   ├── scenes/
│   └── scripts/
├── assets/                     # TOML 数据文件（共享）
└── README.md
```

## 快速开始

### 前置条件

- Rust 1.80+ (`rustup update stable`)
- Godot 4.7 (`tools/godot/Godot_v4.7-stable_win64.exe`)

### 构建

```bash
# 编译 Rust workspace（含 GDExtension 动态库）
cargo build --workspace

# 启动 Godot 编辑器
../tools/godot/Godot_v4.7-stable_win64.exe godot/project.godot
```

### 开发工作流

1. 修改 Rust 代码 → `cargo build` → Godot 4.2+ 自动热重载扩展
2. Godot 场景/资源在 `godot/` 目录中编辑
3. TOML 数据文件（群系、物品等）放在 `assets/`

## 技术栈

- **引擎**: Godot 4.7 (Forward+ 渲染器)
- **模拟语言**: Rust stable 1.80+
- **GDExtension**: godot-rust (gdext) 0.5.x
- **体素**: 自建 Transvoxel（Rust 侧）→ Godot ArrayMesh
- **数据库**: LMDB（Phase 2+）
- **目标硬件**: GTX 1660 SUPER 6GB VRAM

## 相关文档

- 设计规格: `../WoWorld-Design/`
- 开发路线图: `../WoWorld-Design/开发路线图/`
- godot-rust 文档: https://godot-rust.github.io/book/
