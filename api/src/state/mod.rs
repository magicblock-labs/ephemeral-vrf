mod macros;
mod oracle;
mod oracles;
mod queue;

pub use oracle::*;
pub use oracles::*;
pub use queue::*;
use solana_program::pubkey;

use steel::*;

use crate::consts::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
pub enum AccountDiscriminator {
    Oracles = 0,
    Oracle = 1,
    Counter = 2,
    Queue = 3,
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
    //Pubkey::find_program_address(&[ORACLES], &crate::id()) ->
    (pubkey!("3ctxDejjAY6YGt7rNUUGaKTwvaTaRvwSwRo2fJQqcqWA"), 255)
}
pub fn oracle_data_pda(identity: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[ORACLE_DATA, identity.to_bytes().as_slice()], &crate::id())
}

pub fn program_identity_pda() -> (Pubkey, u8) {
    //Pubkey::find_program_address(&[IDENTITY], &crate::id()) ->
    (pubkey!("9irBy75QS2BN81FUgXuHcjqceJJRuc9oDkAe8TKVvvAw"), 254)
}

/// Fetch PDA of the queue account.
pub fn oracle_queue_pda(identity: &Pubkey, index: u8) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[QUEUE, identity.to_bytes().as_slice(), &[index]],
        &crate::id(),
    )
}
