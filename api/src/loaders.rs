use solana_program::account_info::AccountInfo;
use solana_program::{bpf_loader_upgradeable, msg};
use solana_program::bpf_loader_upgradeable::UpgradeableLoaderState;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

/// Get the program upgrade authority for a given program
pub fn load_program_upgrade_authority(
    program: &Pubkey,
    program_data: &AccountInfo,
) -> Result<Option<Pubkey>, ProgramError> {
    let program_data_address =
        Pubkey::find_program_address(&[program.as_ref()], &bpf_loader_upgradeable::id()).0;

    // During tests, the upgrade authority is a test pubkey
    #[cfg(feature = "unit_test_config")]
    if program.eq(&crate::ID) {
        return Ok(Some(solana_program::pubkey!(
            "tEsT3eV6RFCWs1BZ7AXTzasHqTtMnMLCB2tjQ42TDXD"
        )));
    }

    if !program_data_address.eq(program_data.key) {
        msg!(
            "Expected program data address to be {}, but got {}",
            program_data_address,
            program_data.key
        );
        return Err(ProgramError::InvalidAccountData);
    }

    let program_account_data = program_data.try_borrow_data()?;
    if let UpgradeableLoaderState::ProgramData {
        upgrade_authority_address,
        ..
    } = bincode::deserialize(&program_account_data).map_err(|_| {
        msg!("Unable to deserialize ProgramData {}", program);
        ProgramError::InvalidAccountData
    })? {
        Ok(upgrade_authority_address)
    } else {
        msg!("Expected program account {} to hold ProgramData", program);
        Err(ProgramError::InvalidAccountData)
    }
}