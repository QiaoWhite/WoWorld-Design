//! 地形修改编排层 — 核心类型
//!
//! 定义稀疏地形修改的双层存储（3D 密度 + 2D 高度投影）、
//! 批量修改积累和 Chunk 脏标记队列。
//! Copy-on-Write 模式：读者零锁开销，写者构建新数据后原子交换。
//!
//! 参见: CLAUDE.md 地形修改编排层设计

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use glam::IVec3;

use crate::id::ChunkCoord;
use crate::prelude::WorldPos;

// ── 修改类型 ──────────────────────────────────────

/// 地形修改操作类型
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ModificationKind {
    /// 移除材质（负密度）
    Dig,
    /// 添加材质（正密度）
    Fill,
    /// 平滑密度场（平均化）
    Smooth,
    /// 仅修改表面材质，不改密度
    Paint,
    /// 整平到指定高度
    Flatten,
}

/// 单次地形修改的描述
#[derive(Clone, Debug)]
pub struct ModificationRequest {
    pub kind: ModificationKind,
    /// 修改中心（世界坐标）
    pub center: WorldPos,
    /// 影响半径（米）
    pub radius: f32,
    /// 填充/绘制的材质 ID（Dig/Smooth 时忽略）
    pub material: u8,
    /// 修改强度 (0.0-1.0)，用于衰减
    pub intensity: f32,
    /// 请求者 EntityId（用于权限验证和 NPC 感知）
    pub requester: Option<crate::types::EntityId>,
}

// ── 3D 稀疏密度修改 ──────────────────────────────

/// 不可变的密度修改快照（CoW 读者侧）
///
/// 键：量化到体素网格的世界坐标 `(floor(x/vs), floor(y/vs), floor(z/vs))`
/// 值：(密度增量, 材质 ID)
#[derive(Clone, Debug, Default)]
pub struct EditDensitySnapshot {
    edits: Arc<HashMap<IVec3, (f32, u8)>>,
}

impl EditDensitySnapshot {
    /// 查询某位置的密度增量
    pub fn density_delta_at(&self, voxel: IVec3) -> Option<f32> {
        self.edits.get(&voxel).map(|&(d, _)| d)
    }

    /// 查询某位置的材质覆盖
    pub fn material_at(&self, voxel: IVec3) -> Option<u8> {
        self.edits.get(&voxel).map(|&(_, m)| m)
    }

    /// 该 Chunk 是否有任何修改
    pub fn chunk_has_edits(&self, cc: ChunkCoord, voxels_per_chunk: u32, voxel_size: f64) -> bool {
        let vs = voxel_size;
        let vpc = voxels_per_chunk as i32;
        let min_x = (cc.x as f64 * vpc as f64 * vs / vs) as i32;
        let min_z = (cc.z as f64 * vpc as f64 * vs / vs) as i32;
        let min = IVec3::new(min_x, i32::MIN, min_z);
        let max = IVec3::new(min_x + vpc, i32::MAX, min_z + vpc);

        self.edits
            .keys()
            .any(|k| k.x >= min.x && k.x < max.x && k.z >= min.z && k.z < max.z)
    }

    /// 该 Chunk 内的修改数量（用于调试）
    pub fn edit_count_in_chunk(
        &self,
        cc: ChunkCoord,
        voxels_per_chunk: u32,
        voxel_size: f64,
    ) -> usize {
        let vs = voxel_size;
        let vpc = voxels_per_chunk as i32;
        let min_x = (cc.x as f64 * vpc as f64 * vs / vs) as i32;
        let min_z = (cc.z as f64 * vpc as f64 * vs / vs) as i32;

        self.edits
            .keys()
            .filter(|k| k.x >= min_x && k.x < min_x + vpc && k.z >= min_z && k.z < min_z + vpc)
            .count()
    }

    /// 总修改数（用于内存估算）
    pub fn total_edits(&self) -> usize {
        self.edits.len()
    }
}

/// 可变的密度修改构建器（CoW 写者侧）
#[derive(Clone, Debug, Default)]
pub struct EditDensityBuilder {
    edits: HashMap<IVec3, (f32, u8)>,
}

impl EditDensityBuilder {
    pub fn new() -> Self {
        Self {
            edits: HashMap::new(),
        }
    }

