use curve25519_dalek::constants::RISTRETTO_BASEPOINT_POINT;
use curve25519_dalek::{RistrettoPoint, Scalar};
use ephemeral_vrf_api::prelude::*;
use solana_curve25519::ristretto::{add_ristretto, multiply_ristretto, PodRistrettoPoint};
use solana_curve25519::scalar::PodScalar;
use solana_program::hash::hash;
use steel::*;

const VRF_PREFIX_CHALLENGE: &[u8] = b"VRF-Ephem-Challenge";
const VRF_PREFIX_HASH_TO_POINT: &[u8; 32] = b"VRF-Ephem-HashToPoint-Prefix-Hsh";

pub fn process_provide_randomness(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse args
    let args = ProvideRandomness::try_from_bytes(data)?;

    // Load accounts
    let [signer_info, approved_oracles_info, oracle_queue_info, system_program_info] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // Verify signer
    signer_info.is_signer()?;

    // Load oracle queue
    {
        let mut oracle_queue = QueueAccount::try_from_bytes_with_discriminator(
            &mut oracle_queue_info.try_borrow_data()?,
        )?;
        // let item = oracle_queue
        //     .items
        //     .get(&args.input)
        //     .ok_or(EphemeralVrfError::RandomnessRequestNotFound)?;
        let output = &args.output;
        let commitment_base_compressed = &args.commitment_base_compressed;
        let commitment_hash_compressed = &args.commitment_hash_compressed;
        let s = &args.s;

        let verified = verify_vrf(
            &PodRistrettoPoint(args.input),
            &args.input,
            output,
            (commitment_base_compressed, commitment_hash_compressed, s),
        );

        if !verified {
            return Err(EphemeralVrfError::InvalidProof.into());
        }

        log(format!("Is proof valid? {:?}", verified));
    }

    Ok(())
}

fn verify_vrf(
    pk: &PodRistrettoPoint,
    input: &[u8; 32],
    output_compressed: &PodRistrettoPoint,
    proof: (&PodRistrettoPoint, &PodRistrettoPoint, &PodScalar),
) -> bool {
    let (commitment_base_compressed, commitment_hash_compressed, s) = proof;

    // Recompute h (with domain separation)
    let h = PodRistrettoPoint(hash_to_point(input).compress().to_bytes());

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
    let c = PodScalar(hash_to_scalar(&challenge_hash.to_bytes()).to_bytes());

    // ---------------------------
    // 1) Schnorr check for G:
    // s·G == commitment_base + c·pk
    // ---------------------------
    let lhs_base = match multiply_ristretto(
        s,
        &PodRistrettoPoint(RISTRETTO_BASEPOINT_POINT.compress().to_bytes()),
    ) {
        Some(result) => result,
        None => return false,
    };

    let rhs_base_r = match multiply_ristretto(&c, pk) {
        Some(result) => result,
        None => return false, // Return false instead of an error
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

fn hash_to_point(input: &[u8; 32]) -> RistrettoPoint {
    let mut concatenated = [0u8; 64];
    concatenated[..32].copy_from_slice(VRF_PREFIX_HASH_TO_POINT);
    concatenated[32..].copy_from_slice(input);
    RistrettoPoint::from_uniform_bytes(&concatenated)
}

fn hash_to_scalar(input: &[u8; 32]) -> Scalar {
    Scalar::from_bytes_mod_order(input.clone())
}
