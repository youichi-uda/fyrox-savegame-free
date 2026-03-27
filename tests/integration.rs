use fyrox_savegame_free::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct PlayerState {
    name: String,
    level: u32,
    hp: f32,
    position: [f64; 3],
    inventory: Vec<String>,
}

fn sample_player() -> PlayerState {
    PlayerState {
        name: "Hero".to_string(),
        level: 42,
        hp: 87.5,
        position: [100.0, 0.5, -200.0],
        inventory: vec!["Sword".into(), "Shield".into(), "Potion".into()],
    }
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fyrox_sg_free_test_{}", name));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn test_save_and_load_roundtrip() {
    let dir = temp_dir("roundtrip");
    let manager = SaveManager::new(&dir, SaveConfig::default()).unwrap();

    let player = sample_player();
    let meta = SaveMetadata::new()
        .with_description("Test save")
        .with_play_time(3600.0)
        .with_custom("chapter", "3");

    manager.save("slot1", &player, meta).unwrap();
    assert!(manager.slot_exists("slot1"));

    let (loaded, loaded_meta): (PlayerState, SaveMetadata) = manager.load("slot1").unwrap();
    assert_eq!(loaded, player);
    assert_eq!(loaded_meta.description, "Test save");
    assert_eq!(loaded_meta.play_time_secs, 3600.0);
    assert_eq!(loaded_meta.custom.get("chapter").unwrap(), "3");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_quick_save_load() {
    let dir = temp_dir("quicksave");
    let manager = SaveManager::new(&dir, SaveConfig::default()).unwrap();

    let player = sample_player();
    manager
        .quick_save(&player, SaveMetadata::new().with_description("Quick!"))
        .unwrap();

    let (loaded, meta): (PlayerState, SaveMetadata) = manager.quick_load().unwrap();
    assert_eq!(loaded, player);
    assert_eq!(meta.description, "Quick!");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_list_slots() {
    let dir = temp_dir("list_slots");
    let manager = SaveManager::new(&dir, SaveConfig::default()).unwrap();

    let player = sample_player();
    manager
        .save("save_a", &player, SaveMetadata::new().with_description("A"))
        .unwrap();
    std::thread::sleep(std::time::Duration::from_secs(1));
    manager
        .save("save_b", &player, SaveMetadata::new().with_description("B"))
        .unwrap();

    let slots = manager.list_slots().unwrap();
    assert_eq!(slots.len(), 2);
    assert_eq!(slots[0].name, "save_b");
    assert_eq!(slots[1].name, "save_a");

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_delete_slot() {
    let dir = temp_dir("delete_slot");
    let manager = SaveManager::new(&dir, SaveConfig::default()).unwrap();

    let player = sample_player();
    manager.save("doomed", &player, SaveMetadata::new()).unwrap();
    assert!(manager.slot_exists("doomed"));

    manager.delete_slot("doomed").unwrap();
    assert!(!manager.slot_exists("doomed"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_slot_not_found() {
    let dir = temp_dir("not_found");
    let manager = SaveManager::new(&dir, SaveConfig::default()).unwrap();

    let result: SaveResult<(PlayerState, SaveMetadata)> = manager.load("nonexistent");
    assert!(matches!(result.unwrap_err(), SaveError::SlotNotFound(_)));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn test_encode_decode_no_compression() {
    let config = SaveConfig {
        version: 1,
        compression: CompressionKind::None,
    };

    let player = sample_player();
    let bytes = encode_save(&player, SaveMetadata::new().with_description("raw"), &config).unwrap();
    let (loaded, meta): (PlayerState, SaveMetadata) = decode_save(&bytes, &config, None).unwrap();

    assert_eq!(loaded, player);
    assert_eq!(meta.description, "raw");
}

#[test]
fn test_corrupted_data_detected() {
    let config = SaveConfig::default();
    let player = sample_player();

    let mut bytes = encode_save(&player, SaveMetadata::new(), &config).unwrap();
    if let Some(b) = bytes.last_mut() {
        *b ^= 0xFF;
    }

    let result: SaveResult<(PlayerState, SaveMetadata)> = decode_save(&bytes, &config, None);
    assert!(result.is_err());
}

#[test]
fn test_migration() {
    let config_v1 = SaveConfig {
        version: 1,
        compression: CompressionKind::None,
    };
    let bytes = encode_save(&100u32, SaveMetadata::new(), &config_v1).unwrap();

    let config_v2 = SaveConfig {
        version: 2,
        compression: CompressionKind::None,
    };

    let mut migrations = MigrationRegistry::new();
    migrations.register(1, |old_data| {
        let val: u32 = bincode::deserialize(&old_data).unwrap();
        Ok(bincode::serialize(&(val, "migrated".to_string())).unwrap())
    });

    let (loaded, _): ((u32, String), SaveMetadata) =
        decode_save(&bytes, &config_v2, Some(&migrations)).unwrap();

    assert_eq!(loaded.0, 100);
    assert_eq!(loaded.1, "migrated");
}

#[test]
fn test_metadata_builder() {
    let meta = SaveMetadata::new()
        .with_description("Chapter 5")
        .with_play_time(7200.0)
        .with_thumbnail(vec![0xFF, 0xD8, 0xFF])
        .with_custom("location", "Dark Forest")
        .with_custom("difficulty", "Hard");

    assert_eq!(meta.description, "Chapter 5");
    assert_eq!(meta.play_time_secs, 7200.0);
    assert!(meta.thumbnail.is_some());
    assert_eq!(meta.custom.len(), 2);
    assert_eq!(meta.custom["location"], "Dark Forest");
}
