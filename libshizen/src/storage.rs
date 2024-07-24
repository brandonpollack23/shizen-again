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

// TODO setup schema and versioning/migrations. https://chatgpt.com/share/06f0d605-0b3c-45b1-b0ab-5723628e9574
// TODO write down the sync strat (list of peers with their claimed sync version, request with stuff more, apply and overwrite conflicts for now, rewrite my own changes back on top)
