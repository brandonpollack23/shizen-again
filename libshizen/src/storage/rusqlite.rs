//! Sqlite storage engine using rusqlite.
use rusqlite::{Connection, Row};
use tracing::{info, trace};
use uuid::Uuid;

use crate::entities::{Note, NoteId};
use crate::storage::TodoStorage;
use crate::{ShizenError, ShizenResult};

pub struct RusqliteStorage {
  conn: Connection,
}

impl RusqliteStorage {
  pub fn create(db_path: &std::path::PathBuf) -> ShizenResult<()> {
    if db_path.exists() {
      return Err(ShizenError::ErrorCreatingDb(format!(
        "Database file already exists: {}",
        db_path.to_string_lossy().to_owned()
      )));
    }

    Connection::open(db_path)?;
    Ok(())
  }

  pub fn open(db_path: Option<&std::path::PathBuf>) -> ShizenResult<RusqliteStorage> {
    if db_path.is_none() {
      info!("Opening sqlite database in memory");
    } else {
      info!("Opening sqlite database: {:?}", db_path.as_ref().unwrap());
    }

    let conn = if let Some(p) = db_path {
      if !p.exists() {
        return Err(ShizenError::ErrorOpeningDb(format!(
          "No such file: {}",
          p.to_string_lossy()
        )));
      }

      Connection::open(p).map_err(|e| ShizenError::ErrorOpeningDb(format!("{:?}", e)))?
    } else {
      Connection::open_in_memory()?
    };

    // DB configuration sane defaults.
    conn.query_row("PRAGMA journal_mode = WAL", (), |_| Ok(()))?;
    conn.execute("PRAGMA synchronous = NORMAL", ())?;
    conn.execute("PRAGMA foreign_keys = ON", ())?;

    if !database_is_initialized(&conn)? {
      info!("Initializing database...");
      conn.execute_batch(libshizen_sql_init_str())?;
    }

    run_schema_migrations(&conn)?;

    info!("Database running at version {}", get_schema_version(&conn)?);

    Ok(RusqliteStorage { conn })
  }

  fn note_exists_conn(conn: &Connection, note_id: &NoteId) -> ShizenResult<bool> {
    Ok(conn.query_row(
      "SELECT count(uuid) FROM Notes where uuid = ?",
      [note_id.0.to_string()],
      |r| r.get::<_, bool>(0),
    )?)
  }

  fn is_descendent_of(
    conn: &Connection, ancestor: &NoteId, descendent: &NoteId,
  ) -> ShizenResult<bool> {
    // Check if you can traverse upward from descenent to parent.
    let mut query = conn.prepare(
      r#"
WITH RECURSIVE NoteHeirarchy AS (
  SELECT
    parent,
    child
  FROM Children
  WHERE child = ?1 -- descendent start

  UNION ALL

  SELECT
    c.parent,
    c.child
  FROM Children AS c
  INNER JOIN NoteHeirarchy AS nh ON nh.parent = c.child -- Where the parents of what we have are children themselves.
)
SELECT
  EXISTS(
    SELECT 1 FROM NoteHeirarchy WHERE parent = ?2 -- Ancestor
  );
"#,
    )?;

    Ok(
      query.query_row((descendent.0.to_string(), ancestor.0.to_string()), |r| {
        r.get(0)
      })?,
    )
  }

  fn get_all_descendents_cte(stmt: &str) -> String {
    format!(
      r#"
WITH RECURSIVE NoteHeirarchy AS (
  SELECT
    child
  FROM Children
  WHERE parent = ?1

  UNION ALL

  SELECT
    c.child
  FROM Children AS c
  INNER JOIN NoteHeirarchy AS curr ON c.parent = curr.child
)
{}
"#,
      stmt
    )
  }

  fn get_all_descendents(conn: &Connection, note_id: &NoteId) -> ShizenResult<Vec<Note>> {
    // Check if you can traverse upward from descenent to parent.
    let mut stmt = conn.prepare(&Self::get_all_descendents_cte(
      r#"
SELECT 
  n.uuid,
  n.title,
  n.description,
  n.parent_id
FROM NoteHeirarchy as nh
INNER JOIN Notes AS n ON nh.child = n.uuid
"#,
    ))?;

    let result: ShizenResult<Vec<_>> = stmt
      .query_and_then([note_id.0.to_string()], Self::row_to_note)?
      .map(|v: ShizenResult<_>| v.map_err(Into::into))
      .collect();

    result
  }

