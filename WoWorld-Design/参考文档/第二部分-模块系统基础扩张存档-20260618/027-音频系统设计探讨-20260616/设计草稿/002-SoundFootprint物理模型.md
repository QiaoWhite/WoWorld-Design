# 002-SoundFootprint 物理模型（大纲）

> **开发代号**: WoWorld (Wonder World)
> **状态**: 设计草稿（大纲·要点）

---

## 一、核心原则

- SoundFootprint 描述**物理现实**（表面接触+运动+发声），不描述游戏语义
- 音频模块拥有"物理→声音"的映射逻辑
- 所有类型在 woworld_types

## 二、关键结构

### SoundFootprint
- surface_contacts: [SurfaceContact; ≤4] — 材质+速度+力度+接触类型
- motion: MotionSnapshot — 速度/加速度/角速度/介质/步态/质量/体积
- vocalization: Option<VocalizationState> — 发声类型+强度+频率+ExpressionRef
- item_interaction: Option<ItemInteraction> — 交互类型+材质+力度
- silence: Option<SilenceIntent> — 潜行/躲藏/魔法沉默/装死/憋气

### ActionRingBuffer
- 每实体 64 条×16B 环形记录
- 各模块本来就要记录"最近做了什么"——音频多一个消费者
- ActionAtom：~40 变体（Step/Land/Jump/Swing/Strike/CastStart/Explosion/...）

### SurfaceMaterial
- ~20 变体：BareEarth/Grass/Stone/Gravel/Sand/SnowFresh/SnowPacked/Ice/Mud/WoodPlank/WoodSolid/MetalGrate/Carpet/Tile/WaterShallow/WaterDeep/Lava/Glass/Bone/Flesh

### AudioMaterial（装备材质·对标ConsumableEffect模式）
- ~15 变体：Cloth/Leather/ChainMail/PlateArmor/Wood/Bone/Scale/Chitin/Glass/Crystal/Stone/...
- 音频模块定义枚举，物品系统在 ItemProperties 上存储

### SilenceIntent
- Sneaking/Hiding/MagicallySilenced/Dead/Unconscious/HoldingBreath/PlayingDead
- NPC 感知"太安静了"的信息来源

### PlayerSoundFootprint
- 玩家专用——Godot 每帧填充→GDExtension（玩家没有 action_log/NpcData）
- 包含 motion/surface_contacts/equipment_materials/body_audio

### HasSoundEmitter trait（持续声源）
- active_sounds() → [ContinuousSound; ≤4]
- 引擎/结界/瀑布/篝火/齿轮/心跳...

### 身体声音——直接读 Vitals
- 心跳：health<0.3 加强 + fear×0.5 + exertion×0.3
- 呼吸：stamina<0.3 触发粗重呼吸
- 肚子叫：hunger>0.6 概率性事件

## 三、性能

- SoundFootprint ~128B/实体·帧
- ActionRingBuffer ~1KB/实体
- 10,000 实体 ~11.5MB 合计
