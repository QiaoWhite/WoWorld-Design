# 006 — Piper 离线 TTS 合成管道

> **模块**: 音频系统
> **版本**: v1.0
> **创建日期**: 2026-06-21
> **依赖**: [[005-语音管道|005 语音管道]] · [[../语言表达/012-语音输出接口|语言表达 012]] · [[../NPC活人感模块/NPC活人感开发文档ver2.0|NPC 活人感]]
> **参考**: [[../../../../参考文档/038-Godot4.7技术栈升级评估-20260621/003-语音系统架构修正-Rust解耦|语音系统架构修正]]

---

## 〇、架构修正摘要

| 修正 | 旧（005/012 已有） | 新（本文档） |
|------|------------------|------------|
| **TTS 位置** | Godot 侧渲染层 | **Rust 侧——Piper ONNX + fundsp DSP** |
| **TtsEngine trait** | `fn speak() → Result<(), TtsError>` | `fn synthesize() → Result<SynthesizedUtterance, TtsError>` |
| **VoiceManager** | 双定义（012+005） | 合并至音频模块——本文档 |
| **发言节奏** | 仅 VoiceProfile（声学） | VoiceProfile（声学） + SpeakingProfile（节奏） |

---

## 一、合成管道——5 步

```
══════ 音频模块 · synthesize() 内部 ══════

输入: VoicePacket { text(标点版本), voice_profile_id, emotion_mod, delivery, priority }

Step 1 — 音素化 (espeak-ng · <1ms)
  text → phoneme_sequence: Vec<(Phoneme, Duration)>
  标点自动映射为静默音素: "……" → Silence(~500ms), "." → Silence(~350ms)

Step 2 — 文字时间轴（从 phoneme_sequence 直接算 · <0.1ms）
  → 不等 Piper 完成。与 Step 3 并行。
  → text_timeline: Vec<(char_index, time_ms)>
  → total_duration_ms

Step 3 — Piper 整句生成（ONNX 后台线程 · 800-1500ms）
  phoneme_sequence → Piper ONNX → raw_audio: Vec<f32>
  整句——不分段。自然韵律完整。CancellationToken 支持取消（打断时）。

Step 4 — DSP 后处理（fundsp · <2ms）
  pitch_shift(emotion_mod.pitch_shift_semitones)
  → formant_shift(voice_profile.formant_shift_ratio)
  → saturation(voice_profile.roughness + emotion_mod.roughness_add)
  → tremolo(emotion_mod.tremor)
  → eq_high_boost(voice_profile.breathiness + emotion_mod.breathiness_add)
  → processed_audio: Vec<f32>

Step 5 — 返回
  SynthesizedUtterance { audio_buffer, text_timeline, total_duration_ms, voice_profile_id }
```

### 核心设计：标点即编码

静默时长的控制不经过额外的数据通道。标点本身就是编码：

```
SpeakingProfile（语言表达）            Piper（音频）
  clause_pause=200ms → ","             看到 "," → ~120ms 停顿 ✓
  clause_pause=1800ms → "……\n\n"      看到 "……\n\n" → ~1200ms 停顿 ✓
```

标点映射在 VoicePacket 构造前完成。不额外传停顿参数。±200ms 不可感知。**音频模块不知道 SpeakingProfile——标点就是跨模块的协议。**

---

## 二、VoiceManager —— 合成队列管理

```rust
pub struct VoiceManager {
    engine: Box<dyn TtsEngine>,
    queue: VecDeque<VoicePacket>,
    current: Option<VoicePacket>,
    model_pool: ModelPool,       // Piper ONNX 实例池
    audio_pool: AudioPool,       // 已合成音频缓存
    formulaic_cache: FormulaicCache,  // 高频套话预生成缓存
    config: TtsConfig,
}

impl VoiceManager {
    pub fn enqueue(&mut self, packet: VoicePacket) {
        match packet.priority {
            VoicePriority::Critical => {
                self.engine.cancel();
                self.current = None;
                self.queue.push_front(packet);
            }
            VoicePriority::High | VoicePriority::Normal =>
                self.queue.push_back(packet),
            VoicePriority::Ambient | VoicePriority::Background => {
                if self.queue.len() < 2 { self.queue.push_back(packet); }
                // 否则丢弃——远处闲聊不值得消耗合成资源
            }
        }
        self.try_synthesize_next();
    }

    fn try_synthesize_next(&mut self) {
        if self.current.is_some() { return; }
        if let Some(packet) = self.queue.pop_front() {
            // 高频套话缓存检查
            if let Some(cached_raw) = self.formulaic_cache.get(&packet) {
                let processed = apply_dsp(cached_raw, &packet);  // 个体 DSP
                self.audio_pool.store(processed);
                // 写入 NpcData.audio_pool_id
                return;
            }

            // 正常 Piper 管道
            self.current = Some(packet.clone());
            let engine = &self.engine;
            let cancel = CancellationToken::new();
            rayon::spawn(move || {
                let result = engine.synthesize(&packet, &cancel);
                // 合成完成 → channel 回主线程 → audio_pool
            });
        }
    }

    pub fn interrupt(&mut self, listener: EntityId, intent: InterruptIntent) {
        if self.current.is_some() {
            self.engine.cancel();
            self.current = None;
        }
        self.try_synthesize_next();
    }
}
```

