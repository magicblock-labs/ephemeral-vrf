use crate::state::AccountDiscriminator;
use solana_curve25519::ristretto::PodRistrettoPoint;
use steel::{account, trace, Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Oracle {
    pub vrf_pubkey: PodRistrettoPoint,
    pub registration_slot: u64,
    pub open_queue: u64,
}

account!(AccountDiscriminator, Oracle);
