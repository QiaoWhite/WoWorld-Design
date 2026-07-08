//! 种子→名字生成器（临时方案）
//!
//! 当前使用音节拼接的确定性名字生成。未来迁移至 CultureCoreParams 驱动的
//! 文化敏感名字系统。
//!
//! 参见: `开发阶段/模型动作与物理系统/007-调试可视化与EntityRenderer架构.md` §六

/// NPC 姓名
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcName {
    pub given: String,
    pub family: String,
}

impl NpcName {
    pub fn full(&self) -> String {
        format!("{} {}", self.given, self.family)
    }
}

/// 种子→名字生成（中文风格双音节名 + 单字姓）
///
/// 确定性：同一种子永远返回同一名字。
pub fn generate_name(seed: u64) -> NpcName {
    // splitmix64 派生两个独立子种子
    let s1 = splitmix(seed);
    let s2 = splitmix(s1);

    let given_syllables = [
        "明", "志", "勇", "慧", "文", "武", "安", "康", "瑞", "祥", "俊", "杰", "英", "华", "思",
        "远", "清", "正", "仁", "义", "云", "风", "山", "海", "雨", "雪", "霜", "露", "春", "秋",
        "光", "辉", "德", "信", "诚", "善", "美", "和", "平", "宁",
    ];
    let family_names = [
        "王", "李", "张", "刘", "陈", "杨", "赵", "黄", "周", "吴", "徐", "孙", "马", "胡", "朱",
        "郭", "何", "林", "罗", "高", "梁", "郑", "谢", "宋", "唐", "韩", "冯", "于", "董", "萧",
    ];

    let g1 = given_syllables[(s1 as usize) % given_syllables.len()];
    let g2 = given_syllables[((s1 >> 8) as usize) % given_syllables.len()];
    let fam = family_names[(s2 as usize) % family_names.len()];

    NpcName {
        given: format!("{}{}", g1, g2),
        family: fam.to_string(),
    }
}

/// splitmix64——快速确定性哈希，把单个 seed 裂变成两个独立子种子
fn splitmix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_name_deterministic() {
        let a = generate_name(42);
        let b = generate_name(42);
        assert_eq!(a, b);
    }

    #[test]
    fn test_name_different_seeds_different() {
        let a = generate_name(1);
        let b = generate_name(2);
        // 极大概率不同（碰撞概率 ~1/50²）
        assert_ne!(a.full(), b.full());
    }

    #[test]
    fn test_name_non_empty() {
        for seed in 0..20 {
            let name = generate_name(seed);
            assert!(!name.given.is_empty(), "seed {seed}: given name empty");
            assert!(!name.family.is_empty(), "seed {seed}: family name empty");
            assert!(
                name.full().len() >= 3,
                "seed {seed}: full name too short: {}",
                name.full()
            );
        }
    }

    #[test]
    fn test_full_name_format() {
        let name = NpcName {
            given: "明志".into(),
            family: "王".into(),
        };
        assert_eq!(name.full(), "明志 王");
    }

    #[test]
    fn test_name_is_valid_utf8() {
        let name = generate_name(999);
        // 确认不含替换字符
        assert!(!name.given.contains('\u{FFFD}'));
        assert!(!name.family.contains('\u{FFFD}'));
    }
}