### ModelPool

```rust
struct ModelPool {
    instances: HashMap<VoiceProfileId, Vec<PiperSession>>,
    max_per_model: usize,  // 默认 2——同模型避免竞争
}
// 4模型 × 2实例 = 8槽位。同时活跃合成 ≤ 3。绝对安全。
// 内存: 4×50MB = 200MB。RAM 32GB → 0.6%。
```

### AudioPool

已合成音频的 LRU 缓冲池。最大 20MB。最旧未播放缓冲优先回收。

---

## 三、套话预生成缓存

游戏启动 loading 阶段（后台 rayon 线程）预生成 20 个高频短语 × 4 个音色：

```
"Mm.", "Ah.", "Oh.", "早。", "谢谢。", "好。", "再见。", "嗯。",
"嗯？", "请。", "抱歉。", "哈！", "啧。", "好的。", "来了。",
"欢迎。", "慢走。", "没错。", "不对。", "请问。"
```

加载: ~3 秒后台。缓存: ~1MB。运行时命中（>90% 首句对话）→ 0ms 延迟。

缓存命中 → 取原始音频 → 经 VoiceProfile DSP 个体化 → 播放。和实时生成同等质量。

---

## 四、GDExtension 传输

```
Rust → Godot（拉取模式——不推送）:

fn get_audio_buffer(pool_id: u32) → PackedFloat32Array
fn get_text_timeline(npc_id: u64) → PackedInt32Array  // [char_i, time_ms, ...]
fn release_audio_buffer(pool_id: u32)

Godot: 检测到 audio_pool_id 变化 → 拉取音频缓冲 → AudioStreamGenerator → play()
每帧: pos = AudioStreamPlayer.get_playback_position() → 文字同步
播放完毕: release_audio_buffer(pool_id) → Rust 回收槽位
```

---

## 五、与已有 VoiceProfile 和 SpeakingProfile 的关系

| Struct | Owner | 内容 | VoiceProfile 对比 |
|--------|-------|------|-----------------|
| **VoiceProfile** | woworld_audio | 声学——音高/音色/气声/粗糙度 | — |
| **SpeakingProfile** | woworld_language | 节奏——语速/停顿/犹豫/咬字/换气 | 无重叠。base_speed_wpm ↔ base_speed_ms 可互相转换 |

两者从同一 NPC 数据派生（BigFive + 种族 + 性别 + 年龄 + 情绪 + 文化）——但管完全不同的维度。SpeakingProfile 的 `interruption_recovery_seconds` 管被打断后的沉默时长。VoiceProfile 管声音本身的音色。不冲突。

---

## 六、无音频模式

TtsConfig.enabled = false 或系统静音时:
- 文字逐字出现由 SpeakingProfile.base_speed_ms 驱动——和音频模式相同的逐字节奏
- 打断仍然生效——文字在当前 char_index 截断
- 所有其他机制不变

---

## 七、性能

| 操作 | 频率 | 位置 | 成本 |
|------|------|------|------|
| espeak-ng 音素化 | 每次合成 | Rust 后台线程 | <1ms |
| Piper ONNX 推理 | 每次合成 | Rust 后台线程 | 800-1500ms（异步） |
| fundsp DSP | 每次合成 | Rust 后台线程 | <2ms |
| VoiceManager.enqueue | 每次发言 | Rust 主线程 | <1µs |
| SynthesizedUtterance 拉取 | 合成完成时 | GDExtension | <0.1ms |
| 文字同步 | 每帧 per 活跃说话者 | Godot | <0.01ms |

Rust 主线程增量: <0.1ms（合成在后台——不计入 7.0ms 预算）

---

> **关联**: [[005-语音管道|005 语音管道]] · [[007-Bark语声系统|007 Bark 系统]] · [[../语言表达/012-语音输出接口|012 语音输出]]
