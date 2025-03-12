use steel::*;
use borsh::{BorshDeserialize, BorshSerialize};
use crate::{impl_to_bytes_with_discriminator_borsh, impl_try_from_bytes_with_discriminator_borsh};
use crate::prelude::{AccountDiscriminator, AccountWithDiscriminator};

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Default)]
pub struct Oracles {
    pub items: Vec<QueueItem>,
}

// Each queue item. Customize fields as you need.
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Default)]
pub struct QueueItem {
    pub public_key: Pubkey,
    pub seed: [u8; 32],
    pub blockhash: [u8; 32],
    pub callback_discriminator: [u8; 8],
    pub callback_accounts_meta: Vec<Pubkey>,
}

impl AccountWithDiscriminator for Oracles {
    fn discriminator() -> AccountDiscriminator {
        AccountDiscriminator::Oracles
    }
}

impl_to_bytes_with_discriminator_borsh!(Oracles);
impl_try_from_bytes_with_discriminator_borsh!(Oracles);
