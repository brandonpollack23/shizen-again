//! Generic storage for TODOs
use crate::{
  entities::{Note, NoteId},
  result::ShizenResult,
};

pub mod rusqlite;

pub trait TodoStorage {
  // Create
  fn create_new_note(
    &mut self, title: &str, description: Option<&str>, parent: Option<NoteId>,
  ) -> ShizenResult<Note>;
  // Read
  fn load_all_notes(&self) -> ShizenResult<Vec<Note>>;
  fn load_note(&self, note_id: &NoteId) -> ShizenResult<Note>;
  fn note_exists(&self, note_id: &NoteId) -> ShizenResult<bool>;
  fn get_all_descendents(&self, note_id: &NoteId) -> ShizenResult<Vec<Note>>;
  // Update
  // TODO

  // Delete
  fn delete_note(&mut self, note_id: &NoteId) -> crate::ShizenResult<usize>;
}

// TODO write down the sync strat (list of peers with their claimed sync version, request with stuff more, apply and overwrite conflicts for now, rewrite my own changes back on top)
