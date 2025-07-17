#[macro_export]
macro_rules! impl_to_bytes_with_discriminator_rkyv {
    ($struct_name:ident) => {
        impl $struct_name {
            pub fn to_bytes_with_discriminator(
                &self,
            ) -> Result<Vec<u8>, ::solana_program::program_error::ProgramError> {
                // Allocate a buffer with the discriminator (8 bytes) + size (8 bytes) + estimated serialized size
                let mut buffer = Vec::with_capacity(16 + std::mem::size_of::<Self>());

                // Write the discriminator
                buffer.extend_from_slice(&Self::discriminator().to_bytes());

                // Serialize the struct with rkyv
                let serialized = rkyv::to_bytes::<_, 256>(self).map_err(|_| {
                    ::solana_program::program_error::ProgramError::InvalidAccountData
                })?;

                // Write the size of the serialized data (8 bytes for alignment)
                let size = serialized.len() as u64;
                buffer.extend_from_slice(&size.to_le_bytes());

                // Append the serialized data
                buffer.extend_from_slice(&serialized);

                Ok(buffer)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_try_from_bytes_with_discriminator_rkyv {
    ($struct_name:ident) => {
        impl $struct_name {
            pub fn try_from_bytes_with_discriminator(
                data: &[u8],
            ) -> Result<Self, ::solana_program::program_error::ProgramError> {
                // Check if data is long enough to contain the discriminator and size
                if data.len() < 16 {
                    return Err(::solana_program::program_error::ProgramError::InvalidAccountData);
                }

                // Verify the discriminator
                if Self::discriminator().to_bytes().ne(&data[..8]) {
                    return Err(::solana_program::program_error::ProgramError::InvalidAccountData);
                }

                // Read the size of the serialized data (8 bytes)
                let size = u64::from_le_bytes([
                    data[8], data[9], data[10], data[11], data[12], data[13], data[14], data[15],
                ]) as usize;

                // Check if data is long enough to contain the serialized data
                if data.len() < 16 + size {
                    return Err(::solana_program::program_error::ProgramError::InvalidAccountData);
                }

                // Use the high-level from_bytes function to deserialize only the valid data
                let deserialized =
                    rkyv::from_bytes::<Self>(&data[16..(16 + size)]).map_err(|_| {
                        ::solana_program::program_error::ProgramError::InvalidAccountData
                    })?;

                Ok(deserialized)
            }
        }
    };
}

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
                let deserialized = borsh::from_slice::<Self>(&data[8..]).map_err(|_| {
                    ::solana_program::program_error::ProgramError::InvalidAccountData
                })?;

                Ok(deserialized)
            }
        }
    };
}
