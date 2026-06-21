# 002-视觉感知 — VisionQuery

> **状态**: 开发规格 v1.0
> **关联**: [[../001-感官系统总纲|总纲]] · [[002-视觉参数与种族差异|视觉参数]] · [[003-可见性与射线策略|射线策略]] · [[../../音频系统/001-音频系统总纲|音频系统]]

---

## 〇、总览

`VisionQuery` 是感官系统从零实现的唯一主要通道。不同于听觉（被动接收事件广播），视觉是**主动的、定向的、断断续续的**扫描——你只能看到你朝向的方向、不被遮挡的物体，且距离衰减的是角分辨率而非信号强度。

**核心原则**：视觉不辨识实体。视觉只回答"此刻能看见什么"。辨识留给信念推导层。

---

## 一、VisionQuery trait 定义

```rust
/// 定义在 woworld_core，实现在感官 crate
pub trait VisionQuery: Send + Sync {
    /// 完整视觉感知——包括视锥筛选、射线检测、细节分级、噪声注入
    fn perceive(
        &self,
        position: Vec3,
        facing: Vec3,
        params: &VisionParams,
        spatial: &dyn SpatialQuery,
        weather: &WeatherSample,
        cache: &PerceptualCache,
        dark_adaptation: &DarkAdaptation,
        attention: &AttentionState,
        modifiers: &PerceptualModifiers,
        time: GameInstant,
        rng: &mut impl Rng,
    ) -> (Vec<PerceivedVisualEntity>, VisualScene);
}

pub struct PerceivedVisualEntity {
    pub entity_id: Option<EntityId>,
    pub position_estimate: Vec3,
    pub velocity_estimate: Option<Vec3>,
    pub distance_estimate: f32,
    pub direction: Vec3,
    pub detail_level: PerceptDetail,
    pub confidence: f32,
    pub visual_features: VisualEntityFeatures,
    pub body_state: Option<PerceivedBodyState>,
    pub held_object: Option<HeldObjectFeature>,
    pub body_covering: Option<BodyCoveringFeature>,
}

pub struct VisualEntityFeatures {
    pub size: f32,
    pub silhouette_complexity: f32,
    pub dominant_color: (f32, f32, f32),
    pub is_backlit: bool,
    pub is_glowing: bool,
    pub contrast_against_background: f32,
    pub texture_regularity: f32,
}
```

---

## 二、四阶段算法流程

```
visible_entities(pos, facing, params, spatial, weather, cache, attention, time, rng):

    // ═══ 阶段0: 预计算（零射线，一次查询） ═══
    effective_range = compute_effective_range(params, weather, modifiers)
    scene = compute_scene(pos, facing, spatial, weather, time)
    dark_adaptation.update(scene.light_level, dt)
    frustum_aabb = build_frustum(pos, facing, params.fov_h, effective_range)
    candidates = spatial.entities_in_aabb(frustum_aabb, filter=VISUALLY_PERCEPTIBLE)
    ray_budget = adaptive_ray_budget(scene, attention)

    // ═══ 阶段1: 实体分类（O(candidates)，零射线） ═══
    signals = []
    for entity in candidates:
        category = classify(entity, attention, cache, params)
        match category:
            Locked → signals.push((entity, Priority::Critical))
            Motion | Threat → signals.push((entity, Priority::High))
            GoalRelevant → signals.push((entity, Priority::Medium))
            SociallySalient | Novel → signals.push((entity, Priority::Low))
            Background → skip

    // ═══ 阶段2: 射线分配（按优先级，≤ray_budget条） ═══
    signals.sort_by_priority_desc()
    allotted = signals.take(ray_budget)

    // ═══ 阶段3: 可见性判定（射线检测） ═══
    visible = []
    for (entity, priority) in allotted:
        if !spatial.line_of_sight(pos, entity.center, effective_range): continue
        detail = classify_detail(entity, pos, facing, params, effective_range, scene.light_level)
        noisy = apply_vision_noise(entity, detail, params, scene, rng)
        visible.push(noisy)

    // ═══ 阶段4: 外周运动感知（无射线——仅速度向量） ═══
    for entity in candidates.not_ray_tested():
        if motion_detectable(entity, cache, params.motion_sensitivity):
            visible.push(motion_only_percept(entity, rng))

    return (visible, scene)
```

---

## 三、实体分类——背景/信号二分类

**不是"所有实体显著性评分排序取top N"。** 是"绝大多数实体标记为背景→忽略→零处理"。只有触发分类规则的实体才进入感知管线。

