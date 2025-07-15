use borsh::{BorshDeserialize, BorshSerialize};
use rkyv::{Archive, Deserialize, Serialize};
use solana_program::pubkey::Pubkey;

/// A wrapper around Pubkey that implements Rkyv traits
/// Instead of trying to serialize the Pubkey directly, we'll serialize its bytes
#[derive(Debug, Clone, Copy, PartialEq, Default, Archive, Serialize, Deserialize)]
#[archive(compare(PartialEq), check_bytes)]
pub struct RkyvPubkey {
    /// The bytes of the Pubkey
    pub bytes: [u8; 32],
}

impl RkyvPubkey {
    /// Create a new RkyvPubkey from a Pubkey
    pub fn new(pubkey: Pubkey) -> Self {
        Self {
            bytes: pubkey.to_bytes(),
        }
    }

    /// Get the Pubkey from this RkyvPubkey
    pub fn pubkey(&self) -> Pubkey {
        Pubkey::new_from_array(self.bytes)
    }

    /// Check if this RkyvPubkey equals a Pubkey
    pub fn equals(&self, other: &Pubkey) -> bool {
        self.bytes == other.to_bytes()
    }
}

impl From<Pubkey> for RkyvPubkey {
    fn from(pubkey: Pubkey) -> Self {
        Self::new(pubkey)
    }
}

impl From<RkyvPubkey> for Pubkey {
    fn from(wrapper: RkyvPubkey) -> Self {
        wrapper.pubkey()
    }
}

// Implement PartialEq between [u8; 32] and RkyvPubkey
impl PartialEq<RkyvPubkey> for [u8; 32] {
    fn eq(&self, other: &RkyvPubkey) -> bool {
        self == &other.bytes
    }
}

impl PartialEq<[u8; 32]> for RkyvPubkey {
    fn eq(&self, other: &[u8; 32]) -> bool {
        &self.bytes == other
    }
}

// Implement BorshSerialize for RkyvPubkey
impl BorshSerialize for RkyvPubkey {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        // Convert to Pubkey and serialize
        let pubkey = self.pubkey();
        pubkey.serialize(writer)
    }
}

// Implement BorshDeserialize for RkyvPubkey
impl BorshDeserialize for RkyvPubkey {
    fn deserialize(buf: &mut &[u8]) -> std::io::Result<Self> {
        let pubkey = <Pubkey as BorshDeserialize>::deserialize(buf)?;
        Ok(Self::new(pubkey))
    }

    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let pubkey = <Pubkey as BorshDeserialize>::deserialize_reader(reader)?;
        Ok(Self::new(pubkey))
    }
}
