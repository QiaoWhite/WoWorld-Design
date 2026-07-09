# 海洋渲染

> **开发代号**: WoWorld (Wonder World)
> **引擎**: Godot 4.7
> **日期**: 2026-07-10
> **状态**: 设计完成
> **定位**: 双 Tier Gerstner 海洋渲染——近处此起彼伏的戏剧性涌浪，远处延伸至 40km 地平线。
> **关联**: [[../../Change/CHG-068-视距扩展与海洋升级-20260710|CHG-068]] · [[../../世界生成/017-双层海陆噪声与异世界拓扑|017 噪声]] · [[../../世界生成/018-LOD层级40km扩展|018 LOD]]
> **数据接口**: `OceanProvider` trait 定义于 `woworld_core/src/ocean.rs`；`HeightfieldOcean` 实现于 `woworld_worldgen/src/ocean.rs`
> **渲染**: `OceanPlane` GodotClass 定义于 `woworld_godot/src/ocean.rs`

---

## 一、当前问题

单个 22km × 22km PlaneMesh，200×200 细分，110m 顶点间距。6 个 Gerstner 波波长 3-18m，Nyquist 需要 ≤1.5-9m 间距——全部走样。海面看起来"对"仅因为切线/副法线计算正确——视觉欺骗，不是真正的波浪形状。

无法表现"此起彼伏"的戏剧性海浪。

## 二、设计

### 2.1 双 Tier 架构

```
Tier 0 "near" — 内圈 0-5km:
  5km × 5km PlaneMesh, 512×512 细分 (263K 顶点)
  9.8m 顶点间距, Nyquist 限 = 19.6m
  3 个 Gerstner 涌浪: 25m / 35m / 50m 波长
  全 PBR shader: 深度色变 + 白沫 + Fresnel + 法线
  VRAM: ~6.3 MB

Tier 1 "far" — 外圈 0-50km:
  50km × 50km PlaneMesh, 200×200 细分 (40K 顶点)
  250m 顶点间距
  无 Gerstner 位移 — 完全平坦
  极简 shader: 仅 Fresnel 颜色渐变
  VRAM: ~1.0 MB

总计: ~304K 顶点, ~7 MB VRAM
```

**Near tier 的波选择原理**：9.8m 间距下，Nyquist 限 19.6m。3 个 vertex 波（25/35/50m）全部正确表示——这是"此起彼伏"的宏观涌浪（浪高 2-5m，浪峰到浪谷可见的起伏）。

短于 25m 的原 6 个波全部删除——它们在 110m 旧间距下已经走样，在 9.8m 新间距下仍然走样。微细节将来通过 normal map 补充（远期）。

**Far tier 的 Fresnel 原理**：无波浪 → 法线恒为 `(0,1,0)`。Fresnel 基于视角与水平面夹角：
- 俯视（近处）：`fresnel ≈ 0` → 深色，能看到"海底"
- 平视（地平线）：`fresnel ≈ 1` → 浅色/反射色，与天空融为一体
- `fresnel = pow(1.0 - abs(dot(view_dir, vec3(0,1,0))), 5.0)`

### 2.2 为什么是两层不是三层

三层（near + mid + far）中圈在 4-20km 填补过渡。但 15-20km 处大气散射已显著消隐——50m 和 250m 间距的差异不可见。两层复杂度是三层的一半，收益相同。若测试证明中圈需要，增加第三层是非破坏性扩展（TOML 加一条 `[[tiers]]`）。

## 三、单一真相源：`ocean_waves.toml`

旧系统波参数硬编码在两个地方：
- Rust: `default_waves()` 返回 `[GerstnerWave; 6]`
- GLSL: `ocean.gdshader` 中 6 个独立 `wave_a..wave_f` uniform

新系统合并为单一 TOML：

```toml
# assets/ocean_waves.toml
# 由 HeightfieldOcean (Rust 物理) 和 ocean.gdshader (GLSL 渲染) 共同消费

[[vertex_waves]]
direction = [1.0, 0.0]     # 传播方向 (xz)
steepness = 0.12           # 陡度 (0-1)
wavelength = 25.0          # 波长 (m)

[[vertex_waves]]
direction = [0.7, 0.5]
steepness = 0.10
wavelength = 35.0

[[vertex_waves]]
direction = [-0.3, 0.9]
steepness = 0.08
wavelength = 50.0

[[tiers]]
name = "near"
extent_km = 5.0
subdivision = 512

[[tiers]]
name = "far"
extent_km = 50.0
subdivision = 200
```

