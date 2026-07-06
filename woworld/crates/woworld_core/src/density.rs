//! 密度场层叠体系 — P2a 架构地基
//!
//! 定义 DensityProvider trait（各模块实现）和 DensityStack（世界生成编排器消费）。
//! 仅依赖 glam — 引擎无关，满足宪法 §14.1 Rust 权威原则。

use std::sync::Arc;

use crate::prelude::WorldPos;

/// 单层密度场 — 正值 = 实体，负值 = 空
///
/// 各领域模块实现此 trait 注册自己的密度贡献：
/// - woworld_worldgen: TerrainBaseDensity（地形基底）
/// - 洞穴系统: CaveDensity（Worley 3D 噪声）
/// - 建筑模块: BuildingFoundationDensity（地基切削）
/// - NPC 系统: NpcEditDensity（挖掘/踩踏修改）
/// - 玩家系统: PlayerSdfDensity（SDF 雕刻）
pub trait DensityProvider: Send + Sync + std::fmt::Debug {
    /// 查询指定世界坐标的密度值
    fn density_at(&self, pos: WorldPos) -> f32;

    /// 查询该位置的材质 ID（映射自 SurfaceMaterial 枚举）
    fn material_at(&self, pos: WorldPos) -> u8;

    /// 该层的优先级——低优先级先叠加，高优先级后覆盖/切削
    fn priority(&self) -> u8;

    /// 人类可读层名（用于调试和存档）
    fn layer_name(&self) -> &'static str {
        "unnamed"
    }
}

/// 有序密度层叠——按 priority 升序排列，density_at 累加所有层
///
/// 使用 `Arc<dyn DensityProvider>` 而非 `Box<dyn DensityProvider>`——
/// 支持 Clone（Arc 引用计数）和 Debug。
#[derive(Clone, Debug)]
pub struct DensityStack {
    layers: Vec<Arc<dyn DensityProvider>>,
}

impl DensityStack {
    pub fn new() -> Self {
        Self {
            layers: Vec::new(),
        }
    }

    /// 插入层并保持 priority 升序
    pub fn push(&mut self, layer: Arc<dyn DensityProvider>) {
        let pos = self
            .layers
            .binary_search_by_key(&layer.priority(), |l| l.priority())
            .unwrap_or_else(|e| e);
        self.layers.insert(pos, layer);
    }

    /// 累加所有层的密度贡献
    pub fn density_at(&self, pos: WorldPos) -> f32 {
        self.layers
            .iter()
            .fold(0.0f32, |acc, l| acc + l.density_at(pos))
    }

    /// 组合所有层的材质——最高 priority 有密度贡献的层获胜
    ///
    /// 从高 priority 向低遍历，返回第一个对 pos 有显著密度贡献的层的材质。
    /// 若所有层均无贡献，回退到最低 priority 层（基底）。
    /// EditDensity (priority 10) 可覆盖 TerrainBaseDensity (priority 0) 的材质。
    pub fn material_at(&self, pos: WorldPos) -> Option<u8> {
        // 从高 priority 向低查找"所有权"
        for layer in self.layers.iter().rev() {
            let d = layer.density_at(pos);
            if d.abs() > 0.001 {
                return Some(layer.material_at(pos));
            }
        }
        // 回退：最低 priority 层（基底——即使它在表面处密度为 0）
        self.layers.first().map(|l| l.material_at(pos))
    }

    /// 在 (x, z) 处查找密度场的零等值面高度（即实际表面）
    ///
    /// 从 `y_max` 向下扫描到 `y_min`，返回第一个 `density >= 0` 的高度。
    /// 若找不到零等值面（纯空或纯实），返回 None。
    /// 步长 `step` 控制精度 vs 性能（建议 0.5m）。
    ///
    /// ★ 当前未被调用——保留作为 EditHeightfield 更新的工具方法。
    pub fn find_surface_y(
        &self,
        x: f64,
        z: f64,
        y_min: f64,
        y_max: f64,
        step: f64,
    ) -> Option<f64> {
        // 防御：y_min 必须 ≤ y_max
        if y_min > y_max || step <= 0.0 {
            return None;
        }
        let mut y = y_max;
        // 从高空向下扫描
        while y >= y_min {
            let d = self.density_at(WorldPos { x, y, z });
            if d >= 0.0 {
                // 找到固体——向上精确定位零等值面
                // 在 [y, y+step] 区间内二分搜索
                let mut lo = y;
                let mut hi = y + step;
                for _ in 0..8 {
                    // 8 次迭代 → 精度 step/256
                    let mid = (lo + hi) * 0.5;
                    let dm = self.density_at(WorldPos { x, y: mid, z });
                    if dm >= 0.0 {
                        lo = mid;
                    } else {
                        hi = mid;
                    }
                }
                return Some(lo);
            }
            y -= step;
        }
        None
    }
}

impl Default for DensityStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestLayer {
        priority: u8,
        density: f32,
        name: &'static str,
    }
    impl DensityProvider for TestLayer {
        fn density_at(&self, _pos: WorldPos) -> f32 {
            self.density
        }
        fn material_at(&self, _pos: WorldPos) -> u8 {
            1
        }
        fn priority(&self) -> u8 {
            self.priority
        }
        fn layer_name(&self) -> &'static str {
            self.name
        }
    }

    #[test]
    fn test_empty_stack_returns_zero() {
        let stack = DensityStack::new();
        assert!((stack.density_at(WorldPos::default()) - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_layer_ordering() {
        let mut stack = DensityStack::new();
        stack.push(Arc::new(TestLayer {
            priority: 20,
            density: 10.0,
            name: "high",
        }));
        stack.push(Arc::new(TestLayer {
            priority: 10,
            density: 5.0,
            name: "low",
        }));
        // 顺序：priority 10 在前，20 在后 → 累加 = 5.0 + 10.0 = 15.0
        let h = stack.density_at(WorldPos::default());
        assert!((h - 15.0).abs() < 0.001);
    }
}
