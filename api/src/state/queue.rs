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
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Default, Clone)]
pub struct QueueItem {
    pub seed: [u8; 32], // optionally remove if the key is always the same
    pub slot: u64,
    pub slothash: [u8; 32],
    pub callback_discriminator: Vec<u8>,
    pub callback_program_id: Pubkey,
    pub callback_accounts_meta: Vec<SerializableAccountMeta>,
    pub callback_args: Vec<u8>,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Default, Clone)]
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
        for item in self.items.values() {
            // 32 bytes for the key
            size += 32;

            // QueueItem size:
            // - seed: 32 bytes
            // - slot: 8 bytes
            // - slothash: 32 bytes
            // - callback_discriminator: 4 bytes (length) + actual bytes
            // - callback_program_id: 32 bytes
            // - callback_accounts_meta: 4 bytes (length) + (34 bytes * count)
            // - callback_args: 4 bytes (length) + actual bytes
            size += 32
                + 8
                + 32
                + 4
                + item.callback_discriminator.len()
                + 32
                + 4
                + (item.callback_accounts_meta.len() * 34)
                + 4
                + item.callback_args.len();
        }

        size
    }
}

// -- Borsh helper macros for (de)serialization with a discriminator --

impl_to_bytes_with_discriminator_borsh!(QueueAccount);
impl_try_from_bytes_with_discriminator_borsh!(QueueAccount);
