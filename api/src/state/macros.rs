#[macro_export]
macro_rules! impl_to_bytes_with_discriminator_borsh {
    ($struct_name:ident) => {
        impl $struct_name {
            pub fn to_bytes_with_discriminator(
                &self,
            ) -> Result<Vec<u8>, ::solana_program::program_error::ProgramError> {
                // Allocate a buffer with the discriminator (8 bytes) + estimated serialized size
                let mut buffer = Vec::with_capacity(8 + std::mem::size_of::<Self>());

                // Write the discriminator
                buffer.extend_from_slice(&Self::discriminator().to_bytes());

                // Serialize the struct with borsh
                let serialized = borsh::to_vec(self).map_err(|_| {
                    ::solana_program::program_error::ProgramError::InvalidAccountData
                })?;

                // Append the serialized data
                buffer.extend_from_slice(&serialized);

                Ok(buffer)
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
                // Check if data is long enough to contain the discriminator
                if data.len() < 8 {
                    return Err(::solana_program::program_error::ProgramError::InvalidAccountData);
                }

                // Verify the discriminator
                if Self::discriminator().to_bytes().ne(&data[..8]) {
                    return Err(::solana_program::program_error::ProgramError::InvalidAccountData);
                }

                // Use borsh to deserialize
                let deserialized = <Self as ::borsh::BorshDeserialize>::try_from_slice(&data[8..])
                    .map_err(|_| {
                        ::solana_program::program_error::ProgramError::InvalidAccountData
                    })?;

                Ok(deserialized)
            }
        }
    };
}
