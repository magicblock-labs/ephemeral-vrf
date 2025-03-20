use ephemeral_vrf_api::prelude::EphemeralVrfError::Unauthorized;
use ephemeral_vrf_api::prelude::*;
use steel::*;

pub fn process_initialize_oracle_queue(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = InitializeOracleQueue::try_from_bytes(data)?;

    // Load accounts.
    let [signer_info, oracles_info, approved_oracles_info, oracle_queue_info, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;

    approved_oracles_info.has_seeds(&[ORACLES], &ephemeral_vrf_api::ID)?;

    oracle_queue_info.is_writable()?.is_empty()?.has_seeds(
        &[QUEUE, oracles_info.key.to_bytes().as_ref(), &[args.index]],
        &ephemeral_vrf_api::ID,
    )?;

    let approved_oracles_data = approved_oracles_info.try_borrow_data()?;
    let approved_oracles = Oracles::try_from_bytes_with_discriminator(&approved_oracles_data)?;

    #[cfg(not(feature = "test-sbf"))]
    let current_slot = Clock::get()?.slot;

    #[cfg(feature = "test-sbf")]
    let current_slot = 500;

    // Check that the oracle is the approved list, from at least 200 slots (> 1.5 min).
    // This is to prevent that an oracle mine a new pubkey to influence the VRF.
    if approved_oracles.oracles.iter().all(|oracle| {
        oracle.identity != *oracles_info.key || current_slot - oracle.registration_slot < 200
    }) {
        log(format!(
            "Oracle {} not authorized or not yet reached an operational slot",
            oracles_info.key
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
        &[QUEUE, oracles_info.key.to_bytes().as_ref(), &[args.index]],
        oracle_queue_pda(oracles_info.key.clone(), args.index).1,
        system_program,
        signer_info,
    )?;

    {
        let mut oracle_queue_data = oracle_queue_info.try_borrow_mut_data()?;
        oracle_queue_data.copy_from_slice(&oracle_queue_bytes);
    }

    Ok(())
}