## 四、代码变更

### 4.1 数据面（`woworld_worldgen/src/ocean.rs`）

- `GerstnerWave` 结构体不变（4 字段：dir_x, dir_z, steepness, wavelength）
- `default_waves()` → `from_toml_str()`，通过 `include_str!` 编译时嵌入
- 波数组 `[GerstnerWave; 6]` → `Vec<GerstnerWave>`（TOML 驱动，不硬编码数量）
- `OceanProvider` trait **不变**——4 个方法签名完全兼容（`sea_level_at` / `wave_height_at` / `water_depth_at` / `is_underwater`）

### 4.2 渲染面（`woworld_godot/src/ocean.rs`）

- `OceanPlane` 结构体从 1 个 `MeshInstance3D` → 2 个 tier 管理
- `ready()`: 创建 near 和 far 两个 PlaneMesh；加载 `ocean.gdshader` (near) + `ocean_far.gdshader` (far)
- `process()`: 两个 tier 各自跟随玩家 XZ + 更新 shader uniform
- 从 `ocean_waves.toml` 读取波参数 → 设置 near tier shader 的 uniform 数组
- Far tier 无 Gerstner uniform——仅 Fresnel 颜色渐变

### 4.3 Shader

**`ocean.gdshader`**（修改）：
- 6 个独立 `uniform vec4 wave_a..wave_f` → `uniform vec4 waves[10]` + `uniform int wave_count`
- 顶点着色器循环：`for (int i = 0; i < wave_count; i++) { ... }` 替代展开 6 次计算
- 其余逻辑（深度色变、白沫、Fresnel、ALPHA 透明度）不变

**`ocean_far.gdshader`**（新文件，~30 行）：
- 无 vertex 位移——`VERTEX` 不变
- Fragment：Fresnel 基于 `dot(view_dir, (0,1,0))`
- `ALBEDO = mix(deep_color, horizon_color, fresnel)`
- `render_mode blend_mix, depth_draw_opaque, cull_back`

### 4.4 Godot 场景

OceanPlane 节点下有两个子 MeshInstance3D（near 和 far），渲染顺序由深度测试自然处理——near tier 的 Gerstner 波在 y≈0 有正位移（几米），覆盖在 far tier 之上。

## 五、性能

| 指标 | 当前 | 新系统 | Delta |
|------|------|--------|-------|
| 顶点总数 | 40K | 304K | +264K |
| VRAM（仅 mesh） | ~0.9 MB | ~7 MB | +6.1 MB |
| Shader 复杂度 | 6 次 Gerstner/vert | 3 次 Gerstner/vert (near) + 0 (far) | **降低** |
| Draw calls | 1 | 2 | +1 |
| 海洋质量 | 全部走样 | 3 个大涌正确表示 | 质的飞跃 |

## 六、涌现交互

`OceanProvider::wave_height_at(pos, time)` 已就位。结合 `OceanProvider` trait + `WeatherDriver` + NPC 决策系统：

- 风暴天气 → `WeatherDriver` 增大波 steepness → `wave_height_at()` 返回更大值 → 船只摇晃剧烈
- NPC 决策读取 `wave_height_at()` → 恶劣海况下 NPC 避免出海 → 渔业经济受影响、海盗活动减少
- 无需新建"风暴出海危险"系统——从已有模块涌现

## 七、已知缺口

| # | 缺口 | 计划 |
|---|------|------|
| 1 | Normal map 层叠替代短波碎浪 | 远期 |
| 2 | Ocean near tier `set_extra_cull_margin(~10m)` | 上线前 |
| 3 | FFT 海洋升级（替换 Gerstner） | 远期——仅影响 near tier |

---

> **关联文档**: [[../../世界生成/017-双层海陆噪声与异世界拓扑|017 噪声]] · [[../../世界生成/018-LOD层级40km扩展|018 LOD]] · [[../../UI与UX系统/视距调节系统|视距调节]] · [[../../Change/CHG-068-视距扩展与海洋升级-20260710|CHG-068]]
