#[macro_export]
macro_rules! impl_to_bytes_with_discriminator_borsh {
    ($struct_name:ident) => {
        impl $struct_name {
            pub fn to_bytes_with_discriminator<W: std::io::Write>(
                &self,
                data: &mut W,
            ) -> Result<(), ::solana_program::program_error::ProgramError> {
                data.write_all(&Self::discriminator().to_bytes())?;
                self.serialize(data)?;
                Ok(())
            }
        }
    };
}

#[macro_export]
macro_rules! impl_try_from_bytes_with_discriminator_borsh {
    ($struct_name:ident) => {
        impl $struct_name {
            pub fn try_from_bytes_with_discriminator(
                data: &[u8],
            ) -> Result<Self, ::solana_program::program_error::ProgramError> {
                if data.len() < 8 {
                    return Err(::solana_program::program_error::ProgramError::InvalidAccountData);
                }
                if Self::discriminator().to_bytes().ne(&data[..8]) {
                    return Err(::solana_program::program_error::ProgramError::InvalidAccountData);
                }
                // Use deserialize with a mutable reference to handle cases where more data is allocated
                Self::deserialize(&mut &data[8..]).or(Err(
                    ::solana_program::program_error::ProgramError::InvalidAccountData,
                ))
            }
        }
    };
}
