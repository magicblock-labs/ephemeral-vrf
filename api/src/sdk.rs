use solana_curve25519::ristretto::PodRistrettoPoint;
use solana_curve25519::scalar::PodScalar;
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

pub fn add_oracle(signer: Pubkey, identity: Pubkey, oracle_pubkey: [u8; 32]) -> Instruction {
    let oracle_pubkey = PodRistrettoPoint(oracle_pubkey);
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(oracles_pda().0, false),
            AccountMeta::new(oracle_data_pda(&identity).0, false),
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
            AccountMeta::new(oracle_data_pda(&identity).0, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: ModifyOracle {
            identity,
            oracle_pubkey: PodRistrettoPoint::default(),
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
            AccountMeta::new_readonly(oracle_data_pda(&identity).0, false),
            AccountMeta::new(oracle_queue_pda(&identity, index).0, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: InitializeOracleQueue { index }.to_bytes(),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn provide_randomness(
    oracle_identity: Pubkey,
    oracle_queue: Pubkey,
    callback_program_id: Pubkey,
    rnd_seed: [u8; 32],
    output: PodRistrettoPoint,
    commitment_base_compressed: PodRistrettoPoint,
    commitment_hash_compressed: PodRistrettoPoint,
    s: PodScalar,
) -> Instruction {
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(oracle_identity, true),
            AccountMeta::new_readonly(program_identity_pda().0, false),
            AccountMeta::new(oracle_data_pda(&oracle_identity).0, false),
            AccountMeta::new(oracle_queue, false),
            AccountMeta::new_readonly(callback_program_id, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: ProvideRandomness {
            oracle_identity,
            input: rnd_seed,
            output,
            commitment_base_compressed,
            commitment_hash_compressed,
            s,
        }
        .to_bytes(),
    }
}
