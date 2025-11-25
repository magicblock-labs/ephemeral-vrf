use core::mem::size_of;
use crate::prelude::{AccountDiscriminator, EphemeralVrfError};
use borsh::{BorshDeserialize, BorshSerialize};
use steel::{account, trace, AccountMeta, Pod, ProgramError, Pubkey, Zeroable};

/// Header of the queue account (fixed size, lives at the start of the account
/// after the 8-byte discriminator).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct Queue {
    /// Number of active (used) items.
    pub item_count: u32,
    /// Cursor in bytes from the start of the account data (after discriminator)
    /// pointing to the next free byte in the variable region.
    pub cursor: u32,
    /// Logical index or shard id of the queue.
    pub index: u8,
    pub _padding: [u8; 3],
}

/// Single queue entry. This is written into the variable region and
/// references its own metas/args by byte offsets.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod, PartialEq)]
pub struct QueueItem {
    pub slot: u64,
    pub id: [u8; 32],
    pub callback_program_id: [u8; 32],
    pub callback_discriminator_offset: u32,
    pub metas_offset: u32,
    pub args_offset: u32,
    pub callback_discriminator_len: u16,
    pub metas_len: u16, // number of SerializableAccountMeta
    pub args_len: u16, // number of bytes
    pub priority_request: u8,
    pub used: u8, // Flag: 1 = used, 0 = free (logically removed)
    pub _padding: [u8; 4],
}

impl QueueItem {
    pub fn callback_discriminator<'a>(&self, acc: &'a [u8]) -> &'a [u8] {
        let start = self.callback_discriminator_offset as usize;
        let end = start + self.callback_discriminator_len as usize;
        &acc[start..end]
    }

    pub fn account_metas<'a>(&self, acc: &'a [u8]) -> &'a [SerializableAccountMeta] {
        let start = self.metas_offset as usize;
        let count = self.metas_len as usize;
        let byte_len = count * size_of::<SerializableAccountMeta>();
        let end = start + byte_len;

        let bytes = &acc[start..end];

        unsafe {
            core::slice::from_raw_parts(
                bytes.as_ptr() as *const SerializableAccountMeta,
                count,
            )
        }
    }

    pub fn callback_args<'a>(&self, acc: &'a [u8]) -> &'a [u8] {
        let start = self.args_offset as usize;
        let end = start + self.args_len as usize;
        &acc[start..end]
    }
}

/// Serializable meta, Borsh compatible and Pod/Zeroable for zero copy.
#[repr(C)]
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    Zeroable,
    Pod,
    PartialEq,
    BorshDeserialize,
    BorshSerialize,
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

/// View over a queue account: header + variable region in the same account data.
pub struct QueueAccount<'a> {
    /// Header, mapped on the first bytes after discriminator.
    pub header: &'a mut Queue,
    /// Full account data including header and variable data.
    pub acc: &'a mut [u8],
}

impl<'a> QueueAccount<'a> {
    /// Load from an account data slice (without discriminator).
    /// Caller is responsible for stripping the 8-byte discriminator if present.
    pub fn load(acc: &'a mut [u8]) -> Result<Self, ProgramError> {
        let header_size = size_of::<Queue>();
        if acc.len() < header_size {
            return Err(ProgramError::InvalidAccountData);
        }

        let (header_bytes, _rest) = acc.split_at_mut(header_size);
        let header: &mut Queue = unsafe {
            &mut *(header_bytes.as_mut_ptr() as *mut Queue)
        };

        // If this is a freshly created account, cursor 0 means "no data yet":
        if header.cursor == 0 {
            header.cursor = header_size as u32;
        }

        Ok(Self { header, acc })
    }

    /// Internal helper to write bytes into the variable region.
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<u32, ProgramError> {
        let start = self.header.cursor as usize;
        let end = start + bytes.len();

        if end > self.acc.len() {
            return Err(ProgramError::AccountDataTooSmall);
        }

        self.acc[start..end].copy_from_slice(bytes);
        self.header.cursor = end as u32;

        Ok(start as u32)
    }

