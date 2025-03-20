mod macros;
mod oracles;
mod queue;

pub use oracles::*;
pub use queue::*;

use steel::*;

use crate::consts::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
pub enum AccountDiscriminator {
    Oracles = 0,
    Counter = 1,
    Queue = 2,
}

impl AccountDiscriminator {
    pub fn to_bytes(&self) -> [u8; 8] {
        let num = (*self) as u64;
        num.to_le_bytes()
    }
}

pub trait AccountWithDiscriminator {
    fn discriminator() -> AccountDiscriminator;
}

/// Fetch PDA of the oracles account.
pub fn oracles_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[ORACLES], &crate::id())
}

/// Fetch PDA of the queue account.
pub fn oracle_queue_pda(identity: Pubkey, index: u8) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[QUEUE, identity.to_bytes().as_slice(), &[index]],
        &crate::id(),
    )
}
