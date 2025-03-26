#[cfg(feature = "anchor")]
extern crate anchor_lang;

#[cfg(feature = "anchor")]
pub use ephemeral_vrf_sdk_vrf_macro::*;

#[cfg(feature = "anchor")]
pub struct VrfProgram;

#[cfg(feature = "anchor")]
impl anchor_lang::Id for VrfProgram {
    fn id() -> ::solana_program::pubkey::Pubkey {
        crate::consts::VRF_PROGRAM_ID
    }
}
