//! WoWorld Godot Bridge — GDExtension 入口
//!
//! 编译为 cdylib（动态 C 库），由 Godot 4.7 在运行时通过
//! GDExtension API 加载。
//!
//! 参见 godot-rust 文档: https://godot-rust.github.io/book/

mod terrain_chunk;
pub mod terrain_mesh;

use godot::prelude::*;

/// GDExtension 入口标记类型
struct WoWorldExtension;

#[gdextension]
unsafe impl ExtensionLibrary for WoWorldExtension {}
