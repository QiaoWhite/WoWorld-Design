# CHG-053 — Godot 4.7 技术栈升级与设计深化

> **日期**: 2026-06-21
> **类型**: 技术栈升级 + 模块新建 + 跨模块扩展
> **关联**: [[CHG-052-玩家游玩内容全貌设计-20260620|CHG-052]] · [[../参考文档/038-Godot4.7技术栈升级评估-20260621/README|参考 038]]

---

## 一、变更摘要

本次变更为 WoWorld 技术栈从 Godot 4.6 LTS 升级至 Godot 4.7 正式版（2026-06-18 发布·"导演剪辑版"）的全面评估与设计深化。评估从新功能可行性出发，逐步深化至画面渲染管线、语音合成、氛围系统、磨损维护、面部表情、语声系统等 12 个子系统设计。

---

## 二、核心变更

### 2.1 技术栈升级

| 维度 | 旧 | 新 |
|------|-----|-----|
| Godot 版本 | 4.6 LTS | **4.7** |
| 渲染管线 | 单 diffuse pass · flat/cel | **Toon 2-tone + Fresnel rim + SSAO + Bloom + AgX + 大气透视** |
| 室内光源 | 无 | **AreaLight3D**（4.7 新节点） |
| UI 动画 | 手动补偿 | **Control offset_transform**（4.7 新属性） |
| 纹理生成 | — | **DrawableTexture2D**（地图/告示牌） |

### 2.2 新建模块

| 模块 | 编号 | 内容 |
|------|------|------|
| **大气与氛围系统** | 25 | BiomeAtmosphere 17 参数 × 四层调制 + SkyEvent + 水下/室内 |

### 2.3 扩展模块

| 模块 | 新文档 | 内容 |
|------|--------|------|
| **音频系统** | 006-Piper合成管道 | Piper ONNX + fundsp DSP + 音频装配 + GDExtension |
| **音频系统** | 007-Bark语声系统 | 12 类非语言发声 + SoundFootprint 集成 |
| **语言表达** | 012-修订 | TtsEngine 重设计 + SpeakingProfile 新增 |

### 2.4 跨模块扩展

- **Wear/维护**：建筑 + 物品 + 装备 × 经济 × 审美
- **面部表情**：512² 图集 + shader 合成 + MultiMesh per-instance data
- **Piper 离线 TTS**：替换已有"Godot 侧 TTS 渲染层"架构
- **PlayerInput::Interrupt**：新增玩家打断对话输入
- **ConversationInterruption::ListenerInterrupt**：新增听者打断
- **水下氛围**：深度分层 × OceanProvider
- **室内氛围**：光源派生 × BuildingQuery
- **季节调制**：消费已有 SeasonClock
- **维护材料映射**：maintenance_materials.toml × ItemRegistry

---

## 三、受影响文档

### 开发阶段修改

| 文档 | 改动 |
|------|------|
| `技术栈方案/001-WoWorld正式技术栈方案v3.md` | Godot 4.6→4.7，新增 6 行技术栈条目，架构图 |
| `音频系统/005-语音管道.md` | CancelReason 扩展 + VoiceManager 合并声明 |
| `语言表达/012-语音输出接口.md` | TtsEngine 重设计 + SpeakingProfile |
| `建筑模块/002-建筑数据模型.md` | BuildingRuntime.wear_level |
| `天气与季节系统/004-跨模块接口与数据合同.md` | season_modulation 消费声明 |

### 开发阶段新建

| 文档 | 位置 |
|------|------|
| `大气与氛围系统/README.md` | 模块概览 |
| `大气与氛围系统/001-氛围参数与调制.md` | ResolvedAtmosphere + 四层调制 |
| `大气与氛围系统/002-天空事件与水下室内.md` | SkyEvent + 水下/室内 |
| `大气与氛围系统/003-跨模块依赖与接口全面清单.md` | 依赖清单 |
| `音频系统/006-Piper合成管道.md` | TTS 合成管道 |
| `音频系统/007-Bark语声系统.md` | Bark 系统 |

### 参考文档新建（`参考文档/038-Godot4.7技术栈升级评估-20260621/`）

| 文档 | 内容 |
|------|------|
| `README.md` | 会话全貌 + 文档导航 |
| `001-Godot4.7功能评估与采纳清单.md` | 14 项功能逐项评估 |
| `002-画面渲染管线v4.7修订.md` | Toon + 后处理 + 氛围 + 水下/室内 + wear + 面部 |
| `003-语音系统架构修正-Rust解耦.md` | 完整语音系统架构 |
| `004-Wear维护系统设计.md` | wear 状态机 + 跨四模块 |
| `005-面部表情与视觉标识.md` | 图集+shader + 伤口 |

---

## 四、模块接头总览更新

- 新建 `模块接头总览/25-大气与氛围/` — 出口/入口/影响链/变更日志
- 更新 `模块接头总览/README.md` — 模块列表 24→25

---

## 五、Godot 4.7 采纳关键决策

**硬依赖（4.7 独有）**：AreaLight3D、Control offset_transform

**被动受益**：AgX 色调映射、锥形渐变、内联 Shader 预览、3D 编辑器改进

**弱依赖（有备选）**：DrawableTexture2D（备选：Godot Image 类）

**不采纳**：HDR、Nearest-neighbor 3D、GABE、VirtualJoystick、XR、OneCore TTS

---

## 六、架构边界确认

Rust/Godot 边界在本次变更中保持明确：

```
Rust：氛围计算·SkyEvent管理·wear状态机·Piper合成·DSP·Bark触发·表情映射·维护GOAP
Godot：Toon shader·世界环境·AreaLight3D·DrawableTexture2D·AudioStreamPlayer·文字同步·室内光影
```

零新增 trait。零新增 Godot 模块。全部新系统挂在已有模块或 TOML 数据驱动。

---

> **下一步**: 转正条件——Godot 4.7 + godot-rust/gdext 支持确认后，参考文档内容转入开发规格。
