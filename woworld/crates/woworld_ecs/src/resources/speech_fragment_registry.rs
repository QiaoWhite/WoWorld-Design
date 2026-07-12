//! SpeechFragmentRegistry — 对话气泡片段库（TOML 数据驱动）
//!
//! 设计依据: `语言表达/003-文本生成引擎` §一「片段组合，非单体模板」+ §2.1 `FragmentCondition`
//!           `语言表达/006-对话系统-社交层` §1 PhaticLayer
//!
//! 涌现在「选择」：按 `SpeechAct` 过滤 → 满足全部 `FragmentCondition` 的候选 →
//! 概率加权（种子确定性）选一句。同状态不同人格/时段/关系 → 不同候选集 → 不同话。
//! **非**造词——词是预写片段（设计规格）；涌现落在「谁在什么情境说哪句」。
//!
//! ⚠️ `social_effect` 仅**声明式存储**——V4a **不施加**（关系效果归 `social_system`，D4）。

use woworld_core::speech_bubble::{BubbleType, SpeechAct};
use woworld_core::time::TimeOfDay;

/// 片段选择上下文——喂 `FragmentCondition` 过滤的涌现状态快照。
#[derive(Debug, Clone, Copy)]
pub struct SpeechContext<'a> {
    pub time_of_day: TimeOfDay,
    /// formality 代理——来自 `RelationStorage.trust`（生人 None → 0.0 陌生档）
    pub trust: f32,
    /// 情绪 pleasure（-1..1）
    pub pleasure: f32,
    /// 外向性（0..1）——发话频率/热情度
    pub extraversion: f32,
    /// 需求/目标匹配键（`need_mutter` 用；`greeting`/`farewell` 传 None）
    pub topic: Option<&'a str>,
}

/// 解析后的片段——条件为 typed，避免运行时字符串比对。
#[derive(Debug, Clone)]
struct Fragment {
    act: SpeechAct,
    text: String,
    bubble_type: BubbleType,
    topic: Option<String>,
    time_of_day: Option<TimeOfDay>,
    min_trust: Option<f32>,
    max_trust: Option<f32>,
    min_pleasure: Option<f32>,
    max_pleasure: Option<f32>,
    min_extraversion: Option<f32>,
    max_extraversion: Option<f32>,
    /// (trust_delta, familiarity_delta) — 声明式·V4a 不施加
    #[allow(dead_code)]
    social_effect: Option<(f32, f32)>,
}

impl Fragment {
    /// 该片段是否满足上下文全部条件（缺省字段=无约束）。
    fn matches(&self, act: SpeechAct, ctx: &SpeechContext) -> bool {
        if self.act != act {
            return false;
        }
        // topic：片段声明了 topic 则必须与 ctx.topic 相等
        if let Some(ref t) = self.topic {
            if ctx.topic != Some(t.as_str()) {
                return false;
            }
        }
        if let Some(tod) = self.time_of_day {
            if ctx.time_of_day != tod {
                return false;
            }
        }
        if let Some(m) = self.min_trust {
            if ctx.trust < m {
                return false;
            }
        }
        if let Some(m) = self.max_trust {
            if ctx.trust > m {
                return false;
            }
        }
        if let Some(m) = self.min_pleasure {
            if ctx.pleasure < m {
                return false;
            }
        }
        if let Some(m) = self.max_pleasure {
            if ctx.pleasure > m {
                return false;
            }
        }
        if let Some(m) = self.min_extraversion {
            if ctx.extraversion < m {
                return false;
            }
        }
        if let Some(m) = self.max_extraversion {
            if ctx.extraversion > m {
                return false;
            }
        }
        true
    }
}

/// 对话气泡片段库。
#[derive(Debug, Default)]
pub struct SpeechFragmentRegistry {
    fragments: Vec<Fragment>,
}

