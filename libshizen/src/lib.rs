pub mod entities;
mod result;
pub mod storage;
mod sync;

pub use result::*;

pub type DefaultStorage = storage::rusqlite::RusqliteStorage;

// TODO list
// * dioxus UI for desktop/tui
// * Work to run as systemd service server with cli.
// * Attempt to port to web by making a rusqlite that runs in browser
//   (either by reimplementing sqlite interface as a trait and delegating to sqlite wasm in Rusqlite, or by using WASI and vfs feature of rusqlite)
// * merge conflict detection/handling
