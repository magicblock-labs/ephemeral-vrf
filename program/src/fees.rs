use solana_program::account_info::AccountInfo;
use solana_program::program_error::ProgramError;

// Transfer a specific amount of lamports from the oracle queue account to the oracle account.
// Assumes caller already validated seeds/ownership/writability and any signer requirements.
pub fn transfer_fee(
    oracle_queue_info: &AccountInfo<'_>,
    oracle_info: &AccountInfo<'_>,
    amount: u64,
) -> Result<(), ProgramError> {
    let (mut queue_lamports, mut oracle_lamports) = (
        oracle_queue_info.try_borrow_mut_lamports()?,
        oracle_info.try_borrow_mut_lamports()?,
    );

    **queue_lamports = (**queue_lamports)
        .checked_sub(amount)
        .ok_or(ProgramError::InsufficientFunds)?;
    **oracle_lamports = (**oracle_lamports)
        .checked_add(amount)
        .ok_or(ProgramError::InvalidArgument)?;

    Ok(())
}
