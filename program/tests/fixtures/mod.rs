pub mod accounts;

#[allow(unused_imports)]
pub(crate) use accounts::*;
use ephemeral_vrf_api::steel::Pubkey;
use solana_program::pubkey;

pub(crate) const TEST_CALLBACK_PROGRAM: Pubkey =
    pubkey!("CDiutifqugEkabdqwc5TK3FmSAgFpkP3RPE1642BCEhi");
