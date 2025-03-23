pub mod accounts;

use solana_program::pubkey;
use steel::Pubkey;
#[allow(unused_imports)]
pub(crate) use accounts::*;

pub(crate) const TEST_CALLBACK_PROGRAM: Pubkey =  pubkey!("AL32mNVFdhxHXztaWuNWvwoiPYCHofWmVRNH49pMCafD");
pub(crate) const TEST_CALLBACK_DISCRIMINATOR: [u8; 8] = [190, 217, 49, 162, 99, 26, 73, 234];
