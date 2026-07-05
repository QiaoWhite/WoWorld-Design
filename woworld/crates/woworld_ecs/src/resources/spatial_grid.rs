//! SpatialGrid — 均匀网格 EntityIndex 实现
//!
//! 世界空间划分为 CELL_SIZE³ 的均匀网格。
//! 每个 cell 存储实体列表，支持 AABB 查询、注册/注销、变换更新。
//!
//! 参见: `woworld_core::spatial::EntityIndex`

use std::collections::HashMap;

use glam::{Quat, Vec3};
use woworld_core::spatial::EntityIndex;
use woworld_core::types::{Aabb, AcousticTag, EntityId, SpatialEntity, WorldPos};

/// 网格 Cell 边长（米）
const CELL_SIZE: f64 = 50.0;

/// Cell 坐标
type CellKey = (i32, i32, i32);

#[inline]
fn pos_to_cell(pos: &WorldPos) -> CellKey {
    (
        (pos.x / CELL_SIZE).floor() as i32,
        (pos.y / CELL_SIZE).floor() as i32,
        (pos.z / CELL_SIZE).floor() as i32,
    )
}

fn aabb_to_cells(aabb: &Aabb) -> impl Iterator<Item = CellKey> {
    let min_cell = pos_to_cell(&aabb.min);
    let max_cell = pos_to_cell(&aabb.max);
    let cells: Vec<CellKey> = (min_cell.0..=max_cell.0)
        .flat_map(move |cx| {
            (min_cell.1..=max_cell.1).flat_map(move |cy| {
                (min_cell.2..=max_cell.2).map(move |cz| (cx, cy, cz))
            })
        })
        .collect();
    cells.into_iter()
}

// ── SpatialGrid ───────────────────────

pub struct SpatialGrid {
    cells: HashMap<CellKey, Vec<SpatialEntity>>,
    entity_cell: HashMap<EntityId, CellKey>,
    acoustic_tags: HashMap<CellKey, AcousticTag>,
}

impl SpatialGrid {
    pub fn new() -> Self {
        Self {
            cells: HashMap::new(),
            entity_cell: HashMap::new(),
            acoustic_tags: HashMap::new(),
        }
    }

    pub fn entity_count(&self) -> usize {
        self.entity_cell.len()
    }
}

impl Default for SpatialGrid {
    fn default() -> Self {
        Self::new()
    }
}

impl EntityIndex for SpatialGrid {
    fn register(&mut self, entity: SpatialEntity) {
        let cell = pos_to_cell(&entity.pos);
        self.entity_cell.insert(entity.id, cell);
        self.cells.entry(cell).or_default().push(entity);
    }

    fn unregister(&mut self, entity_id: EntityId) {
        if let Some(cell) = self.entity_cell.remove(&entity_id) {
            if let Some(entities) = self.cells.get_mut(&cell) {
                entities.retain(|e| e.id != entity_id);
                if entities.is_empty() {
                    self.cells.remove(&cell);
                }
            }
        }
    }

    fn update_transform(
        &mut self,
        entity_id: EntityId,
        pos: WorldPos,
        rot: Quat,
        velocity: Vec3,
    ) {
        let new_cell = pos_to_cell(&pos);

        // 从旧 cell 移除
        if let Some(old_cell) = self.entity_cell.get(&entity_id).copied() {
            if let Some(entities) = self.cells.get_mut(&old_cell) {
                if let Some(idx) = entities.iter().position(|e| e.id == entity_id) {
                    let mut e = entities.swap_remove(idx);
                    e.pos = pos;
                    e.rot = rot;
                    e.velocity = velocity;
                    let half = WorldPos { x: 1.0, y: 1.0, z: 1.0 };
                    e.aabb = Aabb {
                        min: WorldPos { x: pos.x - half.x, y: pos.y - half.y, z: pos.z - half.z },
                        max: WorldPos { x: pos.x + half.x, y: pos.y + half.y, z: pos.z + half.z },
                    };
                    self.cells.entry(new_cell).or_default().push(e);
                    self.entity_cell.insert(entity_id, new_cell);
                }
            }
        }
    }

    fn entities_in_aabb(&self, aabb: &Aabb, layer_mask: u32) -> Vec<SpatialEntity> {
        let mut result = Vec::new();
        for cell in aabb_to_cells(aabb) {
            if let Some(entities) = self.cells.get(&cell) {
                for e in entities {
                    if (e.layer_mask & layer_mask) != 0 && aabb_overlap(&e.aabb, aabb) {
                        result.push(e.clone());
                    }
                }
            }
        }
        result
    }

    fn entity_aabb(&self, entity_id: EntityId) -> Option<Aabb> {
        let cell = self.entity_cell.get(&entity_id)?;
        let entities = self.cells.get(cell)?;
        entities.iter().find(|e| e.id == entity_id).map(|e| e.aabb)
    }

    fn acoustic_tag_at(&self, pos: WorldPos) -> AcousticTag {
        let cell = pos_to_cell(&pos);
        self.acoustic_tags.get(&cell).copied().unwrap_or(AcousticTag(0))
    }
}

