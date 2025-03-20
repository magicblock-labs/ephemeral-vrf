use solana_curve25519::ristretto::PodRistrettoPoint;
use solana_curve25519::scalar::PodScalar;
use solana_program::sysvar::slot_hashes;
use steel::*;

use crate::prelude::*;

pub fn initialize(signer: Pubkey) -> Instruction {
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(oracles_pda().0, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: Initialize {}.to_bytes(),
    }
}

pub fn add_oracle(signer: Pubkey, identity: Pubkey, oracle_pubkey: Pubkey) -> Instruction {
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(oracles_pda().0, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: ModifyOracle {
            identity,
            oracle_pubkey,
            operation: 0,
        }
        .to_bytes(),
    }
}

pub fn remove_oracle(signer: Pubkey, identity: Pubkey) -> Instruction {
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(oracles_pda().0, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: ModifyOracle {
            identity,
            oracle_pubkey: Pubkey::default(),
            operation: 1,
        }
        .to_bytes(),
    }
}

pub fn initialize_oracle_queue(signer: Pubkey, identity: Pubkey, index: u8) -> Instruction {
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new_readonly(identity, false),
            AccountMeta::new_readonly(oracles_pda().0, false),
            AccountMeta::new(oracle_queue_pda(identity, index).0, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: InitializeOracleQueue { index }.to_bytes(),
    }
}

pub fn request_randomness(
    signer: Pubkey,
    oracle_queue: Pubkey,
    callback_program_id: Pubkey,
    callback_discriminator: [u8; 8],
    accounts_metas: Option<Vec<SerializableAccountMeta>>,
    caller_seed: Option<[u8; 32]>,
) -> Instruction {
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(oracle_queue, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(slot_hashes::ID, false),
        ],
        data: RequestRandomness {
            caller_seed: caller_seed.unwrap_or(Pubkey::new_unique().to_bytes()),
            callback_program_id,
            callback_discriminator,
            callback_accounts_metas: accounts_metas.unwrap_or(vec![]),
        }
        .to_bytes(),
    }
}

pub fn provide_randomness(
    oracle_identity: Pubkey,
    oracle_queue: Pubkey,
    input: [u8; 32],
    randomness: [u8; 32],
    output: PodRistrettoPoint,
    commitment_base_compressed: PodRistrettoPoint,
    commitment_hash_compressed: PodRistrettoPoint,
    s: PodScalar,
) -> Instruction {
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(oracle_identity, true),
            AccountMeta::new(oracles_pda().0, false),
            AccountMeta::new(oracle_queue, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: ProvideRandomness {
            oracle_identity,
            input,
            output,
            commitment_base_compressed,
            commitment_hash_compressed,
            s,
            randomness,
        }
        .to_bytes(),
    }
}
