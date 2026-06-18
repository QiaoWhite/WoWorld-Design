# 008-MaterialProperties与物品系统对接

> **关联**: [[001-NPC行动涌现总纲|001-总纲]] · [[../../物品系统/001-物品系统总纲|物品系统 001]] · [[002-物理原子层定义与签名|002-物理原子]]
> **日期**: 2026-06-18

## §一、当前状态

**MaterialProperties 在物品系统中不存在。** 虽然 NPC 认知系统文档（006-创新管线与跨领域对接.md）前向引用了 MaterialProperties 为"已有基础"——但从未被设计。

当前材料相关属性分散在 4 个结构体中:
- `ItemProperties`: weight_grams, bulk_factor, volume_liters, element_affinity, magic_capacity_ke, max_durability, audio_material
- `AestheticProps`: color_palette, fabric_quality, ornamentation_level, cleanliness_factor
- `ParamSchema/ItemParams`: 维度参数 (length, width, thickness, draw_weight, curvature...)
- `EnchantRuneProps`: element, effect, uses_remaining

**缺失**: hardness, density, thermal_conductivity, specific_heat, melting_point, toughness, elasticity, friction, mana_conductivity, elemental_affinity[10], flammability, oxidation_behavior, porosity... — 这些驱动物理原子差异化行为的核心属性全部缺失。

---

## §二、设计方案

### 2.1 核心原则: 遵循现有委托模式

物品系统的核心哲学——"基础设施，不解释含义"——不改变。遵循已有 `Option<ConsumableEffect>` 和 `Option<AudioMaterial>` 模式。

### 2.2 MaterialDef TOML 注册表

```toml
# materials/iron.toml
[[material]]
id = "Iron"
category = "Metal"

[mechanics]
density_kgm3 = 7870
hardness = 0.45          # Mohs归一化 0-1
toughness = 0.55
elasticity = 0.3
friction = 0.6

[thermal]
specific_heat = 450       # J/(kg·K)
thermal_conductivity = 80 # W/(m·K)
melting_point_k = 1811
ignition_point_k = 9999   # 不燃
combustibility = 0.0

[electrical]
electrical_conductivity = 0.15
mana_conductivity = 0.3

[chemical]
flammability = 0.0
solubility = 0.0
acidity = 0.0
toxicity = 0.0
porosity = 0.05
oxidizes_when_heated = true

[sensory]
reflectivity = 0.6       # 抛光金属
sound_damping = 0.1      # 金属传声好
scent_intensity = 0.0

[magic]
elemental_affinity = [0.6, 0.0, 0.1, 0.0, 0.2, 0.0, 0.1, 0.0, 0.05, 0.0]
# 金木水火土风雷电血灵 — iron has strong Metal affinity
enchantment_capacity = 0.5
aether_retention = 0.4
```

### 2.3 ItemProperties 改动

```rust
pub struct ItemProperties {
    // ... 现有 ~20 字段全部保留 ...
    
    /// 新增 (仅此一个字段):
    pub material: Option<MaterialDefId>,
    // 铁矿石: Some(MaterialDefId("IronOre"))
    // 铁锭:   Some(MaterialDefId("IronIngot"))
    // 铁剑:   None ← 材质从装配树的 StrikingHead 组件继承
    // 药水:   None ← 非材料类物品不填充
    // 金币:   None
}
```

**仅 Material-category 物品 (0x20-0x2F) 填充 `material` 字段。** 武器/盔甲/工具通过装配树从组件材料继承——不单独存储。

### 2.4 装配树材质继承

```rust
/// 遍历装配树，解析任意物品的有效材料属性
fn resolve_material(item: &ItemEnt, assembly: &ItemAssembly) -> MaterialDef {
    match assembly {
        ItemAssembly::Simple { def } => {
            // 简单物品: 直接读 ItemProperties.material
            def.properties.material.map(|mid| MATERIAL_REGISTRY.get(mid))
                .unwrap_or(DEFAULT_MATERIAL)
        }
        ItemAssembly::Composite { components, .. } => {
            // 复合物品: 找到"干活的部分"——战斗找 StrikingHead, 工具找 ToolHead
            // 如果组件本身是 Simple → 查它的 material
            // 如果组件也是 Composite → 递归
            let working_part = find_working_component(components);
            resolve_material(working_part, working_part.assembly)
        }
    }
}
```

### 2.5 配方材料约束（替代具体材料ID）

