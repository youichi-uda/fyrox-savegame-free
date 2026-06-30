//! # fyrox-savegame-free
//!
//! Game-oriented save/load system built on top of serde.
//!
//! ## Features
//!
//! - **Save slots** with named or numbered slots, quick-save/quick-load
//! - **Rich metadata**: timestamps, play time, description, custom key-value pairs, optional
//!   thumbnail
//! - **Versioning & migration**: automatic migration from older save formats
//! - **Compression**: LZ4 (default)
//! - **Integrity checks**: CRC32 (always on)
//! - **Atomic writes**: temp-file + rename to prevent corruption on crash
//!
//! ## Pro Version
//!
//! The Pro version adds SHA-256 integrity, AES-256-GCM encryption, Zstd compression,
//! async background saving, and auto-save with rotating slots.
//!
//! This crate is engine-agnostic (it depends only on `serde`/`bincode`), so it works
//! with Fyrox or any other Rust game.

mod error;
mod integrity;
mod metadata;
mod migration;
mod save_file;
mod slot;

pub use error::*;
pub use integrity::*;
pub use metadata::*;
pub use migration::*;
pub use save_file::*;
pub use slot::*;
