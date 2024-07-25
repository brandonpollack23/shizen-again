//! Generic storage for TODOs
use crate::result::ShizenResult;

pub mod rusqlite;

pub trait TodoStorage {
  // Create
  fn create_new_note() -> ShizenResult<()>;
  // Read
  fn load_all_notes() -> ShizenResult<()>;
  // Update
  // TODO

  // Delete
  fn delete_note() -> ShizenResult<()>;
}

// TODO write down the sync strat (list of peers with their claimed sync version, request with stuff more, apply and overwrite conflicts for now, rewrite my own changes back on top)
