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
/// 1. `[writable]` The Oracle queue account to be closed
///
/// Requirements:
///
/// - The Oracle (account 0) must be a signer.
/// - The Oracle queue (account 1) must be a valid PDA with seeds [QUEUE, oracle.key, index].
/// - The queue account must be owned by the ephemeral VRF program.
/// - The queue must be empty (no unprocessed requests).
///
/// Process:
///
/// 1. Parse the instruction data and extract arguments (CloseOracleQueue).
/// 2. Verify the Oracle account is a signer.
/// 3. Validate the Oracle queue account PDA seeds with the provided index.
/// 4. Ensure the queue is empty.
/// 5. Close the Oracle queue account and transfer lamports to the Oracle.
pub fn process_close_oracle_queue(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = CloseOracleQueue::try_from_bytes(data)?;

    // Load accounts.
    let [oracle_info, oracle_queue_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    oracle_info.is_signer()?;

    oracle_queue_info
        .is_writable()?
        .has_owner(&ephemeral_vrf_api::ID)?
        .has_seeds(
            &[QUEUE, oracle_info.key.to_bytes().as_ref(), &[args.index]],
            &ephemeral_vrf_api::ID,
        )?;

    // Ensure the queue has no pending items before closing.
    {
        let queue = oracle_queue_info.as_account::<Queue>(&ephemeral_vrf_api::ID)?;
        if !queue.is_empty() {
            return Err(EphemeralVrfError::QueueNotEmpty.into());
        }
    }

    close_account(oracle_queue_info, oracle_info)?;

    Ok(())
}
