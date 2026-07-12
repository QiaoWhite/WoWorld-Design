//! 对话气泡类型（NPC 头顶浮现的文字气泡）
//!
//! 纯数据（引擎无关，无依赖）——由 woworld_ecs 的 speech_bubble_system 产出，
//! 经 EntityVisual 数据合同传给 woworld_godot 的 EntityRenderer 渲染。
//!
//! ⚠️ 命名说明：本类型是 **UI 文字气泡**（眼睛看的文字），与
//! `音频系统/007-Bark语声系统.md` 的 `BarkType`（12 类**语声**，耳朵听的非语言声音）
//! 是两个正交概念。为避免与音频系统权威的 `BarkType` 命名冲突，UI 气泡侧统一用
//! `speech_bubble` 词根 + `BubbleType`。
//!
//! ⚠️ 层次说明：`BubbleType` 是**渲染颜色分类**（对应 `UI与UX系统/002` §"Bark 气泡"
//! 的 5 类内容颜色表），不是数据合同层的语义类型。UI 气泡的正式数据合同是
//! `UI与UX系统/005` 的 `BarkEvent { speaker_id, text, emotion, priority }`，其分类字段
//! 是 `emotion`。本枚举是 MVP 桩化阶段的简化——未来会被 `emotion` 字段驱动取代。
//!
//! ⚠️ 实现偏离：设计文档 `UI与UX系统/005` 规定用 GDExtension `signal bark_emitted`
//! push 到 GDScript。本项目改用现有 pull 管线（EntityVisual → EntityRenderer::sync），
//! 逻辑全留在 Rust 侧，符合宪法 §2（Godot 侧无游戏逻辑）。详见 CHG 文档。

/// 对话气泡内容类型——决定气泡文字颜色。
///
/// 颜色编码取自 `开发阶段/UI与UX系统/002-HUD与常驻界面.md` §"Bark 气泡"内容类型表：
/// 普通=白 / 任务=金 / 情绪=黄 / 伤害=红 / 环境=蓝灰。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BubbleType {
    /// 普通对话（问候/寒暄）——白字
    Normal,
    /// 情绪表达（心情波动）——黄字
    Emotion,
    /// 环境评论（自言自语/日常需求）——蓝灰字
    Ambient,
    /// 任务相关——金字
    Quest,
    /// 伤害反应（受击本能喊叫，"啊！"）——红字
    Damage,
}

impl BubbleType {
    /// 气泡文字 RGB 颜色（各通道 0.0-1.0）
    pub fn color(&self) -> [f32; 3] {
        match self {
            BubbleType::Normal => [1.0, 1.0, 1.0],  // 白
            BubbleType::Emotion => [1.0, 0.9, 0.2], // 黄
            BubbleType::Ambient => [0.6, 0.7, 0.8], // 蓝灰
            BubbleType::Quest => [1.0, 0.82, 0.2],  // 金
            BubbleType::Damage => [1.0, 0.25, 0.2], // 红
        }
    }

    /// 从 TOML 字符串键解析（数据驱动·ecs 侧 SpeechFragmentRegistry 用）。
    /// 未知键回退 `Normal`（宽容解析——不 panic）。
    pub fn from_key(key: &str) -> BubbleType {
        match key {
            "emotion" => BubbleType::Emotion,
            "ambient" => BubbleType::Ambient,
            "quest" => BubbleType::Quest,
            "damage" => BubbleType::Damage,
            _ => BubbleType::Normal,
        }
    }
}

/// 语言行为语义类别——问候/告别/需求嘟囔/情绪宣泄。
///
/// 与渲染色 `BubbleType` **正交**：`SpeechAct` = 说的是哪类话（语义），
/// `BubbleType` = 气泡什么颜色（表现）。二者分离，类型系统降耦。
///
/// 对齐设计 `语言表达/006` PhaticLayer + `005` DialogueIntentType 的
/// **Phatic + 自言自语子集**（本冲刺不实现 DialogueIntent 的 GOAP 主动评估腿）：
/// - `Greeting` ↔ 006 `Reactive::SomeoneEntered` / `Opening`
/// - `Farewell` ↔ 006 `Reactive::SomeoneLeft` / `Closing`
/// - `NeedMutter` ↔ 005 `GoalDriven` 自言自语侧 + 006 `Idle`
/// - `EmotionVent` ↔ 005 `Emotional` + 006 `Idle`
///
/// ⚠️ 非 `DialogueIntentType` 的平行竞品——是其 MVP 投影，未来两体系就位后驱动同一批 `SpeechAct`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpeechAct {
    /// 问候——遭遇他者时的应酬开场
    Greeting,
    /// 告别——离场收尾
    Farewell,
    /// 需求嘟囔——自言自语表达生理/目标需求
    NeedMutter,
    /// 情绪宣泄——自言自语表达情绪极值
    EmotionVent,
}

