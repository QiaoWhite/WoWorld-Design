# 038 — Godot 4.7 技术栈升级评估与设计深化

> **日期**: 2026-06-21
> **状态**: 参考文档。转正条件：Godot 4.7 + godot-rust/gdext 支持确认。
> **会话性质**: Godot 4.6→4.7 引擎升级全面评估 + 画面/语音/氛围/wear/表情/Bark 系统深化设计

---

## 一、会话概况

本会话源于将 WoWorld 技术栈从 Godot 4.6 LTS 升级至 Godot 4.7 正式版（2026-06-18 发布·"导演剪辑版"）的需求。设计评估从新功能可行性分析出发，逐步深化至画面渲染管线、语音合成架构、氛围系统、磨损维护、面部表情、语声 Bark 系统等全面设计。

### 涉及的模块

| 模块 | 类型 | 说明 |
|------|------|------|
| **技术栈方案** | 修改 | Godot 4.6→4.7，新增 AreaLight3D/DrawableTexture2D/Control offset_transform 等 |
| **画面渲染管线** | 🆕 新增 | Toon 2-tone + Fresnel rim + SSAO + Bloom + AgX + 大气透视 + 体积雾 |
| **BiomeAtmosphere** | 🆕 新增 | 群系×时间×天气×季节 四层调制氛围系统 |
| **SkyEvent** | 🆕 新增 | 叙事性天空事件覆盖——涌现式触发 |
| **语音合成管道** | 扩展 | Piper ONNX + fundsp DSP + 音频装配 + GDExtension 传输 |
| **Bark 语声系统** | 🆕 新增 | 非语言发声——生理/情感/社会/群体 12 类 × VoiceProfile DSP |
| **SpeakingProfile** | 🆕 新增 | 说话节奏——语速/停顿/犹豫/咬字/换气——从 BigFive 派生 |
| **Wear/维护系统** | 🆕 新增 | 建筑+物品+装备 磨损度 × 时间×天气×材质 × 维护→经济·审美 |
| **面部表情** | 🆕 新增 | 512² 图集 + shader 合成 + MultiMesh per-instance data |
| **水下/室内氛围** | 🆕 新增 | 深度分层水下覆盖 + 光源驱动室内切换 |
| **DrawableTexture2D** | 🆕 应用 | 探索地图·告示牌/墓碑/书页 动态纹理生成 |

---

## 二、文档导航

### 参考文档（本文件夹）

| 编号 | 文档 | 内容 |
|------|------|------|
| **001** | Godot 4.7 功能评估与采纳清单 | 14 项功能逐项评估——采纳/跳过/被动受益 |
| **002** | 画面渲染管线 v4.7 修订 | Toon 2-tone + 后处理 + BiomeAtmosphere + 水下/室内 |
| **003** | 语音系统架构修正 · Rust 解耦 | Piper 合成管道 + Bark + SpeakingProfile + VoiceManager + GDExtension |
| **004** | Wear 维护系统设计 | wear_level 状态机 + 建筑/物品/装备 + GOAP + 经济·审美联动 |
| **005** | 面部表情与 Bark 语声系统 | 图集+shader + 22/12类Bark + 文化Bark + 群体涌现 |

### 开发规格文档（开发阶段/）

| 位置 | 文档 | 内容 |
|------|------|------|
| `技术栈方案/` | 001-WoWorld正式技术栈方案v3.md | Godot 4.6→4.7 修订 |
| `音频系统/` | 005-语音管道.md | VoiceManager合并 + CancelReason扩展 + char_timeline |
| | 006-Piper合成管道.md 🆕 | Piper + espeak-ng + fundsp + 装配 |
| | 007-Bark语声系统.md 🆕 | 12类Bark + SoundFootprint + 文化Bark + 群体涌现 |
| `语言表达/` | 012-语音输出接口.md | TtsEngine重设计 + SpeakingProfile新增 |
| `天气与季节系统/` | 004-跨模块接口与数据合同.md | season_modulation 消费新增 |
| `建筑模块/` | 002-建筑数据模型.md | wear_level字段 + BuildingRuntime 扩展 |

---

## 三、核心架构决策

### Rust/Godot 边界

```
Rust 模拟核心（规则·状态·决策）:
  BiomeAtmosphere 计算 · SkyEvent 管理 · wear_level 状态机
  Piper 合成 + DSP · Bark 触发判定 · SpeakingProfile 派生
  FaceExpression 映射 · 维护 GOAP · 维护材料需求

Godot 渲染客户端（视觉·听觉·UI）:
  Toon shader · SSAO/Bloom/AgX · 大气透视 · 体积雾
  AreaLight3D 放置 · DrawableTexture2D · AudioStreamPlayer
  Godot 全局 shader uniform · 文字逐字同步
 室内氛围派生 · Control offset_transform

GDExtension 通道:
  ResolvedAtmosphere (每帧) · FaceExpression (per NPC)
  SynthesizedUtterance (AudioPool ID) · BarkEvent (轮询)
  wear_level (per 建筑/NPC) · 地图探索数据 (per 5s)
```

### 关键设计原则

1. **只创造视觉规则，不逐个调教模型** —— BiomeAtmosphere + toon shader 全局生效
2. **标点即编码** —— SpeakingProfile → 标点映射 → Piper。音频模块不知道 SpeakingProfile
3. **Bark 不是独立系统** —— 走已有 SoundFootprint → AudioQuery 管道
4. **数据驱动** —— 13 个 TOML 配置文件驱动全部氛围/表情/Bark/wear 参数

---

## 四、Godot 4.7 采纳清单

| # | 功能 | 采纳 | 等级 |
|---|------|------|------|
| 1 | **AreaLight3D** | ✅ | 核心——室内面光源 |
| 2 | **AgX 色调映射** | ✅ | 核心——被动受益 |
| 3 | **Control offset_transform** | ✅ | 核心——UI 动画 |
| 4 | **体积雾** | ✅ | 核心——场景氛围 |
| 5 | **DrawableTexture2D** | ✅ | 弱依赖——有 Image 备选 |
| 6 | **锥形渐变** | ✅ | 小——魔法阵/罗盘 |
| 7 | **内联 Shader 预览** | ✅ | 编辑器——开发效率 |
| 8 | **3D 编辑器改进** | ✅ | 编辑器——开发效率 |
| 9 | **新 Asset Store** | ✅ | 编辑器 |
| 10 | HDR 输出 | ❌ | cel/toon 不需要 |
| 11 | Nearest-neighbor 3D | ❌ | 不走像素风格 |
| 12 | GABE / VirtualJoystick | ❌ | 非目标平台 |
| 13 | XR / Foveated Rendering | ❌ | 非目标平台 |
| 14 | OneCore TTS | ❌ | Piper 替代 |

---

> **关联**: [[../031-技术栈全量审计-20260618/README|031 技术栈审计]] · [[../037-世界生成重构跨模块补充需求-20260620/|037 世界生成跨模块]]