  fn row_to_note(r: &Row) -> ShizenResult<Note> {
    let uuid = Uuid::parse_str(&r.get::<_, String>(0)?)?;
    let parent_uuid = r
      .get::<_, Option<String>>(3)?
      .map(|p| Uuid::parse_str(&p))
      .transpose()?;
    Ok(Note {
      id: NoteId(uuid),
      title: r.get(1)?,
      description: r.get(2)?,
      parent_id: parent_uuid.map(NoteId),
    })
  }
}

impl TodoStorage for RusqliteStorage {
  fn create_new_note(
    &mut self, title: &str, description: Option<&str>, parent_id: Option<&NoteId>,
  ) -> ShizenResult<Note> {
    let txn = self.conn.transaction()?;

    if let Some(pid) = parent_id {
      if !Self::note_exists_conn(&txn, pid)? {
        return Err(ShizenError::NoSuchNote(pid.clone()));
      }
    }

    let uuid = Uuid::new_v4();
    txn.execute(
      "INSERT INTO Notes (uuid, title, description, parent_id) VALUES (?, ?, ?, ?)",
      (
        uuid.to_string(),
        title,
        description,
        parent_id.as_ref().map(|p| p.0.to_string()),
      ),
    )?;

    if let Some(p) = parent_id {
      trace!("Adding parent to new note: {parent_id:?}");
      let i = txn.execute(
        "INSERT INTO Children (parent, child) VALUES (?, ?)",
        (p.0.to_string(), uuid.to_string()),
      )?;
      if i != 1 {
        return Err(ShizenError::UnexpectedMutationResult(1, i));
      }
    }

    txn.commit()?;

    Ok(Note {
      id: NoteId(uuid),
      title: title.to_string(),
      description: description.map(|s| s.to_string()),
      parent_id: parent_id.cloned(),
    })
  }

  fn load_all_notes(&self) -> ShizenResult<Vec<Note>> {
    let mut stmt = self.conn.prepare(
      r#"
SELECT 
  uuid, title, description, parent_id 
FROM Notes
"#,
    )?;

    let result: ShizenResult<Vec<_>> = stmt
      .query_and_then((), Self::row_to_note)?
      .map(|v: ShizenResult<_>| v.map_err(Into::into))
      .collect();

    result
  }

  fn load_note(&self, note_id: &NoteId) -> ShizenResult<Note> {
    Ok(self.conn.query_row(
      r#"
  SELECT 
    uuid, title, description, parent_id 
  FROM Notes
  WHERE uuid = ?
"#,
      [note_id.0.to_string()],
      |r| {
        Ok(Note {
          id: note_id.clone(),
          title: r.get(1)?,
          description: r.get(2)?,
          parent_id: r.get::<_, Option<_>>(3)?.map(NoteId),
        })
      },
    )?)
  }

  fn note_exists(&self, note_id: &NoteId) -> ShizenResult<bool> {
    Self::note_exists_conn(&self.conn, note_id)
  }

  fn get_all_descendents(&self, note_id: &NoteId) -> ShizenResult<Vec<Note>> {
    Self::get_all_descendents(&self.conn, note_id)
  }

  fn delete_note(&mut self, note_id: &NoteId) -> ShizenResult<()> {
    let txn = self.conn.transaction()?;

    // TODO can i do this more efficently and not recalculate the CTE?
    txn
      .prepare(&Self::get_all_descendents_cte(
        "DELETE FROM Notes WHERE uuid IN (SELECT child FROM NoteHeirarchy);",
      ))?
      .execute([note_id.0.to_string()])?;
    txn
      .prepare(&Self::get_all_descendents_cte(
        "DELETE FROM Children WHERE child IN (SELECT child FROM NoteHeirarchy);",
      ))?
      .execute([note_id.0.to_string()])?;

    txn.execute("DELETE FROM Notes WHERE uuid = ?", [note_id.0.to_string()])?;
    txn.execute(
      "DELETE FROM Children WHERE child = ?1 OR parent = ?1",
      [note_id.0.to_string()],
    )?;

    txn.commit()?;

    Ok(())
  }
}

