# 003 — 语音系统架构修正 · Rust 解耦

> **来源**: Godot 4.7 技术栈升级讨论——语音系统审计
> **日期**: 2026-06-21
> **状态**: 参考文档。待 Godot 4.7 + gdext 支持确认后转正。
> **审计对象**: `音频系统/005-语音管道.md` · `语言表达/012-语音输出接口.md`
> **关联**: [[../001-Godot4.7功能评估|001 功能评估]] · [[../002-画面渲染管线v4.7修订|002 渲染管线]]

---

## 〇、审计发现的致命问题

| # | 问题 | 已有设计 | 修正 |
|---|------|---------|------|
| 1 | **TTS 合成位置矛盾** | 012: "TTS 是 Godot 侧的渲染层" | **推翻**——TTS 合成在 Rust。理由：Piper 是 ONNX Rust 原生；DSP（fundsp）是 Rust 库；espeak-ng 音素信息在 Rust 侧 |
| 2 | **TtsEngine trait 输出不足** | `fn speak() → Result<(), TtsError>` | 重命名为 `synthesize()`，返回 `SynthesizedUtterance { audio_buffer, text_timeline, total_duration_ms }` |
| 3 | **SpeakingProfile 与 VoiceProfile** | VoiceProfile 在音频模块。SpeakingProfile 不存在 | 新增 SpeakingProfile（语言表达模块）。两者从同一 NPC 数据派生——VoiceProfile 管声学，SpeakingProfile 管说话节奏 |
| 4 | **ListenerInterrupt 缺失** | CancelReason 无"听众打断" | 新增变体 |
| 5 | **VoiceManager 双定义** | 012-VoiceManager（语言表达）和 005-VoiceManager（音频） | 合并至音频模块。Godot 侧不负责队列管理 |
| 6 | **文字时间轴不存在** | CurrentSpeech 只有 word_count / words_per_second | 扩展 char_timeline 字段 |

---

## 一、Rust 解耦架构全景

```
═══════════ 模块边界 ═══════════

woworld_types（共享类型·零依赖）:
  ├─ VoiceProfile                   // OWNER: 音频模块
  ├─ SpeakingProfile                // OWNER: 语言表达模块
  ├─ VoiceEmotionModulation         // OWNER: 音频模块
  ├─ SynthesizedUtterance           // OWNER: 音频模块（TtsEngine 产出）
  ├─ CurrentSpeech                  // OWNER: 语言表达模块（写入 NpcData）
  ├─ CancelReason                   // OWNER: 音频模块
  ├─ VoicePacket                    // OWNER: 语言表达模块
  ├─ SpeechDelivery                 // OWNER: 语言表达模块
  └─ VoicePriority                  // OWNER: 语言表达模块

woworld_core（核心 trait）:
  └─ TtsEngine trait                // 定义合成接口——消费 VoicePacket，产出 SynthesizedUtterance

woworld_language（语言表达模块）:
  ├─ 文本生成 → 原始文本
  ├─ SpeakingProfile::derive(npc)   // 从 BigFive + 情绪 + 文化派生
  ├─ 标点映射                       // SpeakingProfile → 文本标点版本
  ├─ VoicePacket 构造               // 标点文本 + VoiceProfile引用 + 情绪调制
  ├─ CurrentSpeech 写入 NpcData     // 音频模块的轮询入口
  ├─ DialogueIntent / Conversation / TurnAllocator
  └─ 零音频依赖——不知道声音的存在

woworld_audio（音频模块）:
  ├─ VoiceProfile::derive(npc)      // 从 种族 + 性别 + 年龄 + BigFive 派生
  ├─ PiperEngine                    // impl TtsEngine for PiperEngine
  ├─ 合成管道                       // espeak-ng音素化 → Piper → fundsp DSP → 静默调整 → 装配
  ├─ 文字时间轴计算                  // 从音素序列精确计算
  ├─ VoiceManager                   // 合成队列 + 打断控制 + ModelPool 调度
  ├─ OngoingSpeech 追踪              // 轮询 CurrentSpeech → 跟踪播放状态
  ├─ SpeechEvent → SpatialEventBus  // 周围 NPC 通过 AudioQuery 感知
  ├─ AudioQuery::perceived_speech() // 听者查询——fraction_heard + clarity + 传播延迟
  └─ 不接触标点规则——不知道"SpeakingProfile"

woworld_godot（Godot 绑定层）:
  ├─ AudioStreamGenerator ← 接收音频缓冲区 → 播放
  ├─ 每帧文字同步: get_playback_position() → 文字显示进度
  ├─ AudioBus 环境混响（唯一在 Godot 处理的音频效果）
  └─ 零合成逻辑——不知道 Piper 存在
```

**关键解耦**：
- 语言表达不知道音频的存在（已有原则——保持不变）
- 音频不知道标点规则的存在（SpeakingProfile 只在语言表达）
- Godot 不知道 TTS 引擎的存在（只消费音频缓冲区）
- 所有新模块均可独立编译和测试

---

## 二、共享类型定义（woworld_types）

### 2.1 VoiceProfile（音频模块 OWN · 已有 · 不变）

```rust
/// 每个 NPC 的声学身份——"听起来如何"
/// OWNER: woworld_audio
/// 派生: 种族 × 性别 × 年龄 × BigFive
#[derive(Clone)]
pub struct VoiceProfile {
    pub profile_id: VoiceProfileId,
    pub base_pitch_hz: f32,        // 基频
    pub pitch_variance: f32,       // 自然波动 0-1
    pub timbre: TimbrePreset,      // 音色
    pub base_speed_wpm: f32,       // 词/分钟（与 SpeakingProfile.base_speed 互为转换）
    pub expressiveness: f32,       // 0=单音调, 1=丰富抑扬
    pub breathiness: f32,          // 0=清爽, 1=气声
    pub roughness: f32,            // 0=平滑, 1=粗粝
    pub source: VoiceSource,
}
```

