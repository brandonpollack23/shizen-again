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
