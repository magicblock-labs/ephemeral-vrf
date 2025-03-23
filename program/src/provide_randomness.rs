use curve25519_dalek::Scalar;
use ephemeral_vrf_api::prelude::*;
use solana_curve25519::ristretto::{add_ristretto, multiply_ristretto, PodRistrettoPoint};
use solana_curve25519::scalar::PodScalar;
use solana_program::hash::hash;
use steel::*;

pub fn process_provide_randomness(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args
    let args = ProvideRandomness::try_from_bytes(data)?;

    // Load accounts
    let (
        [signer_info, oracle_data_info, oracle_queue_info, callback_program_info, system_program_info],
        remaining_accounts,
    ) = accounts.split_at(5)
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Verify signer
    signer_info.is_signer()?;

    // Load oracle data
    oracle_data_info.has_seeds(
        &[ORACLE_DATA, signer_info.key.to_bytes().as_ref()],
        &ephemeral_vrf_api::ID,
    )?;
    let oracle_data = oracle_data_info.as_account::<Oracle>(&ephemeral_vrf_api::ID)?;

    let output = &args.output;
    let commitment_base_compressed = &args.commitment_base_compressed;
    let commitment_hash_compressed = &args.commitment_hash_compressed;
    let s = &args.s;

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

    let mut oracle_queue_bytes = vec![];
    let item: QueueItem;
    // Load and remove oracle item from the queue
    {
        let mut oracle_queue = QueueAccount::try_from_bytes_with_discriminator(
            &mut oracle_queue_info.try_borrow_data()?,
        )?;
        item = oracle_queue
            .items
            .get(&args.input)
            .ok_or(EphemeralVrfError::RandomnessRequestNotFound)?
            .clone();
        oracle_queue.items.remove(&args.input);

        oracle_queue.to_bytes_with_discriminator(&mut oracle_queue_bytes)?;
    }
    // Resize and serialize oracle queue
    {
        resize_pda(
            signer_info,
            oracle_queue_info,
            system_program_info,
            oracle_queue_bytes.len(),
        )?;
        let mut oracle_queue_data = oracle_queue_info.try_borrow_mut_data()?;
        oracle_queue_data.copy_from_slice(&oracle_queue_bytes);
    }

    // Invoke callback with randomness
    callback_program_info.has_address(&item.callback_program_id)?;
    let accounts_metas = item
        .callback_accounts_meta
        .iter()
        .map(|acc| AccountMeta {
            pubkey: acc.pubkey,
            is_signer: acc.is_signer,
            is_writable: acc.is_writable,
        })
        .collect::<Vec<_>>();

    let ix = Instruction {
        program_id: item.callback_program_id,
        accounts: accounts_metas,
        data: [
            item.callback_discriminator.to_vec(),
            output.0.to_vec(),
            item.callback_args,
        ]
        .concat(),
    };

    let mut all_accounts = vec![callback_program_info.clone()];
    all_accounts.extend_from_slice(remaining_accounts);

    // Check that the oracle signer is not in the callback accounts
    if item
        .callback_accounts_meta
        .iter()
        .any(|acc| acc.pubkey == *signer_info.key)
    {
        return Err(EphemeralVrfError::InvalidCallbackAccounts.into());
    }

    solana_program::program::invoke_signed(&ix, &all_accounts, &[])?;

    Ok(())
}

fn verify_vrf(
    pk: &PodRistrettoPoint,
    input: &[u8; 32],
    output_compressed: &PodRistrettoPoint,
    proof: (&PodRistrettoPoint, &PodRistrettoPoint, &PodScalar),
) -> bool {
    let (commitment_base_compressed, commitment_hash_compressed, s) = proof;

    // Recompute h
    let h = hash_to_point(input);

    // Recompute challenge
    let challenge_input = [
        VRF_PREFIX_CHALLENGE.to_vec(),
        output_compressed.0.to_vec(),
        commitment_base_compressed.0.to_vec(),
        commitment_hash_compressed.0.to_vec(),
        pk.0.to_vec(),
        input.to_vec(),
    ]
    .concat();

    let challenge_hash = hash(challenge_input.as_slice());
    let c = hash_to_scalar(&challenge_hash.to_bytes());

    // ---------------------------
    // 1) Schnorr check for G:
    // s·G == commitment_base + c·pk
    // ---------------------------
    let lhs_base = match multiply_ristretto(s, &RISTRETTO_BASEPOINT_POINT) {
        Some(result) => result,
        None => return false,
    };

    let rhs_base_r = match multiply_ristretto(&c, pk) {
        Some(result) => result,
        None => return false,
    };

    let rhs_base = match add_ristretto(commitment_base_compressed, &rhs_base_r) {
        Some(result) => result,
        None => return false,
    };

    // 2) Schnorr-like check for h:
    // s·h == commitment_hash + c·(sk·h)
    // But sk·h = output
    // => s·h == commitment_hash + c·output
    let lhs_hash = match multiply_ristretto(s, &h) {
        Some(result) => result,
        None => return false,
    };

    let rhs_hash_r = match multiply_ristretto(&c, output_compressed) {
        Some(result) => result,
        None => return false,
    };

    let rhs_hash = match add_ristretto(&commitment_hash_compressed, &rhs_hash_r) {
        Some(result) => result,
        None => return false,
    };

    lhs_base == rhs_base && lhs_hash == rhs_hash
}

/// Hash the input with a prefix, convert the result to a scalar, and multiply it with the base point
fn hash_to_point(input: &[u8]) -> PodRistrettoPoint {
    let hashed_input = hash(
        [VRF_PREFIX_HASH_TO_POINT.to_vec(), input.to_vec()]
            .concat()
            .as_slice(),
    );
    multiply_ristretto(
        &PodScalar(Scalar::from_bytes_mod_order(hashed_input.to_bytes()).to_bytes()),
        &RISTRETTO_BASEPOINT_POINT,
    )
    .unwrap()
}

/// Convert the input to a scalar using the modulus order of the curve
fn hash_to_scalar(input: &[u8; 32]) -> PodScalar {
    PodScalar(Scalar::from_bytes_mod_order(input.clone()).to_bytes())
}