### 2.2 SpeakingProfile（语言表达模块 OWN · 🆕 新增）

```rust
/// 每个 NPC 的说话节奏——"怎么说的"
/// OWNER: woworld_language
/// 派生: BigFive × 情绪 × 文化
/// 
/// 与 VoiceProfile 的关系:
///   - 两者从同一 NPC 数据的不同维度派生
///   - VoiceProfile 管声学（音色/音高/气声）
///   - SpeakingProfile 管节奏（语速/停顿/犹豫/咬字/换气）
///   - VoiceProfile.base_speed_wpm ←→ SpeakingProfile.base_speed 可互相转换:
///       base_speed_wpm = 60,000 / (base_speed_ms × avg_chars_per_word)
#[derive(Clone)]
pub struct SpeakingProfile {
    pub base_speed_ms: f32,       // 平均每个字的间隔（ms）。60=极快, 180=极慢
    pub clause_pause_ms: u32,     // 分句之间的停顿时长（ms）。150-2000
    pub hesitation_rate: f32,     // 0-1, 插入"嗯…""……"的频率
    pub interruption_tolerance: f32, // 0-1, 被打断后需要多少秒才能重新开口。0.1=马上, 2.0=沉默很久
    pub enunciation: f32,         // 0-1, 咬字清晰度。影响 Piper noise_scale
    pub breath_rhythm: f32,       // 0.3-1.5, 换气节奏。<1=短句多换气, >1=长句从呼吸容
}
```

### 2.3 SpeakingProfile 派生

```rust
impl SpeakingProfile {
    /// 从 NPC 已有字段派生——零新增数据字段
    pub fn derive(
        personality: &BigFive,
        emotion: &EmotionState,
        culture_speaking: &CultureSpeakingStyle,
        individual_seed: u64,
    ) -> Self {
        // ── 基础映射 ──
        let mut base = SpeakingProfile {
            base_speed_ms: Self::derive_base_speed(personality, emotion),
            clause_pause_ms: Self::derive_clause_pause(personality),
            hesitation_rate: Self::derive_hesitation(personality),
            interruption_tolerance: Self::derive_interruption_tolerance(personality),
            enunciation: Self::derive_enunciation(personality),
            breath_rhythm: Self::derive_breath_rhythm(personality, emotion),
        };

        // ── 情感调制 ──
        base.apply_emotion(emotion);

        // ── 文化偏移 ──
        base.apply_culture(culture_speaking);

        // ── 个体差异 ±12% ──
        base.apply_individual(individual_seed);

        base
    }

    fn derive_base_speed(personality: &BigFive, emotion: &EmotionState) -> f32 {
        // 外向→快，开放→中，神经质→快（焦虑）或慢（焦虑到说不出话）——双模态
        let mut speed = 100.0 // 基线 ~100ms/字 = ~150词/分钟
            - personality.extraversion * 25.0   // 外向快
            - personality.openness * 10.0       // 开放略快
            + personality.neuroticism * 5.0;     // 微弱加速

        // 双模态: 高神经质有30%概率反而是"焦虑到说不出话"→语速更慢
        if personality.neuroticism > 0.7 {
            // 确定性随机——从 individual_seed 派生
            if seeded_bool(individual_seed, 0.3) {
                speed += 40.0;  // 更慢——"我在犹豫"
            }
        }

        speed.clamp(50.0, 200.0)
    }

    fn derive_clause_pause(personality: &BigFive) -> u32 {
        let base = 600  // 基线 600ms 分句停顿
            - (personality.extraversion * 300.0) as u32   // 外向→短停顿
            + (personality.openness * 400.0) as u32       // 开放→长停顿（思考措辞）
            + (personality.conscientiousness * 200.0) as u32; // 尽责→长停顿（挑词）
        base.clamp(150, 2000)
    }

    fn derive_hesitation(personality: &BigFive) -> f32 {
        (personality.neuroticism * 0.5 + personality.openness * 0.3
         - personality.extraversion * 0.2
         - personality.conscientiousness * 0.4)
        .clamp(0.0, 1.0)
    }

    fn derive_interruption_tolerance(personality: &BigFive) -> f32 {
        // 低宜人性 + 高外向 → 高容忍——"被吼回来也能立刻继续"
        // 高神经质 + 高尽责 → 低容忍——"组织了半天被打断需要很久恢复"
        (1.5 - personality.agreeableness * 0.8 - personality.extraversion * 0.5
         + personality.neuroticism * 0.8 + personality.conscientiousness * 0.5)
        .clamp(0.1, 2.5)  // 秒
    }

    fn derive_enunciation(personality: &BigFive) -> f32 {
        // 尽责性高→咬字清晰
        // 低神经质→咬字稳定
        (0.5 + personality.conscientiousness * 0.5
         - personality.neuroticism * 0.3)
        .clamp(0.2, 1.0)
    }

    fn derive_breath_rhythm(personality: &BigFive, emotion: &EmotionState) -> f32 {
        // 高神经质→短促呼吸。高开放性→从容。
        let base = 1.0 - personality.neuroticism * 0.4 + personality.openness * 0.3;
        // 情绪: 愤怒→急促，悲伤→深慢，恐惧→急促
        let emotional = -emotion.anger * 0.3 - emotion.fear * 0.3 + emotion.sadness * 0.1;
        (base + emotional).clamp(0.3, 1.5)
    }

    fn apply_emotion(&mut self, emotion: &EmotionState) {
        // 愤怒: 加速 + 短停顿 + 无犹豫
        self.base_speed_ms -= emotion.anger * 15.0;
        self.clause_pause_ms = self.clause_pause_ms.saturating_sub((emotion.anger * 300.0) as u32);
        self.hesitation_rate = (self.hesitation_rate - emotion.anger * 0.4).max(0.0);

        // 悲伤: 减速 + 长停顿 + 多犹豫 + 气声
        self.base_speed_ms += emotion.sadness * 30.0;
        self.clause_pause_ms += (emotion.sadness * 800.0) as u32;
        self.hesitation_rate = (self.hesitation_rate + emotion.sadness * 0.5).min(1.0);

        // 恐惧: 加速 + 震音
        self.base_speed_ms -= emotion.fear * 10.0;
    }

    fn apply_culture(&mut self, culture: &CultureSpeakingStyle) {
        self.base_speed_ms = (self.base_speed_ms as f32 * culture.speed_factor) as _;
        self.clause_pause_ms = (self.clause_pause_ms as f32 * culture.pause_factor) as u32;
    }

    fn apply_individual(&mut self, seed: u64) {
        let r = seeded_f32_range(seed, 0.88, 1.12);
        self.base_speed_ms *= r;
        self.clause_pause_ms = (self.clause_pause_ms as f32 * r) as u32;
        self.hesitation_rate = (self.hesitation_rate * r).clamp(0.0, 1.0);
    }
}
```