    /// 设置单个体素的密度和材质
    pub fn set(&mut self, voxel: IVec3, density_delta: f32, material: u8) {
        // 如果修改结果与默认值无差异，删除条目（压缩）
        if density_delta.abs() < 0.001 {
            self.edits.remove(&voxel);
        } else {
            self.edits.insert(voxel, (density_delta, material));
        }
    }

    /// 从球形体素集批量设置（用于爆炸等）
    pub fn set_sphere(
        &mut self,
        center: IVec3,
        radius_voxels: i32,
        density_delta: f32,
        material: u8,
    ) {
        let r2 = radius_voxels * radius_voxels;
        for dx in -radius_voxels..=radius_voxels {
            for dy in -radius_voxels..=radius_voxels {
                for dz in -radius_voxels..=radius_voxels {
                    let offset = IVec3::new(dx, dy, dz);
                    if offset.x * offset.x + offset.y * offset.y + offset.z * offset.z <= r2 {
                        // 球面衰减
                        let dist = ((offset.x * offset.x
                            + offset.y * offset.y
                            + offset.z * offset.z) as f32)
                            .sqrt();
                        let falloff = 1.0 - (dist / radius_voxels as f32).clamp(0.0, 1.0);
                        let d = density_delta * falloff;
                        self.set(center + offset, d, material);
                    }
                }
            }
        }
    }

    /// 冻结为不可变快照
    pub fn freeze(self) -> EditDensitySnapshot {
        EditDensitySnapshot {
            edits: Arc::new(self.edits),
        }
    }

    /// 从现有快照初始化（用于增量修改）
    pub fn from_snapshot(snapshot: &EditDensitySnapshot) -> Self {
        Self {
            edits: (*snapshot.edits).clone(),
        }
    }

    /// 合并另一个快照的所有条目（用于合并并发修改）
    pub fn merge_snapshot(&mut self, other: &EditDensitySnapshot) {
        for (k, v) in other.edits.iter() {
            self.edits.insert(*k, *v);
        }
    }
}

// ── 2D 表面高度投影 ───────────────────────────────

/// 不可变的表面高度快照
///
/// 键：量化到 Chunk 网格的 (cx, cz) 或 (qx, qz) 坐标
/// 值：修改后的新表面高度（以米为单位）
///
/// 当修改改变了某 (x,z) 位置的表面时，该位置的项被更新。
/// height_at() 优先查询此映射，未命中则回退到噪声高度。
#[derive(Clone, Debug, Default)]
pub struct EditHeightfieldSnapshot {
    /// 量化坐标 (floor(x), floor(z)) → 新表面高度
    heights: Arc<HashMap<(i32, i32), f32>>,
}

impl EditHeightfieldSnapshot {
    /// 查询 (x,z) 处的表面高度覆盖
    pub fn height_at(&self, x: f64, z: f64) -> Option<f32> {
        let qx = x.floor() as i32;
        let qz = z.floor() as i32;
        self.heights.get(&(qx, qz)).copied()
    }

    /// 该 1m² 列是否有任何高度覆盖
    pub fn has_override(&self, x: f64, z: f64) -> bool {
        let qx = x.floor() as i32;
        let qz = z.floor() as i32;
        self.heights.contains_key(&(qx, qz))
    }
}

/// 可变的表面高度构建器
#[derive(Clone, Debug, Default)]
pub struct EditHeightfieldBuilder {
    heights: HashMap<(i32, i32), f32>,
}

impl EditHeightfieldBuilder {
    pub fn new() -> Self {
        Self {
            heights: HashMap::new(),
        }
    }

    /// 设置 (x,z) 处的新表面高度
    pub fn set_height(&mut self, x: f64, z: f64, new_height: f32) {
        let qx = x.floor() as i32;
        let qz = z.floor() as i32;
        self.heights.insert((qx, qz), new_height);
    }

    /// 清除 (x,z) 处的高度覆盖（修改被撤销）
    pub fn clear_height(&mut self, x: f64, z: f64) {
        let qx = x.floor() as i32;
        let qz = z.floor() as i32;
        self.heights.remove(&(qx, qz));
    }

    pub fn freeze(self) -> EditHeightfieldSnapshot {
        EditHeightfieldSnapshot {
            heights: Arc::new(self.heights),
        }
    }

    pub fn from_snapshot(snapshot: &EditHeightfieldSnapshot) -> Self {
        Self {
            heights: (*snapshot.heights).clone(),
        }
    }
}

