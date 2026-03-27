use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Rich metadata attached to every save file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveMetadata {
    /// Unix timestamp (seconds since epoch) when the save was created.
    pub created_at: i64,
    /// Unix timestamp when the save was last updated (overwritten).
    pub updated_at: i64,
    /// Total play time in seconds at the time of saving.
    pub play_time_secs: f64,
    /// Human-readable description (e.g. "Chapter 3 - Boss fight").
    pub description: String,
    /// Optional thumbnail as raw bytes (PNG/JPEG). Keep it small.
    pub thumbnail: Option<Vec<u8>>,
    /// Arbitrary key-value pairs for game-specific info
    /// (e.g. "level" = "12", "location" = "Dark Forest").
    pub custom: HashMap<String, String>,
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

impl SaveMetadata {
    pub fn new() -> Self {
        let now = unix_now();
        Self {
            created_at: now,
            updated_at: now,
            play_time_secs: 0.0,
            description: String::new(),
            thumbnail: None,
            custom: HashMap::new(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_play_time(mut self, secs: f64) -> Self {
        self.play_time_secs = secs;
        self
    }

    pub fn with_thumbnail(mut self, data: Vec<u8>) -> Self {
        self.thumbnail = Some(data);
        self
    }

    pub fn with_custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom.insert(key.into(), value.into());
        self
    }
}

impl Default for SaveMetadata {
    fn default() -> Self {
        Self::new()
    }
}