### 2.4 SynthesizedUtterance（音频模块 OWN · 🆕 新增）

```rust
/// TTS 合成的完整产出——音频 + 时间轴
/// OWNER: woworld_audio
#[derive(Clone)]
pub struct SynthesizedUtterance {
    /// 单声道 44.1kHz 16-bit PCM 音频缓冲区
    pub audio_buffer: Vec<f32>,

    /// 逐字时间轴: (字符索引, 时间ms)
    /// 字符索引指向 VoicePacket.text（标点版本）
    pub text_timeline: Vec<(u16, u32)>,

    /// 音频总时长 ms
    pub total_duration_ms: u32,

    /// 使用的音色
    pub voice_profile_id: VoiceProfileId,
}
```

### 2.5 CancelReason 扩展（音频模块 OWN）

```rust
pub enum CancelReason {
    // ── 已有 ──
    SourceDied,
    SourceUnconscious,
    SourceTeleported,
    SourceSubmerged,
    CombatImpact,
    Preempted(SoundEventId),
    MagicallySilenced,
    SourceVoluntaryStop,

    // ── 🆕 新增 ──
    /// 听众主动打断——玩家按打断键，或 NPC 在自由对话中抢话
    /// 被打断的音频和文字立即中止。未显示的后半句永久丢失。
    ListenerInterrupt {
        /// 打断者
        by: EntityId,
        /// 打断的意图类型（对接对话系统的 InterruptionType）
        intent: InterruptIntent,
    },
}

pub enum InterruptIntent {
    /// 抢话——"我有话要说"
    Interject,
    /// 提问——"等等，你说什么？"
    Question,
    /// 欢呼——不打断内容但盖过声音
    Cheer,
    /// 嘘声
    Boo,
    /// 离场——"我不听了"
    WalkOut,
}
```

### 2.6 CurrentSpeech 扩展（语言表达模块 OWN）

```rust
pub struct CurrentSpeech {
    // ── 已有字段 ──
    pub expression_ref: ExpressionRef,
    pub started_at: GameInstant,
    pub word_count: u16,
    pub words_per_second: f32,
    pub delivery: SpeechDelivery,

    // ── 🆕 新增 ──
    /// 逐字时间轴——由 TtsEngine::synthesize() 产生，音频模块填入
    /// 字符索引 → 从音频开始计的时间 (ms)
    /// 音频模块在合成完成后写入此字段——供听者查询"说到第几个字了"
    pub char_timeline: Vec<(u16, u32)>,

    /// 音频缓冲区引用——在音频模块内部管理的池中
    /// 不存储在 CurrentSpeech 本身——通过 audio_pool_id 查找
    pub audio_pool_id: Option<AudioPoolId>,
}
```

---

## 三、TtsEngine trait 重设计（woworld_core）

```rust
/// TTS 合成引擎——仅有这一个方法用于语音合成
/// 
/// OWNER: woworld_core（trait 定义）
/// IMPL: woworld_audio（PiperEngine）
/// 
/// 关键: 这是纯合成——不播放。返回数据。
///       Godot 侧不调用此 trait。音频模块的 VoiceManager 调用。
pub trait TtsEngine: Send + Sync {
    /// 引擎名称——用于日志和调试
    fn name(&self) -> &str;

    /// 引擎是否可用（模型是否已加载）
    fn is_available(&self) -> bool;

    /// 合成——将文本转换为音频 + 时间轴
    /// 
    /// 纯计算——无副作用。不播放音频。
    /// 通过 CancellationToken 支持取消（打断时）。
    fn synthesize(
        &self,
        packet: &VoicePacket,
        cancel: &CancellationToken,
    ) -> Result<SynthesizedUtterance, TtsError>;

    /// 停止当前合成——取消正在进行的 ONNX 推理
    fn cancel(&self);

    /// 预生成预览——用于编辑器中试听 VoiceProfile
    fn preview(
        &self,
        text: &str,
        profile: &VoiceProfile,
    ) -> Result<SynthesizedUtterance, TtsError>;
}
```

---

## 四、合成管道——Rust 侧完整流程

### 4.0 核心设计：标点即编码

静默时长的控制不经过额外的数据通道。标点本身就是编码：

```
SpeakingProfile（语言表达模块）         Piper（音频模块）
  clause_pause_ms = 200                    看到 "," → 自然停顿 ~120ms ✓
  → 标点映射: ","                          看到 "." → 自然停顿 ~350ms ✓
                                          看到 "……" → 自然停顿 ~500ms ✓
  clause_pause_ms = 1800                    
  → 标点映射: "……\n\n"                    看到 "……\n\n" → 自然停顿 ~1200ms ✓
```

标点映射由 SpeakingProfile 驱动——在构造 VoicePacket 之前完成。VoicePacket.text 已经携带了 SpeakingProfile 的完整停顿意图。Piper 对标点符号的停顿行为是**稳定的**（同一音素序列→同一时长，±200ms）。±200ms 对文字逐字呈现的体验不可感知。

