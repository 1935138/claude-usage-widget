//! Locating and aggregating Claude Code usage logs.
//!
//! Deliberately free of any UI or Tauri dependency: the same crate builds for
//! Windows (where it reads both the native logs and the WSL ones over the
//! `\\wsl.localhost` share) and for Linux, and can be exercised from the
//! `probe` binary on either.

pub mod discovery;
pub mod limits;
pub mod live;
pub mod settings;
pub mod usage;
