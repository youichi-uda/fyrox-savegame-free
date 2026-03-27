use crate::error::{SaveError, SaveResult};

/// Compute CRC32 checksum of data.
pub fn crc32(data: &[u8]) -> u32 {
    crc32fast::hash(data)
}

/// Verify CRC32 checksum.
pub fn verify_crc32(data: &[u8], expected: u32) -> SaveResult<()> {
    let got = crc32(data);
    if got != expected {
        Err(SaveError::Crc32Mismatch { expected, got })
    } else {
        Ok(())
    }
}