**因此**：不需要"静默调整"步骤。不需要音频模块知道 SpeakingProfile。解耦是自然的——标点本身就是跨模块的协议。

### 4.1 五步管道

```
══════ 音频模块 · synthesize() 内部 ══════

输入: VoicePacket { text(标点版本), voice_profile_id, emotion_mod, delivery, priority }

Step 1 — 音素化 (espeak-ng · <1ms)
  text → phoneme_sequence: Vec<(Phoneme, Duration)>
  标点自动映射为静默音素: 
    ","      → Silence(~120ms)
    "."      → Silence(~350ms)
    "……"    → Silence(~500ms)
    "\n"     → Silence(~200ms，换气的微停顿)
    "\n\n"   → Silence(~700ms，段落级长停顿)
  Duration 是 espeak-ng 的估算——确定性: 同一输入→同一输出。

Step 2 — 文字时间轴（从 phoneme_sequence 直接算 · <0.1ms）
  → 不等 Piper 完成。与 Step 3 并行。
  audio_pos = 0
  for each (phoneme, duration) in phoneme_sequence:
    if phoneme starts new character boundary:
      text_timeline.push((char_index, audio_pos_ms))
    audio_pos += duration
  → text_timeline: Vec<(u16, u32)>
  → total_duration_ms = audio_pos

Step 3 — Piper 整句生成（ONNX 后台线程 · 800-1500ms）
  phoneme_sequence → Piper ONNX → raw_audio: Vec<f32>
  整句——不分段。自然韵律完整。
  基于 VoiceProfile 选择 ONNX 模型。
  通过 CancellationToken 支持取消（打断时）。

Step 4 — DSP 后处理（fundsp · <2ms）
  VoiceEmotionModulation 驱动的五阶效果链:
    pitch_shift(emotion_mod.pitch_shift_semitones)
    → formant_shift(voice_profile.formant_shift_ratio)
    → saturation(voice_profile.roughness + emotion_mod.roughness_add)
    → tremolo(emotion_mod.tremor)
    → eq_high_boost(voice_profile.breathiness + emotion_mod.breathiness_add)
  → processed_audio: Vec<f32>

  注: formant_shift_ratio 从 VoiceProfile 派生:
    formant_shift_ratio = (adult_ref_pitch / base_pitch_hz).clamp(0.6, 1.7)
    → 高音(儿童/女性) → ratio < 1.0 → 共振峰上移
    → 低音(老年/男性) → ratio > 1.0 → 共振峰下移
    不需要新增 VoiceProfile 字段——从已有 base_pitch_hz 派生。

Step 5 — 返回
  SynthesizedUtterance { 
    audio_buffer: processed_audio,
    text_timeline,            // Step 2 产出——音频生成前已就绪
    total_duration_ms,        // Step 2 产出
    voice_profile_id
  }

══════ 性能 ══════
  Step 1 espeak-ng:         <1ms       (确定·主线程可做)
  Step 2 时间轴:            <0.1ms     (确定·主线程可做)
  Step 3 Piper ONNX:        800-1500ms (后台 rayon 线程——不触主线程)
  Step 4 fundsp DSP:        <2ms       (后台线程·4s 音频 @ 44.1kHz mono)
  主线程阻塞:               <1.1ms     (Step 1+2 同步——需要音素序列来做 Piper 输入和时间轴)
  后台线程总计:              ~1.0-1.7s (Piper 绝对主导)
```

**关键**：文字时间轴（Step 2）在 Piper 生成之前就计算完毕。两者基于同一份 phoneme_sequence——天然对齐。不需要"音频生成后再检测字符边界"。不需要静默调整。不需要音频模块知道 SpeakingProfile。

### 4.2 VoiceEmotionModulation —— DSP 驱动的参数来源

```rust
/// 情绪→声学调制——每次发言时从 NPC 当前情绪派生
/// OWNER: woworld_audio（与已有的 005 §七 保持一致）
#[derive(Clone)]
pub struct VoiceEmotionModulation {
    pub pitch_shift_semitones: f32,   // 半音偏移——高兴+2, 悲伤-3, 愤怒+1.5
    pub speed_multiplier: f32,        // 语速倍率——愤怒1.3, 恐惧1.5, 悲伤0.7
    pub volume_multiplier: f32,       // 音量倍率——愤怒1.4, 恐惧0.6
    pub tremor: f32,                  // 震音 0-1——恐惧0.6, 悲伤0.3
    pub breathiness_add: f32,         // 额外气声——悲伤0.3, 疲惫0.5
    pub roughness_add: f32,           // 额外粗粝——愤怒0.3
}

impl VoiceEmotionModulation {
    pub fn from_emotion(emotion: &EmotionState) -> Self {
        VoiceEmotionModulation {
            pitch_shift_semitones: 
                emotion.joy * 2.0 - emotion.sadness * 3.0 + emotion.anger * 1.5,
            speed_multiplier: 
                1.0 + emotion.anger * 0.3 + emotion.fear * 0.5 - emotion.sadness * 0.3,
            volume_multiplier: 
                1.0 + emotion.anger * 0.4 - emotion.fear * 0.4 - emotion.sadness * 0.2,
            tremor: 
                emotion.fear * 0.6 + emotion.sadness * 0.3,
            breathiness_add: 
                emotion.sadness * 0.3,
            roughness_add: 
                emotion.anger * 0.3,
        }
    }
}
```

### 4.3 TtsError —— 合成失败模式

```rust
/// TTS 合成错误
pub enum TtsError {
    /// ONNX 模型未加载
    ModelNotLoaded { profile_id: VoiceProfileId },
    /// 合成被取消（打断/引擎停止）
    Cancelled,
    /// espeak-ng 音素化失败（文本含有不可发音的字符）
    PhonemizationFailed { text_snippet: String },
    /// ONNX 推理失败
    InferenceFailed { reason: String },
    /// 音频缓冲区分配失败（内存不足——极为罕见）
    BufferAllocationFailed,
}
```

---

