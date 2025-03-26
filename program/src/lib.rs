#![allow(unexpected_cfgs)]
mod delegate_oracle_queue;
mod initialize;
mod initialize_oracle_queue;
mod modify_oracles;
mod process_undelegation;
mod provide_randomness;
mod request_randomness;
mod undelegate_oracle_queue;
mod verify;

use delegate_oracle_queue::*;
use initialize::*;
use initialize_oracle_queue::*;
use modify_oracles::*;
use process_undelegation::*;
use provide_randomness::*;
use request_randomness::*;
use undelegate_oracle_queue::*;

use ephemeral_vrf_api::prelude::*;
use steel::*;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let (ix, data) = parse_instruction(&ephemeral_vrf_api::ID, program_id, data)?;
    log(format!("Instruction: {:?}", ix));
    match ix {
        EphemeralVrfInstruction::Initialize => process_initialize(accounts, data)?,
        EphemeralVrfInstruction::ModifyOracle => process_modify_oracles(accounts, data)?,
        EphemeralVrfInstruction::InitializeOracleQueue => {
            process_initialize_oracle_queue(accounts, data)?
        }
        EphemeralVrfInstruction::RequestRandomness => process_request_randomness(accounts, data)?,
        EphemeralVrfInstruction::ProvideRandomness => process_provide_randomness(accounts, data)?,
        EphemeralVrfInstruction::DelegateOracleQueue => {
            process_delegate_oracle_queue(accounts, data)?
        }
        EphemeralVrfInstruction::UndelegateOracleQueue => {
            process_undelegate_oracle_queue(accounts, data)?
        }
        EphemeralVrfInstruction::ProcessUndelegation => process_undelegation(accounts, data)?,
    }

    Ok(())
}
entrypoint!(process_instruction);
