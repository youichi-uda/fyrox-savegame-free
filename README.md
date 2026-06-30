# fyrox-savegame-free

Game-oriented save/load system for the [Fyrox](https://fyrox.rs) game engine.

While Rust has `serde`, there's no turnkey solution for **game saves** specifically. This crate fills that gap: save slots, rich metadata, data versioning, compression, and integrity checks -- all in one package.

## Quick Start

```rust
use fyrox_savegame_free::*;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct GameState {
    player_name: String,
    level: u32,
    hp: f32,
}

fn main() {
    let manager = SaveManager::new("./saves", SaveConfig::default()).unwrap();

    let state = GameState {
        player_name: "Alice".into(),
        level: 15,
        hp: 92.5,
    };

    // Save with metadata
    let meta = SaveMetadata::new()
        .with_description("Before the boss fight")
        .with_play_time(5400.0)
        .with_custom("chapter", "4");

    manager.save("slot_1", &state, meta).unwrap();

    // Load it back
    let (loaded, meta): (GameState, SaveMetadata) = manager.load("slot_1").unwrap();

    // Quick save / quick load
    manager.quick_save(&state, SaveMetadata::new()).unwrap();
    let (qs, _): (GameState, SaveMetadata) = manager.quick_load().unwrap();
}
```

## Features

| Feature | Free | Pro |
|---------|:----:|:---:|
| Named save slots | :white_check_mark: | :white_check_mark: |
| Quick save / quick load | :white_check_mark: | :white_check_mark: |
| Rich metadata (timestamp, play time, description, thumbnails, custom KV) | :white_check_mark: | :white_check_mark: |
| Save versioning & migration | :white_check_mark: | :white_check_mark: |
| LZ4 compression | :white_check_mark: | :white_check_mark: |
| CRC32 integrity checks | :white_check_mark: | :white_check_mark: |
| Atomic writes (crash-safe) | :white_check_mark: | :white_check_mark: |
| SHA-256 integrity | | :white_check_mark: |
| AES-256-GCM encryption | | :white_check_mark: |
| Zstd compression | | :white_check_mark: |
| Async background saving | | :white_check_mark: |
| Auto-save with rotating slots | | :white_check_mark: |
| Priority support | | :white_check_mark: |

## Save Versioning & Migration

When your game data format changes between updates, register migrations to automatically upgrade old saves:

```rust
let mut migrations = MigrationRegistry::new();

// v1 stored hp as u32, v2 stores as f32
migrations.register(1, |old_data| {
    let old: OldState = bincode::deserialize(&old_data)?;
    let new = NewState { hp: old.hp as f32, ..Default::default() };
    Ok(bincode::serialize(&new)?)
});

let manager = SaveManager::new("./saves", SaveConfig { version: 2, ..Default::default() })
    .unwrap()
    .with_migrations(migrations);
```

## Installation

```toml
[dependencies]
fyrox-savegame-free = "0.1"
```

## Engine Integration

This crate is engine-agnostic: it depends only on `serde`/`bincode`, so it works with
Fyrox or any other Rust game. Serialize your own game-state structs and store them in
named slots -- there are no engine-specific bindings to wire up.

## Pro Version

Need encryption, async saving, or Zstd compression?

**[Get fyrox-savegame-pro on itch.io](https://y1uda.itch.io/fyrox-savegame-pro)**

## License

MIT