## 五、标点映射——语言表达模块

```rust
/// 将原始文本转换为带标点的版本——由 SpeakingProfile 驱动
/// OWNER: woworld_language
/// 调用时机：构造 VoicePacket 之前
pub fn punctuate_for_speaking(raw_text: &str, profile: &SpeakingProfile, seed: u64) -> String {
    let hesitation = if profile.hesitation_rate > seeded_f32_range(seed, 0.0, 1.0) {
        match seeded_u32_range(seed, 0, 3) {
            0 => "……",
            1 => "…",
            2 => "嗯……",
            _ => "",
        }
    } else {
        ""
    };

    // 注意: 这里不精确控制毫秒——标点驱动 Piper 的自然停顿行为
    // 精确控制由音频模块的静默调整完成
    let separator = if profile.clause_pause_ms < 200 {
        ", "            // Piper ~120ms 自然停顿
    } else if profile.clause_pause_ms < 800 {
        ". "            // Piper ~350ms 自然停顿
    } else if profile.clause_pause_ms < 1500 {
        ".\n"           // Piper ~700ms 自然停顿
    } else {
        "……\n\n"       // Piper ~1200ms 自然停顿
    };

    format!("{}{}{}.", hesitation, raw_text, separator)
}
```

**关键**：标点不控制毫秒级精确度。它只给 Piper 一个信号——"这里有停顿"。停多长由 Piper 自然韵律决定，再经音频模块的静默调整微调到 SpeakingProfile 的期望值。

---

## 六、VoiceManager 合并（音频模块）

```rust
/// 语音合成管理器——统管 Piper 生成队列 + 打断控制
/// OWNER: woworld_audio
/// 取代: 012-VoiceManager（语言表达）和 005-VoiceManager（音频）的双定义
pub struct VoiceManager {
    /// TTS 合成引擎
    engine: Box<dyn TtsEngine>,

    /// 合成队列——按优先级排序
    queue: VecDeque<VoicePacket>,

    /// 当前正在合成的语音包（在后台线程中）
    current: Option<VoicePacket>,

    /// 模型实例池——同模型避免竞争
    model_pool: ModelPool,

    /// 已合成音频的缓存池
    audio_pool: AudioPool,

    /// TTS 配置
    config: TtsConfig,
}

impl VoiceManager {
    /// 入队——高优先级打断当前合成
    pub fn enqueue(&mut self, packet: VoicePacket) {
        match packet.priority {
            VoicePriority::Critical => {
                self.engine.cancel();          // 中断当前 ONNX 推理
                self.current = None;
                self.queue.push_front(packet); // 插队到最前
            }
            VoicePriority::High | VoicePriority::Normal => {
                self.queue.push_back(packet);
            }
            VoicePriority::Ambient | VoicePriority::Background => {
                // 环境语音只在空闲时合成
                if self.queue.len() < 2 {
                    self.queue.push_back(packet);
                }
                // 否则丢弃——远处闲聊不值得消耗合成资源
            }
        }
        self.try_synthesize_next();
    }

    /// 消耗队列——在专用 rayon 线程上启动合成
    fn try_synthesize_next(&mut self) {
        if self.current.is_some() { return; }
        if let Some(packet) = self.queue.pop_front() {
            self.current = Some(packet.clone());
            let engine = &self.engine;
            let cancel = CancellationToken::new();

            rayon::spawn(move || {
                let result = engine.synthesize(&packet, &cancel);
                // 合成完成 → 通过 channel 送回主线程
                // → 主线程将 SynthesizedUtterance 写入 audio_pool
                // → 写入 NpcData.audio_pool_id
                // → Godot 在下一帧读取并开始播放
            });
        }
    }

    /// 打断——外部触发（玩家按键 / NPC 抢话）
    pub fn interrupt(&mut self, listener: EntityId, intent: InterruptIntent) {
        if let Some(ref packet) = self.current {
            self.engine.cancel();
            // 打断事件 → 写入 SpatialEventBus
            // CancelReason::ListenerInterrupt { by: listener, intent }
            self.current = None;
        }
        // 立即尝试下一个
        self.try_synthesize_next();
    }
}

/// 模型实例池——避免同一 ONNX 模型被两个合成竞争
struct ModelPool {
    instances: HashMap<VoiceProfileId, Vec<PiperSession>>,
    max_instances_per_model: usize,  // 默认 2
}

/// 已合成音频的缓存池
struct AudioPool {
    utterances: HashMap<AudioPoolId, SynthesizedUtterance>,
}
```

---

## 七、语音合成到播放的完整时间线

```
时刻线 →

t=0ms    语言表达模块:
          NPC 心智产生对话文本
          → SpeakingProfile.punctuate(文本) → 标点版本
          → 构造 VoicePacket { text, voice_profile_id, emotion_mod, ... }
          → 写入 NpcData.current_speech
          → VoiceManager.enqueue(VoicePacket)

t=0ms    音频模块:
          VoiceManager.try_synthesize_next()
          → rayon::spawn(PiperEngine.synthesize(packet))
          → 后台线程开始合成 (~0.8-1.5s)

t=0ms    Godot:
          本帧检测到 NpcData.current_speech.is_some()
          → 文字区域开始显示（但尚无文字——合成尚未完成）
          → 可显示 "……" 或说话者名字

t≈200ms  合成进行中——espeak-ng 音素化完成
          Piper ONNX 推理中

t≈900ms  合成完成
          → SynthesizedUtterance 写入 audio_pool
          → NpcData.current_speech.audio_pool_id = Some(id)
          → NpcData.current_speech.char_timeline = utterance.text_timeline

t≈900ms  Godot 下一帧:
          检测到 audio_pool_id 就绪
          → AudioStreamGenerator.set_buffer(utterance.audio_buffer)
          → AudioStreamPlayer.play()
          → 文字逐字显示开始

t=900ms-4900ms  播放中（4秒话语）
          每帧: pos = AudioStreamPlayer.get_playback_position()
               chars = char_timeline.filter(t < pos).last_index
               UI.display(text[0..chars])

t=1800ms 玩家按打断键
          → VoiceManager.interrupt(player_id, Interject)
          → AudioStreamPlayer.stop()
          → 文字停在第 N 个字
          → CancelReason::ListenerInterrupt → SpatialEventBus
          → 说话者的 interruption_tolerance → 沉默期
          → 新说话者的 Piper 合成开始

t=4900ms 正常说完（未被打断）
          → AudioStreamPlayer.finished
          → SpatialEventBus: SpeechEvent::Stopped(SourceVoluntaryStop)
          → 听者获得完整 fraction_heard = 1.0
```

