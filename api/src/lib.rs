#[macro_use]
pub mod macros;

pub mod consts;
pub mod error;
pub mod instruction;
pub mod loaders;
pub mod pda;
pub mod sdk;
pub mod state;
pub mod steel;
pub mod verify;

pub mod prelude {
    pub use crate::consts::*;
    pub use crate::error::*;
    pub use crate::instruction::*;
    pub use crate::pda::*;
    pub use crate::sdk::*;
    pub use crate::state::*;
    pub use crate::steel::*;
}

use crate::steel::*;

declare_id!("Vrf1RNUjXmQGjmQrQLvJHs9SNkvDJEsRVFPkfSQUwGz");
