use thiserror::Error;

#[derive(Debug, Error)]
pub enum SaveError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialize(#[from] bincode::Error),

    #[error("Invalid save file magic: expected {expected:#010x}, got {got:#010x}")]
    InvalidMagic { expected: u32, got: u32 },

    #[error("CRC32 mismatch: expected {expected:#010x}, got {got:#010x}")]
    Crc32Mismatch { expected: u32, got: u32 },

    #[error("Unsupported save version {version} (max supported: {max_supported})")]
    UnsupportedVersion { version: u32, max_supported: u32 },

    #[error("No migration registered for version {0}")]
    NoMigration(u32),

    #[error("Save slot not found: {0}")]
    SlotNotFound(String),

    #[error("Compression error: {0}")]
    Compression(String),

    #[error("Decompression error: {0}")]
    Decompression(String),

    #[error("Encryption error: {0}")]
    Encryption(String),

    #[error("Decryption error: {0}")]
    Decryption(String),

    #[error("Integrity check failed: {0}")]
    Integrity(String),
}

pub type SaveResult<T> = Result<T, SaveError>;
