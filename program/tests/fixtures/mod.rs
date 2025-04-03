pub mod accounts;

#[allow(unused_imports)]
pub(crate) use accounts::*;
use solana_program::pubkey;
use steel::Pubkey;

pub(crate) const TEST_CALLBACK_PROGRAM: Pubkey =
    pubkey!("CDiutifqugEkabdqwc5TK3FmSAgFpkP3RPE1642BCEhi");
