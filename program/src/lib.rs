mod add;
mod initialize;

use add::*;
use initialize::*;

use ephemeral_vrf_api::prelude::*;
use steel::*;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let (ix, data) = parse_instruction(&ephemeral_vrf_api::ID, program_id, data)?;

    match ix {
        EphemeralVrfInstruction::Initialize => process_initialize(accounts, data)?,
        EphemeralVrfInstruction::AddOracle => process_add(accounts, data)?,
    }

    Ok(())
}

entrypoint!(process_instruction);