impl SpeechFragmentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.fragments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.fragments.is_empty()
    }

    /// 从内嵌 TOML 加载（生产路径）。解析失败 panic——片段表是构建期资产。
    pub fn load_embedded() -> Self {
        let mut r = Self::new();
        r.load_from_toml(include_str!("../../../../assets/speech_fragments.toml"))
            .expect("speech_fragments.toml 应能解析");
        r
    }

    /// 从 TOML 字符串加载。
    pub fn load_from_toml(&mut self, toml_str: &str) -> Result<(), toml::de::Error> {
        #[derive(serde::Deserialize)]
        struct SocialEffectToml {
            trust_delta: f32,
            familiarity_delta: f32,
        }
        #[derive(serde::Deserialize)]
        struct FragmentToml {
            act: String,
            text: String,
            color: Option<String>,
            topic: Option<String>,
            time_of_day: Option<String>,
            min_trust: Option<f32>,
            max_trust: Option<f32>,
            min_pleasure: Option<f32>,
            max_pleasure: Option<f32>,
            min_extraversion: Option<f32>,
            max_extraversion: Option<f32>,
            social_effect: Option<SocialEffectToml>,
        }
        #[derive(serde::Deserialize)]
        struct FragmentsToml {
            fragment: Vec<FragmentToml>,
        }

        let parsed: FragmentsToml = toml::from_str(toml_str)?;
        for f in parsed.fragment {
            // 未知 act 键跳过（宽容——不 panic 整个加载）
            let Some(act) = SpeechAct::from_key(&f.act) else {
                continue;
            };
            let bubble_type = f
                .color
                .as_deref()
                .map(BubbleType::from_key)
                .unwrap_or_else(|| act.default_bubble_type());
            let time_of_day = f.time_of_day.as_deref().and_then(parse_time_of_day);
            self.fragments.push(Fragment {
                act,
                text: f.text,
                bubble_type,
                topic: f.topic,
                time_of_day,
                min_trust: f.min_trust,
                max_trust: f.max_trust,
                min_pleasure: f.min_pleasure,
                max_pleasure: f.max_pleasure,
                min_extraversion: f.min_extraversion,
                max_extraversion: f.max_extraversion,
                social_effect: f
                    .social_effect
                    .map(|s| (s.trust_delta, s.familiarity_delta)),
            });
        }
        Ok(())
    }

    /// 选一句——按 `act` + 上下文条件过滤候选，`seed` 确定性加权选一。
    ///
    /// 返回 `(text, bubble_type)`；无匹配候选返回 `None`（如实呈现"无话可说"）。
    /// `seed` 由调用方混入实体/tick/配对信息，保证同情境可复现、不同情境错峰。
    pub fn select(
        &self,
        act: SpeechAct,
        ctx: &SpeechContext,
        seed: u64,
    ) -> Option<(String, BubbleType)> {
        // 收集候选索引（避免克隆）
        let candidates: Vec<usize> = self
            .fragments
            .iter()
            .enumerate()
            .filter(|(_, f)| f.matches(act, ctx))
            .map(|(i, _)| i)
            .collect();
        if candidates.is_empty() {
            return None;
        }
        let pick = candidates[(seed % candidates.len() as u64) as usize];
        let f = &self.fragments[pick];
        Some((f.text.clone(), f.bubble_type))
    }
}

