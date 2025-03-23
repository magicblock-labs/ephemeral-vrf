use crate::prelude::{AccountDiscriminator, AccountWithDiscriminator};
use crate::{impl_to_bytes_with_discriminator_borsh, impl_try_from_bytes_with_discriminator_borsh};
use borsh::{BorshDeserialize, BorshSerialize};
use steel::*;

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Default)]
pub struct Oracles {
    pub oracles: Vec<Pubkey>,
}

impl AccountWithDiscriminator for Oracles {
    fn discriminator() -> AccountDiscriminator {
        AccountDiscriminator::Oracles
    }
}

impl Oracles {
    pub fn size_with_discriminator(&self) -> usize {
        let item_size = 32;
        8 + 4 + (item_size * self.oracles.len())
    }
}

impl_to_bytes_with_discriminator_borsh!(Oracles);
impl_try_from_bytes_with_discriminator_borsh!(Oracles);
