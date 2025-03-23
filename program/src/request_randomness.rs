use ephemeral_vrf_api::prelude::*;
use solana_program::hash::hashv;
use steel::*;

/// Process a request for randomness
///
/// Accounts:
///
/// 0. `[signer]` signer - The account requesting randomness and paying for the transaction
/// 1. `[]` oracle_queue_info - The oracle queue account that will store the randomness request
/// 2. `[]` system_program_info - The system program
/// 3. `[]` slothashes_account_info - The SlotHashes sysvar account
///
/// Requirements:
///
/// - The signer must be a valid signer
/// - The oracle queue must be properly initialized
/// - The request is stored in the oracle queue with a combined hash derived from:
///   - caller_seed
///   - current slot
///   - slot hash
///   - callback discriminator
///   - callback program ID
///
/// 1. Verify the signer
/// 2. Get the current slot and slot hash
/// 3. Create a combined hash from inputs to uniquely identify this request
/// 4. Insert the request into the oracle queue
/// 5. Resize the oracle queue PDA if needed
/// 6. Update the oracle queue data
pub fn process_request_randomness(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args
    let args = RequestRandomness::try_from_bytes(data)?;

    // Load accounts
    let [signer_info, oracle_queue_info, system_program_info, slothashes_account_info] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Verify signer
    signer_info.is_signer()?;

    // Load slot and slothash
    let slothash: [u8; 32] = slothashes_account_info.try_borrow_data()?[16..48]
        .try_into()
        .map_err(|_| ProgramError::UnsupportedSysvar)?;
    let slot = Clock::get()?.slot;

    let combined_hash = hashv(&[
        &args.caller_seed,
        &slot.to_le_bytes(),
        &slothash,
        &args.callback_discriminator,
        &args.callback_program_id.to_bytes(),
    ]);

    let mut oracle_queue =
        QueueAccount::try_from_bytes_with_discriminator(&mut oracle_queue_info.try_borrow_data()?)?;

    oracle_queue.items.insert(
        combined_hash.to_bytes(),
        QueueItem {
            seed: args.caller_seed,
            slot,
            slothash,
            callback_discriminator: args.callback_discriminator,
            callback_program_id: args.callback_program_id,
            callback_accounts_meta: args.callback_accounts_metas,
            callback_args: args.callback_args,
        },
    );

    resize_pda(
        signer_info,
        oracle_queue_info,
        system_program_info,
        oracle_queue.size_with_discriminator(),
    )?;

    {
        let mut oracle_queue_data = oracle_queue_info.try_borrow_mut_data()?;
        let mut oracle_queue_bytes = vec![];
        oracle_queue.to_bytes_with_discriminator(&mut oracle_queue_bytes)?;
        oracle_queue_data.copy_from_slice(&oracle_queue_bytes);
    }

    Ok(())
}
