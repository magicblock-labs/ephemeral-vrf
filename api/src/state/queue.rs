use crate::prelude::{AccountDiscriminator, AccountWithDiscriminator, RkyvPubkey};
use crate::{impl_to_bytes_with_discriminator_rkyv, impl_try_from_bytes_with_discriminator_rkyv};
use borsh::{BorshDeserialize, BorshSerialize};
use rkyv::{Archive, Deserialize, Serialize};
use solana_program::pubkey::Pubkey;

/// The maximum number of accounts allowed in a QueueItem
pub const MAX_ACCOUNTS: usize = 5;
/// The maximum size of callback args in bytes
pub const MAX_ARGS_SIZE: usize = 8;
/// The maximum number of items in the queue
pub const MAX_QUEUE_ITEMS: usize = 5;

/// Fixed-size QueueAccount with preallocated space
#[derive(Archive, Serialize, Deserialize, Debug)]
#[archive(compare(PartialEq), check_bytes)]
pub struct QueueAccount {
    /// The index of the queue.
    pub index: u8,
    /// Array of optional queue items
    pub items: [Option<QueueItem>; MAX_QUEUE_ITEMS],
    /// Number of items currently in the queue
    pub item_count: u8,
}

/// Fixed-size QueueItem with size constraints
#[derive(Archive, Serialize, Deserialize, Debug, PartialEq, Default, Clone)]
#[archive(compare(PartialEq), check_bytes)]
pub struct QueueItem {
    pub id: [u8; 32],
    pub callback_discriminator: Vec<u8>,
    pub callback_program_id: RkyvPubkey,
    pub callback_accounts_meta: Vec<SerializableAccountMeta>,
    pub callback_args: Vec<u8>,
    pub slot: u64,
}

#[derive(Archive, Serialize, Deserialize, Debug, PartialEq, Default, Clone, BorshSerialize, BorshDeserialize)]
#[archive(compare(PartialEq), check_bytes)]
pub struct SerializableAccountMeta {
    pub pubkey: RkyvPubkey,
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
        //     - callback_program_id: 32 bytes (RkyvPubkey)
        //     - callback_accounts_meta: 4 bytes (Vec length) + MAX_ACCOUNTS * (32 + 1 + 1) = 4 + MAX_ACCOUNTS * 34 bytes
        //     - callback_args: 4 bytes (Vec length) + MAX_ARGS_SIZE bytes (max content)
        //     - slot: 8 bytes

        // Rkyv serialization adds some overhead for alignment and metadata
        // We'll add a 20% buffer to account for this
        let queue_item_size = 32 + (4 + 8) + 32 + (4 + (MAX_ACCOUNTS * 34)) + (4 + MAX_ARGS_SIZE) + 8;
        let rkyv_overhead = (queue_item_size as f32 * 0.2) as usize;
        let total_size = 8 + 1 + 1 + (MAX_QUEUE_ITEMS * (1 + queue_item_size + rkyv_overhead)); // 1 byte for Option discriminant

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

// -- Rkyv helper macros for (de)serialization with a discriminator --

impl_to_bytes_with_discriminator_rkyv!(QueueAccount);
impl_try_from_bytes_with_discriminator_rkyv!(QueueAccount);

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

    #[test]
    fn test_queue_account_serialization() {
        // Create a queue with one item
        let mut queue = QueueAccount::default();
        let item = QueueItem {
            id: [1; 32],
            callback_discriminator: vec![1, 2, 3, 4],
            callback_program_id: RkyvPubkey::new(Pubkey::new_unique()),
            callback_accounts_meta: vec![
                SerializableAccountMeta {
                    pubkey: RkyvPubkey::new(Pubkey::new_unique()),
                    is_signer: true,
                    is_writable: true,
                }
            ],
            callback_args: vec![5, 6, 7, 8],
            slot: 123,
        };
        queue.add_item(item).unwrap();

        // Serialize the queue with discriminator
        let serialized = queue.to_bytes_with_discriminator().unwrap();

        // Deserialize the queue
        let deserialized = QueueAccount::try_from_bytes_with_discriminator(&serialized).unwrap();

        // Verify the deserialized queue matches the original
        assert_eq!(deserialized.index, queue.index);
        assert_eq!(deserialized.item_count, queue.item_count);

        // Check the item was correctly deserialized
        let original_item = queue.items[0].as_ref().unwrap();
        let deserialized_item = deserialized.items[0].as_ref().unwrap();

        assert_eq!(deserialized_item.id, original_item.id);
        assert_eq!(deserialized_item.callback_discriminator, original_item.callback_discriminator);
        assert_eq!(deserialized_item.callback_program_id.bytes, original_item.callback_program_id.bytes);
        assert_eq!(deserialized_item.callback_accounts_meta.len(), original_item.callback_accounts_meta.len());
        assert_eq!(deserialized_item.callback_accounts_meta[0].pubkey.bytes, original_item.callback_accounts_meta[0].pubkey.bytes);
        assert_eq!(deserialized_item.callback_accounts_meta[0].is_signer, original_item.callback_accounts_meta[0].is_signer);
        assert_eq!(deserialized_item.callback_accounts_meta[0].is_writable, original_item.callback_accounts_meta[0].is_writable);
        assert_eq!(deserialized_item.callback_args, original_item.callback_args);
        assert_eq!(deserialized_item.slot, original_item.slot);
    }

    #[test]
    fn test_serializable_account_meta_borsh_serialization() {
        // Create a SerializableAccountMeta
        let meta = SerializableAccountMeta {
            pubkey: RkyvPubkey::new(Pubkey::new_unique()),
            is_signer: true,
            is_writable: false,
        };

        // Serialize with Borsh
        let serialized = borsh::to_vec(&meta).unwrap();

        // Deserialize with Borsh
        let deserialized = borsh::from_slice::<SerializableAccountMeta>(&serialized).unwrap();

        // Verify the deserialized meta matches the original
        assert_eq!(deserialized.pubkey.bytes, meta.pubkey.bytes);
        assert_eq!(deserialized.is_signer, meta.is_signer);
        assert_eq!(deserialized.is_writable, meta.is_writable);
    }
}