---

## 八、多人对话与闹市场景

### 8.1 多人对话——生成队列

```
精灵少女说话 → Piper 生成中
战士打断     → VoiceManager.interrupt(战士, Interject)
             → 少女当前合成取消。未播放的音频丢弃。
             → 少女的 interruption_tolerance=0.2 → 需要 2.5s 沉默期
             → 战士 Piper 开始生成队列

商人插话     → VoiceManager.enqueue(商人, Normal)
             → 战士仍在生成中——商人排队

0.8s 后战士生成完成 → 开始播放
2.0s 后战士说完  → VoiceManager.try_synthesize_next()
                → 队列下一个是商人 → 商人生成开始
                → 此时少女的沉默期已过 → 她可以重新入队
```

### 8.2 闹市——LOD 驱动的合成策略

```
距离       语音类型           并发合成数    说明
────       ────────           ────────    ────
0-5m       完整 TTS 合成      1-2         正在对话的 NPC
5-15m      完整 TTS 合成      1-2         明确可闻的说话者
15-30m     Bark 为主          0           Bark 是预生成基元音——不触发 Piper
30m+       环境嗡鸣           0           循环音频——不触发任何合成

同时活跃 Piper 合成: ≤3（ModelPool 限制）
同时活跃 Godot AudioStreamPlayer: ≤8（LOD 0-1）
同时活跃 Bark 播放: 无上限（预生成样本 + DSP——开销极低）
```

### 8.3 观众涌现

```
演讲者说话 → 每个在场 NPC 独立消费:
  AudioQuery::perceived_speech(NPC, 演讲者, now)
  → fraction_heard + clarity + delivery

  每个 NPC 的独立反应:
    高赞同 + 高外向 → bark(Cheer)
    高反对 + 高神经质 → VoiceManager.enqueue("这是谎言！") ← 可能触发完整 TTS
    无立场 + 远距 → 沉默
    
  集群效应:
    相邻 NPC 的 bark 类型相同 → AudioBus 混合形成"声浪"
    → 不需要 100 个 TTS——3-5 个 bark 就形成观众纹理
```

---

## 九、GDExtension 传输 —— Rust → Godot

### 9.1 设计原则

不复制音频数据过 FFI 边界。Rust 管理缓冲生命周期，Godot 按需拉取。

```
Rust 侧:
  合成完成 → AudioPool 分配槽位 → 存储 SynthesizedUtterance
  → NpcData.current_speech.audio_pool_id = Some(pool_id)
  → NpcData.current_speech.char_timeline = utterance.text_timeline

Godot 侧:
  每帧遍历 L1 NPC:
    if npc.speech.audio_pool_id changed since last frame:
      → GDExtension 拉取: get_audio_buffer(pool_id) → PackedFloat32Array
      → AudioStreamGenerator.set_buffer()
      → AudioStreamPlayer.play()
      → GDExtension 拉取: get_text_timeline(npc_id) → PackedInt32Array
         (编码为 [char_index, time_ms, char_index, time_ms, ...])
```

### 9.2 GDExtension API

```rust
// woworld_godot 暴露给 Godot 的函数

/// 获取已合成音频缓冲区
/// 返回空数组 → 尚未合成完成
#[gdextension]
fn get_audio_buffer(pool_id: u32) -> PackedFloat32Array {
    audio_pool.get(pool_id)
        .map(|u| PackedFloat32Array::from(u.audio_buffer.as_slice()))
        .unwrap_or_default()
}

/// 获取 NPC 当前话语的文字时间轴
/// 返回: [char_index, time_ms, ...] 平坦数组
#[gdextension]
fn get_text_timeline(npc_id: u64) -> PackedInt32Array {
    npc_query.current_speech(npc_id)
        .and_then(|s| s.char_timeline.as_ref())
        .map(|tl| {
            let mut flat = Vec::with_capacity(tl.len() * 2);
            for (ci, t) in tl {
                flat.push(*ci as i32);
                flat.push(*t as i32);
            }
            PackedInt32Array::from(flat.as_slice())
        })
        .unwrap_or_default()
}

/// 通知 Rust 播放已完成——回收 AudioPool 槽位
#[gdextension]
fn release_audio_buffer(pool_id: u32) {
    audio_pool.release(pool_id);
}
```

### 9.3 Godot 侧播放与文字同步

