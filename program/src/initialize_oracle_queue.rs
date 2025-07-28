use ephemeral_vrf_api::loaders::is_empty_or_zeroed;
use ephemeral_vrf_api::prelude::EphemeralVrfError::Unauthorized;
use ephemeral_vrf_api::prelude::*;
use ephemeral_vrf_api::ID;
use solana_program::msg;
use steel::*;
const MAX_EXTRA_BYTES: usize = 10_240;

/// Process the initialization of the Oracle queue
///
/// This instruction is designed to be repeated until the Oracle queue is
/// successfully created and initialized (the discriminator is set).
/// This is due to the max allocation size of 10_240 bytes per instruction and the queue possibly
/// being larger than 10_240 bytes.
///
/// The queue uses zero-copy serialization and can be as big as the max account size on Solana
///
///
/// Accounts:
///
/// 0; `[signer]` The payer of the transaction fees
/// 1; `[]`       The Oracle public key
/// 2; `[]`       The Oracle data account
/// 3; `[]`       The Oracle queue account (PDA to be created)
/// 4; `[]`       The System program
///
/// Requirements:
///
/// - The payer (account 0) mus be a signer.
/// - The Oracle data account (account 2) must have the correct seeds ([ORACLE_DATA, oracle.key]).
/// - The Oracle queue account (account 3) must be empty and use the correct seeds ([QUEUE, oracle.key, index]).
/// - The Oracle must have been registered for at least 200 slots.
///
/// 1. Parse the instruction data and extract arguments (InitializeOracleQueue).
/// 2. Confirm the Oracle is authorized (enough time has passed since registration).
/// 3. Create the Oracle queue PDA.
/// 4. Write the default QueueAccount data to the new PDA.
pub fn process_initialize_oracle_queue(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args
    let args = InitializeOracleQueue::try_from_bytes(data)?;

    // Destructure and validate accounts
    let [signer_info, oracle_info, oracle_data_info, oracle_queue_info, system_program] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;

    // Oracle must be the signer to prevent unauthorized queue creation
    oracle_info.is_signer()?;

    let oracle_key_bytes = oracle_info.key.to_bytes();
    let oracle_key_ref = oracle_key_bytes.as_ref();

    // Validate seeds
    oracle_data_info.has_seeds(&[ORACLE_DATA, oracle_key_ref], &ID)?;
    oracle_queue_info
        .is_writable()?
        .has_seeds(&[QUEUE, oracle_key_ref, &[args.index]], &ID)?;
    is_empty_or_zeroed(oracle_queue_info)?;

    let oracle_data = oracle_data_info.as_account::<Oracle>(&ID)?;

    // Check slot timing
    let current_slot = {
        #[cfg(not(feature = "test-sbf"))]
        {
            Clock::get()?.slot
        }
        #[cfg(feature = "test-sbf")]
        {
            500u64
        }
    };

    let slots_since_registration = current_slot.saturating_sub(oracle_data.registration_slot);

    if slots_since_registration == 0 {
        log(format!(
            "Invalid: current slot {} is before registration slot {}",
            current_slot, oracle_data.registration_slot
        ));
        return Err(Unauthorized.into());
    }

    if slots_since_registration < 200 {
        log(format!(
            "Oracle {} not yet authorized – wait {} more slots",
            oracle_info.key,
            200 - slots_since_registration
        ));
        return Err(Unauthorized.into());
    }

    // PDA creation or reallocation
    let seeds: &[&[u8]] = &[QUEUE, oracle_key_ref, &[args.index]];
    let bump = Pubkey::find_program_address(seeds, &ID).1;

    let target_size = Queue::size_with_discriminator();
    let current_size = oracle_queue_info.data_len();

    let extra_bytes = target_size.saturating_sub(current_size);

    if extra_bytes > MAX_EXTRA_BYTES {
        let realloc_size = current_size + MAX_EXTRA_BYTES;
        if oracle_queue_info.owner != &ID {
            create_pda(
                oracle_queue_info,
                &ID,
                MAX_EXTRA_BYTES,
                seeds,
                bump,
                system_program,
                signer_info,
            )?;
        } else {
            resize_pda(signer_info, oracle_queue_info, system_program, realloc_size)?;
        }
        msg!(
            "Reallocating oracle queue account by 10_240 bytes, execute one more time. Current size: {}, target size: {}",
            current_size,
            target_size
        );
        return Ok(());
    }

    // Finalize PDA size if needed
    if oracle_queue_info.owner != &ID {
        create_pda(
            oracle_queue_info,
            &ID,
            target_size,
            seeds,
            bump,
            system_program,
            signer_info,
        )?;
    } else {
        resize_pda(signer_info, oracle_queue_info, system_program, target_size)?;
    }

    // Set discriminator and initialize queue
    oracle_queue_info.data.borrow_mut()[0] = AccountDiscriminator::Queue as u8;
    let queue = oracle_queue_info.as_account_mut::<Queue>(&ID)?;
    queue.index = args.index;

    Ok(())
}