```rust
fn classify(
    entity: &SpatialEntity,
    attention: &AttentionState,
    cache: &PerceptualCache,
    params: &VisionParams,
) -> EntityCategory {
    // 1. 注意锁定——"我正在跟这个人说话/盯着这个猎物"
    if attention.locked_to(entity.id) {
        return Locked;
    }

    // 2. 运动检测——"有东西在动"
    if magnitude(entity.velocity) > params.motion_sensitivity * 0.5 {
        return Motion;
    }

    // 3. 威胁检测——"那个东西看起来危险"
    if entity.entity_kind == SpatialEntityKind::Creature
       && is_potential_threat(entity, cache) {
        return Threat;
    }

    // 4. 目标相关——"我找的铁砧那个形状"
    if is_goal_relevant(entity) {
        return GoalRelevant;
    }

    // 5. 社交显著——高地位/仇恨/亲密对象
    if is_socially_salient(entity.id, cache) {
        return SociallySalient;
    }

    // 6. 新奇——"没见过这个"
    if cache.find(entity.id).is_none()
       && !is_obviously_background(entity) {
        return Novel;
    }

    // —— 全部不满足 → Background → 零处理 ——
    Background
}
```

### 分类规则的性能特征

| 规则 | 成本 | 说明 |
|------|------|------|
| Locked | O(1) | 直接查 attention state |
| Motion | O(1) | 读 entity.velocity 的 magnitude |
| Threat | O(1) | 查 entity.entity_kind + 粗粒度威胁标签 |
| GoalRelevant | O(1) | 闭包调用（NPC crate 传入） |
| SociallySalient | O(log n) | 查 PerceptualCache 中的关系标记 |
| Novel | O(log n) | 查 PerceptualCache 是否存在 |
| → Background | O(1 归并) | 以上全不满足→跳过 |

**绝大多数实体在规则2就被标记为Background（静止的建筑物/树木/地形）。**

---

## 四、分级细节等级

不是"可见/不可见"的二值判定。是**连续退化**的四档：

```rust
fn classify_detail(
    entity: &SpatialEntity,
    observer_pos: Vec3,
    facing: Vec3,
    params: &VisionParams,
    effective_range: f32,
    light_level: f32,
    dark_adaptation: &DarkAdaptation,
) -> PerceptDetail {
    let angle = angle_between(facing, entity.center - observer_pos);
    let distance = magnitude(entity.center - observer_pos);

    let foveal = angle < 5.0;
    let parafoveal = angle < 30.0;

    // 距离+光照调制有效范围
    let actual_range = effective_range * light_level
        * dark_adaptation.current_level.clamp(0.05, 1.0)
        * modifiers.vision_range_mult;

    if distance > actual_range {
        return Invisible;
    }

    if foveal && distance < actual_range * 0.5 {
        FullRecognition     // 能辨识物种、读表情、看铭文
    } else if foveal || (parafoveal && distance < actual_range * 0.3) {
        BlurredIdentity     // 能辨识类别但认不出具体个体
    } else if parafoveal || distance < actual_range * 0.2 {
        MotionAndShape      // 能看到形状和运动
    } else {
        BarelyVisible       // 仅检测到有东西
    }
}
```

| 等级 | 能看到 | 不能看到 |
|------|--------|---------|
| FullRecognition | 物种、表情、铭文、装备细节、伤势 | — |
| BlurredIdentity | 类别（人/动物/建筑）、大致装备 | 具体身份、表情细节 |
| MotionAndShape | 形状轮廓、运动方向 | 是什么东西 |
| BarelyVisible | "那边有东西" | 形状、大小、是什么 |
| Invisible | — | 全部 |

---

## 五、自适应射线预算

```rust
fn adaptive_ray_budget(scene: &VisualScene, attention: &AttentionState, combat: bool) -> u8 {
    let mut budget: i32 = 8;

    // 环境修正
    if scene.openness < 0.3 { budget += 4; }   // 封闭空间——更多遮挡→需更多射线
    if scene.entity_density > 0.5 { budget += 2; } // 拥挤→更多竞争

    // 状态修正
    if combat { budget *= 2; }                  // 战斗→翻倍
    if attention.load > 0.7 { budget -= 3; }    // 高度专注→视野缩窄

    budget.clamp(1, 20) as u8
}
```

**射线分配顺序**（优先级从高到低，用完budget即停）：
1. 注意锁定的实体（1条，必须）
2. 高优先级信号（Motion/Threat，最多3条）
3. 中优先级信号（GoalRelevant，最多2条）
4. 低优先级信号（SociallySalient/Novel，最多1条）
5. 随机扫视（剩余budget，随机方向采样——模拟无意识扫视）

