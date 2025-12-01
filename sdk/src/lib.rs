#[cfg(feature = "anchor")]
pub mod anchor;
pub mod consts;
pub mod instructions;
pub mod pda;
pub mod rnd;
pub mod types;

#[cfg(not(feature = "anchor"))]
mod solana {
    pub use solana_program::pubkey::Pubkey;
    pub use solana_program::system_program;
}

#[cfg(feature = "anchor")]
mod solana {
    pub use anchor_lang::prelude::Pubkey;
    pub use anchor_lang::system_program;
}

pub use solana::Pubkey;

pub const fn id() -> Pubkey {
    consts::VRF_PROGRAM_ID
}
