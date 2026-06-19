# 002-跨模块trait接口与数据合同

> **状态**: 开发规格 v1.0
> **日期**: 2026-06-17
> **关联**: [[001-外部依赖与接口审计|外部依赖审计]] · [[001-模型动作与物理系统总纲|系统总纲]] · [[001-空间查询与物理系统|空间查询系统]]

---

## 〇、本模块定义的所有 trait（woworld_core）

### 0.1 空间查询 trait

```rust
// ── 地形查询——纯函数，world_gen 实现 ──
pub trait TerrainQuery: Send + Sync {
    fn height_at(&self, x: f64, z: f64) -> f64;
    fn normal_at(&self, pos: Vec3) -> Vec3;
    fn terrain_raycast(&self, origin: Vec3, dir: Vec3, max_dist: f32) -> Option<TerrainHit>;
    fn density_at(&self, pos: Vec3) -> (f32, u8);
    fn is_walkable(&self, pos: Vec3) -> bool;
    fn surface_material_at(&self, pos: Vec3) -> SurfaceMaterial;
    fn medium_at(&self, pos: Vec3) -> Medium;
    fn light_level_at(&self, pos: Vec3, time: GameInstant) -> f32;
    fn sample_horizon(&self, origin: Vec3, azimuth: f32) -> HorizonSample;
}

// ── 实体索引——可变数据结构，woworld_spatial 实现 ──
pub trait EntityIndex: Send + Sync {
    fn register(&mut self, entity: SpatialEntity);
    fn unregister(&mut self, id: EntityId);
    fn update_transform(&mut self, id: EntityId, transform: Mat4);
    fn entities_in_aabb(&self, aabb: Aabb, filter: EntityFilter) -> &[SpatialEntity];
    fn entity_aabb(&self, id: EntityId) -> Option<Aabb>;
    fn acoustic_tag_at(&self, pos: Vec3) -> AcousticTag;
}

// ── 空间事件总线——ephemeral append-only，woworld_spatial 实现 ──
pub trait SpatialEventBus: Send + Sync {
    fn recent_events_in(&self, aabb: Aabb, since: GameInstant) -> &[SpatialEvent];
    fn push_event(&self, event: SpatialEvent);
    fn scent_sources_in(&self, aabb: Aabb) -> &[ScentSource];
}

// ── 视线检测——跨 terrain+entity DDA，woworld_spatial 实现 ──
pub trait VisibilityQuery: Send + Sync {
    fn line_of_sight(&self, from: Vec3, to: Vec3, max_dist: f32) -> bool;
    fn line_of_sight_hit(&self, from: Vec3, to: Vec3, max_dist: f32) -> Option<VisibilityHit>;
}
```

### 0.2 动画系统 trait

```rust
pub trait SkeletonProvider: Send + Sync {
    fn skeleton(&self, id: SkeletonId) -> Option<&SkeletonDef>;
    fn body_plan(&self, id: SkeletonId) -> BodyPlan;
}

pub trait PoseProvider: Send + Sync {
    fn module_pose(&self, id: ModulePoseId) -> Option<&ModulePose>;
    fn primitive_trajectory(&self, primitive: ActionPrimitive) -> Option<&PrimitiveStrike>;
    fn gait_params(&self, style_id: GaitStyleId) -> Option<&GaitStyleParams>;
}

pub trait TransitionProvider: Send + Sync {
    fn transition(&self, from: ActionId, to: ActionId) -> AnimationTransition;
}

pub trait AnimationDataProvider:
    SkeletonProvider + PoseProvider + TransitionProvider + Send + Sync {}

pub trait AnimationLayer: Send + Sync {
    fn apply(&self, local: &mut [Mat4; 50], skeleton: &SkeletonDef,
             state: &AnimationBodyState, dt: f32);
    fn priority(&self) -> LayerPriority;
}

/// MOD 自定义接口
pub trait CustomAnimationLayer: AnimationLayer {
    fn layer_name(&self) -> &str;
    fn enabled_for(&self, entity_id: EntityId, action: ActionId) -> bool;
}

pub trait CustomGaitGenerator: Send + Sync {
    fn gait_name(&self) -> &str;
    fn applicable_to(&self, body_plan: BodyPlan) -> bool;
    fn generate(&self, params: &GaitStyle, phase: f32, dt: f32) -> [Mat4; 50];
}

pub trait CustomPrimitiveTrajectory: Send + Sync {
    fn primitive_name(&self) -> &str;
    fn applicable_to(&self, weapon_type: WeaponType) -> bool;
    fn strike_frames(&self) -> &[StrikeKeyFrame];
}

pub trait CustomFacialMapper: Send + Sync {
    fn map_emotion_to_face(&self, emotion: &EmotionState, personality: &BigFive) -> FacialExpression;
}
```

