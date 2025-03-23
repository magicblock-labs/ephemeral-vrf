use crate::prelude::SerializableAccountMeta;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_curve25519::ristretto::PodRistrettoPoint;
use solana_curve25519::scalar::PodScalar;
use steel::*;
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum EphemeralVrfInstruction {
    Initialize = 0,
    ModifyOracle = 1,
    InitializeOracleQueue = 2,
    RequestRandomness = 3,
    ProvideRandomness = 4,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Initialize {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ModifyOracle {
    pub identity: Pubkey,
    pub oracle_pubkey: PodRistrettoPoint,
    pub operation: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct InitializeOracleQueue {
    pub index: u8,
}

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Default)]
pub struct RequestRandomness {
    pub caller_seed: [u8; 32],
    pub callback_program_id: Pubkey,
    pub callback_discriminator: Vec<u8>,
    pub callback_accounts_metas: Vec<SerializableAccountMeta>,
    pub callback_args: Vec<u8>,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ProvideRandomness {
    pub oracle_identity: Pubkey,
    pub input: [u8; 32],
    pub output: PodRistrettoPoint,
    pub commitment_base_compressed: PodRistrettoPoint,
    pub commitment_hash_compressed: PodRistrettoPoint,
    pub s: PodScalar,
}

instruction!(EphemeralVrfInstruction, Initialize);
instruction!(EphemeralVrfInstruction, ModifyOracle);
instruction!(EphemeralVrfInstruction, InitializeOracleQueue);
instruction!(EphemeralVrfInstruction, ProvideRandomness);

impl RequestRandomness {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![
            EphemeralVrfInstruction::RequestRandomness as u8,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ];
        self.serialize(&mut bytes).unwrap();
        bytes
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self, std::io::Error> {
        Self::deserialize(&mut bytes[7..].as_ref())
    }
}
