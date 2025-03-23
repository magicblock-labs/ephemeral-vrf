use solana_curve25519::ristretto::PodRistrettoPoint;
use solana_program::pubkey;
use steel::Pubkey;

/// seed of the oracles account PDA.
pub const ORACLES: &[u8] = b"oracles";

/// seed of the oracle data account PDA.
pub const ORACLE_DATA: &[u8] = b"oracle";

/// Seed of the identity account PDA.
pub const IDENTITY: &[u8] = b"identity";

/// Seed of the queue account PDA.
pub const QUEUE: &[u8] = b"queue";

/// The admin pubkey of the authority allowed to whitelist validators.
#[cfg(feature = "unit_test_config")]
pub const ADMIN_PUBKEY: Pubkey = pubkey!("tEsT3eV6RFCWs1BZ7AXTzasHqTtMnMLCB2tjQ42TDXD");
#[cfg(not(feature = "unit_test_config"))]
pub const ADMIN_PUBKEY: Pubkey = pubkey!("3FwNxjbCqdD7G6MkrAdwTd5Zf6R3tHoapam4Pv1X2KBB");

pub const VRF_PREFIX_CHALLENGE: &[u8] = b"VRF-Ephem-Challenge";
pub const VRF_PREFIX_HASH_TO_POINT: &[u8] = b"VRF-Ephem-HashToPoint";

pub const RISTRETTO_BASEPOINT_POINT: PodRistrettoPoint = PodRistrettoPoint([
    226, 242, 174, 10, 106, 188, 78, 113, 168, 132, 169, 97, 197, 0, 81, 95, 88, 227, 11, 106, 165,
    130, 221, 141, 182, 166, 89, 69, 224, 141, 45, 118,
]);
