//! ActionRegistry — 动作定义的 HashMap 存储
//!
//! TOML 数据驱动: `action_registry.toml` → ActionDef 列表。
//! ActionController 通过 `get(id)` 查询动作定义。
//!
//! 参见: `WoWorld-Design/.../角色控制器/003-ActionController与离散动作.md` §三

use std::collections::HashMap;
use woworld_core::action::{ActionDef, ActionId};

/// 动作注册表——TOML 加载的动作定义集合。
#[derive(Debug, Default)]
pub struct ActionRegistry {
    /// ActionId → ActionDef 主存储
    definitions: HashMap<ActionId, ActionDef>,
    /// ActionId 列表（保持插入顺序）
    ids: Vec<ActionId>,
    /// ★ cancel_set 的 TOML key 预解析为 ActionId（can_interrupt 按 id 比对）
    cancel_id_map: HashMap<ActionId, Vec<ActionId>>,
}

impl ActionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册一个动作定义。
    ///
    /// 使用 ActionDef 的 action_id (目前为 name 的 hash) 作为 key。
    /// 若 id 已存在，覆盖旧定义。
    pub fn register(&mut self, id: ActionId, def: ActionDef) {
        if !self.definitions.contains_key(&id) {
            self.ids.push(id);
        }
        // ★ 预解析 cancel_set（TOML key）→ ActionId，供 can_interrupt 按 id 比对
        let cancel_ids = def.cancel_set.iter().map(|k| ActionId(fnv_hash(k))).collect();
        self.cancel_id_map.insert(id, cancel_ids);
        self.definitions.insert(id, def);
    }

    /// 查询动作定义。
    pub fn get(&self, id: ActionId) -> Option<&ActionDef> {
        self.definitions.get(&id)
    }

    /// 动作数量。
    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    /// 所有已注册的 ActionId。
    pub fn action_ids(&self) -> &[ActionId] {
        &self.ids
    }

    /// 当前动作可被哪些 ActionId 取消（cancel_set 的预解析结果）。
    ///
    /// 空 slice 表示不可被 cancel_set 主动取消。
    pub fn cancel_set_ids(&self, id: ActionId) -> &[ActionId] {
        self.cancel_id_map
            .get(&id)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    /// 从 TOML 字符串加载动作定义。
    ///
    /// TOML 格式:
    /// ```toml
    /// [action.light_attack]
    /// name = "轻攻击"
    /// ...
    /// ```
    ///
    /// ActionId 从 TOML key (如 "light_attack") 通过 hash 生成。
    /// Sprint 1: 使用简单的 FNV hash。后续可改用编译时 const hash。
    pub fn load_from_toml(&mut self, toml_str: &str) -> Result<(), toml::de::Error> {
        #[derive(serde::Deserialize)]
        struct ActionRegistryToml {
            action: HashMap<String, ActionDef>,
        }

        let parsed: ActionRegistryToml = toml::from_str(toml_str)?;
        for (key, def) in parsed.action {
            let id = ActionId(fnv_hash(&key));
            self.register(id, def);
        }
        Ok(())
    }
}

/// 简单 FNV-1a 32-bit hash（用于 TOML key → ActionId 映射）。
///
/// 不需要外部依赖——几行代码实现。
fn fnv_hash(s: &str) -> u32 {
    let mut hash: u32 = 0x811c9dc5;
    for byte in s.bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;
    use woworld_core::action::{
        ActionKind, CommitmentLevel, MovementLockDef, RotationLockDef,
    };
    use woworld_core::kinematics::PhysicsRequirement;

    fn make_test_def(name: &str) -> ActionDef {
        ActionDef {
            name: name.into(),
            category: "Combat".into(),
            kind: ActionKind::Discrete,
            priority: 15,
            commitment: CommitmentLevel::Hard,
            windup_ms: 120,
            active_ms: 100,
            recovery_ms: 250,
            cancel_set: vec![],
            cancel_window_ms: 120,
            bufferable: false,
            buffer_window_ms: 0,
            physics_req: PhysicsRequirement::Grounded,
            movement_lock: MovementLockDef::Full,
            rotation_lock: RotationLockDef::TargetDirection,
            interrupt_on_move: false,
            sustain_drain: None,
            release_behavior: None,
            overextend_threshold_secs: None,
            critical_threshold_secs: None,
        }
    }

    #[test]
    fn test_action_registry_new_empty() {
        let r = ActionRegistry::new();
        assert!(r.is_empty());
    }

    #[test]
    fn test_action_registry_register_and_get() {
        let mut r = ActionRegistry::new();
        let id = ActionId(1);
        r.register(id, make_test_def("test"));
        assert_eq!(r.len(), 1);
        assert!(r.get(id).is_some());
        assert_eq!(r.get(id).unwrap().name, "test");
    }

    #[test]
    fn test_action_registry_missing_id_returns_none() {
        let r = ActionRegistry::new();
        assert!(r.get(ActionId(999)).is_none());
    }

    #[test]
    fn test_action_registry_ids_list() {
        let mut r = ActionRegistry::new();
        r.register(ActionId(2), make_test_def("a"));
        r.register(ActionId(1), make_test_def("b"));
        assert_eq!(r.action_ids().len(), 2);
    }

    #[test]
    fn test_load_real_action_registry_toml_parses() {
        // ★ 集成前置：验证真实 assets/action_registry.toml 能被解析。
        //   registry 其它测试用内联字符串，此测试覆盖真实文件字段对齐——
        //   若字段不匹配会在 cargo test 阶段暴露，而非 Godot 运行时 panic。
        let mut r = ActionRegistry::new();
        let toml = include_str!("../../../../assets/action_registry.toml");
        r.load_from_toml(toml).expect("action_registry.toml 应能解析");
        assert!(r.len() >= 6, "应至少加载 6 个动作，实得 {}", r.len());
    }

    #[test]
    fn test_cancel_set_ids_resolves_keys() {
        // ★ D2 修复验证：cancel_set 的 TOML key 应解析为对应动作的 ActionId
        let mut r = ActionRegistry::new();
        r.load_from_toml(include_str!("../../../../assets/action_registry.toml"))
            .unwrap();
        let light_id = ActionId(fnv_hash("light_attack"));
        let heavy_id = ActionId(fnv_hash("heavy_attack"));
        // light_attack.cancel_set = ["heavy_attack", "special_skill"]
        assert!(
            r.cancel_set_ids(light_id).contains(&heavy_id),
            "light_attack 应可被 heavy_attack 取消"
        );
    }
}
