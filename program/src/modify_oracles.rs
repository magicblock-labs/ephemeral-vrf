use ephemeral_vrf_api::prelude::EphemeralVrfError::Unauthorized;
use ephemeral_vrf_api::prelude::*;
use steel::*;

pub fn process_modify_oracles(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args.
    let args = ModifyOracle::try_from_bytes(data)?;

    // Load accounts.
    let [signer_info, oracles_info, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;

    // Check that the signer is the admin.
    if !signer_info.key.eq(&ADMIN_PUBKEY) {
        log(format!(
            "Signer not authorized, expected: {}, got: {}",
            ADMIN_PUBKEY, signer_info.key
        ));
        return Err(Unauthorized.into());
    }

    oracles_info
        .is_writable()?
        .has_seeds(&[ORACLES], &ephemeral_vrf_api::ID)?;

    let oracles_data = oracles_info.try_borrow_data()?;
    let mut oracles = Oracles::try_from_bytes_with_discriminator(&oracles_data)?;
    drop(oracles_data);

    if args.operation == 0 {
        oracles.oracles.push(Oracle {
            identity: args.identity,
            oracle_publickey: args.oracle_pubkey,
            registration_slot: Clock::get()?.slot,
        });
    } else {
        oracles
            .oracles
            .retain(|oracle| oracle.identity != args.identity);
    }

    resize_pda(
        signer_info,
        oracles_info,
        system_program,
        oracles.size_with_discriminator(),
    )?;

    let mut oracles_bytes = vec![];
    oracles.to_bytes_with_discriminator(&mut oracles_bytes)?;
    let mut oracles_data = oracles_info.try_borrow_mut_data()?;
    oracles_data.copy_from_slice(&oracles_bytes);

    Ok(())
}
