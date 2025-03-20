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
}

error!(EphemeralVrfError);
