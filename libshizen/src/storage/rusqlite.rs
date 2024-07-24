use rusqlite::Connection;

use crate::ShizenResult;

use super::TodoStorage;

struct RusqliteStorage {
  conn: Connection,
}

impl RusqliteStorage {
  pub fn new(db_path: std::path::PathBuf) -> ShizenResult<RusqliteStorage> {
    let conn = Connection::open(db_path)?;
    // TODO setup schema and versioning/migrations. https://chatgpt.com/share/06f0d605-0b3c-45b1-b0ab-5723628e9574
    Ok(RusqliteStorage { conn })
  }
}

impl TodoStorage for RusqliteStorage {
  fn create_new_note() -> crate::ShizenResult<()> {
    todo!()
  }

  fn load_all_notes() -> crate::ShizenResult<()> {
    todo!()
  }

  fn delete_note() -> crate::ShizenResult<()> {
    todo!()
  }
}
