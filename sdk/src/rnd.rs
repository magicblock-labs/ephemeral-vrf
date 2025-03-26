/// Generates a random u8 value from a 32-byte random seed
///
/// # Arguments
///
/// * `bytes` - A 32-byte array containing random data from the VRF
///
/// # Returns
///
/// A random u8 value derived from the input bytes
pub fn random_u8(bytes: &[u8; 32]) -> u8 {
    bytes[0]
}

/// Generates a random u32 value from a 32-byte random seed
///
/// # Arguments
///
/// * `bytes` - A 32-byte array containing random data from the VRF
///
/// # Returns
///
/// A random u32 value derived from the input bytes
pub fn random_u32(bytes: &[u8; 32]) -> u32 {
    u32::from_le_bytes([bytes[0], bytes[3], bytes[7], bytes[12]])
}

/// Generates a random i32 value from a 32-byte random seed
///
/// # Arguments
///
/// * `bytes` - A 32-byte array containing random data from the VRF
///
/// # Returns
///
/// A random i32 value derived from the input bytes
pub fn random_i32(bytes: &[u8; 32]) -> i32 {
    random_u32(bytes) as i32
}

/// Generates a random u64 value from a 32-byte random seed
///
/// # Arguments
///
/// * `bytes` - A 32-byte array containing random data from the VRF
///
/// # Returns
///
/// A random u64 value derived from the input bytes
pub fn random_u64(bytes: &[u8; 32]) -> u64 {
    u64::from_le_bytes([
        bytes[0], bytes[4], bytes[8], bytes[12], bytes[16], bytes[20], bytes[24], bytes[28],
    ])
}

/// Generates a random i64 value from a 32-byte random seed
///
/// # Arguments
///
/// * `bytes` - A 32-byte array containing random data from the VRF
///
/// # Returns
///
/// A random i64 value derived from the input bytes
pub fn random_i64(bytes: &[u8; 32]) -> i64 {
    random_u64(bytes) as i64
}

/// Generates a random boolean value from a 32-byte random seed
///
/// # Arguments
///
/// * `bytes` - A 32-byte array containing random data from the VRF
///
/// # Returns
///
/// A random boolean value (true or false) derived from the input bytes
pub fn random_bool(bytes: &[u8; 32]) -> bool {
    (bytes[0] & 1) == 1
}
