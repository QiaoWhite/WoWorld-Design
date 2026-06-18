# 010-性能预算与L分层

> **状态**: 开发规格 v1.0
> **关联**: [[001-感官系统总纲|总纲]] · [[../技术栈方案/001-WoWorld正式技术栈方案v3|技术栈v3]]

---

## 〇、核心原则

感知模型是**距离的连续函数**——不预设 N 层。层级只是**批量调度优化**——把更新频率相近的 NPC 放在同一调度桶中。层级数量可随意增减，不改变感知算法。

---

## 一、感知退化——距离的连续函数

```rust
fn perception_at_distance(distance_m: f32, npc: &NpcData) -> PerceptionDegradation {
    PerceptionDegradation {
        vision_range_m: npc.sensory.vision.max_range_m * vision_falloff(distance_m),
        max_visual_detail: max_detail_at(distance_m),
        hearing_range_m: npc.sensory.hearing.max_range_m * hearing_falloff(distance_m),
        hearing_threshold_offset_db: 20.0 * log10(distance_m / 1.0),
        tick_interval: tick_interval_at(distance_m),
        event_query_radius_m: event_radius_at(distance_m),
        min_event_intensity: min_intensity_at(distance_m),
    }
}
```

---

## 二、推荐调度层级

| 层级 | 距离 | 更新频率 | 视觉射线 | 事件通道 | NPC数(估算) |
|------|------|---------|---------|---------|------------|
| L0 | 0-10m | 0.1s | 全预算 | 全部事件 | ≤200 |
| L1 | 10-50m | 0.3s | 标准预算 | 全部事件 | ≤1,000 |
| L2a | 50-100m | 0.5s | 半预算 | 高强度事件 | ≤3,000 |
| L2b | 100-200m | 1s | 少量射线 | 极高强度事件 | ≤7,000 |
| L3a | 200-500m | 5s | 无射线 | 仅灾难级事件 | ~20,000 |
| L3b | 500m-2km | 30s | 无 | 无个体事件(聚合) | ~30,000 |
| L4 | >2km | 1d+ | 无 | 宏观统计 | ~数百万 |

---

## 三、Phase 偏移调度

```rust
// NPC 初始化时——从 NpcId hash 派生相位偏移
let phase_offset = hash(npc.id) % total_phases;

// 每帧:
for npc in npcs_where(phase_matches_frame(npc.phase_offset, frame)):
    if npc.decision_timer >= npc.decision_interval:
        percepts = sensory_shell.perceive(npc, ...);
        npc.decision_timer = 0;
```

决策周期不固定（0.3-5s），但相位偏移终身不变。不需要全局调度器。

---

## 四、性能预算总表

| 指标 | 预算值 | 计算依据 |
|------|--------|---------|
| L1 NPC 感知调用/帧 | ~17 | 1000 L1 ÷ 60帧（相位分散） |
| 每次调用视觉射线 | ≤8（自适应） | 环境/状态/负载调制 |
| 射线/帧 | ~136 | 17 × 8 |
| 射线单次开销 | ~10µs | DDA在稀疏体素上 |
| 射线总开销/帧 | ~1.4ms | — |
| 每次调用总开销 | ≤0.05ms | 含分类+射线+噪声+场景 |
| 总感知 CPU/帧 | ≤1ms | ~17调用 × 0.05ms |
| PerceptualCache/L1 NPC | ~6KB | 64实体 × ~80B |
| PerceptualCache总内存 | ~6MB | 1000 L1 × 6KB |
| Knowledge/NPC | ~2.5KB | 64条 × ~40B |
| AestheticFrameworks/NPC | ~120B | ≤5框架 × ~24B |
| SpatialEventBuffer/Chunk | ~4KB | 64事件 × ~64B |
| 活跃Chunk数(L1范围) | ~200 | 50m半径 ~ 16mChunk |

**远低于帧预算 16.7ms 和总体内存预算。**

---

## 五、L1↔L2 过渡

升级（L2→L1）：空缓存→前10周期新奇驱动探索→"东张西望"涌现行为→自然稳定。不需预热。

降级（L1→L2）：PerceptualCache 丢弃——下次升级时空缓存重新初始化。自然涌现"环顾四周"。

**不持久化 PerceptualCache。不持久化 DarkAdaptation。读档=重新进入世界。**

---

> **相关**: [[../技术栈方案/001-WoWorld正式技术栈方案v3|技术栈方案v3.0 硬件目标]]