// ── 组合结构 ───────────────────────────────────────

/// 地形修改的完整状态（CoW 读者侧）
#[derive(Clone, Debug, Default)]
pub struct EditTerrainSnapshot {
    pub density: EditDensitySnapshot,
    pub heightfield: EditHeightfieldSnapshot,
}

/// 地形修改的完整状态（CoW 写者侧）
#[derive(Clone, Debug, Default)]
pub struct EditTerrainBuilder {
    pub density: EditDensityBuilder,
    pub heightfield: EditHeightfieldBuilder,
    /// 本帧受到影响的 Chunk 集合
    pub dirty_chunks: HashSet<ChunkCoord>,
}

impl EditTerrainBuilder {
    pub fn new() -> Self {
        Self {
            density: EditDensityBuilder::new(),
            heightfield: EditHeightfieldBuilder::new(),
            dirty_chunks: HashSet::new(),
        }
    }

    pub fn from_snapshot(snapshot: &EditTerrainSnapshot) -> Self {
        Self {
            density: EditDensityBuilder::from_snapshot(&snapshot.density),
            heightfield: EditHeightfieldBuilder::from_snapshot(&snapshot.heightfield),
            dirty_chunks: HashSet::new(),
        }
    }

    /// 冻结为不可变快照
    pub fn freeze(self) -> EditTerrainSnapshot {
        EditTerrainSnapshot {
            density: self.density.freeze(),
            heightfield: self.heightfield.freeze(),
        }
    }
}

// ── 批量修改积累 ──────────────────────────────────

/// 帧内积累的修改请求（Veloren BlockChange 模式）
#[derive(Clone, Debug, Default)]
pub struct ModificationBatch {
    requests: Vec<ModificationRequest>,
}

impl ModificationBatch {
    pub fn new() -> Self {
        Self {
            requests: Vec::new(),
        }
    }

    pub fn push(&mut self, request: ModificationRequest) {
        self.requests.push(request);
    }

    pub fn extend(&mut self, requests: impl IntoIterator<Item = ModificationRequest>) {
        self.requests.extend(requests);
    }

    pub fn drain(&mut self) -> Vec<ModificationRequest> {
        let taken = std::mem::take(&mut self.requests);
        // 收缩到合理初始容量，防止长期运行后内存滞留
        self.requests = Vec::with_capacity(64);
        taken
    }

    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }

    pub fn len(&self) -> usize {
        self.requests.len()
    }
}

// ── Chunk 脏标记队列 ──────────────────────────────

/// 需要重新生成 mesh 的 Chunk 队列
///
/// 含两级索引：每个脏 Chunk 内的具体修改体素集合
/// （用于确定 Transvoxel 过渡单元是否需要重提取）。
/// 使用 `HashSet` 防重复——同一体素多次修改不累积。
#[derive(Clone, Debug, Default)]
pub struct DirtyChunkQueue {
    /// 脏 Chunk → 该 Chunk 内受影响体素的局部坐标集合
    /// 特殊标记 `(-1, -1, -1)` 表示全量重提取
    chunks: HashMap<ChunkCoord, HashSet<(i32, i32, i32)>>,
}

impl DirtyChunkQueue {
    pub fn new() -> Self {
        Self {
            chunks: HashMap::new(),
        }
    }

    /// 标记一个 Chunk 为脏（含受影响体素局部坐标）
    pub fn mark_dirty(&mut self, cc: ChunkCoord, local_voxel: (i32, i32, i32)) {
        self.chunks.entry(cc).or_default().insert(local_voxel);
    }

    /// 标记整个 Chunk 为脏（不指定具体体素——触发全量重提取）
    pub fn mark_chunk_fully_dirty(&mut self, cc: ChunkCoord) {
        // 用特殊标记表示全量重提取——先清空具体条目避免混合
        self.chunks.entry(cc).or_default().clear();
        self.chunks.entry(cc).or_default().insert((-1, -1, -1));
    }

    /// 检查队列是否为空
    pub fn is_empty(&self) -> bool {
        self.chunks.is_empty()
    }

    /// Chunk 是否为脏
    pub fn is_dirty(&self, cc: ChunkCoord) -> bool {
        self.chunks.contains_key(&cc)
    }

    /// 是否需要全量重提取（含 -1 标记）
    pub fn needs_full_reextract(&self, cc: ChunkCoord) -> bool {
        self.chunks
            .get(&cc)
            .map(|set| set.contains(&(-1, -1, -1)))
            .unwrap_or(false)
    }

