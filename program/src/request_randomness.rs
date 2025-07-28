use ephemeral_vrf_api::prelude::*;
use ephemeral_vrf_api::ID;
use solana_program::hash::hashv;
use solana_program::program::invoke;
use solana_program::system_instruction;
use solana_program::sysvar::slot_hashes;
use steel::*;

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
pub fn process_request_randomness(
    accounts: &[AccountInfo<'_>],
    data: &[u8],
    high_priority: bool,
) -> ProgramResult {
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

    // Load oracle queue
    let oracle_queue = oracle_queue_info.as_account_mut::<Queue>(&ID)?;

    // Load slot and slothash
    slothashes_account_info.is_sysvar(&slot_hashes::id())?;
    let slothash: [u8; 32] = slothashes_account_info.try_borrow_data()?[16..48]
        .try_into()
        .map_err(|_| ProgramError::UnsupportedSysvar)?;
    let slot = Clock::get()?.slot;
    let time = Clock::get()?.unix_timestamp;
    let idx = oracle_queue.get_insertion_index()? + 1;

    let combined_hash = hashv(&[
        &args.caller_seed,
        &slot.to_le_bytes(),
        &slothash,
        &args.callback_discriminator,
        &args.callback_program_id.to_bytes(),
        &time.to_le_bytes(),
        &idx.to_le_bytes(),
    ]);

    // Check limit for the request
    if args.callback_args.len() > MAX_ARGS_SIZE
        || args.callback_accounts_metas.len() > MAX_ACCOUNTS
        || args.callback_discriminator.len() > 8
    {
        return Err(ProgramError::from(EphemeralVrfError::ArgumentSizeTooLarge));
    }

    let mut callback_accounts_meta = [SerializableAccountMeta::default(); MAX_ACCOUNTS];
    let mut callback_args = [0u8; MAX_ARGS_SIZE];
    let mut callback_discriminator = [0u8; 8];

    callback_accounts_meta[..args.callback_accounts_metas.len()]
        .copy_from_slice(&args.callback_accounts_metas);
    callback_args[..args.callback_args.len()].copy_from_slice(&args.callback_args);
    callback_discriminator[..args.callback_discriminator.len()]
        .copy_from_slice(&args.callback_discriminator);

    let item = QueueItem {
        id: combined_hash.to_bytes(),
        callback_discriminator,
        callback_program_id: args.callback_program_id.to_bytes(),
        callback_accounts_meta,
        callback_args: CallbackArgs(callback_args),
        slot,
        args_size: args.callback_args.len() as u8,
        num_accounts_meta: args.callback_accounts_metas.len() as u8,
        discriminator_size: args.callback_discriminator.len() as u8,
        priority_request: high_priority as u8,
    };

    // Add the item to the queue
    oracle_queue.add_item(item)?;

    // Transfer request cost to the queue PDA
    let cost = if high_priority {
        VRF_HIGH_PRIORITY_LAMPORTS_COST
    } else {
        VRF_LAMPORTS_COST
    };
    invoke(
        &system_instruction::transfer(signer_info.key, oracle_queue_info.key, cost),
        &[
            signer_info.clone(),
            oracle_queue_info.clone(),
            system_program_info.clone(),
        ],
    )?;

    Ok(())
}
