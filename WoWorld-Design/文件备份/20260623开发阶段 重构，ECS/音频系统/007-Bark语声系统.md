# 007 — Bark 语声系统

> **模块**: 音频系统
> **版本**: v1.0
> **创建日期**: 2026-06-21
> **依赖**: [[005-语音管道|005 语音管道]] · [[006-Piper合成管道|006 Piper 管道]] · [[../NPC活人感模块/08-NPC行动涌现与分类/002-物理原子层定义与签名|物理原子层]]
> **参考**: [[../../../../参考文档/038-Godot4.7技术栈升级评估-20260621/003-语音系统架构修正-Rust解耦|语音系统架构修正]]

---

## 一、定位——不是独立系统

Bark（语声）是 **SoundFootprint 的一个 sound_type 变体**。和脚步声、关门声、敲铁声一样——走完全相同的音频管道。

```
Bark → SoundFootprint { sound_type: SoundType::Bark, bark_type: u8, ... }
  → AudioQuery 传播
  → AudioStreamPlayer 池播放
  → 听者 NPC 通过已有感官管道感知——"他在生气"
```

不需要 BarkEvent 独立类型。不需要 BarkRenderer 独立系统。不需要新 trait。

---

## 二、BarkType —— 12 类

```rust
enum BarkType {
    // 生理 (3)——物理原子副作用
    Pain,          // 疼痛——强度区分闷哼 vs 尖叫
    Exertion,      // 用力——举重"哼！" vs 被击倒"呃！"
    Fatigue,       // 疲惫——触发时 stamina < 阈值

    // 情感 (5)——情绪阈值触发·有迟滞
    Anger,         // 愤怒——强度区分低吼 vs 咆哮
    Fear,          // 恐惧——强度区分呜咽 vs 尖叫
    Sadness,       // 悲伤——强度区分叹息 vs 抽泣
    Joy,           // 喜悦——强度区分轻笑 vs 大笑
    Surprise,      // 惊讶——倒吸气

    // 社会 (3)——PhaticFragment 渲染层
    Greet,         // 问候——关系值调制暖/冷/中性
    Assent,        // 认同/否定——短促"Mm" vs "啧"
    Farewell,      // 道别——"嗯"(降调)

    // 群体 (1)——非个体·多个 NPC 同步触发
    CrowdReaction, // 欢呼/嘘声——intensity 区分
}
```

**12 种——非 22 或 25。** PainGrunt/PainSharp 合并为 Pain（强度区分）。AngryGrowl/AngryShout 合并为 Anger（强度区分）。CollectiveGasp/SurpriseGasp 合并为 Surprise。WalkHeavy/WorkExertion/IdleHum 移出 Bark——它们是持续性动作呼吸声，归步态和物理原子。

---

## 三、触发源——不新增 trait

| 触发源 | 机制 | 映射 |
|--------|------|------|
| **物理原子副作用** | AtomEvent 发布 → Bark 系统订阅 | atom_bark_mapping.toml |
| **情绪阈值** | EmotionState × 迟滞 × 冷却 | emotion_bark_check() 纯函数 |
| **PhaticFragment** | 对话系统产出社会短语 → 走 Bark（不需要 TTS） | PhaticFragment.display_mode |
| **群体事件** | 多个 NPC 对同一事件独立反应 → 自然涌现 | 无中心控制 |

### 3.1 atom_bark_mapping.toml

```toml
[atom.STRIKE]
self_damage_threshold = 0.3
bark_type = "pain"
intensity = "self_damage"
can_interrupt_tts = true

[atom.LIFT]
stamina_threshold = 0.6
bark_type = "exertion"
intensity = "stamina_consumed"
can_interrupt_tts = false
```

### 3.2 情绪 Bark 触发——迟滞+冷却

```rust
fn emotion_bark_check(emotion: &EmotionState, cooldown: &mut BarkCooldown, now: u64) -> Option<(BarkType, f32)> {
    // 最高情绪 + 强度
    let (dominant, intensity) = emotion.dominant();

    // 迟滞: 触发后抬高阈值——防止同一 NPC 连续 3 次愤怒咆哮
    let threshold = cooldown.elevated_threshold(dominant);
    if intensity < threshold { return None; }

    let bark_type = match dominant {
        EmotionType::Anger    => BarkType::Anger,
        EmotionType::Fear     => BarkType::Fear,
        EmotionType::Sadness  => BarkType::Sadness,
        EmotionType::Joy      => BarkType::Joy,
        EmotionType::Surprise => BarkType::Surprise,
        _ => return None,
    };

    // 同类冷却 ×2
    if !cooldown.can_bark(now, bark_type) { return None; }

    Some((bark_type, intensity))
}
```

---

## 四、空间密度限制——Godot 侧

```gdscript
const CELL_SIZE = 10.0  # 米
var active_bark_cells: Dictionary = {}  # Vector3i → Vec<ActiveBark>

func can_play(pos: Vector3, bark_type: int) -> bool:
    var cell = Vector3i(pos / CELL_SIZE)
    var in_cell = active_bark_cells.get(cell, [])

    if in_cell.size() >= 5: return false
    var same_type = in_cell.filter(func(b): return b.type == bark_type)
    if same_type.size() >= 2: return false
    return true
```

---

## 五、样本——预生成

```
12 种 Bark × 4 个 VoiceProfile 底色 = 48 个 Piper 生成样本
+ 8 种"不可说型"(pain/exertion/fear/joy/crowd×3) × 1 真人底版 = 8 个自行录制
  → XTTS 克隆为 4 音色 = 32 个样本

总计: 80 个 .ogg。~2MB 包体。
实时: Godot AudioStreamPlayer.pitch_scale + volume_db —— 不经过 Rust fundsp。
```

**文化 Bark**：同一种 BarkType 在不同文化中样本不同——通过 TOML 映射。

```toml
# barks.toml
[bark.assent]
default_sample = "bark_assent_mm.ogg"

[culture.elven_woodland.bark.assent]
sample = "bark_assent_soft_aah.ogg"  # 精灵用轻"啊"代替"嗯"
```

---

## 六、与 Piper TTS 的分工

```
TTS 管道                     Bark 管道
════════                     ════════
有文本内容/信息需要传达       无文本——非语言声
Piper ONNX (800-1500ms)      预生成样本 (<1ms 取出)
合成昂贵 (≤3 并发)            播放极廉 (无上限)
触发: GOAP 对话目标           触发: 物理原子·情绪·社交·群体
LOD 自动降解语义              LOD 纯距离衰减——无内容可降解
```

---

## 七、群体 Bark 涌现

```
50 个 NPC 在广场听演讲 → 演讲者激昂 → 每个 NPC 独立评估:
  情感上升 → anger > 0.7 → Bark(Anger)
  
  不是同时触发——注意力差异 + 个体随机延迟 → 自然分布在 0.3-0.7s 内
  → 20 个 Bark 在 ~0.5s 窗口内 → AudioServer 自动混合 → "咆哮声浪"
  → 密度检查: 10m 网格 ≤5 并发, ≤2 同类 → 自动限制
```

**不需要"群体同步"代码。每个 NPC 独立决策。AudioServer 的自然混合产生声浪。这是涌现。**

---

> **关联**: [[005-语音管道|005 语音管道]] · [[006-Piper合成管道|006 Piper 管道]] · [[../NPC活人感模块/08-NPC行动涌现与分类/002-物理原子层定义与签名|002 物理原子层]]
