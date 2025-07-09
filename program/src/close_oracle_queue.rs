use ephemeral_vrf_api::prelude::*;
use steel::*;

/// Process the closing of an Oracle queue account
///
/// This instruction allows an Oracle to close one of their queue accounts,
/// reclaiming the rent lamports back to their account.
///
/// Accounts:
///
/// 0. `[signer]` The Oracle account that owns the queue
/// 1. `[]` The Oracle data account (PDA validation)
/// 2. `[writable]` The Oracle queue account to be closed
///
/// Requirements:
///
/// - The Oracle (account 0) must be a signer.
/// - The Oracle data account (account 1) must be a valid PDA with seeds [ORACLE_DATA, oracle.key].
/// - The Oracle queue (account 2) must be a valid PDA with seeds [QUEUE, oracle.key, index].
/// - All accounts must be owned by the ephemeral VRF program.
///
/// Process:
///
/// 1. Parse the instruction data and extract arguments (CloseOracleQueue).
/// 2. Verify the Oracle account is a signer.
/// 3. Validate the Oracle data account PDA seeds.
/// 4. Validate the Oracle queue account PDA seeds with the provided index.
/// 5. Close the Oracle queue account and transfer lamports to the Oracle.
pub fn process_close_oracle_queue(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = CloseOracleQueue::try_from_bytes(data)?;

    // Load accounts.
    let [oracle_info, oracle_data_info, oracle_queue_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    oracle_info.is_signer()?;

    oracle_data_info
        .has_owner(&ephemeral_vrf_api::ID)?
        .has_seeds(
            &[ORACLE_DATA, oracle_info.key.to_bytes().as_ref()],
            &ephemeral_vrf_api::ID,
        )?;

    oracle_queue_info
        .is_writable()?
        .has_owner(&ephemeral_vrf_api::ID)?
        .has_seeds(
            &[QUEUE, oracle_info.key.to_bytes().as_ref(), &[args.index]],
            &ephemeral_vrf_api::ID,
        )?;

    close_account(oracle_queue_info, oracle_info)?;

    Ok(())
}
