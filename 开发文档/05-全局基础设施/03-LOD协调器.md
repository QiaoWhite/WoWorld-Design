# LOD 协调器 — ECS 架构

> **关联原文档**: [[CHG-049-LOD架构全面深化-20260620]]
> **关联**: [[../00-ECS哲学与架构总纲/002-Component拆装机制]] · [[../00-ECS哲学与架构总纲/005-调度模型]]

---

## 这个模块关心什么"细节"

LODCoordinator 决定每个 Entity 在当前帧的**计算精度预算**。它不是一个"流程步骤"——它是一个独立的 System，每帧为所有 Entity 写入 `LodLevel` Component。其他 System 根据自己的需求查询合适的 LodLevel。

---

## Component 定义

### LodLevel（核心输出）

```rust
/// 每实体 LOD 等级——由 LodCoordinatorSystem 每帧写入
#[derive(Debug, Clone, Copy)]
struct LodLevel {
    scene_lod: u8,      // 0-7, 地形/建筑/海洋/云/植被
    skeleton_lod: u8,   // 0-4, 骨骼精度
    animation_lod: u8,  // 0-4, 动画层数
    render_lod: u8,     // 0-4, 渲染精度
    physics_lod: u8,    // 0-4, 物理精度
    audio_lod: u8,      // 0-4, 音频精度
    ai_lod: u8,         // 0-4, AI 精度
}
impl Component for LodLevel {}
```

| 字段 | 含义 | 0 = 最高精度 | 最大值 = 最低精度 |
|------|------|------------|-----------------|
| scene_lod | 场景渲染 | 0.5m体素 | 64m Billboard |
| skeleton_lod | 骨骼 | 35骨 | 0骨 |
| animation_lod | 动画 | 9层全栈 | 无动画 |
| render_lod | 渲染面数 | 1500面 | 不可见 |
| physics_lod | 物理 | 全碰撞+IK | 无碰撞 |
| audio_lod | 音频 | 全传播 | 静默 |
| ai_lod | AI | 全GOAP | 仅存在 |

---

## System 定义

### LodCoordinatorSystem

- **触发条件**: 每帧执行（Phase 0，必须先运行）
- **读**: `&CameraState`（Resource）, `&FrameBudget`（Resource）, `&VramPressure`（Resource）
- **读 Entity**: `&Position`（所有有 Position 的 Entity）
- **写**: `&mut LodLevel`（通过 CommandBuffer 或直接写入）
- **逻辑**: 完整 8 步算法（基础分配→硬约束→级联拉升→Attention→VRAM降级→帧预算降级→Clamp→迟滞）

```
Step 1: 距离 → scene/char/audio LOD
Step 2: 玩家=0, 战斗=0, 交互目标拉升
Step 3: 级联拉升 (预算制, <=0.3ms)
Step 4: 30°视线锥→各+1档; 聚焦目标→拉满
Step 5: VRAM≥70%→scene+1; ≥85%→scene/render+1; ≥95%→全局+1
Step 6: 帧预算<3ms→远距NPC降级; <1.5ms→中距; <0.5ms→紧急降级
Step 7: skeleton=4→anim/render/physics=4 等 6 条约���
Step 8: 降级延迟500ms, 升级即时
```

- **与其他 System 的关系**: 无——其他 System 通过查询 `LodLevel` 组件自行决定精度，不与被本 System 通信

---

## 其他 System 如何消费 LodLevel

每个 System 自行查询 Entity 的 LodLevel，决定是否处理该 Entity：

```rust
// AI System: 只处理 ai_lod <= 2 的 NPC
fn goap_system(world: &hecs::World, cmd: &mut CommandBuffer) {
    for (e, (lod, npc, goal)) in world.query::<(&LodLevel, &NpcCore, &mut Goal)>().iter() {
        if lod.ai_lod > 2 { continue; }  // 低精度 NPC 跳过 GOAP
        
        // 正常 GOAP 规划
    }
}

// Animation System: 根据 animation_lod 选择动画层数
fn animation_system(world: &hecs::World) {
    for (e, (lod, skeleton)) in world.query::<(&LodLevel, &Skeleton)>().iter() {
        let layers = match lod.animation_lod {
            0 => 9,  // 全栈
            1 => 6,  // 核心层
            2 => 4,  // 行为层
            3 => 2,  // 姿态+战斗
            _ => 0,  // 无动画
        };
        apply_animation(e, layers);
    }
}
```

**关键**：LodCoordinatorSystem 不"命令"其他 System 做什么——它只是写入 `LodLevel`。每个消费 System 自行解释 `LodLevel` 的含义。这是解耦的核心。

---

## Resource 依赖

| Resource | 用途 | 提供者 |
|----------|------|--------|
| CameraState | 玩家相机位置+FOV | WorldDriver (从 Godot Camera3D 每帧提取) |
| FrameBudget | 帧时间预算 | WorldDriver (从 delta 计算) |
| VramPressure | VRAM 使用率 | WorldDriver (Godot RenderingServer 查询, 暂无则 default) |

---

## 与 Phase 1 的关系

LodCoordinatorSystem 在 **Phase 0** 执行（必须先于所有游戏逻辑 System）。

为什么它在 Phase 0 而不是 Phase 1？因为其他 System 需要在本帧内读到最新的 LodLevel，而不是下帧。这是一个**极少数**的、有正当理由的顺序依赖——LOD 信息必须在其他所有 System 处理实体之前就绪。

它不是"before/after 耦合"——它是"基础设施必须先就绪"。就像体检机构必须先开门，各个部门才能开始工作。

---

## 新想法接入点

如果未来想加入"玩家注意力热力图"来影响 LOD：

1. 定义 `AttentionHeatmap` Resource
2. LodCoordinatorSystem 在 Step 4 读取 `AttentionHeatmap`
3. 不需要改其他 System——它们继续读 `LodLevel`，不关心 LOD 是怎么算出来的

## 实现回查索引
> ⚠️ 以下实现必须回查原文档。

| 实现任务 | 必须回查 |
|---------|---------|
| 8 步算法完整规格 | [[CHG-049-LOD架构全面深化-20260620#§五 LODCoordinator 完整设计]] |
| 场景距离带 (8 层) | [[CHG-049-LOD架构全面深化-20260620#2.1 场景线]] |
| 角色距离带 (5 层) | [[CHG-049-LOD架构全面深化-20260620#2.2 角色线]] |
| 音频距离带 | [[CHG-049-LOD架构全面深化-20260620#2.3 音频基础距离带]] |
| 跨维硬约束 7 条 | [[CHG-049-LOD架构全面深化-20260620#§四 跨维硬约束]] |
| VRAM 降级 3 阈值 | [[CHG-049-LOD架构全面深化-20260620#Step 5 VRAM 压力降级]] |
| 帧预算降级 3 阈值 | [[CHG-049-LOD架构全面深化-20260620#Step 6 帧预算降级]] |
| 级联拉升预算制 | [[CHG-049-LOD架构全面深化-20260620#§六 级联交互机制]] |
