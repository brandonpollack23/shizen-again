pub mod entities;
mod result;
pub mod storage;
mod sync;

pub use result::*;
pub use sync::SyncServer;

pub type DefaultStorage = storage::rusqlite::RusqliteStorage;

// TODO list
// * user defined ordering
// * cli support for sort ordering
// * dioxus UI for desktop/tui
// * tracing spans/benchmarks/profiling
// * Work to run as systemd service server with cli.
// * merge conflict detection/handling
// * Attempt to port to web by making a rusqlite that runs in browser
//   (either by reimplementing sqlite interface as a trait and delegating to sqlite wasm in Rusqlite, or by using WASI and vfs feature of rusqlite)
