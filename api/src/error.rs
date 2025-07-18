use steel::*;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq, IntoPrimitive)]
#[repr(u32)]
pub enum EphemeralVrfError {
    #[error("Unauthorized authority")]
    Unauthorized = 0,
    #[error("Randomness request not found")]
    RandomnessRequestNotFound = 1,
    #[error("Invalid proof")]
    InvalidProof = 2,
    #[error("Invalid vrf-macro accounts")]
    InvalidCallbackAccounts = 3,
    #[error("Queue is full and cannot accept more items")]
    QueueFull = 4,
    #[error("Invalid queue index")]
    InvalidQueueIndex = 5,
    #[error("Invalid account data")]
    InvalidAccountData = 6,
    #[error("Account is already initialized")]
    AccountAlreadyInitialized = 7,
    #[error("Argument size exceeds the maximum allowed")]
    ArgumentSizeTooLarge = 8,
    #[error("Oracle is not registered")]
    OracleNotRegistered = 9,
    #[error("Oracle is not authorized")]
    OracleNotAuthorized = 10,
    #[error("Queue is not empty - cannot close queue with pending requests")]
    QueueNotEmpty = 11,
}

error!(EphemeralVrfError);
