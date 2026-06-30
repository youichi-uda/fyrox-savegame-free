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

    fn backup_path(&self, slot: &str) -> PathBuf {
        self.save_dir.join(format!("{}.fxsave.bak", slot))
    }

    /// Save game data to a named slot.
    ///
    /// If [`SaveConfig::keep_backup_on_overwrite`] is `true` and a slot file
    /// already exists, the previous file is moved to `<slot>.fxsave.bak`
    /// before the new file is written. The backup can be restored via
    /// [`Self::restore_backup`].
    pub fn save<T: Serialize>(
        &self,
        slot: &str,
        data: &T,
        metadata: SaveMetadata,
    ) -> SaveResult<()> {
        let bytes = save_file::encode_save(data, metadata, &self.config)?;
        let path = self.slot_path(slot);

        // Optional backup of the previous slot file before overwrite.
        if self.config.keep_backup_on_overwrite && path.exists() {
            let bak = self.backup_path(slot);
            std::fs::rename(&path, &bak).map_err(|source| SaveError::IoAt {
                path: bak.clone(),
                source,
            })?;
        }

        let tmp_path = path.with_extension("fxsave.tmp");
        std::fs::write(&tmp_path, &bytes).map_err(|source| SaveError::IoAt {
            path: tmp_path.clone(),
            source,
        })?;
        std::fs::rename(&tmp_path, &path).map_err(|source| SaveError::IoAt {
            path: path.clone(),
            source,
        })?;
        Ok(())
    }

    /// Restore the `.fxsave.bak` backup of a slot back to the live slot.
    ///
    /// Only meaningful when saves were written with
    /// [`SaveConfig::keep_backup_on_overwrite`] enabled. The current slot
    /// file (if any) is overwritten by the backup. Returns
    /// [`SaveError::SlotNotFound`] if no backup exists for the slot.
    pub fn restore_backup(&self, slot: &str) -> SaveResult<()> {
        let bak = self.backup_path(slot);
        if !bak.exists() {
            return Err(SaveError::SlotNotFound(format!("{slot}.fxsave.bak")));
        }
        let path = self.slot_path(slot);
        std::fs::rename(&bak, &path).map_err(|source| SaveError::IoAt {
            path: path.clone(),
            source,
        })?;
        Ok(())
    }

    /// Load game data from a named slot.
    pub fn load<T: DeserializeOwned>(&self, slot: &str) -> SaveResult<(T, SaveMetadata)> {
        let path = self.slot_path(slot);
        if !path.exists() {
            return Err(SaveError::SlotNotFound(slot.to_string()));
        }
        let bytes = std::fs::read(&path).map_err(|source| SaveError::IoAt {
            path: path.clone(),
            source,
        })?;
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

    /// Rename a slot file from `from` to `to`.
    ///
    /// Returns [`SaveError::SlotNotFound`] if `from` does not exist, and
    /// [`SaveError::IoAt`] (pointing at `to`) if `to` already exists (no
    /// overwrite). Use [`Self::delete_slot`] on `to` first if you want to
    /// overwrite.
    pub fn rename_slot(&self, from: &str, to: &str) -> SaveResult<()> {
        let src = self.slot_path(from);
        if !src.exists() {
            return Err(SaveError::SlotNotFound(from.to_string()));
        }
        let dst = self.slot_path(to);
        if dst.exists() {
            return Err(SaveError::IoAt {
                path: dst.clone(),
                source: std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    format!("destination slot '{to}' already exists"),
                ),
            });
        }
        std::fs::rename(&src, &dst).map_err(|source| SaveError::IoAt {
            path: dst.clone(),
            source,
        })?;
        Ok(())
    }

    /// Copy a slot file from `from` to `to`.
    ///
    /// Returns [`SaveError::SlotNotFound`] if `from` does not exist, and
    /// [`SaveError::IoAt`] (pointing at `to`) if `to` already exists (no
    /// overwrite).
    pub fn copy_slot(&self, from: &str, to: &str) -> SaveResult<()> {
        let src = self.slot_path(from);
        if !src.exists() {
            return Err(SaveError::SlotNotFound(from.to_string()));
        }
        let dst = self.slot_path(to);
        if dst.exists() {
            return Err(SaveError::IoAt {
                path: dst.clone(),
                source: std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    format!("destination slot '{to}' already exists"),
                ),
            });
        }
        std::fs::copy(&src, &dst).map_err(|source| SaveError::IoAt {
            path: dst.clone(),
            source,
        })?;
        Ok(())
    }

    /// Delete a save slot.
    pub fn delete_slot(&self, slot: &str) -> SaveResult<()> {
        let path = self.slot_path(slot);
        if path.exists() {
            std::fs::remove_file(&path).map_err(|source| SaveError::IoAt {
                path: path.clone(),
                source,
            })?;
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
                match self.peek_metadata_at(&path) {
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
        slots.sort_by_key(|s| std::cmp::Reverse(s.metadata.updated_at));
        Ok(slots)
    }

    /// Peek the [`SaveMetadata`] header of a slot without fully deserializing
    /// the payload.
    ///
    /// This reads the slot file and decodes only the leading envelope
    /// (magic + version + compression + metadata), so for non-trivial saves
    /// this is significantly faster than a full [`Self::load`] and avoids
    /// having to know the user's `T: Deserialize` type at the call site.
    ///
    /// Returns [`SaveError::SlotNotFound`] if the slot file does not exist
    /// and [`SaveError::IoAt`] for filesystem errors with the offending path.
    pub fn peek_metadata(&self, slot: &str) -> SaveResult<SaveMetadata> {
        let path = self.slot_path(slot);
        if !path.exists() {
            return Err(SaveError::SlotNotFound(slot.to_string()));
        }
        self.peek_metadata_at(&path)
    }

    fn peek_metadata_at(&self, path: &Path) -> SaveResult<SaveMetadata> {
        let bytes = std::fs::read(path).map_err(|source| SaveError::IoAt {
            path: path.to_path_buf(),
            source,
        })?;
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

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_temp_dir(name: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("fyrox_sg_free_unit_{name}_{nanos}"));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn peek_metadata_roundtrip() {
        use crate::metadata::SaveMetadata;

        let dir = unique_temp_dir("peek_metadata_roundtrip");
        let manager = SaveManager::new(&dir, SaveConfig::default()).unwrap();

        let payload: Vec<u32> = (0..1000).collect();
        let meta = SaveMetadata::new()
            .with_description("Peek test")
            .with_play_time(123.0)
            .with_custom("zone", "F1");
        manager.save("peek_slot", &payload, meta).unwrap();

        // Public peek API: returns the metadata without needing the payload
        // type, and (per design) without deserializing the payload.
        let peeked = manager.peek_metadata("peek_slot").unwrap();
        assert_eq!(peeked.description, "Peek test");
        assert_eq!(peeked.play_time_secs, 123.0);
        assert_eq!(peeked.custom.get("zone").unwrap(), "F1");

        // Missing slot -> SlotNotFound, not an IoAt.
        let err = manager.peek_metadata("does_not_exist").unwrap_err();
        assert!(matches!(err, SaveError::SlotNotFound(_)));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rename_slot_moves_file_and_refuses_overwrite() {
        use crate::metadata::SaveMetadata;

        let dir = unique_temp_dir("rename_slot");
        let manager = SaveManager::new(&dir, SaveConfig::default()).unwrap();

        manager.save("a", &42u32, SaveMetadata::new()).unwrap();
        assert!(manager.slot_exists("a"));
        assert!(!manager.slot_exists("b"));

        manager.rename_slot("a", "b").unwrap();
        assert!(!manager.slot_exists("a"));
        assert!(manager.slot_exists("b"));
        let (val, _): (u32, _) = manager.load("b").unwrap();
        assert_eq!(val, 42);

        // Rename of missing source returns SlotNotFound.
        let err = manager.rename_slot("a", "c").unwrap_err();
        assert!(matches!(err, SaveError::SlotNotFound(_)));

        // Refuses to overwrite an existing destination.
        manager.save("c", &7u32, SaveMetadata::new()).unwrap();
        let err = manager.rename_slot("b", "c").unwrap_err();
        match err {
            SaveError::IoAt { path, source } => {
                assert!(path.ends_with("c.fxsave"));
                assert_eq!(source.kind(), std::io::ErrorKind::AlreadyExists);
            }
            other => panic!("expected IoAt(AlreadyExists), got {other:?}"),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn copy_slot_duplicates_file_and_refuses_overwrite() {
        use crate::metadata::SaveMetadata;

        let dir = unique_temp_dir("copy_slot");
        let manager = SaveManager::new(&dir, SaveConfig::default()).unwrap();

        manager.save("src", &99u32, SaveMetadata::new()).unwrap();
        manager.copy_slot("src", "dst").unwrap();

        assert!(manager.slot_exists("src"));
        assert!(manager.slot_exists("dst"));
        let (a, _): (u32, _) = manager.load("src").unwrap();
        let (b, _): (u32, _) = manager.load("dst").unwrap();
        assert_eq!(a, 99);
        assert_eq!(b, 99);

        // Missing source.
        let err = manager.copy_slot("nope", "x").unwrap_err();
        assert!(matches!(err, SaveError::SlotNotFound(_)));

        // Existing destination.
        let err = manager.copy_slot("src", "dst").unwrap_err();
        match err {
            SaveError::IoAt { path, source } => {
                assert!(path.ends_with("dst.fxsave"));
                assert_eq!(source.kind(), std::io::ErrorKind::AlreadyExists);
            }
            other => panic!("expected IoAt(AlreadyExists), got {other:?}"),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn keep_backup_on_overwrite_creates_bak_file() {
        use crate::metadata::SaveMetadata;

        let dir = unique_temp_dir("backup_create");
        let config = SaveConfig {
            keep_backup_on_overwrite: true,
            ..SaveConfig::default()
        };
        let manager = SaveManager::new(&dir, config).unwrap();

        manager
            .save("slot1", &1u32, SaveMetadata::new().with_description("v1"))
            .unwrap();
        // First save: no previous file, no backup expected.
        assert!(!dir.join("slot1.fxsave.bak").exists());

        manager
            .save("slot1", &2u32, SaveMetadata::new().with_description("v2"))
            .unwrap();
        // Second save: backup file must exist.
        assert!(dir.join("slot1.fxsave.bak").exists());

        // Live slot now holds v2.
        let (val, meta): (u32, _) = manager.load("slot1").unwrap();
        assert_eq!(val, 2);
        assert_eq!(meta.description, "v2");

        // list_slots must not surface the .bak file as a slot.
        let slots = manager.list_slots().unwrap();
        assert_eq!(slots.len(), 1);
        assert_eq!(slots[0].name, "slot1");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn restore_backup_reverts_to_previous_version() {
        use crate::metadata::SaveMetadata;

        let dir = unique_temp_dir("backup_restore");
        let config = SaveConfig {
            keep_backup_on_overwrite: true,
            ..SaveConfig::default()
        };
        let manager = SaveManager::new(&dir, config).unwrap();

        manager
            .save("slot1", &10u32, SaveMetadata::new().with_description("first"))
            .unwrap();
        manager
            .save(
                "slot1",
                &20u32,
                SaveMetadata::new().with_description("second"),
            )
            .unwrap();

        // Confirm current live save is "second"/20.
        let (val, meta): (u32, _) = manager.load("slot1").unwrap();
        assert_eq!(val, 20);
        assert_eq!(meta.description, "second");

        // Restore: live slot should become "first"/10 again, .bak consumed.
        manager.restore_backup("slot1").unwrap();
        assert!(!dir.join("slot1.fxsave.bak").exists());
        let (val, meta): (u32, _) = manager.load("slot1").unwrap();
        assert_eq!(val, 10);
        assert_eq!(meta.description, "first");

        // Restoring again with no backup present returns SlotNotFound.
        let err = manager.restore_backup("slot1").unwrap_err();
        assert!(matches!(err, SaveError::SlotNotFound(_)));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ioat_carries_path_on_missing_file() {
        let dir = unique_temp_dir("ioat_missing");
        let manager = SaveManager::new(&dir, SaveConfig::default()).unwrap();

        // Load a slot whose .fxsave was created but is unreadable garbage so
        // bincode fails; alternatively, use peek on a path that does not
        // exist. We construct a deliberately missing slot file path and call
        // peek_metadata_at (used internally by list_slots) to confirm the
        // IoAt variant carries the offending path.
        let bogus = dir.join("does_not_exist.fxsave");
        let err = manager.peek_metadata_at(&bogus).unwrap_err();
        match err {
            SaveError::IoAt { path, source: _ } => {
                assert_eq!(path, bogus, "IoAt must report the path that failed");
            }
            other => panic!("expected SaveError::IoAt, got {other:?}"),
        }

        let _ = std::fs::remove_dir_all(&dir);
    }
}
