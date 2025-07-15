use crate::prelude::{AccountDiscriminator, AccountWithDiscriminator};
use crate::{impl_to_bytes_with_discriminator_borsh, impl_try_from_bytes_with_discriminator_borsh};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;

/// The maximum number of accounts allowed in a QueueItem
pub const MAX_ACCOUNTS: usize = 5;
/// The maximum size of callback args in bytes
pub const MAX_ARGS_SIZE: usize = 8;
/// The maximum number of items in the queue
pub const MAX_QUEUE_ITEMS: usize = 5;

/// Fixed-size QueueAccount with preallocated space
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct QueueAccount {
    /// The index of the queue.
    pub index: u8,
    /// Array of optional queue items
    pub items: [Option<QueueItem>; MAX_QUEUE_ITEMS],
    /// Number of items currently in the queue
    pub item_count: u8,
}

/// Fixed-size QueueItem with size constraints
#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Default, Clone)]
pub struct QueueItem {
    pub id: [u8; 32],
    pub callback_discriminator: Vec<u8>,
    pub callback_program_id: Pubkey,
    pub callback_accounts_meta: Vec<SerializableAccountMeta>,
    pub callback_args: Vec<u8>,
    pub slot: u64,
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

/// Helper methods for QueueAccount
impl QueueAccount {
    /// Add an item to the queue
    pub fn add_item(&mut self, item: QueueItem) -> Result<usize, solana_program::program_error::ProgramError> {
        if self.item_count as usize >= MAX_QUEUE_ITEMS {
            return Err(solana_program::program_error::ProgramError::AccountDataTooSmall);
        }

        // Find an empty slot
        for i in 0..MAX_QUEUE_ITEMS {
            if self.items[i].is_none() {
                self.items[i] = Some(item);
                self.item_count += 1;
                return Ok(i);
            }
        }

        // This should never happen if item_count is accurate
        Err(solana_program::program_error::ProgramError::AccountDataTooSmall)
    }

    /// Find an item by its id
    pub fn find_item_by_id(&self, id: &[u8; 32]) -> Option<(usize, &QueueItem)> {
        for i in 0..MAX_QUEUE_ITEMS {
            if let Some(item) = &self.items[i] {
                if item.id == *id {
                    return Some((i, item));
                }
            }
        }
        None
    }

    /// Remove an item from the queue
    pub fn remove_item(&mut self, index: usize) -> Option<QueueItem> {
        if index >= MAX_QUEUE_ITEMS {
            return None;
        }

        let item = self.items[index].take();
        if item.is_some() {
            self.item_count -= 1;
        }
        item
    }

    /// Get the number of items in the queue
    pub fn len(&self) -> usize {
        self.item_count as usize
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.item_count == 0
    }

    /// Calculate the fixed size of this account, including the 8-byte discriminator.
    pub fn size_with_discriminator(&self) -> usize {
        // 8 bytes for the account discriminator
        // 1 byte for the index
        // 1 byte for the item_count
        // For each queue item (MAX_QUEUE_ITEMS):
        //   1 byte for Option<QueueItem> discriminant
        //   For each QueueItem (when Some):
        //     - id: 32 bytes
        //     - callback_discriminator: 4 bytes (Vec length) + 8 bytes (typical discriminator size)
        //     - callback_program_id: 32 bytes
        //     - callback_accounts_meta: 4 bytes (Vec length) + MAX_ACCOUNTS * (32 + 1 + 1) = 4 + MAX_ACCOUNTS * 34 bytes
        //     - callback_args: 4 bytes (Vec length) + MAX_ARGS_SIZE bytes (max content)
        //     - slot: 8 bytes

        let queue_item_size = 32 + (4 + 8) + 32 + (4 + (MAX_ACCOUNTS * 34)) + (4 + MAX_ARGS_SIZE) + 8;
        let total_size = 8 + 1 + 1 + (MAX_QUEUE_ITEMS * (1 + queue_item_size)); // 1 byte for Option discriminant

        total_size
    }
}

// -- Default implementation for QueueAccount --

impl Default for QueueAccount {
    fn default() -> Self {
        Self {
            index: 0,
            items: std::array::from_fn(|_| None),
            item_count: 0,
        }
    }
}

// -- Borsh helper macros for (de)serialization with a discriminator --

impl_to_bytes_with_discriminator_borsh!(QueueAccount);
impl_try_from_bytes_with_discriminator_borsh!(QueueAccount);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_account_default() {
        let queue = QueueAccount::default();
        assert_eq!(queue.index, 0);
        assert_eq!(queue.item_count, 0);
        assert_eq!(queue.items.len(), MAX_QUEUE_ITEMS);
        for item in queue.items.iter() {
            assert!(item.is_none());
        }
    }
}
