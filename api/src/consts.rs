use solana_program::pubkey;
use steel::Pubkey;

/// seed of the oracles account PDA.
pub const ORACLES: &[u8] = b"oracles";

/// Seed of the counter account PDA.
pub const COUNTER: &[u8] = b"counter";

/// Seed of the queue account PDA.
pub const QUEUE: &[u8] = b"queue";

/// The admin pubkey of the authority allowed to whitelist validators.
#[cfg(feature = "unit_test_config")]
pub const ADMIN_PUBKEY: Pubkey = pubkey!("tEsT3eV6RFCWs1BZ7AXTzasHqTtMnMLCB2tjQ42TDXD");
#[cfg(not(feature = "unit_test_config"))]
pub const ADMIN_PUBKEY: Pubkey = pubkey!("3FwNxjbCqdD7G6MkrAdwTd5Zf6R3tHoapam4Pv1X2KBB");