```toml
# recipes/iron_sword.toml — 不指定具体材料ID
[[recipe]]
id = "SwordBlade"
skill = "Blacksmith"

[[recipe.input]]
role = "BladeMaterial"
acceptable = { 
    category = "Metal", 
    hardness_min = 0.3, 
    melting_point_max = 2000 
}
# → 铁可以 (hardness 0.45, mp 1811)
# → 钢可以 (hardness 0.7, mp 1700)
# → 铜可以 (hardness 0.35, mp 1356 — 但偏软)
# → 秘银可以 (hardness 0.8, mp 2500 — 但 mp 超过2000)
# → 金不行 (hardness 0.15, 太软)
# → 铅不行 (hardness 0.1, 太软)
# → 龙钢可以 (传说, hardness 0.9)

quantity = 4
```

---

## §三、对现有物品系统的影响

### 3.1 需要新增

| 新增内容 | 位置 | 大小 |
|---------|------|------|
| MaterialDef TOML 注册表 | `woworld_core::materials` 或物品系统数据目录 | ~200条目 × 120B = 24KB |
| `ItemProperties.material` 字段 | 物品系统 003 | 1 个 `Option<MaterialDefId>` (8 bytes) |
| `RecipeInput.acceptable` 材料约束 | 物品系统 006 | 1 个 Option 子结构 |
| `resolve_material()` 工具函数 | 物品系统 | ~50 行纯函数 |
| MaterialDefId bit 布局 | 物品系统 002 | 类似 ItemDefId 的编码方案 |

### 3.2 不需要改变

- ItemCategory 编码方案
- ItemAssembly 框架 (4 JointType + 组件树)
- Quality × Rarity 系统
- 耐久度/修理机制
- 附魔系统 (卡槽 + 直接)
- 库存/仓储
- ItemEntId / SlotInstanceId
- 音频材质 (AudioMaterial 独立保留)

### 3.3 内存影响

```
ItemProperties 新增 8B × 5,000 DefId = 40KB (一次性加载)
~500 个 Material-category 物品实际填充
MaterialDef 表: 200 × 120B = 24KB
总计: 64KB → 可忽略
```

### 3.4 对装配树性能的影响

`resolve_material()` 调用频率:
- 武器首次持有: 1次 (结果缓存到 GripHandle)
- 锻造每个STRIKE: 0次 (GripHandle 热缓存)
- OBSERVE(检视陌生物品): 1次冷路径 ~200ns (10节点遍历+哈希查找)
- → 每帧总查询成本 <0.05ms

---

## §四、涌现空间

有了 MaterialProperties 后，物理基元的差异化行为完全由数据驱动:

### 4.1 材料替换涌现

铜矿枯竭 → 铁匠试了新的黑色矿石 → 同一配方、符合材料约束 → 打出了历史上第一把铁剑。不是"铁剑配方被解锁了"——是铁匠在同一配方下用了符合约束的新材料。

### 4.2 材质交互涌现

铁+水+空气 → 锈（`oxidizes_when_heated=true` + 露天存放 → durability 自然下降）。不需要"生锈系统"——物质属性和环境自动计算。

### 4.3 跨域材料实验

高开放性铁匠尝试把火元素魔力注入淬火液（CONDUCT + HEAT + DIP）→ 发现用龙血淬火比用水淬火更好 → 新知识通过 TeachingSession 传播。不需要预设"龙血淬火配方"。

### 4.4 旷野之息式元素涌现

```
IGNITE 原子 × MaterialProperties:
  干草 (flammability=0.9, ignition_point=250°C) → 火把(temperature=400°C) → 点燃
  
  连锁: IGNITE(干草) → HEAT(周围空气) → 木屋(flammability=0.8, ignition_point=300°C) 
  → 接收到的热量 > 300°C → 木屋也 IGNITE
  
  雨水: WET(干草, 雨水) → 干草.flammability *= 0.1 → 火灭了
  
  → "一整片森林被点燃"或"幸好下雨"——不是预设剧本，是材质属性自动计算
```

---

## §五、对物品系统的 Phase 5 联动修改要求

1. `物品系统/003-物品属性与品质.md` — 新增 `MaterialProperties` 字段说明 + MaterialDef 引用说明
2. `物品系统/006-制造与配方.md` — 新增 `RecipeInput.acceptable` 材料约束字段说明
3. `物品系统/001-物品系统总纲.md` — 更新 ItemProperties 结构体 + 材料系统概述
4. 新增 `物品系统/010-材料属性与物理模拟.md` — MaterialDef TOML 格式规范 + 完整字段定义

---

## §六、权威性声明

MaterialProperties 的**字段语义和物理公式**由本模块（08-物理原子层）权威定义。物品系统提供 MaterialDef TOML 的数据存储和查询。消费模块（Combat/Magic/Physics）各自读取关心的字段——物品系统不解释材料属性的含义。这与 ConsumableEffect（Life 模块定义含义，物品系统存储）和 AudioMaterial（音频模块定义含义，物品系统存储）的模式完全一致。
