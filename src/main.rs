use curve25519_dalek::constants::{RISTRETTO_BASEPOINT_POINT, RISTRETTO_BASEPOINT_TABLE};
use curve25519_dalek::ristretto::{CompressedRistretto, RistrettoPoint};
use curve25519_dalek::scalar::Scalar;
use sha2::Sha512;
use hkdf::Hkdf;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;

// Domain prefixes
const VRF_PREFIX_HASH_TO_POINT: &[u8] = b"VRF-Ephem-HashToPoint";
const VRF_PREFIX_NONCE: &[u8] = b"VRF-Ephem-Nonce";
const VRF_PREFIX_CHALLENGE: &[u8] = b"VRF-Ephem-Challenge";

fn main() {
    let (sk, pk) = generate_vrf_keypair();
    let bs58_pk = Pubkey::new_from_array(pk.compress().to_bytes());
    print!("Generated PK: {:?}", bs58_pk);

    let blockhash = b"blockhash";
    let seed = b"test-seed";
    let input: Vec<u8> = blockhash.iter().chain(seed.iter()).cloned().collect();
    let (x, (commitment_base_compressed, commitment_hash_compressed, s)) = compute_vrf(sk, &input);

    let is_valid = verify_vrf(
        pk,
        &input,
        x,
        (commitment_base_compressed, commitment_hash_compressed, s),
    );
    print!("\nVRF proof is valid: {:?}", is_valid);
}

// Key Generation (done once by the oracle)
fn generate_vrf_keypair() -> (Scalar, RistrettoPoint) {
    let keypair = Keypair::new();
    println!("Solana Ed25519 Pubkey: {:?}", keypair.pubkey());
    let hkdf = Hkdf::<Sha512>::new(Some(b"VRF-Solana-SecretKey"), &keypair.to_bytes());
    let mut okm = [0u8; 64];
    hkdf.expand(b"VRF-Key", &mut okm).expect("HKDF expansion failed");
    let sk = Scalar::from_bytes_mod_order(okm[..32].try_into().unwrap());
    let pk = &sk * RISTRETTO_BASEPOINT_TABLE;
    (sk, pk)
}

// Hash-to-Point using built-in hash_to_group function, plus domain separation
fn hash_to_point(input: &[u8]) -> RistrettoPoint {
    RistrettoPoint::hash_from_bytes::<Sha512>(&[VRF_PREFIX_HASH_TO_POINT, input].concat())
}

// VRF computation
fn compute_vrf(
    sk: Scalar,
    input: &[u8],
) -> (
    CompressedRistretto,
    (CompressedRistretto, CompressedRistretto, Scalar),
) {
    // Hash the input
    let h = hash_to_point(input);
    // VRF output = sk·h
    let vrf_output = sk * h;
    // Public key = sk·G
    let pk = &sk * RISTRETTO_BASEPOINT_TABLE;

    // RFC 9381 Nonce generation with domain separation (updated to derive from sk)
    let k = Scalar::hash_from_bytes::<Sha512>(
        &[
            VRF_PREFIX_NONCE,
            &sk.to_bytes(), // Secret key is included here
            input,
        ]
        .concat(),
    );

    // Commitments: one for basepoint G, one for hashed point h
    let commitment_base = k * RISTRETTO_BASEPOINT_POINT;
    let commitment_hash = k * h;

    // Compute Challenge (domain-tagged)
    let challenge_input = [
        VRF_PREFIX_CHALLENGE.to_vec(),
        vrf_output.compress().to_bytes().to_vec(),
        commitment_base.compress().to_bytes().to_vec(),
        commitment_hash.compress().to_bytes().to_vec(),
        pk.compress().to_bytes().to_vec(),
        input.to_vec(),
    ].concat();
    let c = Scalar::hash_from_bytes::<Sha512>(&challenge_input);

    // Response
    let s = k + c * sk;

    (
        vrf_output.compress(),
        (commitment_base.compress(), commitment_hash.compress(), s),
    )
}

// Verify VRF Proof
fn verify_vrf(
    pk: RistrettoPoint,
    input: &[u8],
    output_compressed: CompressedRistretto,
    proof: (CompressedRistretto, CompressedRistretto, Scalar),
) -> bool {
    let (commitment_base_compressed, commitment_hash_compressed, s) = proof;

    let output = match output_compressed.decompress() {
        Some(p) => p,
        None => return false,
    };
    let commitment_base = match commitment_base_compressed.decompress() {
        Some(p) => p,
        None => return false,
     };
    let commitment_hash = match commitment_hash_compressed.decompress() {
        Some(p) => p,
        None => return false,
    };

    // Recompute h (with domain separation)
    let h = hash_to_point(input);

    // Recompute challenge
    let challenge_input = [
        VRF_PREFIX_CHALLENGE.to_vec(),
        output_compressed.to_bytes().to_vec(),
        commitment_base_compressed.to_bytes().to_vec(),
        commitment_hash_compressed.to_bytes().to_vec(),
        pk.compress().to_bytes().to_vec(),
        input.to_vec(),
    ]
    .concat();
    let c: Scalar = Scalar::hash_from_bytes::<Sha512>(&challenge_input);

    // ---------------------------
    // 1) Schnorr check for G:
    // s·G == commitment_base + c·pk
    // ---------------------------
    let lhs_base = &s * RISTRETTO_BASEPOINT_TABLE;
    let rhs_base = commitment_base + c * pk;

    // ---------------------------
    // 2) Schnorr-like check for h:
    // s·h == commitment_hash + c·(sk·h)
    // But sk·h = output
    // => s·h == commitment_hash + c·output
    // ---------------------------
    let lhs_hash = s * h;
    let rhs_hash = commitment_hash + c * output;

    lhs_base == rhs_base && lhs_hash == rhs_hash
}
