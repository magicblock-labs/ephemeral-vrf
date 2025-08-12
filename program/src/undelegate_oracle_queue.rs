use ephemeral_rollups_sdk::ephem::commit_and_undelegate_accounts;
use ephemeral_vrf_api::prelude::*;
use steel::*;

/// Process the undelegation of an Oracle queue from the delegation program
///
/// This instruction allows an authority to undelegate an Oracle queue that was previously
/// delegated to the delegation program, removing the ability for other programs to interact
/// with the queue through the delegation mechanism.
///
/// Accounts:
///
/// 0. `[signer]` The authority that controls the Oracle queue
/// 1. `[writable]` The Oracle queue account to be undelegated
/// 2. `[]` The Magic context account
/// 3. `[]` The Magic program
///
/// Requirements:
///
/// - The authority (account 0) must be a signer.
/// - The Oracle queue (account 1) must be a valid PDA with seeds [QUEUE, authority.key, index].
/// - The Oracle queue must have been previously delegated to the delegation program.
///
/// Process:
/// 1. Parse the instruction data and extract arguments (UndelegateOracleQueue).
/// 2. Verify the authority is a signer.
/// 3. Verify the Oracle queue account has the correct PDA structure.
/// 4. Call the delegation program to commit and undelegate the Oracle queue account.
pub fn process_undelegate_oracle_queue(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    let args = UndelegateOracleQueue::try_from_bytes(data)?;

    // Load accounts.
    let [authority_info, oracle_queue_info, magic_context, magic_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Checks
    authority_info.is_signer()?;
    magic_context.has_address(&MAGIC_CONTEXT_ID)?;
    magic_program.has_address(&MAGIC_PROGRAM_ID)?;
    let pda_seeds: &[&[u8]] = &[QUEUE, &authority_info.key.to_bytes(), &[args.index]];
    oracle_queue_info
        .is_writable()?
        .has_seeds(pda_seeds, &ephemeral_vrf_api::ID)?;

    // Undelegate
    commit_and_undelegate_accounts(
        authority_info,
        vec![oracle_queue_info],
        magic_context,
        magic_program,
    )?;

    Ok(())
}