/// AABB 重叠检测
fn aabb_overlap(a: &Aabb, b: &Aabb) -> bool {
    a.min.x <= b.max.x
        && a.max.x >= b.min.x
        && a.min.y <= b.max.y
        && a.max.y >= b.min.y
        && a.min.z <= b.max.z
        && a.max.z >= b.min.z
}

#[cfg(test)]
mod tests {
    use super::*;
    use woworld_core::types::EntityKind;

    fn make_entity(id: u64, x: f64, y: f64, z: f64) -> SpatialEntity {
        let pos = WorldPos { x, y, z };
        let half = WorldPos { x: 1.0, y: 1.0, z: 1.0 };
        SpatialEntity {
            id: EntityId(id),
            pos,
            rot: Quat::IDENTITY,
            velocity: Vec3::ZERO,
            aabb: Aabb {
                min: WorldPos { x: pos.x - half.x, y: pos.y - half.y, z: pos.z - half.z },
                max: WorldPos { x: pos.x + half.x, y: pos.y + half.y, z: pos.z + half.z },
            },
            entity_kind: EntityKind::Creature,
            layer_mask: 1,
        }
    }

    #[test]
    fn test_register_and_query() {
        let mut grid = SpatialGrid::new();
        let e = make_entity(1, 10.0, 0.0, 10.0);
        grid.register(e);

        let aabb = Aabb {
            min: WorldPos { x: 0.0, y: -5.0, z: 0.0 },
            max: WorldPos { x: 20.0, y: 5.0, z: 20.0 },
        };
        let found = grid.entities_in_aabb(&aabb, 1);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].id, EntityId(1));
    }

    #[test]
    fn test_query_outside_range_returns_empty() {
        let mut grid = SpatialGrid::new();
        grid.register(make_entity(1, 10.0, 0.0, 10.0));

        let far_aabb = Aabb {
            min: WorldPos { x: 100.0, y: -5.0, z: 100.0 },
            max: WorldPos { x: 120.0, y: 5.0, z: 120.0 },
        };
        assert!(grid.entities_in_aabb(&far_aabb, 1).is_empty());
    }

    #[test]
    fn test_unregister_removes_entity() {
        let mut grid = SpatialGrid::new();
        grid.register(make_entity(1, 10.0, 0.0, 10.0));
        assert_eq!(grid.entity_count(), 1);

        grid.unregister(EntityId(1));
        assert_eq!(grid.entity_count(), 0);

        let aabb = Aabb {
            min: WorldPos { x: 0.0, y: -50.0, z: 0.0 },
            max: WorldPos { x: 100.0, y: 50.0, z: 100.0 },
        };
        assert!(grid.entities_in_aabb(&aabb, 1).is_empty());
    }

    #[test]
    fn test_update_transform_moves_entity() {
        let mut grid = SpatialGrid::new();
        let e = make_entity(1, 10.0, 0.0, 10.0);
        grid.register(e.clone());

        // 移动到远处
        grid.update_transform(
            EntityId(1),
            WorldPos { x: 500.0, y: 0.0, z: 500.0 },
            Quat::IDENTITY,
            Vec3::ZERO,
        );

        let near = Aabb {
            min: WorldPos { x: 0.0, y: -5.0, z: 0.0 },
            max: WorldPos { x: 20.0, y: 5.0, z: 20.0 },
        };
        assert!(grid.entities_in_aabb(&near, 1).is_empty());

        let far = Aabb {
            min: WorldPos { x: 490.0, y: -5.0, z: 490.0 },
            max: WorldPos { x: 510.0, y: 5.0, z: 510.0 },
        };
        assert_eq!(grid.entities_in_aabb(&far, 1).len(), 1);
    }

    #[test]
    fn test_layer_mask_filtering() {
        let mut grid = SpatialGrid::new();
        let mut e1 = make_entity(1, 10.0, 0.0, 10.0);
        e1.layer_mask = 0b001; // layer 0

        let mut e2 = make_entity(2, 12.0, 0.0, 12.0);
        e2.layer_mask = 0b010; // layer 1

        grid.register(e1);
        grid.register(e2);

        let aabb = Aabb {
            min: WorldPos { x: 0.0, y: -5.0, z: 0.0 },
            max: WorldPos { x: 20.0, y: 5.0, z: 20.0 },
        };

        assert_eq!(grid.entities_in_aabb(&aabb, 0b001).len(), 1);
        assert_eq!(grid.entities_in_aabb(&aabb, 0b010).len(), 1);
        assert_eq!(grid.entities_in_aabb(&aabb, 0b011).len(), 2);
    }

    #[test]
    fn test_empty_grid_does_not_panic() {
        let grid = SpatialGrid::new();
        let aabb = Aabb {
            min: WorldPos { x: 0.0, y: 0.0, z: 0.0 },
            max: WorldPos { x: 10.0, y: 10.0, z: 10.0 },
        };
        assert!(grid.entities_in_aabb(&aabb, 1).is_empty());
        assert!(grid.entity_aabb(EntityId(99)).is_none());
    }

    #[test]
    fn test_entity_aabb_lookup() {
        let mut grid = SpatialGrid::new();
        let e = make_entity(42, 15.0, 2.0, 15.0);
        grid.register(e.clone());

        let found = grid.entity_aabb(EntityId(42));
        assert!(found.is_some());

        // 不存在
        assert!(grid.entity_aabb(EntityId(999)).is_none());
    }
}
