//! Sqlite storage engine using rusqlite.
use rusqlite::Connection;
use tracing::info;
use uuid::Uuid;

use crate::entities::{Note, NoteId};
use crate::storage::TodoStorage;
use crate::{ShizenError, ShizenResult};

struct RusqliteStorage {
  conn: Connection,
}

impl RusqliteStorage {
  pub fn new(db_path: Option<std::path::PathBuf>) -> ShizenResult<RusqliteStorage> {
    if db_path == None {
      info!("Opening sqlite database in memory");
    } else {
      info!("Opening sqlite database: {:?}", db_path.as_ref().unwrap());
    }

    let conn = if let Some(p) = db_path {
      Connection::open(p)?
    } else {
      Connection::open_in_memory()?
    };

    // TODO setup schema and versioning/migrations. https://chatgpt.com/share/06f0d605-0b3c-45b1-b0ab-5723628e9574
    if !database_is_initialized(&conn)? {
      info!("Initializing database...");
      conn.execute_batch(&libshizen_sql_init_str())?;
    }

    run_schema_migrations(&conn)?;

    info!("Database running at version {}", get_schema_version(&conn)?);

    Ok(RusqliteStorage { conn })
  }
}

impl TodoStorage for RusqliteStorage {
  fn create_new_note(
    &self, title: &str, description: Option<&str>, parent_id: Option<NoteId>,
  ) -> ShizenResult<Note> {
    if let Some(ref pid) = parent_id {
      if !self.note_exists(&pid)? {
        return Err(ShizenError::NoSuchNote(pid.clone()));
      }
    }

    let note_id = Uuid::new_v4();
    self.conn.execute(
      "INSERT INTO Notes (uuid, title, description, parent_id) VALUES (?, ?, ?, ?)",
      (note_id, title, description, parent_id.as_ref().map(|p| p.0)),
    )?;

    Ok(Note {
      id: NoteId(note_id),
      title: title.to_string(),
      description: description.map(|s| s.to_string()),
      parent_id,
    })
  }

  fn load_all_notes(&self) -> ShizenResult<Vec<Note>> {
    let mut stmt = self.conn.prepare(
      r#"
SELECT 
  id, title, description, parent_id 
FROM Notes
"#,
    )?;

    let result: ShizenResult<Vec<_>> = stmt
      .query_map((), |r| {
        Ok(Note {
          id: NoteId(r.get(0)?),
          title: r.get(1)?,
          description: r.get(2)?,
          parent_id: r.get::<_, Option<_>>(3)?.map(|p| NoteId(p)),
        })
      })?
      .map(|v| v.map_err(Into::into))
      .collect();

    result
  }

  fn load_note(&self) -> ShizenResult<Note> {
    todo!()
  }

  fn note_exists(&self, note_id: &NoteId) -> ShizenResult<bool> {
    Ok(self.conn.query_row(
      "SELECT count(id) FROM Notes where id = ?",
      [note_id.0],
      |r| r.get::<_, bool>(0),
    )?)
  }

  fn delete_note(&self) -> crate::ShizenResult<()> {
    todo!()
  }
}

fn database_is_initialized(conn: &Connection) -> ShizenResult<bool> {
  Ok(conn.query_row(&check_initialized_str(), (), |row| Ok(row.get(0)?))?)
}

macro_rules! sqlite_str {
  ($path:expr) => {
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), $path))
  };
}

pub fn libshizen_sql_init_str() -> &'static str {
  sqlite_str!("/sql/sqlite/init.sql")
}

pub fn check_initialized_str() -> &'static str {
  "SELECT count(name) FROM sqlite_master WHERE type='table' AND name='SchemaVersion'"
}

const CURRENT_VERSION: usize = 1;
const SCHEMA_MIGRATIONS: [&'static str; 0] = [];

fn run_schema_migrations(conn: &Connection) -> ShizenResult<()> {
  let version = get_schema_version(conn)?;

  for v in version..CURRENT_VERSION {
    conn
      .execute_batch(SCHEMA_MIGRATIONS[v])
      .map_err(|e| ShizenError::MigrationError(v, e))?;
  }

  Ok(())
}

fn get_schema_version(conn: &Connection) -> Result<usize, ShizenError> {
  Ok(conn.query_row("SELECT version FROM SchemaVersion", (), |r| r.get(0))?)
}

#[cfg(test)]
mod test {
  use tracing_test::traced_test;

  use super::*;

  #[test]
  #[traced_test]
  fn database_init() {
    RusqliteStorage::new(None).unwrap();
  }

  #[test]
  #[traced_test]
  fn write_read_note() {
    let s = RusqliteStorage::new(None).unwrap();
    let picard_note = s
      .create_new_note("Picard", "captain of the enterprise".into(), None)
      .unwrap();
    let riker = s
      .create_new_note(
        "Riker",
        "Number One of the enterprise".into(),
        picard_note.parent_id.into(),
      )
      .unwrap();
  }
}
