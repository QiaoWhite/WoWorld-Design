# 003-嗅觉感知 — ScentQuery

> **状态**: 开发规格 v1.0
> **关联**: [[../001-感官系统总纲|总纲]] · [[../视觉/001-视觉感知-VisionQuery|视觉]] · [[../../天气与季节系统/001-天气系统总纲|天气]]

---

## 〇、总览

嗅觉是**懒采样模型**——不模拟气味粒子扩散。气味源由各模块写入空间 `SpatialIndex`，感官在查询时根据距离、风场和衰变解析式计算瞬时浓度。对标 `WeatherQuery::sample()` 的查询模式。

---

## 一、ScentQuery trait

```rust
pub trait ScentQuery: Send + Sync {
    fn sample(
        &self,
        position: Vec3,
        spatial: &dyn SpatialQuery,
        wind: WindVector,
        time: GameInstant,
        rng: &mut impl Rng,
    ) -> (Vec<PerceivedScent>, OlfactoryLandscape);
}

pub struct PerceivedScent {
    pub scent_type: ScentType,
    pub intensity: f32,              // 0-1
    pub direction_approx: Option<Vec3>,  // None = 太弱，方向不明
}

pub struct OlfactoryLandscape {
    pub dominant_scent: Option<ScentType>,
    pub scent_complexity: f32,       // 0=单一, 1=复杂混合
    pub scent_intensity: f32,        // 全局强度
}
```

---

## 二、气味类型

```rust
pub enum ScentType {
    // 生存相关
    FoodCooking, FreshBread, RoastingMeat, FruitRipe,
    // 危险
    Smoke, Fire, BurntFlesh, Blood, Rot, Decay, Sulfur,
    // 自然
    Floral, PineForest, RainPetrichor, SeaSalt, DampEarth,
    // 生物
    Predator, Prey, HumanSweat, AnimalMusk,
    // 人造
    Perfume, Incense, Alcohol, Leather, MetalForge,
    // 魔法
    Ozone, ArcaneResidue, NecromanticDecay, AlchemicalFumes,
}
```

---

## 三、嗅觉采样算法

```rust
fn sample(position, spatial, wind, time, rng):
    let scents = spatial.scent_sources_in(Aabb::around(position, 50m));
    let mut detected = SmallVec::new();

    for scent in scents:
        let distance = magnitude(scent.position - position);
        let sigma = scent.radius_at_source_m * 0.3;
        let distance_factor = (-distance.powi(2) / (2.0 * sigma.powi(2))).exp();

        // 风场调制——上风(顺风)闻得更远，下风(逆风)几乎闻不到
        let wind_alignment = dot(direction_to(scent), wind.direction);
        let wind_factor = if wind_alignment > 0.0 {
            1.0 + wind_alignment * 3.0    // 顺风: 4x范围
        } else {
            (-wind_alignment * 10.0).exp() // 逆风: 指数衰减
        };

        // 时间衰减
        let age = (time - scent.emitted_at).as_seconds() as f32;
        let time_factor = (-age / scent.persistence.as_seconds() as f32).exp();

        let intensity = scent.intensity * distance_factor * wind_factor * time_factor;

        if intensity > 0.05 {
            detected.push(PerceivedScent {
                scent_type: scent.scent_type,
                intensity,
                direction_approx: if intensity > 0.3 {
                    Some(direction_with_noise(direction_to(scent), 30.0, rng))
                } else { None },
            });
        }

    // 气味景观涌现——多种气味混合 → 复合描述
    let landscape = synthesize_landscape(&detected);
    (detected, landscape)
```

---

## 四、气味景观涌现

```rust
fn synthesize_landscape(scents: &[PerceivedScent]) -> OlfactoryLandscape {
    if scents.is_empty() {
        return OlfactoryLandscape { dominant_scent: None, scent_complexity: 0.0, scent_intensity: 0.0 };
    }

    let dominant = scents.iter().max_by_key(|s| (s.intensity * 1000.0) as u32).unwrap();
    let avg_intensity = scents.iter().map(|s| s.intensity).sum::<f32>() / scents.len() as f32;
    let complexity = (scents.len() as f32 / 8.0).min(1.0);

    // 复合气味涌现
    let dominant_type = match (&dominant.scent_type, scents.len()) {
        (FoodCooking | FreshBread | RoastingMeat | FruitRipe, n) if n >= 4 => Some(MixedMarket),
        (PineForest | Floral | DampEarth, n) if n >= 3 => Some(ForestUnderstory),
        (HumanSweat | Leather | MetalForge | Alcohol, n) if n >= 4 => Some(TavernInterior),
        _ => Some(dominant.scent_type),
    };

    OlfactoryLandscape { dominant_scent: dominant_type, scent_complexity: complexity, scent_intensity: avg_intensity }
}
```

---

## 五、气味源生命周期

气味源由**源实体所属模块**管理，`SpatialIndex` 只做存储+过期清理：

| 气味源 | 管理模块 | 发射条件 | 持久性 |
|--------|---------|---------|--------|
| 尸体腐烂 | 生命 crate | 死亡后开始 | 30游戏日 |
| 食物烹饪 | 物品 crate / NPC行为 | 烹饪行为中 | 1游戏小时 |
| 焚香 | 信仰 crate | 仪式中 | 2游戏小时 |
| 奥术残留 | 魔法 crate | 施法后 | 1游戏日 |
| 篝火 | 世界生成 / NPC行为 | 燃烧中 | 燃烧期间 |

---

## 六、嗅觉→情绪的直连通路

对标已有的"血腥气味→恐惧"硬编码映射：

| 气味 | 情绪效果 |
|------|---------|
| Blood, Rot, Predator | fear +0.1, arousal +0.1 |
| Smoke, Fire | alertness +0.2 |
| FreshBread, Floral | pleasure +0.05 |
| RainPetrichor | calm +0.03 |
| Sulfur, NecromanticDecay | fear +0.15, disgust +0.1 |

---

> **下一个文档**: [[../注意与显著性/001-显著性引擎]]