    /// Append a new item to the queue.
    pub fn add_item(
        &mut self,
        base_item: &QueueItem,
        discriminator: &[u8],
        metas: &[SerializableAccountMeta],
        args: &[u8],
    ) -> Result<usize, ProgramError> {
        // Write discriminator
        let disc_off = self.write_bytes(discriminator)?;
        let disc_len = discriminator.len() as u16;

        // Write metas
        let metas_bytes_len = metas.len() * size_of::<SerializableAccountMeta>();
        let metas_bytes = unsafe {
            core::slice::from_raw_parts(
                metas.as_ptr() as *const u8,
                metas_bytes_len,
            )
        };
        let metas_off = self.write_bytes(metas_bytes)?;
        let metas_len = metas.len() as u16;

        // Write args
        let args_off = self.write_bytes(args)?;
        let args_len = args.len() as u16;

        // Build final item
        let mut item = *base_item;
        item.callback_discriminator_offset = disc_off;
        item.callback_discriminator_len = disc_len;
        item.metas_offset = metas_off;
        item.metas_len = metas_len;
        item.args_offset = args_off;
        item.args_len = args_len;
        item.used = 1;

        let item_bytes = unsafe {
            core::slice::from_raw_parts(
                &item as *const QueueItem as *const u8,
                size_of::<QueueItem>(),
            )
        };

        self.write_bytes(item_bytes)?;

        // Item index is logical position among used items.
        let logical_index = self.header.item_count as usize;
        self.header.item_count = self.header.item_count.saturating_add(1);
        Ok(logical_index)
    }

    /// Iterate over all used items.
    pub fn iter_items(&self) -> impl Iterator<Item = QueueItem> + '_ {
        let header_size = size_of::<Queue>();
        let mut cursor = header_size;

        let mut out = Vec::new();

        while cursor + size_of::<QueueItem>() <= self.acc.len() {
            let bytes = &self.acc[cursor..cursor + size_of::<QueueItem>()];
            let item: &QueueItem = unsafe {
                &*(bytes.as_ptr() as *const QueueItem)
            };

            if item.used == 1 {
                out.push(*item);
            }

            cursor += size_of::<QueueItem>();
        }

        out.into_iter()
    }

    /// Find the nth used item (logical index) and return its value.
    pub fn get_item_by_index(&self, index: usize) -> Option<QueueItem> {
        let mut current = 0usize;

        let header_size = size_of::<Queue>();
        let mut cursor = header_size;

        while cursor + size_of::<QueueItem>() <= self.acc.len() {
            let bytes = &self.acc[cursor..cursor + size_of::<QueueItem>()];
            let item: &QueueItem = unsafe {
                &*(bytes.as_ptr() as *const QueueItem)
            };

            if item.used == 1 {
                if current == index {
                    return Some(*item);
                }
                current += 1;
            }

            cursor += size_of::<QueueItem>();
        }

        None
    }

    /// Remove the nth used item (logical index).
    pub fn remove_item(&mut self, index: usize) -> Result<QueueItem, ProgramError> {
        let mut current = 0usize;

        let header_size = size_of::<Queue>();
        let mut cursor = header_size;

        while cursor + size_of::<QueueItem>() <= self.acc.len() {
            let bytes = &mut self.acc[cursor..cursor + size_of::<QueueItem>()];
            let item: &mut QueueItem = unsafe {
                &mut *(bytes.as_mut_ptr() as *mut QueueItem)
            };

            if item.used == 1 {
                if current == index {
                    item.used = 0;
                    self.header.item_count = self.header.item_count.saturating_sub(1);
                    return Ok(*item);
                }
                current += 1;
            }

            cursor += size_of::<QueueItem>();
        }

        Err(EphemeralVrfError::InvalidQueueIndex.into())
    }

    /// Find first used item by id, returning its logical index and value.
    pub fn find_item_by_id(&self, id: &[u8; 32]) -> Option<(usize, QueueItem)> {
        let mut current = 0usize;

        let header_size = size_of::<Queue>();
        let mut cursor = header_size;

        while cursor + size_of::<QueueItem>() <= self.acc.len() {
            let bytes = &self.acc[cursor..cursor + size_of::<QueueItem>()];
            let item: &QueueItem = unsafe {
                &*(bytes.as_ptr() as *const QueueItem)
            };

            if item.used == 1 {
                if &item.id == id {
                    return Some((current, *item));
                }
                current += 1;
            }

            cursor += size_of::<QueueItem>();
        }

        None
    }

    pub fn is_empty(&self) -> bool {
        self.header.item_count == 0
    }

    pub fn len(&self) -> usize {
        self.header.item_count as usize
    }
}

impl Queue {
    /// Minimum size: discriminator (8 bytes) + header.
    /// The actual account can be larger, this is just the lower bound.
    pub fn size_with_discriminator() -> usize {
        8 + size_of::<Queue>()
    }
}

account!(AccountDiscriminator, Queue);
