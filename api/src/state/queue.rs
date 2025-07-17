use crate::prelude::AccountDiscriminator;
use borsh::{BorshDeserialize, BorshSerialize};
use steel::{account, Pod, ProgramError, Zeroable, trace, AccountMeta, Pubkey};

/// The maximum number of accounts allowed in a QueueItem
pub const MAX_ACCOUNTS: usize = 5;
/// The maximum size of callback args in bytes
pub const MAX_ARGS_SIZE: usize = 8;
/// The maximum number of items in the queue
pub const MAX_QUEUE_ITEMS: usize = 20;

/// Fixed-size QueueAccount with preallocated space
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct Queue {
    pub index: u8,
    pub item_count: u8,
    pub used_bitmap: [u8; MAX_QUEUE_ITEMS], // 0 = free, 1 = used
    pub items: [QueueItem; MAX_QUEUE_ITEMS],
}

/// Fixed-size QueueItem with size constraints
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod, PartialEq)]
pub struct QueueItem {
    pub id: [u8; 32],
    pub callback_discriminator: [u8; 8],
    pub callback_program_id: [u8; 32],
    pub callback_accounts_meta: [SerializableAccountMeta; MAX_ACCOUNTS],
    pub callback_args: [u8; MAX_ARGS_SIZE],
    pub slot: u64,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod, PartialEq, BorshDeserialize, BorshSerialize)]
pub struct SerializableAccountMeta {
    pub pubkey: [u8; 32],
    pub is_signer: u8,
    pub is_writable: u8,
}

impl SerializableAccountMeta {
    pub fn to_account_meta(&self) -> AccountMeta {
        let pubkey = Pubkey::new_from_array(self.pubkey);
        let is_signer = self.is_signer != 0;
        let is_writable = self.is_writable != 0;

        AccountMeta {
            pubkey,
            is_signer,
            is_writable,
        }
    }
}

/// Helper methods for QueueAccount
impl Queue {
    pub fn add_item(&mut self, item: QueueItem) -> Result<usize, ProgramError> {
        for i in 0..MAX_QUEUE_ITEMS {
            if self.used_bitmap[i] == 0 {
                self.items[i] = item;
                self.used_bitmap[i] = 1;
                self.item_count += 1;
                return Ok(i);
            }
        }
        Err(ProgramError::AccountDataTooSmall)
    }

    pub fn remove_item(&mut self, index: usize) -> Result<QueueItem, ProgramError> {
        if index >= MAX_QUEUE_ITEMS || self.used_bitmap[index] == 0 {
            return Err(ProgramError::InvalidArgument);
        }

        let item = self.items[index];
        self.used_bitmap[index] = 0;
        self.item_count -= 1;
        Ok(item)
    }

    pub fn iter_items(&self) -> impl Iterator<Item = &QueueItem> {
        self.items.iter().enumerate()
            .filter_map(|(i, item)| if self.used_bitmap[i] == 1 { Some(item) } else { None })
    }

    pub fn find_item_by_id(&self, id: &[u8; 32]) -> Option<(usize, &QueueItem)> {
        for i in 0..MAX_QUEUE_ITEMS {
            if self.used_bitmap[i] == 1 && self.items[i].id == *id {
                return Some((i, &self.items[i]));
            }
        }
        None
    }

    pub fn is_empty(&self) -> bool {
        self.item_count == 0
    }

    pub fn len(&self) -> usize {
        self.item_count as usize
    }

    pub fn size_with_discriminator() -> usize {
        8 + size_of::<Queue>()
    }
}

account!(AccountDiscriminator, Queue);