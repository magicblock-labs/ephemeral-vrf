use ephemeral_vrf_api::prelude::*;
use ephemeral_vrf_api::verify::verify_vrf;
use solana_program::hash::hash;
use steel::*;

/// Process the provide randomness instruction which verifies VRF proof and executes vrf-macro
///
/// Accounts:
///
/// 0. `[signer]` signer - The oracle signer providing randomness
/// 1. `[]` program_identity_info - Used to allow the vrf-macro program to verify the identity of the oracle program
/// 2. `[]` oracle_data_info - Oracle data account associated with the signer
/// 3. `[writable]` oracle_queue_info - Queue storing randomness requests
/// 4. `[]` callback_program_info - Program to call with the randomness
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
        [oracle_info, program_identity_info, oracle_data_info, oracle_queue_info, callback_program_info],
        remaining_accounts,
    ) = accounts.split_at(5)
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

    // Load oracle queue
    let oracle_queue = oracle_queue_info.as_account_mut::<Queue>(&ephemeral_vrf_api::ID)?;
    oracle_queue_info
        .is_writable()?
        .has_owner(&ephemeral_vrf_api::ID)?
        .has_seeds(
            &[
                QUEUE,
                oracle_info.key.to_bytes().as_ref(),
                &[oracle_queue.index],
            ],
            &ephemeral_vrf_api::ID,
        )?;

    let output = &args.output;
    let commitment_base_compressed = &args.commitment_base_compressed;
    let commitment_hash_compressed = &args.commitment_hash_compressed;
    let s = &args.scalar;

    let (index, item) = {
        let (index, item) = oracle_queue
            .find_item_by_id(&args.input)
            .ok_or::<ProgramError>(EphemeralVrfError::RandomnessRequestNotFound.into())?;

        // Check that the oracle signer is not in the vrf-macro accounts
        if item
            .callback_accounts_meta
            .iter()
            .any(|acc| Pubkey::new_from_array(acc.pubkey).eq(oracle_info.key))
        {
            return Err(EphemeralVrfError::InvalidCallbackAccounts.into());
        }

        // Ensure that fulfillment happens in a different (later) slot than the request
        if Clock::get()?.slot <= item.slot {
            return Err(ProgramError::from(
                EphemeralVrfError::OracleMustProvideInDifferentSlot,
            ));
        }

        (index, *item)
    };

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

    // Remove the item from the queue
    oracle_queue.remove_item(index)?;

    // Invoke vrf-macro with randomness
    callback_program_info.has_address(&Pubkey::new_from_array(item.callback_program_id))?;
    let mut accounts_metas = vec![AccountMeta {
        pubkey: *program_identity_info.key,
        is_signer: true,
        is_writable: false,
    }];
    accounts_metas.extend(item.account_metas().iter().map(|acc| acc.to_account_meta()));

    let mut callback_data = Vec::with_capacity(
        item.callback_discriminator().len() + output.0.len() + item.callback_args().len(),
    );
    callback_data.extend_from_slice(item.callback_discriminator());
    let rdn = hash(&output.0);
    callback_data.extend_from_slice(rdn.to_bytes().as_ref());
    callback_data.extend_from_slice(item.callback_args());

    let ix = Instruction {
        program_id: Pubkey::new_from_array(item.callback_program_id),
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

    // Collect the fees
    let cost = if item.priority_request == 1 {
        VRF_HIGH_PRIORITY_LAMPORTS_COST
    } else {
        VRF_LAMPORTS_COST
    };
    crate::fees::transfer_fee(oracle_queue_info, oracle_info, cost)?;

    Ok(())
}
