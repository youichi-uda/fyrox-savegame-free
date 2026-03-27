use crate::error::{SaveError, SaveResult};
use std::collections::BTreeMap;

/// A function that transforms raw save data from version N to version N+1.
pub type MigrationFn = Box<dyn Fn(Vec<u8>) -> SaveResult<Vec<u8>> + Send + Sync>;

/// Registry of migration functions for upgrading old save data.
///
/// Each migration transforms the raw payload bytes from one version to the next.
/// Migrations are applied sequentially: v1 -> v2 -> v3 -> ... -> current.
pub struct MigrationRegistry {
    migrations: BTreeMap<u32, MigrationFn>,
}

impl MigrationRegistry {
    pub fn new() -> Self {
        Self {
            migrations: BTreeMap::new(),
        }
    }

    /// Register a migration from `from_version` to `from_version + 1`.
    pub fn register(
        &mut self,
        from_version: u32,
        f: impl Fn(Vec<u8>) -> SaveResult<Vec<u8>> + Send + Sync + 'static,
    ) {
        self.migrations.insert(from_version, Box::new(f));
    }

    /// Apply all necessary migrations to bring data from `from_version` up to `target_version`.
    pub fn migrate(
        &self,
        mut data: Vec<u8>,
        from_version: u32,
        target_version: u32,
    ) -> SaveResult<Vec<u8>> {
        for v in from_version..target_version {
            let migration = self
                .migrations
                .get(&v)
                .ok_or(SaveError::NoMigration(v))?;
            data = migration(data)?;
        }
        Ok(data)
    }

    /// Returns the highest version that can be migrated *to*
    /// (i.e. max registered source version + 1), or 0 if empty.
    pub fn max_target_version(&self) -> u32 {
        self.migrations
            .keys()
            .last()
            .map(|v| v + 1)
            .unwrap_or(0)
    }
}

impl Default for MigrationRegistry {
    fn default() -> Self {
        Self::new()
    }
}