```gdscript
# woworld_speech_renderer.gd — 挂在场景中的单例节点

class SpeechRenderer:
    var active_speakers: Dictionary = {}  # EntityId → SpeechState

    class SpeechState:
        var player: AudioStreamPlayer
        var generator: AudioStreamGeneratorPlayback
        var full_text: String
        var timeline: Array       # [[char_index, time_ms], ...]
        var label: Label          # 文字显示节点
        var pool_id: int
        var is_playing: bool

    func _process(_delta):
        # 1. 检测 Rust 侧新的话语
        for npc_id in L1_npc_ids:
            var speech = Rust.get_current_speech(npc_id)
            if speech.audio_pool_id != active_speakers.get(npc_id, {}).pool_id:
                _start_playback(npc_id, speech)

        # 2. 逐字文字同步
        for npc_id in active_speakers:
            var state = active_speakers[npc_id]
            if not state.is_playing:
                continue
            var pos_ms = state.player.get_playback_position() * 1000.0
            var chars = _chars_at_time(state.timeline, pos_ms)
            state.label.text = state.full_text.substr(0, chars + 1)

        # 3. 回收已完成的播放
        for npc_id in active_speakers.keys():
            var state = active_speakers[npc_id]
            if state.is_playing and not state.player.playing:
                Rust.release_audio_buffer(state.pool_id)
                active_speakers.erase(npc_id)

    func _start_playback(npc_id: int, speech: Dictionary):
        var buffer = Rust.get_audio_buffer(speech.audio_pool_id)
        if buffer.is_empty():
            return  # 尚未合成完成——下一帧重试

        var state = SpeechState.new()
        state.pool_id = speech.audio_pool_id
        state.full_text = speech.text
        state.timeline = _unflatten_timeline(Rust.get_text_timeline(npc_id))

        # AudioStreamGenerator 设置
        var stream = AudioStreamGenerator.new()
        stream.buffer_length = buffer.size() / 44100.0 + 0.1
        state.player = AudioStreamPlayer.new()
        state.player.stream = stream
        add_child(state.player)

        # 填充音频缓冲
        state.generator = state.player.get_stream_playback()
        for frame in buffer:
            state.generator.push_frame(Vector2(frame, frame))

        state.player.play()
        active_speakers[npc_id] = state

    func _chars_at_time(timeline: Array, pos_ms: float) -> int:
        var chars = 0
        for entry in timeline:
            if entry[1] > pos_ms:
                break
            chars = entry[0]
        return chars

    func _unflatten_timeline(flat: PackedInt32Array) -> Array:
        var result = []
        for i in range(0, flat.size(), 2):
            result.append([flat[i], flat[i + 1]])
        return result
```

---

## 十、OngoingSpeech 集成 —— 与已有 005 管道对接

### 10.1 char_timeline 驱动 fraction_at()

已有 OngoingSpeech（005 §三）用 `elapsed_at(now) / total_duration` 计算 fraction。当 char_timeline 可用时，精度从"时间比例"提升为"字符级"：

```rust
impl OngoingSpeech {
    /// 话语已传递的字符比例（0-1）
    /// char_timeline 可用 → 精确到字
    /// 不可用 → 回退到时间比例
    fn fraction_at(&self, now: GameInstant, propagation_delay: f32) -> f32 {
        let effective_time = self.elapsed_at(now) - propagation_delay;
        if effective_time <= 0.0 { return 0.0; }

        if let Some(ref timeline) = self.char_timeline {
            // 精确——字符级
            let idx = timeline.partition_point(|(_, t_ms)| 
                (*t_ms as f32) < effective_time * 1000.0
            );
            if idx == 0 { return 0.0; }
            idx as f32 / timeline.len() as f32
        } else {
            // 回退——时间比例。对于 Bark（无时间轴）
            (effective_time / self.total_duration.as_secs_f32()).min(1.0)
        }
    }

    /// 当前已传到的字符索引——用于 UI 显示
    fn current_char_index(&self, now: GameInstant, propagation_delay: f32) -> u16 {
        let effective_time = self.elapsed_at(now) - propagation_delay;
        if effective_time <= 0.0 { return 0; }
        if let Some(ref timeline) = self.char_timeline {
            timeline.partition_point(|(_, t_ms)| 
                (*t_ms as f32) < effective_time * 1000.0
            ) as u16
        } else {
            // 回退: 按时间比例估算
            let frac = effective_time / self.total_duration.as_secs_f32();
            (self.word_count as f32 * frac.min(1.0)) as u16
        }
    }
}
```

### 10.2 AudioQuery::perceived_speech() 增强

已有方法（005 §四）返回 SpeechPerception。当 char_timeline 可用时，fraction_heard 从"估算"变为"精确"：

```rust
fn digest_speech(audio: &dyn AudioQuery, lang: &dyn ExpressionQuery,
                 speaker: EntityId, listener: EntityId, now: GameInstant) 
    → Option<PerceivedUtterance> 
{
    let perception = audio.perceived_speech(listener_pos, speaker, now, hearing)?;
    if perception.fraction_heard <= 0.0 { return None; }

    let level = speech_clarity_level(&perception.clarity);
    let ongoing = audio.get_ongoing(speaker)?;

    // char_timeline 可用 → get_text_up_to_char() 精确截取
    // 不可用 → fraction × word_count 估算
    let partial = if let Some(timeline) = ongoing.char_timeline() {
        let char_idx = ongoing.current_char_index(now, perception.propagation_delay);
        lang.resolve_up_to_char(speech_ref, char_idx as usize, level)
    } else {
        lang.resolve_partial(speech_ref, perception.fraction_heard, level)
    };

    Some(PerceivedUtterance {
        speaker,
        text_snippet: partial,
        is_complete: perception.fraction_heard >= 1.0 && !perception.is_ongoing,
        was_interrupted: perception.cancel_reason.is_some()
            && !matches!(perception.cancel_reason, Some(CancelReason::SourceVoluntaryStop)),
        clarity_level: level,
    })
}
```

---

## 十一、跨模块依赖清单

| 依赖方向 | 消费方 | 提供方 | 内容 | 状态 |
|---------|--------|--------|------|------|
| → | woworld_language | woworld_types | SpeakingProfile, VoicePacket, CurrentSpeech, SpeechDelivery, VoicePriority | 🆕 新增 |
| → | woworld_language | woworld_culture | CultureSpeakingStyle（文化语速因子/停顿因子）| 🆕 需在文化模块新增 |
| → | woworld_language | NPC 模块 | BigFive, EmotionState | 已有 |
| → | woworld_audio | woworld_types | VoiceProfile, VoiceEmotionModulation, SynthesizedUtterance, CancelReason | 部分已有·扩展 |
| → | woworld_audio | woworld_core | TtsEngine trait | 🆕 新增 |
| → | woworld_audio | NPC 模块 | BigFive, EmotionState, LifeEntity (种族/性别/年龄) | 已有 |
| → | woworld_audio | 语言表达模块 | CurrentSpeech (轮询), ExpressionRef | 已有（005 §三） |
| → | woworld_audio | 感官模块 | SpatialEventBus — SpeechEvent 发布 | 已有 |
| → | woworld_godot | woworld_audio | AudioPool 查询, char_timeline 查询 | 🆕 新增 |
| → | 感官模块 | woworld_audio | AudioQuery::perceived_speech() | 已有 |
| → | 经济模块 | woworld_audio | 维护材料需求（语音相关——间接）| 已有 |

