//! 玩家系统 System 模块
//!
//! - possess: 夺舍/退出夺舍逻辑
//! - player_input: 玩家方向输入 → CMoveIntent（Movement 域，不经 ActionResolver）

pub mod player_input;
pub mod possess;