impl SpeechAct {
    /// 该语言行为在无片段显式指定颜色时的默认气泡渲染色。
    pub fn default_bubble_type(&self) -> BubbleType {
        match self {
            SpeechAct::Greeting | SpeechAct::Farewell => BubbleType::Normal,
            SpeechAct::NeedMutter => BubbleType::Ambient,
            SpeechAct::EmotionVent => BubbleType::Emotion,
        }
    }

    /// 从 TOML 字符串键解析。未知键返回 `None`（片段表加载时应报错跳过）。
    pub fn from_key(key: &str) -> Option<SpeechAct> {
        match key {
            "greeting" => Some(SpeechAct::Greeting),
            "farewell" => Some(SpeechAct::Farewell),
            "need_mutter" => Some(SpeechAct::NeedMutter),
            "emotion_vent" => Some(SpeechAct::EmotionVent),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normal_color_white() {
        assert_eq!(BubbleType::Normal.color(), [1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_emotion_color_yellow() {
        let c = BubbleType::Emotion.color();
        assert!(
            c[0] > 0.8 && c[1] > 0.8 && c[2] < 0.5,
            "yellow: high R+G, low B"
        );
    }

    #[test]
    fn test_ambient_color_bluegray() {
        let c = BubbleType::Ambient.color();
        assert!(c[2] >= c[0], "bluegray: B channel not below R");
    }

    #[test]
    fn test_quest_color_gold() {
        let c = BubbleType::Quest.color();
        assert!(
            c[0] > 0.8 && c[1] > 0.7 && c[2] < 0.5,
            "gold: high R, mid-high G, low B"
        );
    }

    #[test]
    fn test_damage_color_red() {
        let c = BubbleType::Damage.color();
        assert!(
            c[0] > 0.8 && c[1] < 0.5 && c[2] < 0.5,
            "red: high R, low G+B"
        );
    }

    #[test]
    fn test_color_channels_in_range() {
        for bt in [
            BubbleType::Normal,
            BubbleType::Emotion,
            BubbleType::Ambient,
            BubbleType::Quest,
            BubbleType::Damage,
        ] {
            for ch in bt.color() {
                assert!((0.0..=1.0).contains(&ch), "channel out of range for {bt:?}");
            }
        }
    }

    // ── SpeechAct ──────────────────────────

    #[test]
    fn test_speech_act_from_key_roundtrip() {
        assert_eq!(SpeechAct::from_key("greeting"), Some(SpeechAct::Greeting));
        assert_eq!(SpeechAct::from_key("farewell"), Some(SpeechAct::Farewell));
        assert_eq!(
            SpeechAct::from_key("need_mutter"),
            Some(SpeechAct::NeedMutter)
        );
        assert_eq!(
            SpeechAct::from_key("emotion_vent"),
            Some(SpeechAct::EmotionVent)
        );
        assert_eq!(SpeechAct::from_key("nonsense"), None);
    }

    #[test]
    fn test_speech_act_default_bubble_type() {
        // 语义 ⊥ 渲染色：Greeting/Farewell=白, NeedMutter=蓝灰, EmotionVent=黄
        assert_eq!(
            SpeechAct::Greeting.default_bubble_type(),
            BubbleType::Normal
        );
        assert_eq!(
            SpeechAct::Farewell.default_bubble_type(),
            BubbleType::Normal
        );
        assert_eq!(
            SpeechAct::NeedMutter.default_bubble_type(),
            BubbleType::Ambient
        );
        assert_eq!(
            SpeechAct::EmotionVent.default_bubble_type(),
            BubbleType::Emotion
        );
    }

    #[test]
    fn test_bubble_type_from_key_fallback_normal() {
        assert_eq!(BubbleType::from_key("ambient"), BubbleType::Ambient);
        assert_eq!(BubbleType::from_key("emotion"), BubbleType::Emotion);
        assert_eq!(BubbleType::from_key("garbage"), BubbleType::Normal);
    }
}
