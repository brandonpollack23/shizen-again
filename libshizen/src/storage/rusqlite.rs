use super::TodoStorage;

struct RusqliteStorage {}

impl RusqliteStorage {
  pub fn new() -> RusqliteStorage {
    RusqliteStorage {}
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