    /// 获取脏 Chunk 列表（消耗队列）
    #[allow(clippy::type_complexity)]
    pub fn drain_chunks(&mut self) -> Vec<(ChunkCoord, Vec<(i32, i32, i32)>)> {
        std::mem::take(&mut self.chunks)
            .into_iter()
            .map(|(cc, set)| (cc, set.into_iter().collect()))
            .collect()
    }

    /// 仅获取脏 Chunk 坐标（不消耗队列）
    pub fn dirty_chunk_keys(&self) -> impl Iterator<Item = &ChunkCoord> {
        self.chunks.keys()
    }

    pub fn len(&self) -> usize {
        self.chunks.len()
    }
}

// ── DensityProvider 桥接 ──────────────────────────

use crate::density::DensityProvider;

/// 将 EditDensitySnapshot 包装为 DensityProvider，使其能插入 DensityStack
///
/// 使用可配置的体素尺寸将 WorldPos 映射到编辑存储的 voxel 坐标。
/// CoW 快照提供零锁读取性能。
#[derive(Clone, Debug)]
pub struct EditDensityLayer {
    /// 当前快照（Clone = Arc::clone，零锁读取）
    snapshot: EditDensitySnapshot,
    /// 体素尺寸（米）——用于 WorldPos → IVec3 转换
    voxel_size: f64,
}

impl EditDensityLayer {
    pub fn new(snapshot: EditDensitySnapshot, voxel_size: f64) -> Self {
        Self {
            snapshot,
            voxel_size,
        }
    }

    /// 更新快照（WorldDriver 每帧调用——原子交换引用）
    pub fn update_snapshot(&mut self, snapshot: EditDensitySnapshot) {
        self.snapshot = snapshot;
    }

    #[inline]
    fn world_to_voxel(&self, pos: WorldPos) -> IVec3 {
        IVec3::new(
            (pos.x / self.voxel_size).floor() as i32,
            (pos.y / self.voxel_size).floor() as i32,
            (pos.z / self.voxel_size).floor() as i32,
        )
    }
}

impl DensityProvider for EditDensityLayer {
    fn density_at(&self, pos: WorldPos) -> f32 {
        let voxel = self.world_to_voxel(pos);
        self.snapshot.density_delta_at(voxel).unwrap_or(0.0)
    }

    fn material_at(&self, pos: WorldPos) -> u8 {
        let voxel = self.world_to_voxel(pos);
        // 返回编辑材质；若无编辑则返回 0（Grass，回退材质——由上层逻辑处理）
        self.snapshot.material_at(voxel).unwrap_or(0)
    }

    fn priority(&self) -> u8 {
        10 // 高于 TerrainBaseDensity(0) 和 CaveDensity(4)
    }

    fn layer_name(&self) -> &'static str {
        "edit_density"
    }
}

