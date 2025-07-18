use crate::prelude::{AccountDiscriminator, EphemeralVrfError};
use borsh::{BorshDeserialize, BorshSerialize};
use steel::{account, trace, AccountMeta, Pod, ProgramError, Pubkey, Zeroable};

pub const MAX_ACCOUNTS: usize = 5;
pub const MAX_ARGS_SIZE: usize = 128;
pub const MAX_QUEUE_ITEMS: usize = 25;

/// Fixed-size QueueAccount with pre-allocated space
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct Queue {
    pub index: u8,
    pub item_count: u8,
    pub used_bitmap: MaxQueueItemsBitmap, // 0 = free, 1 = used
    pub items: MaxQueueItems,
}

/// Fixed-size QueueItem with size constraints
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod, PartialEq)]
pub struct QueueItem {
    pub id: [u8; 32],
    pub callback_discriminator: [u8; 8],
    pub callback_program_id: [u8; 32],
    pub callback_accounts_meta: [SerializableAccountMeta; MAX_ACCOUNTS],
    pub callback_args: CallbackArgs,
    pub slot: u64,
    pub args_size: u8,
    pub num_accounts_meta: u8,
    pub discriminator_size: u8,
    pub priority_request: u8,
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Zeroable, Pod, PartialEq)]
pub struct MaxQueueItems(pub [QueueItem; MAX_QUEUE_ITEMS]);

impl Default for MaxQueueItems {
    fn default() -> Self {
        MaxQueueItems(unsafe { std::mem::zeroed() })
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Zeroable, Pod, PartialEq)]
pub struct MaxQueueItemsBitmap(pub [u8; MAX_QUEUE_ITEMS]);

impl Default for MaxQueueItemsBitmap {
    fn default() -> Self {
        MaxQueueItemsBitmap(unsafe { std::mem::zeroed() })
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Zeroable, Pod, PartialEq)]
pub struct CallbackArgs(pub [u8; MAX_ARGS_SIZE]);

impl Default for CallbackArgs {
    fn default() -> Self {
        CallbackArgs(unsafe { std::mem::zeroed() })
    }
}

impl QueueItem {
    pub fn account_metas(&self) -> &[SerializableAccountMeta] {
        &self.callback_accounts_meta[..self.num_accounts_meta as usize]
    }

    pub fn callback_args(&self) -> &[u8] {
        &self.callback_args.0[..self.args_size as usize]
    }

    pub fn callback_discriminator(&self) -> &[u8] {
        &self.callback_discriminator[..self.discriminator_size as usize]
    }
}

#[repr(C, packed)]
#[derive(
    Clone, Copy, Debug, Default, Zeroable, Pod, PartialEq, BorshDeserialize, BorshSerialize,
)]
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
            if self.used_bitmap.0[i] == 0 {
                self.items.0[i] = item;
                self.used_bitmap.0[i] = 1;
                self.item_count += 1;
                return Ok(i);
            }
        }
        Err(EphemeralVrfError::QueueFull.into())
    }

    pub fn remove_item(&mut self, index: usize) -> Result<QueueItem, ProgramError> {
        if index >= MAX_QUEUE_ITEMS || self.used_bitmap.0[index] == 0 {
            return Err(EphemeralVrfError::InvalidQueueIndex.into());
        }

        let item = self.items.0[index];
        self.used_bitmap.0[index] = 0;
        self.item_count -= 1;
        Ok(item)
    }

    pub fn iter_items(&self) -> impl Iterator<Item = &QueueItem> {
        self.items.0.iter().enumerate().filter_map(|(i, item)| {
            if self.used_bitmap.0[i] == 1 {
                Some(item)
            } else {
                None
            }
        })
    }

    pub fn find_item_by_id(&self, id: &[u8; 32]) -> Option<(usize, &QueueItem)> {
        for i in 0..MAX_QUEUE_ITEMS {
            if self.used_bitmap.0[i] == 1 && self.items.0[i].id == *id {
                return Some((i, &self.items.0[i]));
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
