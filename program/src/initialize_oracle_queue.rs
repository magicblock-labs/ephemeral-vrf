use ephemeral_vrf_api::prelude::EphemeralVrfError::Unauthorized;
use ephemeral_vrf_api::prelude::*;
use steel::*;

pub fn process_initialize_oracle_queue(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = InitializeOracleQueue::try_from_bytes(data)?;

    // Load accounts.
    let [signer_info, oracle_info, oracle_data_info, oracle_queue_info, system_program] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;

    oracle_data_info.has_seeds(
        &[ORACLE_DATA, oracle_info.key.to_bytes().as_ref()],
        &ephemeral_vrf_api::ID,
    )?;

    oracle_queue_info.is_writable()?.is_empty()?.has_seeds(
        &[QUEUE, oracle_info.key.to_bytes().as_ref(), &[args.index]],
        &ephemeral_vrf_api::ID,
    )?;

    let oracle_data = oracle_data_info.as_account_mut::<Oracle>(&ephemeral_vrf_api::ID)?;

    #[cfg(not(feature = "test-sbf"))]
    let current_slot = Clock::get()?.slot;

    #[cfg(feature = "test-sbf")]
    let current_slot = 500;

    // Check that the oracle is the approved from at least 200 slots (> 1.5 min).
    // This is to prevent that an oracle mine a new pubkey to influence the VRF.
    if current_slot - oracle_data.registration_slot < 200 {
        log(format!(
            "Oracle {} not authorized or not yet reached an operational slot",
            oracle_info.key
        ));
        return Err(Unauthorized.into());
    }

    let mut oracle_queue_bytes = vec![];
    let oracle_queue = QueueAccount::default();
    oracle_queue.to_bytes_with_discriminator(&mut oracle_queue_bytes)?;

    create_pda(
        oracle_queue_info,
        &ephemeral_vrf_api::ID,
        oracle_queue_bytes.len(),
        &[QUEUE, oracle_info.key.to_bytes().as_ref(), &[args.index]],
        oracle_queue_pda(&oracle_info.key, args.index).1,
        system_program,
        signer_info,
    )?;

    {
        let mut oracle_queue_data = oracle_queue_info.try_borrow_mut_data()?;
        oracle_queue_data.copy_from_slice(&oracle_queue_bytes);
    }

    Ok(())
}
