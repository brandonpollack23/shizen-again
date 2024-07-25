//! Generic storage for TODOs
use crate::{
  entities::{Note, NoteId},
  result::ShizenResult,
};

pub mod rusqlite;

pub trait TodoStorage {
  // Create
  fn create_new_note(
    &self, title: &str, description: Option<&str>, parent: Option<NoteId>,
  ) -> ShizenResult<Note>;
  // Read
  fn load_all_notes(&self) -> ShizenResult<()>;
  // Update
  // TODO

  // Delete
  fn delete_note(&self) -> ShizenResult<()>;
}

// TODO write down the sync strat (list of peers with their claimed sync version, request with stuff more, apply and overwrite conflicts for now, rewrite my own changes back on top)