// ── 测试 ──────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edit_density_cow_basic() {
        let mut builder = EditDensityBuilder::new();
        builder.set(IVec3::new(1, 2, 3), -1.0, 3); // dig out stone
        builder.set(IVec3::new(4, 5, 6), 1.0, 5); // fill with wood

        let snapshot = builder.freeze();
        assert_eq!(snapshot.density_delta_at(IVec3::new(1, 2, 3)), Some(-1.0));
        assert_eq!(snapshot.material_at(IVec3::new(1, 2, 3)), Some(3));
        assert_eq!(snapshot.density_delta_at(IVec3::new(4, 5, 6)), Some(1.0));
        assert_eq!(snapshot.density_delta_at(IVec3::new(0, 0, 0)), None);
    }

    #[test]
    fn test_edit_density_zero_removed() {
        let mut builder = EditDensityBuilder::new();
        builder.set(IVec3::new(0, 0, 0), 0.0001, 0); // near-zero → removed
        let snapshot = builder.freeze();
        assert_eq!(snapshot.density_delta_at(IVec3::new(0, 0, 0)), None);
    }

    #[test]
    fn test_edit_density_incremental() {
        let mut b1 = EditDensityBuilder::new();
        b1.set(IVec3::new(1, 0, 0), -1.0, 3);
        let snap1 = b1.freeze();

        let mut b2 = EditDensityBuilder::from_snapshot(&snap1);
        b2.set(IVec3::new(2, 0, 0), -2.0, 4);

        let snap2 = b2.freeze();
        assert_eq!(snap2.density_delta_at(IVec3::new(1, 0, 0)), Some(-1.0));
        assert_eq!(snap2.density_delta_at(IVec3::new(2, 0, 0)), Some(-2.0));
    }

    #[test]
    fn test_edit_heightfield_basic() {
        let mut builder = EditHeightfieldBuilder::new();
        builder.set_height(100.5, 200.3, 50.0);

        let snapshot = builder.freeze();
        assert_eq!(snapshot.height_at(100.5, 200.3), Some(50.0));
        assert_eq!(snapshot.height_at(100.0, 200.0), Some(50.0)); // quantized to floor
        assert_eq!(snapshot.height_at(0.0, 0.0), None);
    }

    #[test]
    fn test_modification_batch_drain() {
        let mut batch = ModificationBatch::new();
        batch.push(ModificationRequest {
            kind: ModificationKind::Dig,
            center: WorldPos::default(),
            radius: 1.0,
            material: 0,
            intensity: 1.0,
            requester: None,
        });
        assert_eq!(batch.len(), 1);

        let drained = batch.drain();
        assert_eq!(drained.len(), 1);
        assert!(batch.is_empty());
    }

    #[test]
    fn test_dirty_chunk_queue() {
        let mut queue = DirtyChunkQueue::new();
        let cc = ChunkCoord { x: 1, y: 0, z: 2 };
        queue.mark_dirty(cc, (3, 4, 5));
        queue.mark_dirty(cc, (6, 7, 8));
        // 重复标记同一体素——应被去重
        queue.mark_dirty(cc, (3, 4, 5));

        assert!(queue.is_dirty(cc));
        assert_eq!(queue.len(), 1);

        let drained = queue.drain_chunks();
        assert_eq!(drained.len(), 1);
        // 去重后只有 2 个唯一条目
        assert_eq!(drained[0].1.len(), 2);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_dirty_chunk_fully_dirty_marker() {
        let mut queue = DirtyChunkQueue::new();
        let cc = ChunkCoord { x: 0, y: 0, z: 0 };
        queue.mark_dirty(cc, (3, 4, 5)); // 先加具体条目
        queue.mark_chunk_fully_dirty(cc); // 全量标记应清除具体条目

        assert!(queue.needs_full_reextract(cc));
        let drained = queue.drain_chunks();
        // 全量标记后只有 (-1,-1,-1)
        assert_eq!(drained[0].1.len(), 1);
        assert_eq!(drained[0].1[0], (-1, -1, -1));
    }

    #[test]
    fn test_sphere_edit() {
        let mut builder = EditDensityBuilder::new();
        // 半径 2 体素的球形挖掘
        builder.set_sphere(IVec3::new(10, 10, 10), 2, -1.0, 0);

        let snapshot = builder.freeze();
        // 中心应该被修改（密度 = -1.0）
        let center_d = snapshot.density_delta_at(IVec3::new(10, 10, 10)).unwrap();
        assert!(center_d <= -0.9); // ≈ -1.0

        // 距离 1 体素处：falloff = 1 - 1/2 = 0.5，密度 ≈ -0.5
        let near_d = snapshot.density_delta_at(IVec3::new(11, 10, 10)).unwrap();
        assert!(near_d.abs() < center_d.abs()); // 衰减验证
        assert!(near_d <= -0.3); // ≈ -0.5

        // 球外不应该被修改
        assert!(snapshot.density_delta_at(IVec3::new(20, 20, 20)).is_none());

        // 极端边缘（距离=2，falloff=0）：密度 = 0，应被移除（压缩）
        assert!(snapshot.density_delta_at(IVec3::new(12, 10, 10)).is_none());
    }

    #[test]
    fn test_chunk_has_edits() {
        let mut builder = EditDensityBuilder::new();
        // 在 Chunk (0, 0) 内放置修改
        builder.set(IVec3::new(5, 0, 5), -1.0, 3);
        let snapshot = builder.freeze();

        let cc = ChunkCoord { x: 0, y: 0, z: 0 };
        assert!(snapshot.chunk_has_edits(cc, 32, 0.5));

        let cc_far = ChunkCoord { x: 10, y: 0, z: 10 };
        assert!(!snapshot.chunk_has_edits(cc_far, 32, 0.5));
    }
}
