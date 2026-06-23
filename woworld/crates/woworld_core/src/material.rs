//! 地表材质与介质类型
//!
//! 参见: `WoWorld-Design/开发路线图/002-轨A-正式开发.md` A.2 Tier 0

/// 地表材质（21 变体 — 音频模块权威定义）
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum SurfaceMaterial {
    Grass,
    Sand,
    Rock,
    Stone,
    Wood,
    Metal,
    Water,
    Ice,
    Mud,
    Snow,
    Gravel,
    Clay,
    Moss,
    LeafLitter,
    Cobblestone,
    Marble,
    Glass,
    Fabric,
    Thatch,
    Bone,
    Flesh,
}

/// 介质类型（实体所处的环境介质）
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Medium {
    /// 空气（默认）
    Air,
    /// 水
    Water,
    /// 岩浆
    Magma,
    /// 虚空（世界边界外）
    Void,
}
