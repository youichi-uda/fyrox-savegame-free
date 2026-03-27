//! Basic usage example for fyrox-savegame-free.
//!
//! Run with: cargo run --example basic_usage

use fyrox_savegame_free::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct GameState {
    player_name: String,
    level: u32,
    hp: f32,
    position: [f64; 3],
    inventory: Vec<String>,
    quest_flags: Vec<bool>,
}

fn main() {
    let config = SaveConfig {
        version: 1,
        compression: CompressionKind::default(),
    };

    let manager = SaveManager::new("./example_saves", config).expect("Failed to create save dir");

    let state = GameState {
        player_name: "Alice".into(),
        level: 15,
        hp: 92.5,
        position: [120.0, 5.0, -80.0],
        inventory: vec!["Iron Sword".into(), "Health Potion x3".into()],
        quest_flags: vec![true, true, false, false],
    };

    let metadata = SaveMetadata::new()
        .with_description("Before the boss fight")
        .with_play_time(5400.0)
        .with_custom("chapter", "4")
        .with_custom("location", "Shadow Castle");

    manager.save("slot_1", &state, metadata).expect("Save failed");
    println!("Saved to slot_1!");

    let slots = manager.list_slots().expect("Failed to list slots");
    println!("\nAvailable saves:");
    for slot in &slots {
        println!(
            "  [{}] {} - play time: {:.0}s",
            slot.name, slot.metadata.description, slot.metadata.play_time_secs
        );
    }

    let (loaded, meta): (GameState, SaveMetadata) = manager.load("slot_1").expect("Load failed");
    println!("\nLoaded from slot_1:");
    println!("  Player: {} (Lv.{})", loaded.player_name, loaded.level);
    println!("  HP: {}", loaded.hp);
    println!("  Inventory: {:?}", loaded.inventory);
    println!("  Description: {}", meta.description);
    println!("  Custom data: {:?}", meta.custom);

    manager
        .quick_save(&state, SaveMetadata::new().with_description("Quicksave"))
        .expect("Quick save failed");
    let (qs, _): (GameState, SaveMetadata) = manager.quick_load().expect("Quick load failed");
    println!("\nQuick-loaded: {} Lv.{}", qs.player_name, qs.level);

    std::fs::remove_dir_all("./example_saves").ok();
    println!("\nDone!");
}
