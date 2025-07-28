use crate::prelude::*;
use crate::ID;
use ephemeral_rollups_sdk::consts::{DELEGATION_PROGRAM_ID, MAGIC_CONTEXT_ID, MAGIC_PROGRAM_ID};
use ephemeral_rollups_sdk::pda::{
    delegate_buffer_pda_from_delegated_account_and_owner_program,
    delegation_metadata_pda_from_delegated_account, delegation_record_pda_from_delegated_account,
};
use solana_curve25519::ristretto::PodRistrettoPoint;
use solana_curve25519::scalar::PodScalar;
use solana_program::bpf_loader_upgradeable;
use steel::*;

pub fn initialize(signer: Pubkey) -> Instruction {
    Instruction {
        program_id: ID,
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
    let vrf_program_data =
        Pubkey::find_program_address(&[crate::ID.as_ref()], &bpf_loader_upgradeable::id()).0;
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(oracles_pda().0, false),
            AccountMeta::new(oracle_data_pda(&identity).0, false),
            AccountMeta::new_readonly(vrf_program_data, false),
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
    let vrf_program_data =
        Pubkey::find_program_address(&[crate::ID.as_ref()], &bpf_loader_upgradeable::id()).0;
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(oracles_pda().0, false),
            AccountMeta::new(oracle_data_pda(&identity).0, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(vrf_program_data, false),
        ],
        data: ModifyOracle {
            identity,
            oracle_pubkey: PodRistrettoPoint::default(),
            operation: 1,
        }
        .to_bytes(),
    }
}

/// Returns a list of instructions to initialize an oracle queue. The initialize_oracle_queue is
/// repeated to alloc chunks of 10240 bytes, which is the maximum per instruction.
/// Should still be run in a single transaction.
pub fn initialize_oracle_queue(signer: Pubkey, identity: Pubkey, index: u8) -> Vec<Instruction> {
    let inits = Queue::size_with_discriminator().div_ceil(10240);
    let mut ixs = Vec::with_capacity(inits);
    for _ in 0..inits {
        ixs.push(Instruction {
            program_id: ID,
            accounts: vec![
                AccountMeta::new(signer, true),
                AccountMeta::new_readonly(identity, true),
                AccountMeta::new_readonly(oracle_data_pda(&identity).0, false),
                AccountMeta::new(oracle_queue_pda(&identity, index).0, false),
                AccountMeta::new_readonly(system_program::ID, false),
            ],
            data: InitializeOracleQueue { index }.to_bytes(),
        })
    }
    ixs
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
            AccountMeta::new_readonly(oracle_data_pda(&oracle_identity).0, false),
            AccountMeta::new(oracle_queue, false),
            AccountMeta::new_readonly(callback_program_id, false),
        ],
        data: ProvideRandomness {
            oracle_identity,
            input: rnd_seed,
            output,
            commitment_base_compressed,
            commitment_hash_compressed,
            scalar: s,
        }
        .to_bytes(),
    }
}

pub fn delegate_oracle_queue(signer: Pubkey, queue: Pubkey, index: u8) -> Instruction {
    let buffer = delegate_buffer_pda_from_delegated_account_and_owner_program(&queue, &crate::ID);
    let delegation_record = delegation_record_pda_from_delegated_account(&queue);
    let delegation_metadata = delegation_metadata_pda_from_delegated_account(&queue);
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(queue, false),
            AccountMeta::new(buffer, false),
            AccountMeta::new(delegation_record, false),
            AccountMeta::new(delegation_metadata, false),
            AccountMeta::new_readonly(DELEGATION_PROGRAM_ID, false),
            AccountMeta::new_readonly(crate::ID, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: DelegateOracleQueue { index }.to_bytes(),
    }
}

pub fn undelegate_oracle_queue(signer: Pubkey, queue: Pubkey, index: u8) -> Instruction {
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(queue, false),
            AccountMeta::new(MAGIC_CONTEXT_ID, false),
            AccountMeta::new_readonly(MAGIC_PROGRAM_ID, false),
        ],
        data: UndelegateOracleQueue { index }.to_bytes(),
    }
}

pub fn close_oracle_queue(identity: Pubkey, index: u8) -> Instruction {
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new_readonly(identity, false),
            AccountMeta::new_readonly(oracle_data_pda(&identity).0, false),
            AccountMeta::new(oracle_queue_pda(&identity, index).0, false),
        ],
        data: CloseOracleQueue { index }.to_bytes(),
    }
}
