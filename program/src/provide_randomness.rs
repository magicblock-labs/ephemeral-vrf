use crate::verify::verify_vrf;
use ephemeral_vrf_api::prelude::*;
use solana_program::hash::hash;
use steel::*;

/// Process the provide randomness instruction which verifies VRF proof and executes vrf-macro
///
/// Accounts:
///
/// 0. `[signer]` signer - The oracle signer providing randomness
/// 1. `[]` program_identity_info - Used to allow the vrf-macro program to verify the identity of the oracle program
/// 1. `[]` oracle_data_info - Oracle data account associated with the signer
/// 2. `[writable]` oracle_queue_info - Queue storing randomness requests
/// 3. `[]` callback_program_info - Program to call with the randomness
/// 4. `[]` system_program_info - System program for resizing accounts
/// 5. `[varies]` remaining_accounts - Accounts needed for the vrf-macro
///
/// Requirements:
///
/// - Signer must be a registered oracle with valid VRF keypair
/// - VRF proof must be valid for the given input and output
/// - Request must exist in the oracle queue
/// - Oracle signer must not be included in vrf-macro accounts
///
/// 1. Verify the oracle signer and load oracle data
/// 2. Verify the VRF proof
/// 3. Remove the request from the queue
/// 4. Invoke the vrf-macro with the randomness
pub fn process_provide_randomness(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args
    let args = ProvideRandomness::try_from_bytes(data)?;

    // Load accounts
    let (
        [oracle_info, program_identity_info, oracle_data_info, oracle_queue_info, callback_program_info, system_program_info],
        remaining_accounts,
    ) = accounts.split_at(6)
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Verify signer
    oracle_info.is_signer()?;

    // Load oracle data
    oracle_data_info.has_seeds(
        &[ORACLE_DATA, oracle_info.key.to_bytes().as_ref()],
        &ephemeral_vrf_api::ID,
    )?;
    let oracle_data = oracle_data_info.as_account::<Oracle>(&ephemeral_vrf_api::ID)?;

    let output = &args.output;
    let commitment_base_compressed = &args.commitment_base_compressed;
    let commitment_hash_compressed = &args.commitment_hash_compressed;
    let s = &args.s;

    // Verify proof
    let verified = verify_vrf(
        &oracle_data.vrf_pubkey,
        &args.input,
        output,
        (commitment_base_compressed, commitment_hash_compressed, s),
    );
    if !verified {
        return Err(EphemeralVrfError::InvalidProof.into());
    }

    // Load and remove oracle item from the queue
    let mut oracle_queue =
        QueueAccount::try_from_bytes_with_discriminator(&oracle_queue_info.try_borrow_data()?)?;
    let item = oracle_queue
        .items
        .get(&args.input)
        .ok_or(EphemeralVrfError::RandomnessRequestNotFound)?
        .clone();
    oracle_queue.items.remove(&args.input);

    let mut oracle_queue_bytes = vec![];
    oracle_queue.to_bytes_with_discriminator(&mut oracle_queue_bytes)?;

    // Resize and serialize oracle queue
    resize_pda(
        oracle_info,
        oracle_queue_info,
        system_program_info,
        oracle_queue_bytes.len(),
    )?;
    let mut oracle_queue_data = oracle_queue_info.try_borrow_mut_data()?;
    oracle_queue_data.copy_from_slice(&oracle_queue_bytes);

    // Don't callback if the request is older than 1 hour and just remove the request
    if Clock::get()?.slot - item.slot > 3 * 60 * 60 {
        return Ok(());
    }

    // Check that the oracle signer is not in the vrf-macro accounts
    if item
        .callback_accounts_meta
        .iter()
        .any(|acc| acc.pubkey == *oracle_info.key)
    {
        return Err(EphemeralVrfError::InvalidCallbackAccounts.into());
    }

    // Invoke vrf-macro with randomness
    callback_program_info.has_address(&item.callback_program_id)?;
    let mut accounts_metas = vec![AccountMeta {
        pubkey: *program_identity_info.key,
        is_signer: true,
        is_writable: false,
    }];
    accounts_metas.extend(item.callback_accounts_meta.iter().map(|acc| AccountMeta {
        pubkey: acc.pubkey,
        is_signer: acc.is_signer,
        is_writable: acc.is_writable,
    }));
    let mut callback_data = Vec::with_capacity(
        item.callback_discriminator.len() + output.0.len() + item.callback_args.len(),
    );
    callback_data.extend_from_slice(&item.callback_discriminator);
    let rdn = hash(&output.0);
    callback_data.extend_from_slice(rdn.to_bytes().as_ref());
    callback_data.extend_from_slice(&item.callback_args);

    let ix = Instruction {
        program_id: item.callback_program_id,
        accounts: accounts_metas,
        data: callback_data,
    };
    let mut all_accounts = vec![callback_program_info.clone()];
    all_accounts.extend(vec![program_identity_info.clone()]);
    all_accounts.extend_from_slice(remaining_accounts);

    // Invoke the vrf-macro with randomness and signed identity
    let id = program_identity_pda();
    program_identity_info.has_address(&id.0)?;
    let pda_signer_seeds: &[&[&[u8]]] = &[&[IDENTITY, &[id.1]]];
    solana_program::program::invoke_signed(&ix, &all_accounts, pda_signer_seeds)?;

    Ok(())
}
