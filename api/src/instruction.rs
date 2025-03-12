use steel::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum EphemeralVrfInstruction {
    Initialize = 0,
    AddOracle = 1,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Initialize {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct AddOracle {
    pub identity: Pubkey,
}

instruction!(EphemeralVrfInstruction, Initialize);
instruction!(EphemeralVrfInstruction, AddOracle);
