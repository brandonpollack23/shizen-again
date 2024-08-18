//! Result and error types for libshizen
use thiserror::Error;

use crate::{
  entities::{NoteId, PeerId},
  sync::SyncResponse,
};

pub type ShizenResult<T> = std::result::Result<T, ShizenError>;

#[derive(Error, Debug)]
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
  #[error("Could not obtain lock for database")]
  CouldNotLockDatabase,
  #[error("Serialization/deserialization error")]
  SerdeError(#[from] serde_json::Error),
  #[error("Could not connect to tcp socket: {:?}", .0)]
  PeerTcpConnectionFailed(#[from] std::io::Error),
  #[error("Expected response {}, but got response {:#?}", .0, .1)]
  UnexpectedSyncProtocolResponse(String, SyncResponse),
  #[error("Cannot be a peer of self")]
  CannotBePeerOfSelf,
  #[error("Peer already exists: {}", .0.0.to_string())]
  PeerAlreadyExists(PeerId),
  #[error("Invalid position, one input must be something")]
  InvalidRankAdjustment,
  #[error("Lexorank parse error in rank column: {}", .0)]
  LexorankParseError(#[from] lexorank::error::ParseError),
}