**CultureSpeakingStyle 说明**（需在文化模块新增的最小类型）：
```rust
/// 文化对说话方式的影响
/// 存储在 CulturalTraits 中——世界生成时确定
pub struct CultureSpeakingStyle {
    /// 语速因子: 1.0=世界基线, 0.7=慢文化, 1.3=快文化
    pub speed_factor: f32,
    /// 停顿因子: 1.0=基线, 0.5=短停顿文化(话多), 2.0=长停顿文化(重视沉默)
    pub pause_factor: f32,
    /// 对话中的沉默是否被视为尊重（高=沉默是金）
    pub silence_is_respect: f32,  // 0-1
}
```

---

## 十二、已有文档修订摘要

### 012-语音输出接口.md 需修订:

| 位置 | 旧 | 新 |
|------|-----|-----|
| §一 定位 | "TTS 是 Godot 侧的渲染层" | "TTS 合成在 Rust 侧。Godot 负责播放。" |
| §五 TtsEngine trait | `fn speak() → Result<(), TtsError>` | `fn synthesize() → Result<SynthesizedUtterance, TtsError>` |
| §六 VoiceManager | 在语言表达模块 | "VoiceManager 已迁移至音频模块(005)。本模块只保留 VoicePacket 构造和文本生成。" |
| §二 VoiceProfile | 本模块定义的旧版 | 移除。引用音频模块 005 的权威定义 |

### 005-语音管道.md 需修订:

| 位置 | 旧 | 新 |
|------|-----|-----|
| §九 TTS 渲染层 | "TtsEngine trait + TtsConfig 保留在语言表达（渲染层）" | "TtsEngine 在 woworld_core 定义。PiperEngine 在音频模块实现。Godot 无 TTS 逻辑。" |
| §六 VoiceProfile | 已有——不变 | 新增转换接口: `VoiceProfile.to_wpm() → SpeakingProfile.to_ms()` |
| §八 VoiceManager | 与 012 重复定义 | 合并——此处为权威定义。新增 ModelPool、AudioPool、interrupt() |
| §五 CancelReason | 8 个变体 | 新增 ListenerInterrupt { by: EntityId, intent: InterruptIntent } |
| §二 CurrentSpeech | word_count + words_per_second | 新增 char_timeline: Vec<(u16, u32)> + audio_pool_id: Option<AudioPoolId> |

---

## 十三、性能预算

| 操作 | 频率 | 位置 | 成本 |
|------|------|------|------|
| espeak-ng 音素化 | 每次合成 | Rust 后台线程 | <1ms |
| Piper ONNX 推理 | 每次合成 | Rust 后台线程 | 800-1500ms（异步——不触主线程） |
| fundsp DSP | 每次合成 | Rust 后台线程 | <2ms（4s音频） |
| 静默调整+时间轴 | 每次合成 | Rust 后台线程 | <0.5ms |
| VoiceManager.enqueue | 每次发言 | Rust 主线程（轻量） | <1µs |
| CurrentSpeech 轮询 | L1 NPC × 每帧 | Rust 主线程 | ~5µs（读 Option） |
| AudioStreamGenerator 播放 | 持续 | Godot 渲染线程 | GPU——不计入 Rust 预算 |
| 文字同步 | 每帧 per 活跃说话者 | Godot 主线程 | <0.01ms |
| Bark 播放 | 高频（市场场景） | Godot AudioServer | 极低——预生成样本 |

**Rust 侧增量**: <0.1ms（主线程。合成在后台线程——不计入 7.0ms 帧预算）
**Godot 侧增量**: 零（AudioStreamPlayer 已计入已有音频预算）
**RAM 增量**: Piper 模型 4×50MB = 200MB（可降至 2 模型 + DSP 派生 = 100MB）。音频缓冲池 ~20MB

---

## 十四、TUNING

```toml
[voice]
max_concurrent_synthesis = 3           # [TUNING] 同时活跃的 Piper 合成数
max_concurrent_playback = 8            # [TUNING] Godot 同时播放的语音流数
model_pool_instances = 2               # [TUNING] 每个 Piper 模型的实例数
max_audio_pool_size_mb = 20            # [TUNING] 已合成音频缓存池上限
bark_sample_count = 160                # [TUNING] 预生成基元音样本总数（≈3MB）

[voice.profile]
base_speed_ms = 100                    # [TUNING] 基线字间隔（~150词/分钟）
clause_pause_ms_base = 600             # [TUNING] 基线分句停顿时长
hesitation_rate_base = 0.15            # [TUNING] 基线犹豫频率
interruption_tolerance_base = 0.8      # [TUNING] 基线打断容忍 (s)
individual_variance = 0.12             # [TUNING] 个体差异范围 (±12%)

[voice.emotion]
anger_speed_boost = 15.0               # [TUNING] 愤怒加速 (ms/字减少量)
sadness_slowdown = 30.0                # [TUNING] 悲伤减速 (ms/字增加量)
fear_tremor = 0.6                      # [TUNING] 恐惧震音强度
```

---

> **关联**: [[../001-Godot4.7功能评估|001 功能评估]] · [[../002-画面渲染管线v4.7修订|002 渲染管线]] · [[../../Happy Game/开发阶段/音频系统/005-语音管道|音频 005]] · [[../../Happy Game/开发阶段/语言表达/012-语音输出接口|语言表达 012]]