---

## 六、运动检测

双路径——不依赖单点故障：

```rust
fn motion_detectable(
    entity: &SpatialEntity,
    cache: &PerceptualCache,
    sensitivity: f32,
) -> bool {
    // 路径1: 绝对速度——SpatialEntity自带velocity（世界空间维护）
    let speed = magnitude(entity.velocity);
    let abs_threshold = (0.3 - sensitivity * 0.25).max(0.05);  // m/s
    if speed > abs_threshold { return true; }

    // 路径2: 缓存比较——已知实体的位置变化
    if let Some(cached) = cache.find(entity.id) {
        let delta = distance(entity.center, cached.position_estimate);
        let relative_threshold = entity.distance_from_observer * 0.02;
        if delta > relative_threshold { return true; }
    }

    false
}
```

外周运动输出（无射线——不知道具体是什么）：
```rust
PerceivedVisualEntity {
    entity_id: None,            // 不知道是什么
    position_estimate: fuzzy,   // 带大噪声
    detail_level: MotionOnly,   // 仅运动
    confidence: 0.2,           // 低确信——只是"有东西在动"
}
```

---

## 七、暗适应——指数松弛模型

```rust
impl DarkAdaptation {
    pub fn update(&mut self, ambient_light: f32, dt: TimeDelta) {
        self.target_level = if ambient_light < 0.1 { 1.0 } else { ambient_light };

        let rate = if self.target_level > self.current_level {
            0.0005  // 亮→暗: 慢（~30分钟达到80%）
        } else {
            0.05    // 暗→亮: 快（~30秒达到80%）
        };

        let alpha = 1.0 - (-rate * dt.as_seconds() as f32).exp();
        self.current_level += (self.target_level - self.current_level) * alpha;
    }
}
```

**涌现场景**：
- 从阳光下走进暗洞穴 → 前30秒几乎看不见 → 易被伏击
- 持火把进洞 → 火把光破坏暗适应 → 离开火把范围更看不见
- 精灵 → `dark_adaptation_speed` 更高 → 更快适应

---

## 八、视觉辨识的时间维度

同一实体连续观察的周期数（`consecutive_observations`）调制辨识精度：

| consec_obs | FullRecognition | BlurredIdentity | MotionAndShape |
|-----------|-----------------|-----------------|----------------|
| 1-4 | 基本辨识 | 模糊 | 仅运动 |
| 5-9 | 能识别身份 | 基本辨识 | 模糊 |
| 10+ | 完全辨识（表情/细节） | 能识别身份 | 基本辨识 |

**涌现场景**：玩家刚进门→NPC只是"有人来了"→过2秒（几个周期后）→"哦，是你啊"。

---

## 九、有效视觉距离——天气+光照+介质

```rust
fn compute_effective_range(
    params: &VisionParams,
    weather: &WeatherSample,
    modifiers: &PerceptualModifiers,
    medium: Medium,
) -> f32 {
    let mut range = params.max_range_m * modifiers.vision_range_mult;

    // 天气可见度
    range = range.min(weather.visibility_m);

    // 介质
    match medium {
        Medium::Water => range *= 0.05,    // 水下
        Medium::Air => {}
        Medium::Solid => return 0.0,
    }

    range
}
```

---

## 十、视觉噪声注入

```rust
fn apply_vision_noise(
    entity: &SpatialEntity,
    detail: PerceptDetail,
    params: &VisionParams,
    scene: &VisualScene,
    rng: &mut impl Rng,
) -> PerceivedVisualEntity {
    let distance = magnitude(entity.center - observer_pos);

    let mut perceived = PerceivedVisualEntity {
        position_estimate: entity.center
            + gaussian_noise_3d(0.0, distance * 0.05, rng),  // 距离越远噪声越大
        distance_estimate: distance
            * (1.0 + gaussian_noise(0.0, 0.08, rng)),
        direction: normalize(entity.center - observer_pos)
            + gaussian_noise_3d(0.0, (1.0 - params.acuity_arcmin) * 0.3, rng),
        // ...
    };

    // 低光照 → 颜色信息丢失
    if scene.light_level < 0.3 {
        perceived.visual_features.dominant_color = desaturate(perceived.visual_features.dominant_color, scene.light_level);
    }

    // 外周 → 细节丢失
    if detail < BlurredIdentity {
        perceived.visual_features.texture_regularity = 0.0;  // 看不到纹理
    }

    perceived
}
```

---

> **下一个文档**: [[002-视觉参数与种族差异]]
