//! WoWorld Core — 核心类型与 trait 定义
//!
//! 最少依赖 crate（仅 glam）。所有跨模块共享的类型、ID 注册表、
//! 空间查询 trait 均在此定义。引擎无关——降低未来迁移成本。
//!
//! 参见: `WoWorld-Design/开发路线图/002-轨A-正式开发.md` A.2 阶段二

pub mod culture;
pub mod density;
pub mod economy;
pub mod edit_terrain;
pub mod entity_visual;
pub mod faith;
pub mod id;
pub mod item;
pub mod lod;
pub mod material;
pub mod naming;
pub mod ocean;
pub mod player;
pub mod power;
pub mod spatial;
pub mod time;
pub mod types;
pub mod vegetation;
pub mod weather_types;

/// 常用类型统一导入
pub mod prelude {
    pub use crate::culture::CultureCoreParams;
    pub use crate::culture::CultureId;
    pub use crate::culture::CULTURE_ID_NONE;
    pub use crate::economy::{
        bootstrap::{initial_money_supply, inject_liquidity, BootstrapParams, LiquidityInjection},
        listing::{
            urgency_to_listing_type, ListingStatus, ListingType, NeedCategory, NeedReason, Urgency,
        },
        EconomicHealthIndex, EconomyId, EconomyQuery, Market, MarketId, Order, OrderBook,
        OrderSide, PriceSnapshot, TradeRecord, WalletSnapshot, MARKET_ID_NONE,
    };
    pub use crate::entity_visual::{DebugField, DebugSection, EntityDebugSnapshot, EntityVisual};
    pub use crate::faith::{FaithId, FaithTheology, ReligiousMotivation, FAITH_ID_NONE};
    pub use crate::id::*;
    pub use crate::item::assembly::{
        ComponentConnection, ComponentSlot, ItemAssembly, ItemParams, JointType, ParamDef,
        ParamSchema, SlotInstanceId,
    };
    pub use crate::item::equipment::{
        AccessorySet, CharacterEquipment, ContainerSet, EquipmentVisualToggles, OutfitMode,
        OutfitSet, SlotId,
    };
    pub use crate::item::inventory::{InventoryError, InventorySlot, PersonalInventory};
    pub use crate::item::inventory_tuning;
    pub use crate::item::{
        effective_encumbrance_kg, ConsumableEffect, ItemCategory, ItemDef, ItemPlacementProps,
        ItemProperties, ItemQuery, ItemStack, ItemState, ItemTag, Quality, Rarity,
        ITEM_DEF_ID_NONE,
    };
    pub use crate::lod::*;
    pub use crate::material::*;
    pub use crate::naming::{generate_name, NpcName};
    pub use crate::player::{ActionDomain, ControlMode};
    pub use crate::power::{PowerAtom, PowerEdge, PowerSource, LEGITIMACY_CRISIS_THRESHOLD};
    pub use crate::time::*;
    pub use crate::types::*;
    pub use crate::vegetation::*;
    pub use crate::weather_types::*;
}
