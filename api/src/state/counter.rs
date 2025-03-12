use steel::*;
use crate::state::AccountDiscriminator;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
pub struct Counter {
    pub value: u64,
}

account!(AccountDiscriminator, Counter);
