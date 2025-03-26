use solana_program::pubkey;
use solana_program::pubkey::Pubkey;

/// Verifiable Random Function program id
pub const VRF_PROGRAM_ID: Pubkey = pubkey!("VrffXU38S8MzqTtTYQG3M8GNwheKH8n77HVEZUdakH8");

/// The default queue for randomness requests
pub const DEFAULT_QUEUE: Pubkey = pubkey!("6qqax73tfwwZgkYq59Yebb1xUWpYrZDSutAeHoMihKYS");

/// The default queue for ephemeral randomness requests
pub const DEFAULT_EPHEMERAL_QUEUE: Pubkey = pubkey!("6ykZL44GxESV7sLYZfeNEouD2chTMP5D4JyxG1HJM6ur");

/// Vrf program identity PDA
pub const VRF_PROGRAM_IDENTITY: Pubkey = pubkey!("AwF6egvgtC2RdkfUEcCCtjHP2iWhCzFBMi1a6bjv9Hkp");

/// Seed of the identity PDA
pub const IDENTITY: &[u8] = b"identity";
