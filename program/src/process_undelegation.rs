use ephemeral_rollups_sdk::cpi::undelegate_account;
use ephemeral_vrf_api::prelude::PdaSeeds;
use steel::*;

/// Process the undelegation of an Oracle queue from the delegation program
///
/// This instruction is a callback from the delegation program to complete the undelegation process
/// for an Oracle queue that was previously delegated. It's called by the delegation program as part
/// of the undelegation flow.
///
/// Accounts:
///
/// 0. `[signer]` The payer for the transaction
/// 1. `[writable]` The Oracle queue account being undelegated
/// 2. `[]` The delegation buffer account
/// 3. `[]` The system program
///
/// Requirements:
///
/// - The payer (account 0) must be a signer.
/// - The Oracle queue (account 1) must have been previously delegated to the delegation program.
/// - The delegation buffer (account 2) must be the correct PDA for this delegation.
///
/// Process:
/// 1. Load and validate the required accounts.
/// 2. Call the delegation program's undelegate_account function to complete the undelegation process.
pub fn process_undelegation(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    let pda_seeds: Vec<Vec<u8>> = PdaSeeds::parse(data)?;

    // Load accounts.
    let [payer, oracle_queue_info, delegation_buffer, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Undelegate
    undelegate_account(
        oracle_queue_info,
        &ephemeral_vrf_api::ID,
        delegation_buffer,
        payer,
        system_program,
        pda_seeds,
    )?;

    Ok(())
}