/// TOML 时段键 → `core::time::TimeOfDay`（未知返回 None，条件视为无时段约束丢弃）。
fn parse_time_of_day(key: &str) -> Option<TimeOfDay> {
    match key {
        "dawn" => Some(TimeOfDay::Dawn),
        "day" => Some(TimeOfDay::Day),
        "dusk" => Some(TimeOfDay::Dusk),
        "night" => Some(TimeOfDay::Night),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> SpeechContext<'static> {
        SpeechContext {
            time_of_day: TimeOfDay::Day,
            trust: 0.5,
            pleasure: 0.0,
            extraversion: 0.5,
            topic: None,
        }
    }

    #[test]
    fn test_embedded_loads() {
        let r = SpeechFragmentRegistry::load_embedded();
        assert!(r.len() >= 20, "片段库应加载 ≥20 条，实得 {}", r.len());
    }

    #[test]
    fn test_greeting_selectable() {
        let r = SpeechFragmentRegistry::load_embedded();
        let got = r.select(SpeechAct::Greeting, &ctx(), 0);
        assert!(got.is_some(), "白天应有可选问候");
        assert_eq!(got.unwrap().1, BubbleType::Normal);
    }

    #[test]
    fn test_need_mutter_topic_filter() {
        let r = SpeechFragmentRegistry::load_embedded();
        let mut c = ctx();
        c.topic = Some("hunger");
        let got = r.select(SpeechAct::NeedMutter, &c, 0).unwrap();
        assert_eq!(got.0, "肚子饿了…");
        assert_eq!(got.1, BubbleType::Ambient);
    }

    #[test]
    fn test_topic_mismatch_no_hunger_line() {
        let r = SpeechFragmentRegistry::load_embedded();
        let mut c = ctx();
        c.topic = Some("thirst");
        let got = r.select(SpeechAct::NeedMutter, &c, 0).unwrap();
        assert_eq!(got.0, "口渴…");
    }

    #[test]
    fn test_deterministic_selection() {
        let r = SpeechFragmentRegistry::load_embedded();
        let a = r.select(SpeechAct::Greeting, &ctx(), 42);
        let b = r.select(SpeechAct::Greeting, &ctx(), 42);
        assert_eq!(a, b, "同种子同上下文应可复现");
    }

    #[test]
    fn test_introvert_gets_curt_greeting_option() {
        // 极内向 → "嗯。" 候选可命中（max_extraversion=0.3）
        let r = SpeechFragmentRegistry::load_embedded();
        let mut c = ctx();
        c.extraversion = 0.1;
        c.time_of_day = TimeOfDay::Day;
        // 遍历多个 seed，内向者候选集应含闷哼
        let mut saw_curt = false;
        for seed in 0..50 {
            if let Some((text, _)) = r.select(SpeechAct::Greeting, &c, seed) {
                if text == "嗯。" {
                    saw_curt = true;
                    break;
                }
            }
        }
        assert!(saw_curt, "内向者候选集应含闷哼'嗯。'");
    }

    #[test]
    fn test_extravert_excludes_curt_line() {
        // 极外向 → "嗯。"（max_extraversion=0.3）不在候选
        let r = SpeechFragmentRegistry::load_embedded();
        let mut c = ctx();
        c.extraversion = 0.9;
        for seed in 0..100 {
            if let Some((text, _)) = r.select(SpeechAct::Greeting, &c, seed) {
                assert_ne!(text, "嗯。", "外向者不应闷哼");
            }
        }
    }

    #[test]
    fn test_emotion_vent_requires_pleasure() {
        let r = SpeechFragmentRegistry::load_embedded();
        let mut c = ctx();
        // 中性情绪 → 无宣泄候选
        c.pleasure = 0.0;
        assert!(r.select(SpeechAct::EmotionVent, &c, 0).is_none());
        // 高兴 → 有
        c.pleasure = 0.5;
        assert!(r.select(SpeechAct::EmotionVent, &c, 0).is_some());
    }

    #[test]
    fn test_unknown_act_key_skipped() {
        let mut r = SpeechFragmentRegistry::new();
        r.load_from_toml("[[fragment]]\nact=\"bogus\"\ntext=\"x\"\n")
            .unwrap();
        assert_eq!(r.len(), 0, "未知 act 键应跳过");
    }

    #[test]
    fn test_no_candidate_returns_none() {
        let r = SpeechFragmentRegistry::load_embedded();
        let mut c = ctx();
        c.topic = Some("nonexistent_topic");
        assert!(r.select(SpeechAct::NeedMutter, &c, 0).is_none());
    }
}
