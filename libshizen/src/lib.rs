pub mod entities;
mod result;
pub mod storage;

pub use result::*;

pub type DefaultStorage = storage::rusqlite::RusqliteStorage;
