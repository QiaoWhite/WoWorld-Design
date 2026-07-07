//! WoWorld Godot Bridge — GDExtension 入口
//!
//! 编译为 cdylib（动态 C 库），由 Godot 4.7 在运行时通过
//! GDExtension API 加载。
//!
//! 参见 godot-rust 文档: https://godot-rust.github.io/book/

pub mod debug_console;
mod entity_renderer;
mod ocean;
mod terrain_chunk;
mod voxel_chunk;

use godot::prelude::*;

/// GDExtension 入口标记类型
struct WoWorldExtension;

#[gdextension]
unsafe impl ExtensionLibrary for WoWorldExtension {}
