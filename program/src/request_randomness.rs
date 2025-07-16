use ephemeral_vrf_api::prelude::*;
use solana_program::hash::hashv;
use steel::*;
use ephemeral_vrf_api::ID;

/// Process a request for randomness
///
/// Accounts:
///
/// 0. `[signer]` signer - The account requesting randomness and paying for the transaction
/// 1. `[signer]` program_identity_info - The identity PDA of the calling program
/// 2. `[]` oracle_queue_info - The oracle queue account that will store the randomness request
/// 3. `[]` system_program_info - The system program
/// 4. `[]` slothashes_account_info - The SlotHashes sysvar account
///
/// Requirements:
///
/// - The signer must be a valid signer
/// - The program identity must be a valid signer and derived from the vrf-macro program ID
/// - The oracle queue must be properly initialized
/// - The request is stored in the oracle queue with a combined hash derived from:
///   - caller_seed
///   - current slot
///   - slot hash
///   - vrf-macro discriminator
///   - vrf-macro program ID
///
/// 1. Verify the signer
/// 2. Verify the program identity
/// 3. Get the current slot and slot hash
/// 4. Create a combined hash from inputs to uniquely identify this request
/// 5. Insert the request into the oracle queue
/// 6. Resize the oracle queue PDA if needed
/// 7. Update the oracle queue data
pub fn process_request_randomness(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    let args = RequestRandomness::try_from_bytes(data)?;

    // Load accounts
    let [signer_info, program_identity_info, oracle_queue_info, system_program_info, slothashes_account_info] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Verify signer
    signer_info.is_signer()?;

    // Verify caller program
    program_identity_info
        .has_seeds(&[IDENTITY], &args.callback_program_id)?
        .is_signer()?;

    // Load slot and slothash
    let slothash: [u8; 32] = slothashes_account_info.try_borrow_data()?[16..48]
        .try_into()
        .map_err(|_| ProgramError::UnsupportedSysvar)?;
    let slot = Clock::get()?.slot;
    let time = Clock::get()?.unix_timestamp;

    let combined_hash = hashv(&[
        &args.caller_seed,
        &slot.to_le_bytes(),
        &slothash,
        &args.callback_discriminator,
        &args.callback_program_id.to_bytes(),
        &time.to_le_bytes(),
    ]);

    let mut oracle_queue = oracle_queue_info.as_account_mut::<Queue>(&ID)?;

    // Check if the callback args are within the size limit
    if args.callback_args.len() > MAX_ARGS_SIZE {
        return Err(ProgramError::InvalidArgument);
    }

    // Check if the number of accounts is within the limit
    if args.callback_accounts_metas.len() > MAX_ACCOUNTS {
        return Err(ProgramError::InvalidArgument);
    }

    // Create the QueueItem
    let item = QueueItem {
        id: combined_hash.to_bytes(),
        callback_discriminator: args.callback_discriminator.as_slice().try_into()?,
        callback_program_id: args.callback_program_id.into(),
        callback_accounts_meta: args.callback_accounts_metas.as_slice().try_into()?,
        callback_args: args.callback_args.as_slice().try_into()?,
        slot,
    };

    // Add the item to the queue
    oracle_queue.add_item(item)?;

    Ok(())
}
