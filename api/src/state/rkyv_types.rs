use rkyv::{Archive, Deserialize, Serialize};
use solana_program::pubkey::Pubkey;
use std::ops::{Deref, DerefMut};

/// A wrapper around Pubkey that implements rkyv's Archive, Deserialize, and Serialize traits
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct RkyvPubkey(pub Pubkey);

impl Deref for RkyvPubkey {
    type Target = Pubkey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RkyvPubkey {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Pubkey> for RkyvPubkey {
    fn from(pubkey: Pubkey) -> Self {
        RkyvPubkey(pubkey)
    }
}

impl From<RkyvPubkey> for Pubkey {
    fn from(pubkey: RkyvPubkey) -> Self {
        pubkey.0
    }
}

impl Default for RkyvPubkey {
    fn default() -> Self {
        RkyvPubkey(Pubkey::default())
    }
}

// Implement Archive for RkyvPubkey
impl Archive for RkyvPubkey {
    type Archived = [u8; 32];
    type Resolver = ();

    unsafe fn resolve(&self, _: (), out: *mut Self::Archived) {
        let bytes = self.0.to_bytes();
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out as *mut u8, 32);
    }
}

// Implement Serialize for RkyvPubkey
impl Serialize<()> for RkyvPubkey {
    fn serialize(&self, _: &mut (), writer: &mut std::io::Write) -> Result<(), std::io::Error> {
        writer.write_all(&self.0.to_bytes())
    }
}

// Implement Deserialize for RkyvPubkey
impl<D: rkyv::Fallible> Deserialize<RkyvPubkey, D> for [u8; 32] {
    fn deserialize(&self, _: &mut D) -> Result<RkyvPubkey, D::Error> {
        let pubkey = Pubkey::new_from_array(*self);
        Ok(RkyvPubkey(pubkey))
    }
}