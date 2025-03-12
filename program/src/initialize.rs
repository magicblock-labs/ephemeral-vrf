use ephemeral_vrf_api::prelude::*;
use steel::*;

pub fn process_initialize(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {

    // Load accounts.
    let [signer_info, oracles_info, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    oracles_info
        .is_empty()?
        .is_writable()?
        .has_seeds(&[ORACLES], &ephemeral_vrf_api::ID)?;
    system_program.is_program(&system_program::ID)?;

    let mut oracles_bytes = vec![];
    let oracles = Oracles::default();
    oracles.to_bytes_with_discriminator(&mut oracles_bytes)?;

    create_pda(
        oracles_info,
        &ephemeral_vrf_api::ID,
        oracles_bytes.len(),
        &[ORACLES],
        oracles_pda().1,
        system_program,
        signer_info,
    )?;

    let mut oracles_data = oracles_info.try_borrow_mut_data()?;
    oracles_data.copy_from_slice(&oracles_bytes);

    Ok(())
}
