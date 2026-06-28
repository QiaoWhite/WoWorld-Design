//! WoWorld Vegetation — 植被覆盖层
//!
//! 实现 `VegetationProvider` trait（定义于 `woworld_core`）。
//! 核心模块：Shannon 熵优势种筛选 + 植被噪声 + 物种适应度查询。
//!
//! 参见: `WoWorld-Design/Happy Game/开发阶段/生命/010-植被群落与覆盖.md`
//! 参见: `WoWorld-Design/Happy Game/开发阶段/世界生成/012-植被覆盖生成.md`

pub mod community;
pub mod noise;
pub mod provider;
pub mod species;
