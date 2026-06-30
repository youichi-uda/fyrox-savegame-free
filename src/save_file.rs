use crate::error::{SaveError, SaveResult};
use crate::integrity;
use crate::metadata::SaveMetadata;
use crate::migration::MigrationRegistry;
use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Magic bytes identifying a fyrox-savegame file: "FXSG"
const MAGIC: u32 = 0x46585347;

/// On-disk save file layout (header + payload).
#[derive(Serialize, Deserialize)]
struct SaveFileRaw {
    magic: u32,
    version: u32,
    compression: CompressionKind,
    metadata: SaveMetadata,
    /// CRC32 of the (possibly compressed) payload.
    crc32: u32,
    /// The game data payload (possibly compressed).
    payload: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionKind {
    None,
    #[cfg(feature = "compression-lz4")]
    Lz4,
}

// Not derivable: the default variant depends on the `compression-lz4` feature
// (Lz4 when enabled, None otherwise), which `#[derive(Default)]` cannot express.
#[allow(clippy::derivable_impls)]
impl Default for CompressionKind {
    fn default() -> Self {
        #[cfg(feature = "compression-lz4")]
        {
            CompressionKind::Lz4
        }
        #[cfg(not(feature = "compression-lz4"))]
        {
            CompressionKind::None
        }
    }
}

/// Configuration for how saves are written.
#[derive(Clone)]
pub struct SaveConfig {
    /// Current save data version.
    pub version: u32,
    /// Which compression to use.
    pub compression: CompressionKind,
    /// If `true`, [`crate::SaveManager::save`] moves the previous slot file
    /// (if any) to `<slot>.fxsave.bak` before atomically writing the new
    /// version. The backup can then be restored via
    /// [`crate::SaveManager::restore_backup`].
    ///
    /// Defaults to `false` so existing behavior is unchanged.
    pub keep_backup_on_overwrite: bool,
}

impl Default for SaveConfig {
    fn default() -> Self {
        Self {
            version: 1,
            compression: CompressionKind::default(),
            keep_backup_on_overwrite: false,
        }
    }
}

fn compress(data: &[u8], kind: CompressionKind) -> SaveResult<Vec<u8>> {
    match kind {
        CompressionKind::None => Ok(data.to_vec()),
        #[cfg(feature = "compression-lz4")]
        CompressionKind::Lz4 => Ok(lz4_flex::compress_prepend_size(data)),
    }
}

fn decompress(data: &[u8], kind: CompressionKind) -> SaveResult<Vec<u8>> {
    match kind {
        CompressionKind::None => Ok(data.to_vec()),
        #[cfg(feature = "compression-lz4")]
        CompressionKind::Lz4 => lz4_flex::decompress_size_prepended(data)
            .map_err(|e| SaveError::Decompression(e.to_string())),
    }
}

/// Serialize game state into a complete save file byte vector.
pub fn encode_save<T: Serialize>(
    data: &T,
    metadata: SaveMetadata,
    config: &SaveConfig,
) -> SaveResult<Vec<u8>> {
    let raw = bincode::serialize(data)?;
    let payload = compress(&raw, config.compression)?;
    let crc = integrity::crc32(&payload);

    let file = SaveFileRaw {
        magic: MAGIC,
        version: config.version,
        compression: config.compression,
        metadata,
        crc32: crc,
        payload,
    };

    bincode::serialize(&file).map_err(Into::into)
}

/// Decode a save file, optionally running migrations.
pub fn decode_save<T: DeserializeOwned>(
    bytes: &[u8],
    config: &SaveConfig,
    migrations: Option<&MigrationRegistry>,
) -> SaveResult<(T, SaveMetadata)> {
    let file: SaveFileRaw = bincode::deserialize(bytes)?;

    if file.magic != MAGIC {
        return Err(SaveError::InvalidMagic {
            expected: MAGIC,
            got: file.magic,
        });
    }

    integrity::verify_crc32(&file.payload, file.crc32)?;

    let raw = decompress(&file.payload, file.compression)?;

    let raw = if file.version < config.version {
        if let Some(registry) = migrations {
            registry.migrate(raw, file.version, config.version)?
        } else {
            return Err(SaveError::UnsupportedVersion {
                version: file.version,
                max_supported: config.version,
            });
        }
    } else {
        raw
    };

    let data: T = bincode::deserialize(&raw)?;
    Ok((data, file.metadata))
}
