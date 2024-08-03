//! Result and error types for libshizen
use thiserror::Error;

use crate::entities::NoteId;

pub type ShizenResult<T> = std::result::Result<T, ShizenError>;

// TODO thiserror
#[derive(Error, Debug, PartialEq)]
pub enum ShizenError {
  #[error("Error creating databse {}", .0)]
  ErrorCreatingDb(String),
  #[error("No such file {}", .0)]
  ErrorOpeningDb(String),
  #[error("Rusqlite internal error")]
  RusqliteError(#[from] rusqlite::Error),
  #[error("Error processing UUID")]
  UuidError(#[from] uuid::Error),
  #[error("Error migrating database schema")]
  MigrationError(usize, rusqlite::Error),
  #[error("No note with the id {:?}", .0)]
  NoSuchNote(NoteId),
  #[error("Unexpected number of affected rows in db: expected {}, got {}", .0, .1)]
  UnexpectedMutationResult(usize, usize),
  /// Parent (0) -> Child (1)
  #[error("Circular reference would be created by parenting {} to {}", .1, .0)]
  ParentCircularReference(NoteId, NoteId),
  /// Blocker (0) -> Blockee (1)
  #[error("Circular reference would be created by making {} block {}", .0, .1)]
  DependencyCircularReference(NoteId, NoteId),
  /// Blocker (0) -> Blockee (1)
  #[error("There is no such dependency with blocker: {} blockee: {}", .0, .1)]
  NoSuchDependency(NoteId, NoteId),
}
