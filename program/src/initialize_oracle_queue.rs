use ephemeral_vrf_api::prelude::EphemeralVrfError::Unauthorized;
use ephemeral_vrf_api::prelude::*;
use ephemeral_vrf_api::ID;
use solana_program::msg;
use steel::*;

/// Process the initialization of the Oracle queue
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
/// - The payer (account 0) must be a signer.
/// - The Oracle data account (account 2) must have the correct seeds ([ORACLE_DATA, oracle.key]).
/// - The Oracle queue account (account 3) must be empty and use the correct seeds ([QUEUE, oracle.key, index]).
/// - The Oracle must have been registered for at least 200 slots.
///
/// 1. Parse the instruction data and extract arguments (InitializeOracleQueue).
/// 2. Confirm the Oracle is authorized (enough time has passed since registration).
/// 3. Create the Oracle queue PDA.
/// 4. Write the default QueueAccount data to the new PDA.
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

    let oracle_data = oracle_data_info.as_account::<Oracle>(&ID)?;

    #[cfg(not(feature = "test-sbf"))]
    let current_slot = Clock::get()?.slot;

    #[cfg(feature = "test-sbf")]
    let current_slot = 500;

    if current_slot - oracle_data.registration_slot < 200 {
        log(format!(
            "Oracle {} not authorized or not yet reached an operational slot",
            oracle_info.key
        ));
        return Err(Unauthorized.into());
    }

    // Calculate the fixed size of the account
    let account_size = Queue::size_with_discriminator();
    msg!("Account size: {}", account_size);

    // Create the PDA with the fixed size
    create_program_account::<Queue>(
        oracle_queue_info,
        system_program,
        signer_info,
        &ID,
        &[QUEUE, oracle_info.key.to_bytes().as_ref(), &[args.index]],
    )?;
    let queue = oracle_queue_info.as_account_mut::<Queue>(&ID)?;
    queue.index = args.index;

    Ok(())
}