---

## 一、关键数据结构

### 1.1 骨架

```rust
pub struct SkeletonDef {
    pub id: SkeletonId,
    pub body_plan: BodyPlan,
    pub bones: Vec<BoneDef>,
    pub animation_modules: AnimationModules,
    pub attachment_points: Vec<AttachmentPoint>,
    pub joint_limits: Vec<JointLimit>,
    pub root_bone: usize,
}

pub struct BoneDef {
    pub name: String,
    pub parent: Option<usize>,
    pub rest_transform: Mat4,
    pub length: f32,
    pub mass: f32,
}

pub struct AnimationModules {
    pub lower_body: Vec<usize>,
    pub upper_body: Vec<usize>,
    pub head_neck: Vec<usize>,
    pub face_hands: Vec<usize>,
}

pub struct AttachmentPoint {
    pub name: String,
    pub bone_index: usize,
    pub offset: Mat4,
}
```

### 1.2 姿态

```rust
pub struct ModulePose {
    pub id: ModulePoseId,
    pub module: AnimationModule,
    pub bone_transforms: Vec<BoneLocalTransform>,
    pub sound_markers: Vec<AnimationSoundMarker>,
}

pub struct BoneLocalTransform {
    pub translation: Vec3,
    pub rotation: Quat,
}

pub struct PrimitiveStrike {
    pub primitive: ActionPrimitive,
    pub trajectory: TrajectoryCurve,
    pub key_bone_frames: Vec<StrikeKeyFrame>,
    pub duration_ms: (f32, f32),
    pub residual_energy: f32,
}
```

### 1.3 动画状态（每 NPC，不持久化，降级丢弃）

```rust
pub struct AnimationBodyState {
    pub bone_world_transforms: [Mat4; 50],
    pub bone_velocities: [Vec3; 50],
    pub bone_local_transforms: [Mat4; 50],
    pub center_of_mass: Vec3,
    pub com_velocity: Vec3,
    pub foot_contact: [Option<FootContact>; 2],
    pub balance: BalanceState,
    pub current_action: ActionId,
    pub action_progress: f32,
    pub interrupt_recovery: Option<InterruptRecovery>,
    pub mode: AnimationMode,
    pub limb_splay: f32,
    pub death_progress: f32,
    pub spine_state: SpineState,
    pub gait_style: GaitStyle,
    pub skeleton_lod: SkeletonLod,
    pub lod_transition: Option<LodTransition>,
    pub transition_com_velocity_bias: Vec3,
    pub facial: FacialExpression,
}

pub enum BalanceState {
    Stable,
    Staggering(f32),
    Falling(f32),
    Grounded,
    Airborne,
    Sliding(Vec3),
}

pub enum AnimationMode {
    Daily,
    Combat,
    PhysicsBody,
    Transition,
}
```

### 1.4 步态与战斗风格（涌现式——从 NPC 数据派生，不存储）

```rust
pub struct GaitStyle {
    pub hip_sway: f32,
    pub stride_length: f32,
    pub arm_swing: f32,
    pub bounce: f32,
    pub forward_lean_deg: f32,
    pub rhythm_regularity: f32,
    pub foot_drag: f32,
    pub shoulder_stability: f32,
    pub gaze_level_deg: f32,
}

pub struct CombatStyleParams {
    pub speed_over_power: f32,
    pub flourish: f32,
    pub guard_height: f32,
    pub guard_width: f32,
    pub pacing_ms: f32,
    pub mobility: f32,
    pub feint_tendency: f32,
    pub commitment_threshold: f32,
}
```

### 1.5 面部表情（6 字节，塞入 INSTANCE_CUSTOM）

