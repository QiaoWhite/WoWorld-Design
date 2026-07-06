---
id: TOOL-001
title: cargo test ≠ cargo build .dll 不更新
type: 🟡反直觉陷阱
module: 工具链
status: ✅已修复
confidence: ✅确信
discovered: 2026-07-03
resolved: 2026-07-03
last_verified: 2026-07-07
grep_keys: [cargo test, cargo build, dll, 不更新, cdylib, 动态库, godot加载, extension, 修复无效, 改了没变化, 没反应]
env:
  godot: "4.7-stable"
  renderer: "无关"
  os: [Windows]
  gpu: "无关"
relations:
  - {target: WG-003, type: 症状相似}  # 修复 WG-003 后因 .dll 未更新，看似修了没效果
---

## 症状识别
- **修改了 Rust 代码，运行 `cargo test` 全绿，启动 Godot——修改没有生效**
- 加日志、改数值、甚至删代码——Godot 里的行为完全不变
- 特别隐蔽：`cargo test` 通过说明代码正确，但 .dll 根本没更新

## 误诊路径
| 尝试过的方案 | 结果 | 为什么无效 |
|-------------|------|-----------|
| 怀疑代码逻辑错了 | 反复检查代码 | 代码可能是对的，只是没加载 |
| 重启 Godot | 无效 | .dll 仍是旧的 |
| 删 target 重编 | 有效但没必要 | 浪费时间——只需 build 不是 clean |

## 根因
**`cargo test` 产出测试可执行文件，`cargo build` 产出 cdylib（动态库）。它们是不同的编译产物。**
- `cargo test` → `target/debug/woworld_godot-*.exe`（测试二进制）
- `cargo build` → `target/debug/woworld_godot.dll`（GDExtension 动态库）
- Godot 加载的是 `.dll` → 只有 `cargo build --workspace` 更新它

## 解决方案
**永远用 `cargo build --workspace` 更新 .dll。** 完整验证序列：
```bash
cargo check --workspace && cargo test --workspace && cargo clippy --workspace -- -D warnings && cargo build --workspace
```
注意：`cargo check` 也不产出 cdylib——check 只做类型检查。

## 验证方法
1. 修改 Rust 代码（如加一个 `println!`）
2. `cargo build --workspace`
3. 用 `_console.exe` 启动 Godot 确认 stdout 出现新日志
4. 如果还是没变化：检查 `WoWorld.gdextension` 中 .dll 路径是否正确

## 代码位置
- `woworld/Cargo.toml` — workspace 清单，woworld_godot crate 类型 = cdylib
- `godot/WoWorld.gdextension` — GDExtension 配置，指向 .dll 路径

## 关联 Bug
- [[WG-003]] — 此陷阱直接导致 WG-003 修复后误判"无效"，浪费诊断时间

## 复发记录
| 日期 | 会话 | 症状是否相同 | 原方案是否有效 | 备注 |
|------|------|-------------|---------------|------|
| — | — | — | — | — |
