# Sprint-038: 天气系统涌现化 — 从 Markov 状态机到连续物理参数驱动

> **提案日期**: 2026-07-05
> **提案状态**: ✅ 已完成
> **所属阶段**: Phase 1 — 核心基础
> **涉及模块**: woworld_core (weather_types) + woworld_atmosphere (weather) + woworld_godot (集成)
> **前置**: 无硬依赖——纯增量替换，不影响现有功能

## 问题

当前 `SimpleWeatherDriver` 有 6 个离散状态 + Markov 跳转。视觉效果通过 `match self.state` 硬编码查表——每个状态的 sky_mult/fog/exposure/saturation 是固定值。这意味着：

- 天气变化是**跳变**（Clear → PartlyCloudy 瞬间切换）
- 6 个离散点之间的过渡不存在——没有"多云 30%"
- 状态之间无物理因果关系——湿度不驱动云量，云量不驱动降水

## 目标

### 目标 1: 连续物理参数 → WeatherParams

新建 `WeatherParams` 结构体替换 `WeatherState` 枚举：

```rust
pub struct WeatherParams {
    pub cloud_cover: f32,    // 0=晴空 → 1=全阴
    pub precipitation: f32,  // 0=无降水 → 1=暴雨
    pub wind_speed: f32,     // m/s
    pub humidity: f32,       // 0→1
    pub temperature: f32,    // °C（基线=季节温度 + 天气扰动）
    pub pressure: f32,       // hPa, 1013 标准
}
```

- 所有值**连续变化**，无跳变
- 保留 `to_weather_state()` / `from_weather_state()` 用于调试快捷键兼容

### 目标 2: 物理演化驱动 → WeatherDriver

新建 `WeatherDriver` 替换 `SimpleWeatherDriver`：

```
每帧 tick(delta):
  pressure  += random_walk * delta (模拟气压系统移动)
  humidity  += (evaporation_rate - condensation) * delta
  temperature = season_baseline + cloud_effect + diurnal_cycle
  cloud_cover → humidity + pressure_gradient → 平滑逼近
  precipitation → cloud_cover * humidity → 平滑逼近
  wind_speed = |pressure_gradient| * scale
```

关键性质：
- **无离散状态机**——参数平滑漂移
- **湿度驱动云量**，云量驱动降水——物理因果链
- **季节基线**调制温度和湿度范围
- **昼间循环**叠加微小温度波动

### 目标 3: 连续视觉映射

替换 `match self.state` 硬编码：

```rust
fn sky_mult(&self) -> [f32; 3] {
    let c = self.params.cloud_cover;
    let p = self.params.precipitation;
    // 连续插值：晴空 [1,1,1] → 暴雨 [0.22,0.24,0.30]
    [
        lerp(1.0, 0.22, c.max(p)),
        lerp(1.0, 0.24, c.max(p)),
        lerp(1.0, 0.30, c.max(p)),
    ]
}
```

所有 4 个 `WeatherAtmosQuery` 方法改为连续函数——**无限种中间天气**。

### 目标 4: 调试兼容 + 回归

- 数字键 1-6 仍然工作——映射到 6 个参数预设（对应旧 WeatherState 的视觉效果）
- 旧 `WeatherState` 枚举保留但标记 `#[deprecated]`
- 现有 17 个 atmosphere 测试零回归

## 🧪 研究事项

| 问题 | 级别 | 状态 |
|------|------|------|
| 气压系统模型简化到什么程度？ | 🟡 | 随机游走 + 边界约束——足够涌现，不过度工程 |
| 温度与云量的反馈关系？ | 🟡 | 云量↑→日照↓→温度↓——简化为一阶延迟 |
| `lerp` 函数来源？ | 🟢 | 手写 3 行——不引入依赖 |

## 📊 决策

**单一候选**。WeatherParams 是连续参数化的唯一路径——Markov 状态机没有渐进改进空间。

## ⚠️ 设计约束

- `WeatherAtmosQuery` trait **签名不变**——`sky_mult() / fog_density() / exposure_mult() / saturation_mult()` 均可从连续参数计算
- `WeatherParams` 放在 `woworld_core::weather_types`——与 `WeatherState` 同级
- `WeatherDriver` 放在 `woworld_atmosphere::weather`——替换 `SimpleWeatherDriver`
- 禁止引入 `rand` crate——使用伪随机（已有模式）

## 📋 任务清单

### Step 1: WeatherParams 类型
- [ ] `woworld_core/src/weather_types.rs` 新增 `WeatherParams` struct（6 字段）+ Default
- [ ] `lerp()` 工具函数
- [ ] `WeatherParams::to_weather_state()` — 连续→离散（调试用）

### Step 2: WeatherDriver 物理演化
- [ ] `woworld_atmosphere/src/weather.rs` 新增 `WeatherDriver`
- [ ] `tick(delta, season)` — 参数连续演化
- [ ] `set_params_preset(state)` — 调试快捷键预设
- [ ] 实现 `WeatherAtmosQuery`（连续函数）

### Step 3: WorldDriver 集成
- [ ] 替换 `SimpleWeatherDriver` → `WeatherDriver`
- [ ] `weather_driver.tick(delta)` 传入季节信息
- [ ] 调试快捷键适配 `set_params_preset`

### Step 4: 测试
- [ ] 参数演化测试（tick 后值变化但不跳变）
- [ ] 连续性测试（相邻帧参数差 < 阈值）
- [ ] 视觉映射测试（clear→全晴，storm→全暗）
- [ ] 调试预设测试（6 预设 → 近似旧行为）
- [ ] 17 现有测试零回归

### Step 5: 验证
- [ ] `cargo test --workspace` 136+ 全绿
- [ ] `cargo clippy --workspace -- -D warnings` ✅
- [ ] `cargo build --workspace` ✅

## 预估

- **冲刺数**: 1
- **风险**: 🟢 低——纯增量，不改 trait 签名，不影响渲染管线
- **代码量**: ~300 行（WeatherParams ~60 + WeatherDriver ~120 + 映射函数 ~40 + 测试 ~80）
