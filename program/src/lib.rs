#![allow(unexpected_cfgs)]
mod initialize;
mod initialize_oracle_queue;
mod modify_oracles;
mod provide_randomness;
mod request_randomness;

use initialize::*;
use initialize_oracle_queue::*;
use modify_oracles::*;
use provide_randomness::process_provide_randomness;
use request_randomness::*;

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
    }

    Ok(())
}
entrypoint!(process_instruction);
