//! 稀疏网格实体索引 — EntityIndex trait 实现
//!
//! 将世界划分为 32m×32m×32m 的 Chunk，每个 Chunk 存储实体 ID 列表。
//! 实体详细信息存储在独立的 HashMap 中。

use std::collections::{HashMap, HashSet};
use woworld_core::prelude::*;
use woworld_core::spatial::EntityIndex;

/// Chunk 尺寸（米）
const CHUNK_SIZE: f64 = 32.0;

/// 从世界坐标计算 ChunkKey
fn chunk_key(pos: WorldPos) -> ChunkKey {
    ChunkKey {
        x: (pos.x / CHUNK_SIZE).floor() as i64,
        y: (pos.y / CHUNK_SIZE).floor() as i64,
        z: (pos.z / CHUNK_SIZE).floor() as i64,
    }
}

/// 3D 整数 Chunk 坐标
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct ChunkKey {
    x: i64,
    y: i64,
    z: i64,
}

/// 稀疏网格实体索引
#[derive(Clone, Debug, Default)]
pub struct GridEntityIndex {
    /// 每个 Chunk 中的实体 ID 列表
    chunks: HashMap<ChunkKey, Vec<EntityId>>,
    /// 实体详细数据
    entities: HashMap<EntityId, SpatialEntity>,
}

impl GridEntityIndex {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
            entities: HashMap::new(),
        }
    }

    /// 实体数量
    pub fn len(&self) -> usize {
        self.entities.len()
    }

    /// 是否为空
    pub fn is_empty(&self) -> bool {
        self.entities.is_empty()
    }

    /// 计算 AABB 覆盖的 Chunk 集合
    fn chunks_in_aabb(aabb: &Aabb) -> Vec<ChunkKey> {
        let min_ck = chunk_key(aabb.min);
        let max_ck = chunk_key(aabb.max);
        let mut keys = Vec::new();
        for x in min_ck.x..=max_ck.x {
            for y in min_ck.y..=max_ck.y {
                for z in min_ck.z..=max_ck.z {
                    keys.push(ChunkKey { x, y, z });
                }
            }
        }
        keys
    }

    /// 检查 AABB 是否包含点
    fn aabb_contains(aabb: &Aabb, point: WorldPos) -> bool {
        point.x >= aabb.min.x
            && point.x <= aabb.max.x
            && point.y >= aabb.min.y
            && point.y <= aabb.max.y
            && point.z >= aabb.min.z
            && point.z <= aabb.max.z
    }
}

impl EntityIndex for GridEntityIndex {
    fn register(&mut self, entity: SpatialEntity) {
        let ck = chunk_key(entity.pos);
        self.chunks.entry(ck).or_default().push(entity.id);
        self.entities.insert(entity.id, entity);
    }

    fn unregister(&mut self, entity_id: EntityId) {
        if let Some(entity) = self.entities.remove(&entity_id) {
            let ck = chunk_key(entity.pos);
            if let Some(list) = self.chunks.get_mut(&ck) {
                list.retain(|id| *id != entity_id);
            }
        }
    }

    fn update_transform(&mut self, entity_id: EntityId, pos: WorldPos, rot: Quat, velocity: Vec3) {
        if let Some(entity) = self.entities.get_mut(&entity_id) {
            let old_ck = chunk_key(entity.pos);
            let new_ck = chunk_key(pos);

            // 更新实体数据
            entity.pos = pos;
            entity.rot = rot;
            entity.velocity = velocity;
            // AABB 跟随位置移动（1m³ 包围盒）
            entity.aabb.min = WorldPos {
                x: pos.x - 0.5,
                y: pos.y - 0.5,
                z: pos.z - 0.5,
            };
            entity.aabb.max = WorldPos {
                x: pos.x + 0.5,
                y: pos.y + 0.5,
                z: pos.z + 0.5,
            };

            // 跨 Chunk 迁移
            if old_ck != new_ck {
                if let Some(old_list) = self.chunks.get_mut(&old_ck) {
                    old_list.retain(|id| *id != entity_id);
                }
                self.chunks.entry(new_ck).or_default().push(entity_id);
            }
        }
    }

    fn entities_in_aabb(&self, aabb: &Aabb, layer_mask: u32) -> Vec<SpatialEntity> {
        let mut result = Vec::new();
        let mut seen = HashSet::new();

        for ck in Self::chunks_in_aabb(aabb) {
            if let Some(ids) = self.chunks.get(&ck) {
                for id in ids {
                    if seen.insert(*id) {
                        if let Some(entity) = self.entities.get(id) {
                            if (entity.layer_mask & layer_mask) != 0
                                && Self::aabb_contains(aabb, entity.pos)
                            {
                                result.push(entity.clone());
                            }
                        }
                    }
                }
            }
        }
        result
    }

