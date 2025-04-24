use ephemeral_vrf_api::prelude::*;
use steel::*;

pub fn process_close_oracle_queue(accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = CloseOracleQueue::try_from_bytes(data)?;

    // Load accounts.
    let [oracle_info, oracle_data_info, oracle_queue_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    oracle_info.is_signer()?;

    oracle_data_info
        .has_owner(&ephemeral_vrf_api::ID)?
        .has_seeds(
            &[ORACLE_DATA, oracle_info.key.to_bytes().as_ref()],
            &ephemeral_vrf_api::ID,
        )?;

    oracle_queue_info
        .is_writable()?
        .has_owner(&ephemeral_vrf_api::ID)?
        .has_seeds(
            &[QUEUE, oracle_info.key.to_bytes().as_ref(), &[args.index]],
            &ephemeral_vrf_api::ID,
        )?;

    close_account(oracle_queue_info, oracle_info)?;

    Ok(())
}
