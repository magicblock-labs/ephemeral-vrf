pub mod consts;
pub mod error;
pub mod instruction;
pub mod pda;
pub mod sdk;
pub mod state;

pub mod prelude {
    pub use crate::consts::*;
    pub use crate::error::*;
    pub use crate::instruction::*;
    pub use crate::pda::*;
    pub use crate::sdk::*;
    pub use crate::state::*;
}

use steel::*;

// TODO Set program id
declare_id!("VrffXU38S8MzqTtTYQG3M8GNwheKH8n77HVEZUdakH8");
