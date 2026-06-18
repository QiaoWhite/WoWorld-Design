# 代理 1 发现：感官 + 音频 + 载具 + 技术栈

## 关键发现

### 1. 音频系统已有 58 个 ActionAtom 变体
woworld_types 中已定义完整的 ActionAtom 枚举（58 变体），分类：
- Locomotion (8): Step, Land, Jump, Fall, TurnSharp, StartMove, StopMove, ChangeGait
- Item Interaction (10): Swing, Strike, BlockHit, Parry, ThrowItem, DropItem, OpenContainer, CloseContainer, Pull, Push, TurnLever, Reload
- Combat (7): ArrowLoose, BowDraw, WeaponDraw, WeaponSheathe, WeaponBreak, ShieldBlock, Dodge
- Survival (6): Eat, Drink, BreatheHeavy, Cough, Sneeze, StomachGrowl
- Social (6): Bow, Wave, Salute, Kneel, Embrace, Point
- Vehicle (4): Mount, Dismount, ReinsPull, AnchorDrop
- Construction (5): Hammer, Saw, Dig, Place, Measure
- Magic (5): CastStart, CastRelease, Explosion, Teleport, Summon, Dispel
- Other (3): Write, ReadPage, LightFire, ExtinguishFire

### 2. 20 个 InteractionType（即物理基元）
Strike, Swing, Block, Parry, Throw, Drop, Draw, Open, Close, Pull, Push, Turn, Mine, Chop, Forge, Sew, Stir, Reload, Crank, Pump

### 3. 感官反馈闭环
Physical Atom → SoundFootprint → Audio Engine → AudioQuery → SensoryProvider → PerceptBatch → NPC Decision → Next Action Atom

### 4. 载具系统隐含新原子
STEER, ACCELERATE, BRAKE, ANCHOR, BOARD, DISEMBARK, HOIST, HITCH, UNHITCH, OPERATE_PUMP, CHANNEL_MANA

### 5. Material→Sensory 完整映射表
- Reflectivity → Visual
- Porosity → Scent
- Density → Sound transmission
- Temperature → Scent intensity + Tactile
- Wetness → Specular + Footstep modifier + Petrichor scent

### 6. 所有物理原子（除 WAIT）都自动产生声音
这是音频系统的核心设计原则

### 7. 性能预算
Sensory ≤1.0ms (17 calls/frame × 0.05ms)
Audio ≤0.17ms 持续固定
——已完整映射到 L1/L2/L3/L4 分层
