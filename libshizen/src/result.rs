//! Result and error types for libshizen
use thiserror::Error;

pub type ShizenResult<T> = std::result::Result<T, ShizenError>;

// TODO thiserror
#[derive(Error, Debug)]
pub enum ShizenError {
  #[error("Rusqlite internal error")]
  RusqliteError(#[from] rusqlite::Error),
  #[error("Error migrating database schema")]
  MigrationError(usize, rusqlite::Error),
}