```rust
#[repr(C)]
pub struct FacialExpression {
    pub mouth_shape: u8,      // 0-15
    pub brow_shape: u8,       // 0-15
    pub eye_state: u8,        // 0-7
    pub iris_offset_x: i8,    // -127~127
    pub iris_offset_y: i8,    // -127~127
    pub blush_intensity: u8,  // 0-255
}
```

---

## 二、SpatialEntity 写入协议

各模块创建/更新实体时→写入 `EntityIndex`：

| 模块 | entity_kind | 关键字段 |
|------|------------|---------|
| Life | Creature | velocity, center, body_plan, skeleton_id |
| Items | Item | center, item_assembly_type |
| World Gen | Building/Terrain/Vegetation | center（静态） |
| Vehicles | Vehicle | velocity, mount_points |
| Magic | Effect | velocity, light_emission, duration |
| Combat | Projectile | velocity, weapon_type |

NPC/玩家死亡时不注销——velocity→0，entity_kind 保持 Creature。尸体处理由 Life 模块负责。

---

## 三、SpatialEvent 写入协议

任何模块产生显著物理事件→写入当前 Chunk 的 event ring buffer：

```rust
spatial.push_event(SpatialEvent {
    event_type: SpatialEventType::LoudSound { peak_db: 95.0, frequency_profile: ... },
    position: explosion_center,
    intensity: 0.8,
    timestamp: now,
    source_entity: Some(bomb.id),
    duration: TimeDelta::from_millis(500),
    propagation_medium: Medium::Air,
});
```

事件自动过期：`max(intensity × 10s, 5s)`。Chunk ring buffer：64 事件，LRU 淘汰。

---

## 四、动画→音频同步

动画系统的关键帧携带 `AnimationSoundMarker`。每帧动画 tick 时检查当前动作进度→触发标记→推送 `SpatialEvent`→音频系统通过 `SpatialEventBus` 消费。

音效标记定义在 TOML 的 `ModulePose` 数据中——可在动画里自定义音效触发点。

---

## 五、GDExtension 数据合同

### 5.1 Rust→Godot（每帧）

```rust
/// 双缓冲共享内存布局（每 NPC 条目，64 字节对齐）
#[repr(C)]
struct NpcRenderEntry {
    world_position: [f32; 3],    // 12B
    scale: f32,                  // 4B
    bone_matrices: [[f32; 16]; 33],  // 2112B (33×64B)
    per_instance: PerInstanceData,   // 16B (FacialExpression 6B + action 4B + emotion 4B + fatigue 2B)
}
// 总计 2144B/NPC，对齐到 2176B（cache line 友好）
// 1000 NPC = 2.12MB buffer × 2（双缓冲）= 4.24MB 共享内存
```

### 5.2 Godot→Rust（每帧）

```rust
struct CameraState {
    position: [f32; 3],
    forward: [f32; 3],
    fov_radians: f32,
}
// 28 bytes
```

### 5.3 Godot Shader Uniforms

```glsl
uniform sampler2D bone_data;      // 33 bones × 4 columns × N instances（RGBA32F）
uniform sampler2D facial_atlas;   // 512×512 面部图集（共享）
uniform uint instance_count;
uniform uint bones_per_instance;  // 33
```

---

## 六、TOML 配置索引

| 文件 | 内容 | 格式 |
|------|------|------|
| `models/skeletons.toml` | SkeletonDef 定义 | `[[skeletons]]` |
| `models/model_variants.toml` | ModelVariant→SkeletonId 映射 | `[[model_variants]]` |
| `models/weapon_physics.toml` | WeaponPhysicalParams 数据库 | `[[weapons]]` |
| `animations/poses.toml` | 38 个 ModulePose 关键帧数据 | `[[poses]]` |
| `animations/trajectories.toml` | 15 条 PrimitiveStrike 轨迹 | `[[trajectories]]` |
| `animations/gait_styles.toml` | 步态风格参数覆盖 | `[[gait_styles]]` |
| `animations/combat_styles.toml` | 战斗风格 + 基元加权 | `[[combat_styles]]` |
| `animations/transitions.toml` | 动作对过渡参数 | `[[transitions]]` |
| `animations/facial_atlas.toml` | 面部图集区域配置 | `[[facial_regions]]` |
| `interactions/interactions.toml` | 双人交互模板 | `[[interactions]]` |

---

> **下一个文档**: [[001-模型动作与物理系统总纲|系统总纲]]