    fn entity_aabb(&self, entity_id: EntityId) -> Option<Aabb> {
        self.entities.get(&entity_id).map(|e| e.aabb)
    }

    fn acoustic_tag_at(&self, pos: WorldPos) -> AcousticTag {
        // 查找该位置最近的实体，取其材质声学标签
        // 若无实体，返回默认（Grass = 0）
        let ck = chunk_key(pos);
        if let Some(ids) = self.chunks.get(&ck) {
            for id in ids {
                if let Some(entity) = self.entities.get(id) {
                    let dx = entity.pos.x - pos.x;
                    let dy = entity.pos.y - pos.y;
                    let dz = entity.pos.z - pos.z;
                    let dist_sq = (dx * dx + dy * dy + dz * dz) as f32;
                    if dist_sq < 4.0 {
                        // 2m 范围内
                        return AcousticTag(0); // 默认 Grass
                    }
                }
            }
        }
        AcousticTag(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entity(id: u64, x: f64, y: f64, z: f64) -> SpatialEntity {
        let pos = WorldPos { x, y, z };
        SpatialEntity {
            id: EntityId(id),
            pos,
            rot: Quat::IDENTITY,
            velocity: Vec3::ZERO,
            aabb: Aabb {
                min: WorldPos {
                    x: x - 0.5,
                    y: y - 0.5,
                    z: z - 0.5,
                },
                max: WorldPos {
                    x: x + 0.5,
                    y: y + 0.5,
                    z: z + 0.5,
                },
            },
            entity_kind: EntityKind::Creature,
            layer_mask: 0x01,
        }
    }

    #[test]
    fn test_register_and_query() {
        let mut idx = GridEntityIndex::new();
        idx.register(make_entity(1, 10.0, 0.0, 10.0));
        idx.register(make_entity(2, 15.0, 0.0, 15.0));
        idx.register(make_entity(3, 50.0, 0.0, 50.0)); // 不同 chunk

        let aabb = Aabb {
            min: WorldPos {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
            max: WorldPos {
                x: 20.0,
                y: 1.0,
                z: 20.0,
            },
        };
        let found = idx.entities_in_aabb(&aabb, 0xFF);
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn test_unregister() {
        let mut idx = GridEntityIndex::new();
        idx.register(make_entity(1, 10.0, 0.0, 10.0));
        assert_eq!(idx.len(), 1);

        idx.unregister(EntityId(1));
        assert_eq!(idx.len(), 0);

        let aabb = Aabb {
            min: WorldPos {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
            max: WorldPos {
                x: 20.0,
                y: 1.0,
                z: 20.0,
            },
        };
        let found = idx.entities_in_aabb(&aabb, 0xFF);
        assert!(found.is_empty());
    }

    #[test]
    fn test_update_transform_cross_chunk() {
        let mut idx = GridEntityIndex::new();
        idx.register(make_entity(1, 10.0, 0.0, 10.0));

        // 移动到 100m 外（不同 chunk）
        idx.update_transform(
            EntityId(1),
            WorldPos {
                x: 100.0,
                y: 0.0,
                z: 100.0,
            },
            Quat::IDENTITY,
            Vec3::ZERO,
        );

        // 旧位置查不到
        let old_aabb = Aabb {
            min: WorldPos {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
            max: WorldPos {
                x: 20.0,
                y: 1.0,
                z: 20.0,
            },
        };
        assert!(idx.entities_in_aabb(&old_aabb, 0xFF).is_empty());

        // 新位置可查到
        let new_aabb = Aabb {
            min: WorldPos {
                x: 95.0,
                y: -1.0,
                z: 95.0,
            },
            max: WorldPos {
                x: 105.0,
                y: 1.0,
                z: 105.0,
            },
        };
        assert_eq!(idx.entities_in_aabb(&new_aabb, 0xFF).len(), 1);
    }

    #[test]
    fn test_layer_mask_filter() {
        let mut idx = GridEntityIndex::new();
        let mut e1 = make_entity(1, 10.0, 0.0, 10.0);
        e1.layer_mask = 0x01;
        let mut e2 = make_entity(2, 12.0, 0.0, 12.0);
        e2.layer_mask = 0x02;
        idx.register(e1);
        idx.register(e2);

        let aabb = Aabb {
            min: WorldPos {
                x: 0.0,
                y: -1.0,
                z: 0.0,
            },
            max: WorldPos {
                x: 20.0,
                y: 1.0,
                z: 20.0,
            },
        };

        assert_eq!(idx.entities_in_aabb(&aabb, 0x01).len(), 1);
        assert_eq!(idx.entities_in_aabb(&aabb, 0x02).len(), 1);
        assert_eq!(idx.entities_in_aabb(&aabb, 0xFF).len(), 2);
        assert_eq!(idx.entities_in_aabb(&aabb, 0x04).len(), 0);
    }
}