fn database_is_initialized(conn: &Connection) -> ShizenResult<bool> {
  Ok(conn.query_row(check_initialized_str(), (), |row| row.get(0))?)
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
const SCHEMA_MIGRATIONS: [&str; 0] = [];

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
  use crate::result::ShizenError;
  use tracing_test::traced_test;

  use super::*;

  #[test]
  #[traced_test]
  fn database_init() {
    RusqliteStorage::open(None).unwrap();
  }

  #[test]
  #[traced_test]
  fn write_read_note() {
    let mut s = RusqliteStorage::open(None).unwrap();

    let picard_note = s
      .create_new_note("Picard", "captain of the enterprise".into(), None)
      .unwrap();

    let riker = s
      .create_new_note(
        "Riker",
        "Number One of the enterprise".into(),
        Some(&picard_note.id),
      )
      .unwrap();

    assert_eq!(picard_note, s.load_note(&picard_note.id).unwrap());
    assert_eq!(picard_note, s.load_note(&riker.parent_id.unwrap()).unwrap());
  }

  #[test]
  #[traced_test]
  fn delete_note() {
    let mut s = RusqliteStorage::open(None).unwrap();
    let picard_note = s
      .create_new_note("Picard", "captain of the enterprise".into(), None)
      .unwrap();
    let riker = s
      .create_new_note(
        "Riker",
        "Number One of the enterprise".into(),
        Some(&picard_note.id),
      )
      .unwrap();

    s.delete_note(&riker.id).unwrap();

    assert!(matches!(
      s.load_note(&riker.id),
      Err(ShizenError::RusqliteError(_))
    ));
  }

  #[test]
  #[traced_test]
  fn delete_note_with_children() {
    let mut s = RusqliteStorage::open(None).unwrap();
    let picard_note = s
      .create_new_note("Picard", "captain of the enterprise".into(), None)
      .unwrap();
    let riker = s
      .create_new_note(
        "Riker",
        "Number One of the enterprise".into(),
        Some(&picard_note.id),
      )
      .unwrap();

    let worf = s
      .create_new_note("Worf", "Chief of security".into(), Some(&picard_note.id))
      .unwrap();

    assert_eq!(s.get_all_descendents(&picard_note.id).unwrap().len(), 2);

    s.delete_note(&picard_note.id).unwrap();

    assert_eq!(s.get_all_descendents(&picard_note.id).unwrap().len(), 0);
    assert_eq!(s.load_all_notes().unwrap().len(), 0);

    assert!(matches!(
      s.load_note(&picard_note.id),
      Err(ShizenError::RusqliteError(_))
    ));

    assert!(matches!(
      s.load_note(&riker.id),
      Err(ShizenError::RusqliteError(_))
    ));

    assert!(matches!(
      s.load_note(&worf.id),
      Err(ShizenError::RusqliteError(_))
    ));
  }

  #[test]
  #[traced_test]
  fn is_descendent_of() {
    let mut s = RusqliteStorage::open(None).unwrap();
    let picard_note = s
      .create_new_note("Picard", "captain of the enterprise".into(), None)
      .unwrap();
    let riker = s
      .create_new_note(
        "Riker",
        "Number One of the enterprise".into(),
        Some(&picard_note.id),
      )
      .unwrap();
    let worf = s
      .create_new_note("Worf", "Chief of security".into(), Some(&riker.id))
      .unwrap();

    trace!("{:#?}", s.load_all_notes().unwrap());

    assert!(RusqliteStorage::is_descendent_of(&s.conn, &picard_note.id, &riker.id).unwrap());
    assert!(RusqliteStorage::is_descendent_of(&s.conn, &picard_note.id, &worf.id).unwrap());
    assert!(RusqliteStorage::is_descendent_of(&s.conn, &riker.id, &worf.id).unwrap());
  }
}
