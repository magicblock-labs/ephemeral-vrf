use curve25519_dalek::Scalar;
use ephemeral_vrf_api::prelude::*;
use solana_curve25519::ristretto::{add_ristretto, multiply_ristretto, PodRistrettoPoint};
use solana_curve25519::scalar::PodScalar;
use solana_program::hash::hash;
use steel::*;

/// Process the provide randomness instruction which verifies VRF proof and executes callback
///
/// Accounts:
///
/// 0. `[signer]` signer - The oracle signer providing randomness
/// 1. `[]` oracle_data_info - Oracle data account associated with the signer
/// 2. `[writable]` oracle_queue_info - Queue storing randomness requests
/// 3. `[]` callback_program_info - Program to call with the randomness
/// 4. `[]` system_program_info - System program for resizing accounts
/// 5+ `[varies]` remaining_accounts - Accounts needed for the callback
///
/// Requirements:
///
/// - Signer must be a registered oracle with valid VRF keypair
/// - VRF proof must be valid for the given input and output
/// - Request must exist in the oracle queue
/// - Oracle signer must not be included in callback accounts
///
/// 1. Verify the oracle signer and load oracle data
/// 2. Verify the VRF proof
/// 3. Remove the request from the queue
/// 4. Invoke the callback with the randomness
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

    // Load and remove oracle item from the queue
    let mut oracle_queue = QueueAccount::try_from_bytes_with_discriminator(
        &mut oracle_queue_info.try_borrow_data()?,
    )?;
    let item = oracle_queue
        .items
        .get(&args.input)
        .ok_or(EphemeralVrfError::RandomnessRequestNotFound)?
        .clone();
    oracle_queue.items.remove(&args.input);

    let mut oracle_queue_bytes = vec![];
    oracle_queue.to_bytes_with_discriminator(&mut oracle_queue_bytes)?;

    // Resize and serialize oracle queue
    resize_pda(
        signer_info,
        oracle_queue_info,
        system_program_info,
        oracle_queue_bytes.len(),
    )?;
    let mut oracle_queue_data = oracle_queue_info.try_borrow_mut_data()?;
    oracle_queue_data.copy_from_slice(&oracle_queue_bytes);

    // Check that the oracle signer is not in the callback accounts
    if item
        .callback_accounts_meta
        .iter()
        .any(|acc| acc.pubkey == *signer_info.key)
    {
        return Err(EphemeralVrfError::InvalidCallbackAccounts.into());
    }

    // Invoke callback with randomness
    callback_program_info.has_address(&item.callback_program_id)?;
    let accounts_metas: Vec<_> = item.callback_accounts_meta.iter().map(|acc| AccountMeta {
        pubkey: acc.pubkey,
        is_signer: acc.is_signer,
        is_writable: acc.is_writable,
    }).collect();
    let mut callback_data = Vec::with_capacity(
        item.callback_discriminator.len() + output.0.len() + item.callback_args.len()
    );
    callback_data.extend_from_slice(&item.callback_discriminator);
    callback_data.extend_from_slice(&output.0);
    callback_data.extend_from_slice(&item.callback_args);

    let ix = Instruction {
        program_id: item.callback_program_id,
        accounts: accounts_metas,
        data: callback_data,
    };
    let mut all_accounts = vec![callback_program_info.clone()];
    all_accounts.extend_from_slice(remaining_accounts);

    solana_program::program::invoke_signed(&ix, &all_accounts, &[])?;

    //TODO: Pass also a signer PDA that can be used to enforce CPI from the receiver program

    Ok(())
}

/// Verify a VRF proof
///
/// Accounts: None
///
/// Requirements:
///
/// - Proof must be valid for the given public key, input, and output
///
/// 1. Recompute the hash point from input
/// 2. Recompute the challenge scalar
/// 3. Verify the Schnorr proof for the base point
/// 4. Verify the Schnorr-like proof for the hash point
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
///
/// Accounts: None
///
/// Requirements: None
///
/// 1. Hash the input with the VRF prefix
/// 2. Convert the hash to a scalar
/// 3. Multiply the scalar with the base point
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
///
/// Accounts: None
///
/// Requirements: None
///
/// 1. Convert the input bytes to a scalar using the curve's modulus
fn hash_to_scalar(input: &[u8; 32]) -> PodScalar {
    PodScalar(Scalar::from_bytes_mod_order(*input).to_bytes())
}
