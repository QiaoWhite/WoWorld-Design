# Handoff: 2026-07-12 — Sprint-068 V4a 问候/情绪气泡

> **会话类型**: 冲刺执行（场景 B）· **日期**: 2026-07-12
> **阶段**: Phase 2 探针切片「活着的村庄」·进度 `▓▓▓░░░░░░░ 3/10`
> **状态**: ✅ Sprint-068 完成·~1075 tests 全绿·clippy/fmt 零警告·实机验证问候可用·**未提交**（待推送）

## 📊 本会话做了什么

执行 Sprint-068（V4a）。**先经 8+ 轮设计拷问定稿提案**（哲学定位/barrier-free 骨架/10 处审查缺口/5 项裁决 D1-D5），再编码，再**实机验证 + 修 1 个 bug**。

1. **数据驱动**：`SpeechAct`(core) + `assets/speech_fragments.toml`（`FragmentCondition` 富条件）+ `SpeechFragmentRegistry`（概率加权选句）——桩串全外移（003「片段组合」子集）。
2. **遭遇感知层**：`neighbors_within` 原语（social 重构共用）+ `encounter_system`（迟滞/播种/despawn/朝向门）+ `EncounterState`/`EncounterEvent`。
3. **barrier-free 问候**：`speech_bubble_system` 重写——问候/告别接**既有 `ActionIntent` 涌现**（Fight/Flee 否决·表达非决策），单槽仲裁抢占自言自语。
4. **修 [ECS-001](../../bugs/ECS/ECS-001-seeksafety-veto-silences-greetings.md)**：SeekSafety 误否决→全村问候静默；收窄为 Fight/Flee。

## 📦 产物

**代码**（`woworld/`）：`core/speech_bubble.rs`、`assets/speech_fragments.toml`、`ecs/resources/{speech_fragment_registry,encounter_state}.rs`、`ecs/systems/npc/{encounter,speech_bubble,social}.rs`、`godot/terrain_chunk.rs`。
**文档**（`woworld-dev-plan/`）：提案 sprint-068、bug ECS-001、更新 `02-垂直切片/README`(§1+§4)、`附录E`、本 DEVLOG/Handoff。
**记忆**：`player-equals-npc-possession`（玩家=NPC 铁律机制）。

## 💾 恢复点（下一会话 AI 必读）

> ⚠️ 下一会话按 `00-流程总览` **场景 B（冲刺执行）** 启动——V4a 已完成，下一步 **Vf 食物源落地**。

- **当前阶段**: Phase 2 切片·`3/10`（V0+V1+V4a ✅）·10 步序列见 [[../../02-垂直切片/README]] §3
- **下一步**: **Vf 食物源落地**（第 4/10 步·Poisson disc 采集植被·P2.25 最小子集）——见 [[../../sprint-proposals/BACKLOG-Vf-食物源落地-20260712]]。是**独立中等 worldgen 冲刺**，非接线。执行前逐字精读 `世界生成/012`（P2.25）+ `生命/010`。
- **机械门状态**: `~1075 tests 全绿`（core 401 + worldgen 58 + atmosphere 26 + ecs 590 + godot 0），clippy/fmt 零警告，build 通过（DLL 已更新）。
- **提交状态**: ⚠️ **未提交**——本会话所有改动待 `git commit` + push。承接基线 = 提交后的最新 `master`。唯一无关项：`.obsidian/workspace.json`。
- **A1 铁律**: 纯涌现，涌现不出的如实呈现空白，禁脚手架/假坐标/占位驱动/平行 trait/语音决策 silo。

## ⚠️ 遗留 / 诚实边界（探针前哨）

- **实机世界显荒凉（非 bug·探针结论素材）**：needs 累积 → **全 NPC `SeekSafety`** → 无目标四散游荡。真因 = 安全需求无满足路径（V3a/防护未做）+ 无 worldgen 聚落（NPC 在空地）+ 无 V2 牵引移动。问候功能**可用但稀疏**（NPC 分散少相遇）。
- **🟢 需求权重疑似失衡**：`SeekSafety` 一统全村（f600 起 20/20）——安全压过饥饿太快，值得单独评估（超 V4a 范围）。
- **NPC spawn 是占位测试布局**（30m 环·`terrain_chunk.rs:727`）——最终由 worldgen 聚落放置。
- **长线接缝（缝已留·未做）**：完整 003 引擎（CompositeTemplate/填槽）、真语言生成、`SpatialGrid` 收敛（`neighbors_within` 单点替换）、`social_effect` 施加（归 social_system）、记忆形成（`EncounterEvent` 可消费）、非语言联动（010）、LOD 门控、接口契约登记（CLAUDE-INTERFACES/接头总览/附录D）。
- **NPC 旧 `Movement` vs 玩家新 CC 管线**两套移动——债，留玩家化身切片评估合并。

## 🔗 关联

- **提案**: [[../../sprint-proposals/sprint-068-V4a-问候情绪气泡-20260712]]
- **Bug**: [[../../bugs/ECS/ECS-001-seeksafety-veto-silences-greetings]]
- **DEVLOG**: [[../devlogs/DEVLOG-2026-07-12-sprint068]]
- **上游**: [[handoff-20260712-sprint067]] · **路线图**: [[../../02-垂直切片/README]] · [[../../附录E-开发状态]]
