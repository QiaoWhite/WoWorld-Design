//! PowerRegistry — 权力关系 SoA 存储 + PowerQuery 实现
//!
//! 参见: woworld_core::power::PowerQuery

use woworld_core::power::{
    PowerAtom, PowerDomain, PowerEdge, PowerQuery, PowerSource, SuccessionRule,
};
use woworld_core::types::EntityId;

#[derive(Debug, Default)]
pub struct PowerRegistry {
    holders: Vec<EntityId>,
    subjects: Vec<EntityId>,
    atoms: Vec<PowerAtom>,
    sources: Vec<PowerSource>,
    domains: Vec<PowerDomain>,
    legitimacies: Vec<f32>,
    enforcements: Vec<f32>,
    established_ticks: Vec<u64>,
    valid_until_ticks: Vec<Option<u64>>,
    last_exercised_ticks: Vec<u64>,
    successions: Vec<SuccessionRule>,
    actives: Vec<bool>,
}

impl PowerRegistry {
    pub fn new() -> Self { Self::default() }

    /// 创建权力边
    pub fn create_edge(&mut self, edge: PowerEdge) {
        self.holders.push(edge.holder);
        self.subjects.push(edge.subject);
        self.atoms.push(edge.atom);
        self.sources.push(edge.source);
        self.domains.push(edge.domain);
        self.legitimacies.push(edge.legitimacy);
        self.enforcements.push(edge.enforcement);
        self.established_ticks.push(edge.established_tick);
        self.valid_until_ticks.push(edge.valid_until_tick);
        self.last_exercised_ticks.push(edge.last_exercised_tick);
        self.successions.push(edge.succession);
        self.actives.push(edge.active);
    }

    /// 获取权力边（按索引）
    fn edge_at(&self, idx: usize) -> Option<PowerEdge> {
        if idx >= self.holders.len() { return None; }
        Some(PowerEdge {
            holder: self.holders[idx],
            subject: self.subjects[idx],
            atom: self.atoms[idx],
            source: self.sources[idx],
            domain: self.domains[idx],
            legitimacy: self.legitimacies[idx],
            enforcement: self.enforcements[idx],
            established_tick: self.established_ticks[idx],
            valid_until_tick: self.valid_until_ticks[idx],
            last_exercised_tick: self.last_exercised_ticks[idx],
            succession: self.successions[idx],
            active: self.actives[idx],
        })
    }

    pub fn len(&self) -> usize { self.holders.len() }
    pub fn is_empty(&self) -> bool { self.holders.is_empty() }
}

impl PowerQuery for PowerRegistry {
    fn powers_of(&self, holder: EntityId) -> Vec<PowerEdge> {
        self.holders.iter().enumerate()
            .filter(|(i, &h)| h == holder && self.actives[*i])
            .filter_map(|(i, _)| self.edge_at(i))
            .collect()
    }

    fn constraints_on(&self, subject: EntityId) -> Vec<PowerEdge> {
        self.subjects.iter().enumerate()
            .filter(|(i, &s)| s == subject && self.actives[*i])
            .filter_map(|(i, _)| self.edge_at(i))
            .collect()
    }

    fn powers_by_atom(&self, holder: EntityId, atom: PowerAtom) -> Vec<PowerEdge> {
        self.holders.iter().enumerate()
            .filter(|(i, &h)| h == holder && self.atoms[*i] == atom && self.actives[*i])
            .filter_map(|(i, _)| self.edge_at(i))
            .collect()
    }

    fn perceived_legitimacy(&self, subject: EntityId, holder: EntityId) -> f32 {
        let edges: Vec<f32> = self.subjects.iter().enumerate()
            .filter(|(i, &s)| s == subject && self.holders[*i] == holder && self.actives[*i])
            .map(|(i, _)| self.legitimacies[i])
            .collect();
        if edges.is_empty() { return 0.5; }
        edges.iter().sum::<f32>() / edges.len() as f32
    }

    fn edge_count(&self) -> usize { self.holders.len() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_edge(holder: u64, subject: u64, atom: PowerAtom) -> PowerEdge {
        PowerEdge { holder: EntityId(holder), subject: EntityId(subject), atom, ..Default::default() }
    }

    #[test]
    fn test_create_and_query_powers_of() {
        let mut r = PowerRegistry::new();
        r.create_edge(make_edge(1, 2, PowerAtom::Compel));
        r.create_edge(make_edge(1, 3, PowerAtom::Extract));
        r.create_edge(make_edge(2, 3, PowerAtom::Constrain));

        let p1 = r.powers_of(EntityId(1));
        assert_eq!(p1.len(), 2);
        let p2 = r.powers_of(EntityId(2));
        assert_eq!(p2.len(), 1);
    }

    #[test]
    fn test_constraints_on() {
        let mut r = PowerRegistry::new();
        r.create_edge(make_edge(1, 10, PowerAtom::Compel));
        r.create_edge(make_edge(2, 10, PowerAtom::Extract));
        r.create_edge(make_edge(3, 20, PowerAtom::Sanction));

        let c = r.constraints_on(EntityId(10));
        assert_eq!(c.len(), 2);
    }

    #[test]
    fn test_powers_by_atom() {
        let mut r = PowerRegistry::new();
        r.create_edge(make_edge(1, 2, PowerAtom::Extract));
        r.create_edge(make_edge(1, 3, PowerAtom::Extract));
        r.create_edge(make_edge(1, 4, PowerAtom::Compel));

        let e = r.powers_by_atom(EntityId(1), PowerAtom::Extract);
        assert_eq!(e.len(), 2);
    }

    #[test]
    fn test_perceived_legitimacy() {
        let mut r = PowerRegistry::new();
        let mut e = make_edge(1, 2, PowerAtom::Compel);
        e.legitimacy = 0.75;
        r.create_edge(e);

        assert!((r.perceived_legitimacy(EntityId(2), EntityId(1)) - 0.75).abs() < 0.01);
    }

    #[test]
    fn test_empty_queries() {
        let r = PowerRegistry::new();
        assert!(r.powers_of(EntityId(1)).is_empty());
        assert!(r.constraints_on(EntityId(1)).is_empty());
        assert_eq!(r.perceived_legitimacy(EntityId(1), EntityId(2)), 0.5);
        assert_eq!(r.edge_count(), 0);
    }
}
