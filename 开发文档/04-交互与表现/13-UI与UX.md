# UI 与 UX — ECS 架构

> **关联原文档**: [[开发阶段/UI与UX系统/README]]

---

## 不进 ECS

UI 系统**整个不进 ECS**。

**原因**：
- UI 在 Godot/GDScript 侧实现
- UI 的更新频率和生命周期与游戏模拟不同
- ECS 不适合 UI（UI 是树形结构，不是扁平实体集）

## 与 ECS 的桥接

Rust 侧通过 Resource 暴露 UI 需要的数据：

```rust
struct UiState {
    player_vitals: Vitals,
    player_skills: Vec<(SkillId, u8)>,  // 技能列表+等级
    quest_log: Vec<QuestEntry>,
    minimap_data: MinimapSnapshot,
}
```

`UiSyncSystem` (Phase 2) 每帧更新 `UiState` → Godot UI 消费。
