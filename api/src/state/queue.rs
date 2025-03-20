use crate::prelude::{AccountDiscriminator, AccountWithDiscriminator};
use crate::{impl_to_bytes_with_discriminator_borsh, impl_try_from_bytes_with_discriminator_borsh};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey; // or `use steel::*;` if that's where Pubkey is imported
use std::collections::HashMap;

/// The account now holds a HashMap keyed by [u8; 32].
#[derive(BorshSerialize, BorshDeserialize, Debug, Default)]
pub struct QueueAccount {
    /// Each entry is keyed by the `seed` ([u8; 32]) that was previously part of QueueItem.
    pub items: HashMap<[u8; 32], QueueItem>,
}

/// Same as before, but you no longer need to rely on `seed` inside QueueItem
/// if your key is truly the seed. You can either keep or remove it.
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Default)]
pub struct QueueItem {
    pub seed: [u8; 32], // optionally remove if the key is always the same
    pub slot: u64,
    pub slothash: [u8; 32],
    pub callback_discriminator: [u8; 8],
    pub callback_program_id: Pubkey,
    pub callback_accounts_meta: Vec<SerializableAccountMeta>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Default)]
pub struct SerializableAccountMeta {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

// -- Account trait impls --

impl AccountWithDiscriminator for QueueAccount {
    fn discriminator() -> AccountDiscriminator {
        AccountDiscriminator::Queue
    }
}

/// Estimate the on-chain size of this account, including the 8-byte discriminator.
/// This is not strictly required by Borsh, but sometimes you want a rough
/// upper bound for creating the account.
impl QueueAccount {
    pub fn size_with_discriminator(&self) -> usize {
        // 8 bytes for the account discriminator
        // + 4 bytes for the length of the HashMap (Borsh encodes the map length as u32).
        let mut size = 8 + 4;

        // For each key-value pair:
        for (_key, item) in &self.items {
            // +32 bytes for the key: [u8; 32]
            size += 32;

            // Now add the size of QueueItem fields:
            //   seed: [u8; 32]        => 32
            //   slot: u64             => 8
            //   slothash: [u8; 32]    => 32
            //   callback_discriminator: [u8; 8] => 8
            //   callback_program_id: Pubkey => 32
            //
            //   callback_accounts_meta: Vec<SerializableAccountMeta>
            //     => 4 bytes for length + ( len * size_of_each )
            //     => each SerializableAccountMeta has:
            //          pubkey (32 bytes) + is_signer (1 byte) + is_writable (1 byte)
            //        => 34 bytes each
            let meta_count = item.callback_accounts_meta.len();
            let item_size = 32 + 8 + 32 + 8 + 32    // fixed fields
                + 4                                 // length of callback_accounts_meta
                + (meta_count * 34); // each element
            size += item_size;
        }

        size
    }
}

// -- Borsh helper macros for (de)serialization with a discriminator --

impl_to_bytes_with_discriminator_borsh!(QueueAccount);
impl_try_from_bytes_with_discriminator_borsh!(QueueAccount);
