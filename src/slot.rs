use crate::error::{SaveError, SaveResult};
use crate::metadata::SaveMetadata;
use crate::migration::MigrationRegistry;
use crate::save_file::{self, SaveConfig};
use serde::{de::DeserializeOwned, Serialize};
use std::path::{Path, PathBuf};

/// High-level save slot manager.
///
/// Manages a directory of save files organized by slot name.
/// Each slot is a single file: `<slot_name>.fxsave`.
pub struct SaveManager {
    save_dir: PathBuf,
    config: SaveConfig,
    migrations: Option<MigrationRegistry>,
}

impl SaveManager {
    /// Create a new SaveManager.
    ///
    /// `save_dir` is created if it does not exist.
    pub fn new(save_dir: impl Into<PathBuf>, config: SaveConfig) -> SaveResult<Self> {
        let save_dir = save_dir.into();
        std::fs::create_dir_all(&save_dir)?;
        Ok(Self {
            save_dir,
            config,
            migrations: None,
        })
    }

    /// Attach a migration registry for loading old save versions.
    pub fn with_migrations(mut self, registry: MigrationRegistry) -> Self {
        self.migrations = Some(registry);
        self
    }

    fn slot_path(&self, slot: &str) -> PathBuf {
        self.save_dir.join(format!("{}.fxsave", slot))
    }

    /// Save game data to a named slot.
    pub fn save<T: Serialize>(
        &self,
        slot: &str,
        data: &T,
        metadata: SaveMetadata,
    ) -> SaveResult<()> {
        let bytes = save_file::encode_save(data, metadata, &self.config)?;
        let path = self.slot_path(slot);
        let tmp_path = path.with_extension("fxsave.tmp");
        std::fs::write(&tmp_path, &bytes)?;
        std::fs::rename(&tmp_path, &path)?;
        Ok(())
    }

    /// Load game data from a named slot.
    pub fn load<T: DeserializeOwned>(&self, slot: &str) -> SaveResult<(T, SaveMetadata)> {
        let path = self.slot_path(slot);
        if !path.exists() {
            return Err(SaveError::SlotNotFound(slot.to_string()));
        }
        let bytes = std::fs::read(&path)?;
        save_file::decode_save(&bytes, &self.config, self.migrations.as_ref())
    }

    /// Quick-save to a dedicated "quicksave" slot.
    pub fn quick_save<T: Serialize>(&self, data: &T, metadata: SaveMetadata) -> SaveResult<()> {
        self.save("quicksave", data, metadata)
    }

    /// Quick-load from the "quicksave" slot.
    pub fn quick_load<T: DeserializeOwned>(&self) -> SaveResult<(T, SaveMetadata)> {
        self.load("quicksave")
    }

    /// Delete a save slot.
    pub fn delete_slot(&self, slot: &str) -> SaveResult<()> {
        let path = self.slot_path(slot);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    /// Check if a save slot exists.
    pub fn slot_exists(&self, slot: &str) -> bool {
        self.slot_path(slot).exists()
    }

    /// List all save slots with their metadata (sorted by last updated, newest first).
    pub fn list_slots(&self) -> SaveResult<Vec<SlotInfo>> {
        let mut slots = Vec::new();
        for entry in std::fs::read_dir(&self.save_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "fxsave") {
                let slot_name = path
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                match self.peek_metadata(&path) {
                    Ok(meta) => slots.push(SlotInfo {
                        name: slot_name,
                        metadata: meta,
                    }),
                    Err(_) => {
                        slots.push(SlotInfo {
                            name: slot_name,
                            metadata: SaveMetadata::new(),
                        });
                    }
                }
            }
        }
        slots.sort_by(|a, b| b.metadata.updated_at.cmp(&a.metadata.updated_at));
        Ok(slots)
    }

    fn peek_metadata(&self, path: &Path) -> SaveResult<SaveMetadata> {
        let bytes = std::fs::read(path)?;
        #[derive(serde::Deserialize)]
        struct Envelope {
            magic: u32,
            #[allow(dead_code)]
            version: u32,
            #[allow(dead_code)]
            compression: crate::save_file::CompressionKind,
            metadata: SaveMetadata,
        }
        let env: Envelope = bincode::deserialize(&bytes)?;
        if env.magic != 0x46585347 {
            return Err(SaveError::InvalidMagic {
                expected: 0x46585347,
                got: env.magic,
            });
        }
        Ok(env.metadata)
    }

    /// Get the save directory path.
    pub fn save_dir(&self) -> &Path {
        &self.save_dir
    }
}

/// Summary of a save slot for UI display.
#[derive(Debug, Clone)]
pub struct SlotInfo {
    pub name: String,
    pub metadata: SaveMetadata,
}
