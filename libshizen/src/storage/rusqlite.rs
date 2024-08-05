//! Sqlite storage engine using rusqlite.
use std::sync::{Arc, RwLock};

use rusqlite::{Connection, Row};
use tracing::{info, trace};
use uuid::Uuid;

use crate::entities::{Actions, Note, NoteId};
use crate::storage::TodoStorage;
use crate::{ShizenError, ShizenResult};

#[derive(Clone)]
pub struct RusqliteStorage {
  conn: Arc<RwLock<Connection>>,
}

impl RusqliteStorage {
  pub fn open(db_path: Option<&std::path::PathBuf>) -> ShizenResult<RusqliteStorage> {
    Self::open_create(db_path, false)
  }

  pub fn open_create(
    db_path: Option<&std::path::PathBuf>,
    create: bool,
  ) -> ShizenResult<RusqliteStorage> {
    if !create {
      if let Some(p) = db_path {
        if db_path.unwrap().exists() {
          return Err(ShizenError::ErrorCreatingDb(format!(
            "Database file already exists: {}",
            p.to_string_lossy().to_owned()
          )));
        }
      }
    }

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

    Ok(RusqliteStorage {
      conn: Arc::new(RwLock::new(conn)),
    })
  }

  fn load_note_conn(conn: &Connection, note_id: &NoteId) -> ShizenResult<Note> {
    Ok(conn.query_row_and_then::<Note, ShizenError, _, _>(
      "SELECT uuid, title, description, parent, blocks, blocked, children FROM FullyQualifiedNotes WHERE uuid = ?",
      [note_id.0.to_string()],
      Self::row_to_note
    )?)
  }

  fn note_exists_conn(conn: &Connection, note_id: &NoteId) -> ShizenResult<bool> {
    Ok(conn.query_row(
      "SELECT count(uuid) FROM Notes where uuid = ?",
      [note_id.0.to_string()],
      |r| r.get::<_, bool>(0),
    )?)
  }

  fn is_descendent_of(
    conn: &Connection,
    ancestor: &NoteId,
    descendent: &NoteId,
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

  fn note_blocks_other_note(
    conn: &Connection,
    blocker: &NoteId,
    blockee: &NoteId,
  ) -> ShizenResult<bool> {
    // Check if you can traverse upward from descenent to parent.
    let mut query = conn.prepare(
      r#"
WITH RECURSIVE NoteHeirarchy AS (
  SELECT
    blocker,
    blockee
  FROM Dependencies
  WHERE blocker = ?1 -- dependency start

  UNION ALL

  SELECT
    d.blocker,
    d.blockee
  FROM Dependencies AS d
  INNER JOIN NoteHeirarchy AS nh ON nh.blockee = d.blocker -- Where the blocked notes are blockers for other notes.
)
SELECT
  EXISTS(
    SELECT 1 FROM NoteHeirarchy WHERE blockee = ?2 -- blockee
  );
"#,
    )?;

    Ok(query.query_row((blocker.0.to_string(), blockee.0.to_string()), |r| r.get(0))?)
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

  fn get_all_blocked_cte(stmt: &str) -> String {
    format!(
      r#"
WITH RECURSIVE NoteHeirarchy AS (
  SELECT
    blocked
  FROM Dependencies
  WHERE blocker = ?1

  UNION ALL

  SELECT
    d.blocked
  FROM Dependencies AS d
  INNER JOIN NoteHeirarchy AS curr ON d.blockee = curr.blocker
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
  n.*
FROM NoteHeirarchy as nh
INNER JOIN FullyQualifiedNotes AS n ON nh.child = n.uuid
"#,
    ))?;

    let result: ShizenResult<Vec<_>> = stmt
      .query_and_then([note_id.0.to_string()], |r| Self::row_to_note(r))?
      .map(|v: ShizenResult<_>| v.map_err(Into::into))
      .collect();

    result
  }

  fn get_all_blocked(
    conn: &Connection,
    note_id: &NoteId,
    recursive: bool,
  ) -> ShizenResult<Vec<Note>> {
    if !recursive {
      let mut stmt = conn.prepare("SELECT blocked FROM FullyQualifiedNotes WHERE uuid = ?")?;

      let result: ShizenResult<Vec<_>> = stmt
        .query_and_then([note_id.0.to_string()], |r| Self::row_to_note(r))?
        .map(|v: ShizenResult<_>| v.map_err(Into::into))
        .collect();

      return result;
    }

    let mut stmt = conn.prepare(&Self::get_all_blocked_cte(
      r#"
SELECT 
  n.*
FROM NoteHeirarchy as nh
INNER JOIN FullyQualifiedNotes AS n ON nh.blocker = n.uuid
"#,
    ))?;

    let result: ShizenResult<Vec<_>> = stmt
      .query_and_then([note_id.0.to_string()], |r| Self::row_to_note(r))?
      .map(|v: ShizenResult<_>| v.map_err(Into::into))
      .collect();

    result
  }

  fn row_to_note(row: &Row) -> ShizenResult<Note> {
    let uuid = Uuid::parse_str(&row.get::<_, String>(0)?)?;
    let parent_uuid = row
      .get::<_, Option<String>>(3)?
      .map(|p| Uuid::parse_str(&p))
      .transpose()?;

    let notes_this_blocks = row
      .get::<_, Option<String>>(4)?
      .map(|b| {
        b.split(',')
          .map(Uuid::parse_str)
          .map(|r| r.map_err::<ShizenError, _>(Into::into).map(NoteId))
          .collect::<ShizenResult<Vec<_>>>()
      })
      .unwrap_or(Ok(Vec::new()))?;

    let notes_blocking_this = row
      .get::<_, Option<String>>(5)?
      .map(|b| {
        b.split(',')
          .map(Uuid::parse_str)
          .map(|r| r.map_err::<ShizenError, _>(Into::into).map(NoteId))
          .collect::<ShizenResult<Vec<_>>>()
      })
      .unwrap_or(Ok(Vec::new()))?;

    let children_ids = row
      .get::<_, Option<String>>(6)?
      .map(|b| {
        b.split(',')
          .map(Uuid::parse_str)
          .map(|r| r.map_err::<ShizenError, _>(Into::into).map(NoteId))
          .collect::<ShizenResult<Vec<_>>>()
      })
      .unwrap_or(Ok(Vec::new()))?;

    Ok(Note {
      id: NoteId(uuid),
      title: row.get(1)?,
      description: row.get(2)?,
      parent_id: parent_uuid.map(NoteId),
      children_ids,
      notes_this_blocks,
      notes_blocking_this,
    })
  }

  fn notes_blocked_by_note(
    conn: &Connection,
    note_id: &NoteId,
  ) -> Result<Vec<NoteId>, ShizenError> {
    Ok(
      conn
        .prepare("SELECT blockee FROM Dependencies where blocker = ?")?
        .query_and_then([note_id.0.to_string()], |r| {
          Ok(NoteId(Uuid::parse_str(&r.get::<_, String>(0)?)?))
        })?
        .collect::<ShizenResult<Vec<_>>>()?,
    )
  }

  fn notes_blocking_note(conn: &Connection, note_id: &NoteId) -> Result<Vec<NoteId>, ShizenError> {
    Ok(
      conn
        .prepare("SELECT blocker FROM Dependencies where blockee = ?")?
        .query_and_then([note_id.0.to_string()], |r| {
          Ok(NoteId(Uuid::parse_str(&r.get::<_, String>(0)?)?))
        })?
        .collect::<ShizenResult<Vec<_>>>()?,
    )
  }

  fn insert_mutations(conn: &Connection, actions: &[Actions]) -> ShizenResult<()> {
    for action in actions {
      let action_json = serde_json::to_string(&action).unwrap();
      conn.execute(
        "INSERT INTO Mutations (action_json) VALUES (?)",
        [action_json],
      )?;
    }

    Ok(())
  }

  fn insert_redo_mutations_json(conn: &Connection, actions: &[String]) -> ShizenResult<()> {
    for action_json in actions {
      conn.execute(
        "INSERT INTO RedoMutations (action_json) VALUES (?)",
        [action_json],
      )?;
    }

    Ok(())
  }

  fn create_new_note_txn(
    txn: &Connection,
    id: Option<Uuid>,
    title: &str,
    description: Option<&str>,
    parent_id: Option<&NoteId>,
  ) -> ShizenResult<Note> {
    if let Some(pid) = parent_id {
      if !Self::note_exists_conn(&txn, pid)? {
        return Err(ShizenError::NoSuchNote(pid.clone()));
      }
    }

    let uuid = id.unwrap_or(Uuid::new_v4());
    txn.execute(
      "INSERT INTO Notes (uuid, title, description) VALUES (?, ?, ?)",
      (uuid.to_string(), title, description),
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

    Self::insert_mutations(
      &txn,
      &[Actions::CreateNote {
        id: NoteId(uuid),
        parent: parent_id.cloned(),
        title: title.to_string(),
        description: description.map(|s| s.to_string()),
      }],
    )?;

    Ok(Note {
      id: NoteId(uuid),
      title: title.to_string(),
      description: description.map(|s| s.to_string()),
      parent_id: parent_id.cloned(),
      children_ids: Vec::new(),
      notes_this_blocks: Vec::new(),
      notes_blocking_this: Vec::new(),
    })
  }

  fn update_title_txn(txn: &Connection, note_id: &NoteId, title: &str) -> ShizenResult<()> {
    let old_title = Self::load_note_conn(&txn, note_id)?.title;

    txn.execute(
      r#"UPDATE Notes SET title = ? WHERE uuid = ?"#,
      [title, &note_id.0.to_string()],
    )?;

    Self::insert_mutations(
      &txn,
      &[Actions::UpdateTitle {
        id: note_id.clone(),
        new_title: title.to_string(),
        old_title,
      }],
    )?;

    Ok(())
  }

  fn update_description_txn(
    txn: &Connection,
    note_id: &NoteId,
    description: Option<&str>,
  ) -> ShizenResult<()> {
    let old_description = Self::load_note_conn(&txn, note_id)?.description;
    Self::insert_mutations(
      &txn,
      &[Actions::UpdateDescription {
        id: note_id.clone(),
        old_description,
        new_description: description.map(|d| d.to_string()),
      }],
    )?;

    if description.is_none() {
      txn.execute(
        r#"UPDATE Notes SET description = NULL WHERE uuid = ?"#,
        [&note_id.0.to_string()],
      )?;
    } else {
      txn.execute(
        r#"UPDATE Notes SET description = ? WHERE uuid = ?"#,
        (description.unwrap(), &note_id.0.to_string()),
      )?;
    }

    Ok(())
  }

  fn change_parent_txn(
    txn: &Connection,
    note_id: &NoteId,
    parent: Option<&NoteId>,
  ) -> ShizenResult<()> {
    if !Self::note_exists_conn(&txn, note_id)? {
      return Err(ShizenError::NoSuchNote(note_id.clone()));
    }

    let old_parent = Self::load_note_conn(&txn, note_id)?.parent_id;
    Self::insert_mutations(
      &txn,
      &[Actions::ChangeParent {
        id: note_id.clone(),
        old_parent,
        new_parent: parent.map(Clone::clone),
      }],
    )?;

    if parent.is_none() {
      txn.execute(
        r#"UPDATE Notes SET parent = NULL WHERE uuid = ?"#,
        [&note_id.0.to_string()],
      )?;
      txn.execute(
        "DELETE FROM Children WHERE child = ?",
        [note_id.0.to_string()],
      )?;
    } else {
      if !Self::note_exists_conn(&txn, parent.unwrap())? {
        return Err(ShizenError::NoSuchNote(parent.unwrap().clone()));
      }

      if Self::is_descendent_of(&txn, note_id, parent.unwrap())? {
        return Err(ShizenError::ParentCircularReference(
          parent.unwrap().clone(),
          note_id.clone(),
        ));
      }

      txn.execute(
        "UPDATE Notes SET parent = ? WHERE uuid = ?",
        [parent.unwrap().0.to_string(), note_id.0.to_string()],
      )?;
      txn.execute(
        "INSERT INTO Children (parent, child) VALUES (?, ?)",
        [parent.unwrap().0.to_string(), note_id.0.to_string()],
      )?;
    }

    Ok(())
  }

  fn add_dep_txn(txn: &Connection, note_id: &NoteId, blocked_note: &NoteId) -> ShizenResult<()> {
    if !Self::note_exists_conn(&txn, note_id)? {
      return Err(ShizenError::NoSuchNote(note_id.clone()));
    }

    if !Self::note_exists_conn(&txn, blocked_note)? {
      return Err(ShizenError::NoSuchNote(blocked_note.clone()));
    }

    Self::insert_mutations(
      &txn,
      &[Actions::AddDependency {
        blocker: note_id.clone(),
        blockee: blocked_note.clone(),
      }],
    )?;

    if Self::note_blocks_other_note(&txn, blocked_note, note_id)? {
      // This would create a circular dependency.
      return Err(ShizenError::DependencyCircularReference(
        note_id.clone(),
        blocked_note.clone(),
      ));
    }

    trace!("Adding dependency {:?} blocks {:?}", note_id, blocked_note);

    txn.execute(
      "INSERT INTO Dependencies (blocker, blockee) VALUES (?, ?)",
      [note_id.0.to_string(), blocked_note.0.to_string()],
    )?;

    Ok(())
  }

  fn remove_dep_txn(txn: &Connection, note_id: &NoteId, blocked_note: &NoteId) -> ShizenResult<()> {
    if !Self::note_exists_conn(&txn, note_id)? {
      return Err(ShizenError::NoSuchNote(note_id.clone()));
    }

    if !Self::note_exists_conn(&txn, blocked_note)? {
      return Err(ShizenError::NoSuchNote(blocked_note.clone()));
    }

    Self::insert_mutations(
      &txn,
      &[Actions::RemoveDependency {
        blocker: note_id.clone(),
        blockee: blocked_note.clone(),
      }],
    )?;

    let dep_exists: bool = txn.query_row(
      "SELECT COUNT(*) FROM Dependencies WHERE blocker = ? and blockee = ?",
      [note_id.0.to_string(), blocked_note.0.to_string()],
      |r| r.get(0),
    )?;
    if !dep_exists {
      return Err(ShizenError::NoSuchDependency(
        note_id.clone(),
        blocked_note.clone(),
      ));
    }

    txn.execute(
      "DELETE FROM Dependencies WHERE blocker = ? AND blockee = ?",
      [note_id.0.to_string(), blocked_note.0.to_string()],
    )?;

    Ok(())
  }

  fn delete_note_txn(txn: &Connection, note_id: &NoteId) -> ShizenResult<()> {
    let old_note = Self::load_note_conn(&txn, note_id)?;

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

    Self::insert_mutations(&txn, &[Actions::DeleteNote { note: old_note }])?;

    Ok(())
  }
}

impl TodoStorage for RusqliteStorage {
  fn create_new_note(
    &mut self,
    title: &str,
    description: Option<&str>,
    parent_id: Option<&NoteId>,
  ) -> ShizenResult<Note> {
    let mut conn = self
      .conn
      .write()
      .map_err(|_| ShizenError::CouldNotLockDatabase)?;
    let txn = conn.transaction()?;
    let note = Self::create_new_note_txn(&txn, None, title, description, parent_id)?;
    txn.commit()?;

    Ok(note)
  }

  fn load_all_notes(&self) -> ShizenResult<Vec<Note>> {
    let conn = self
      .conn
      .read()
      .map_err(|_| ShizenError::CouldNotLockDatabase)?;
    let mut stmt = conn.prepare(
      "SELECT uuid, title, description, parent, blocks, blocked, children FROM FullyQualifiedNotes",
    )?;

    let result: ShizenResult<Vec<_>> = stmt
      .query_and_then((), |r| Self::row_to_note(r))?
      .map(|v: ShizenResult<_>| v.map_err(Into::into))
      .collect();

    result
  }

  fn load_all_unblocked_notes(&self) -> ShizenResult<Vec<Note>> {
    let conn = self
      .conn
      .read()
      .map_err(|_| ShizenError::CouldNotLockDatabase)?;
    let mut stmt = conn.prepare(
      "SELECT uuid, title, description, parent, blocks, blocked, children FROM FullyQualifiedNotes WHERE blocked IS NULL"
    )?;

    let result: ShizenResult<Vec<_>> = stmt
      .query_and_then((), |r| Self::row_to_note(r))?
      .map(|v: ShizenResult<_>| v.map_err(Into::into))
      .collect();

    result
  }

  fn load_note(&self, note_id: &NoteId) -> ShizenResult<Note> {
    let conn = self
      .conn
      .read()
      .map_err(|_| ShizenError::CouldNotLockDatabase)?;
    Self::load_note_conn(&conn, note_id)
  }

  fn note_exists(&self, note_id: &NoteId) -> ShizenResult<bool> {
    let conn = self
      .conn
      .read()
      .map_err(|_| ShizenError::CouldNotLockDatabase)?;
    Self::note_exists_conn(&conn, note_id)
  }

  fn get_all_descendents(&self, note_id: &NoteId) -> ShizenResult<Vec<Note>> {
    let conn = self
      .conn
      .read()
      .map_err(|_| ShizenError::CouldNotLockDatabase)?;
    Self::get_all_descendents(&conn, note_id)
  }

  fn get_all_blocked(&self, note_id: &NoteId, recursive: bool) -> ShizenResult<Vec<Note>> {
    let conn = self
      .conn
      .read()
      .map_err(|_| ShizenError::CouldNotLockDatabase)?;
    Self::get_all_blocked(&conn, note_id, recursive)
  }

  fn update_title(&mut self, note_id: &NoteId, title: &str) -> ShizenResult<()> {
    let mut conn = self
      .conn
      .write()
      .map_err(|_| ShizenError::CouldNotLockDatabase)?;

    let txn = conn.transaction()?;

    Self::update_title_txn(&txn, note_id, title)?;

    txn.commit()?;

    Ok(())
  }

  fn update_description(
    &mut self,
    note_id: &NoteId,
    description: Option<&str>,
  ) -> ShizenResult<()> {
    let mut conn = self
      .conn
      .write()
      .map_err(|_| ShizenError::CouldNotLockDatabase)?;
    let txn = conn.transaction()?;

    Self::update_description_txn(&txn, note_id, description)?;
    txn.commit()?;

    Ok(())
  }

  fn update_parent(&mut self, note_id: &NoteId, parent: Option<&NoteId>) -> ShizenResult<()> {
    let mut conn = self
      .conn
      .write()
      .map_err(|_| ShizenError::CouldNotLockDatabase)?;
    let txn = conn.transaction()?;

    Self::change_parent_txn(&txn, note_id, parent)?;

    txn.commit()?;
    Ok(())
  }

  fn add_blocked_note(&mut self, note_id: &NoteId, blocked_note: &NoteId) -> ShizenResult<()> {
    let mut conn = self
      .conn
      .write()
      .map_err(|_| ShizenError::CouldNotLockDatabase)?;
    let txn = conn.transaction()?;

    Self::add_dep_txn(&txn, note_id, blocked_note)?;

    txn.commit()?;
    Ok(())
  }

  fn remove_blocked_note(&mut self, note_id: &NoteId, blocked_note: &NoteId) -> ShizenResult<()> {
    let mut conn = self
      .conn
      .write()
      .map_err(|_| ShizenError::CouldNotLockDatabase)?;
    let txn = conn.transaction()?;

    Self::remove_dep_txn(&txn, note_id, blocked_note)?;

    txn.commit()?;
    Ok(())
  }

  fn delete_note(&mut self, note_id: &NoteId) -> ShizenResult<()> {
    let mut conn = self
      .conn
      .write()
      .map_err(|_| ShizenError::CouldNotLockDatabase)?;
    let txn = conn.transaction()?;

    Self::delete_note_txn(&txn, note_id)?;

    txn.commit()?;

    Ok(())
  }

  fn undo(&mut self) -> ShizenResult<()> {
    let mut conn = self
      .conn
      .write()
      .map_err(|_| ShizenError::CouldNotLockDatabase)?;
    let txn = conn.transaction()?;

    let (id_to_remove, undo_action, undo_action_json) = txn.query_row_and_then(
      "SELECT id, action_json FROM Mutations ORDER BY id DESC LIMIT 1",
      [],
      |r| -> ShizenResult<_> {
        let id_to_remove: u32 = r.get(0)?;
        let undo_action_json: String = r.get(1)?;
        let undo_action: Actions =
          serde_json::from_str(&undo_action_json).map_err(|_| ShizenError::SerdeError)?;

        return Ok((id_to_remove, undo_action, undo_action_json));
      },
    )?;

    match undo_action {
      Actions::CreateNote { id, parent, .. } => {
        txn.execute("DELETE FROM Notes WHERE uuid = ?", [id.0.to_string()])?;
        if parent.is_some() {
          txn.execute("DELETE FROM Children WHERE child = ?", [id.0.to_string()])?;
        }
      }
      Actions::UpdateTitle { id, old_title, .. } => {
        txn.execute(
          "UPDATE Notes SET title = ? WHERE uuid = ?",
          [old_title, id.0.to_string()],
        )?;
      }
      Actions::UpdateDescription {
        id,
        old_description,
        ..
      } => {
        txn.execute(
          "UPDATE Notes SET description = ? WHERE uuid = ?",
          (old_description, id.0.to_string()),
        )?;
      }
      Actions::ChangeParent {
        id,
        old_parent,
        new_parent,
      } => {
        if old_parent.is_some() {
          txn.execute(
            "UPDATE Children SET parent = ? WHERE uuid = ?",
            (new_parent.map(|u| u.0.to_string()), id.0.to_string()),
          )?;
        } else {
          txn.execute(
            "INSERT INTO Children (parent, child) VALUES (?, ?)",
            (new_parent.map(|u| u.0.to_string()), id.0.to_string()),
          )?;
        }
      }
      Actions::AddDependency { blocker, blockee } => {
        txn.execute(
          "DELETE FROM Dependencies WHERE blocker = ? and blockee = ?",
          (blocker.0.to_string(), blockee.0.to_string()),
        )?;
      }
      Actions::RemoveDependency { blocker, blockee } => {
        txn.execute(
          "INSERT INTO Dependencies (blocker, blockee) VALUES (?, ?)",
          (blocker.0.to_string(), blockee.0.to_string()),
        )?;
      }
      Actions::DeleteNote { note } => {
        txn.execute(
          "INSERT INTO Notes (uuid, title, description) VALUES (?, ?, ?)",
          (note.id.0.to_string(), note.title, note.description),
        )?;

        if note.parent_id.is_some() {
          txn.execute(
            "INSERT INTO Children (parent, child) VALUES (?, ?)",
            (note.parent_id.unwrap().0.to_string(), note.id.0.to_string()),
          )?;
        }

        for child in &note.children_ids {
          txn.execute(
            "INSERT INTO Children (parent, child) VALUES (?, ?)",
            (note.id.0.to_string(), child.0.to_string()),
          )?;
        }

        for blocking_this in &note.notes_blocking_this {
          txn.execute(
            "INSERT INTO Dependencies (blocker, blockee) VALUES (?, ?)",
            (blocking_this.0.to_string(), note.id.0.to_string()),
          )?;
        }

        for we_block in &note.notes_this_blocks {
          txn.execute(
            "INSERT INTO Dependencies (blocker, blockee) VALUES (?, ?)",
            (note.id.0.to_string(), we_block.0.to_string()),
          )?;
        }
      }
    }

    txn.execute("DELETE FROM Mutations WHERE id = ?", [id_to_remove])?;
    Self::insert_redo_mutations_json(&txn, &[undo_action_json])?;

    txn.commit()?;
    Ok(())
  }

  fn redo(&mut self) -> ShizenResult<()> {
    let mut conn = self
      .conn
      .write()
      .map_err(|_| ShizenError::CouldNotLockDatabase)?;
    let txn = conn.transaction()?;

    let (id_to_remove, redo_action) = txn.query_row_and_then(
      "SELECT id, action_json FROM RedoMutations ORDER BY id DESC LIMIT 1",
      [],
      |r| -> ShizenResult<_> {
        let id_to_remove: u32 = r.get(0)?;
        let redo_action_json: String = r.get(1)?;
        let redo_action: Actions =
          serde_json::from_str(&redo_action_json).map_err(|_| ShizenError::SerdeError)?;

        return Ok((id_to_remove, redo_action));
      },
    )?;

    match redo_action {
      Actions::CreateNote {
        id,
        parent,
        title,
        description,
      } => {
        Self::create_new_note_txn(
          &txn,
          Some(id.0),
          &title,
          description.as_ref().map(|x| x.as_str()),
          parent.as_ref(),
        )?;
      }
      Actions::UpdateTitle { id, old_title, .. } => {
        Self::update_title_txn(&txn, &id, &old_title)?;
      }
      Actions::UpdateDescription {
        id,
        old_description,
        ..
      } => {
        Self::update_description_txn(&txn, &id, old_description.as_ref().map(|s| s.as_str()))?;
      }
      Actions::ChangeParent {
        id,
        old_parent,
        new_parent,
      } => {
        Self::change_parent_txn(&txn, &id, old_parent.as_ref())?;
      }
      Actions::AddDependency { blocker, blockee } => {
        Self::add_dep_txn(&txn, &blocker, &blockee)?;
      }
      Actions::RemoveDependency { blocker, blockee } => {
        Self::remove_dep_txn(&txn, &blocker, &blockee)?;
      }
      Actions::DeleteNote { note } => {
        Self::delete_note_txn(&txn, &note.id)?;
      }
    }

    txn.execute("DELETE FROM RedoMutations WHERE id = ?", [id_to_remove])?;

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

    let expected = Note {
      children_ids: vec![riker.id.clone()],
      ..picard_note.clone()
    };
    assert_eq!(expected, s.load_note(&picard_note.id).unwrap());
    assert_eq!(expected, s.load_note(&riker.parent_id.unwrap()).unwrap());
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

    let conn = s.conn.read().unwrap();
    assert!(RusqliteStorage::is_descendent_of(&conn, &picard_note.id, &riker.id).unwrap());
    assert!(RusqliteStorage::is_descendent_of(&conn, &picard_note.id, &worf.id).unwrap());
    assert!(RusqliteStorage::is_descendent_of(&conn, &riker.id, &worf.id).unwrap());
  }

  #[test]
  #[traced_test]
  fn update_title() {
    let mut s = RusqliteStorage::open(None).unwrap();
    let picard_note = s
      .create_new_note("Picard", "captain of the enterprise".into(), None)
      .unwrap();

    s.update_title(&picard_note.id, "Patrick").unwrap();

    let readback = s.load_note(&picard_note.id).unwrap();

    assert_eq!(readback.title, "Patrick");
  }

  #[test]
  #[traced_test]
  fn update_description() {
    let mut s = RusqliteStorage::open(None).unwrap();
    let picard_note = s
      .create_new_note("Picard", "captain of the enterprise".into(), None)
      .unwrap();

    s.update_description(&picard_note.id, Some("Former captain of the stargazer"))
      .unwrap();

    let readback = s.load_note(&picard_note.id).unwrap();

    assert_eq!(
      readback.description,
      Some("Former captain of the stargazer".to_string())
    );

    s.update_description(&picard_note.id, None).unwrap();

    let readback = s.load_note(&picard_note.id).unwrap();

    assert_eq!(readback.description, None);
  }

  #[test]
  #[traced_test]
  fn add_deps_work() {
    let mut s = RusqliteStorage::open(None).unwrap();
    let picard_note = s
      .create_new_note("Picard", "captain of the enterprise".into(), None)
      .unwrap();
    let bev = s
      .create_new_note("Beverly Crusher", "Capable and attractive doc".into(), None)
      .unwrap();

    s.add_blocked_note(&bev.id, &picard_note.id).unwrap(); // We all know Jean-Luc depends on and is often blocked by bev.

    let readback_pic = s.load_note(&picard_note.id).unwrap();
    assert_eq!(readback_pic.notes_blocking_this, vec![bev.id.clone()]);
    assert_eq!(readback_pic.notes_this_blocks, vec![]);

    let readback_bev = s.load_note(&bev.id).unwrap();
    assert_eq!(readback_bev.notes_blocking_this, vec![]);
    assert_eq!(readback_bev.notes_this_blocks, vec![picard_note.id.clone()]);
  }

  #[test]
  #[traced_test]
  fn add_deps_loop_fails() {
    let mut s = RusqliteStorage::open(None).unwrap();
    let picard_note = s
      .create_new_note("Picard", "captain of the enterprise".into(), None)
      .unwrap();
    let bev = s
      .create_new_note("Beverly Crusher", "Capable and attractive doc".into(), None)
      .unwrap();

    s.add_blocked_note(&bev.id, &picard_note.id).unwrap();
    let r = s.add_blocked_note(&picard_note.id, &bev.id);
    assert!(r.is_err());

    let wesley = s
      .create_new_note("Wesley Crusher", "Kid".into(), None)
      .unwrap();

    s.add_blocked_note(&wesley.id, &bev.id).unwrap();
    let r = s.add_blocked_note(&picard_note.id, &wesley.id);

    assert!(r.is_err());
  }

  #[test]
  #[traced_test]
  fn remove_deps_work() {
    let mut s = RusqliteStorage::open(None).unwrap();
    let picard_note = s
      .create_new_note("Picard", "captain of the enterprise".into(), None)
      .unwrap();
    let bev = s
      .create_new_note("Beverly Crusher", "Capable and attractive doc".into(), None)
      .unwrap();

    s.add_blocked_note(&bev.id, &picard_note.id).unwrap(); // We all know Jean-Luc depends on and is often blocked by bev.
    s.remove_blocked_note(&bev.id, &picard_note.id).unwrap();

    let readback_pic = s.load_note(&picard_note.id).unwrap();
    assert_eq!(readback_pic.notes_blocking_this, vec![]);
    assert_eq!(readback_pic.notes_this_blocks, vec![]);

    let readback_bev = s.load_note(&bev.id).unwrap();
    assert_eq!(readback_bev.notes_blocking_this, vec![]);
    assert_eq!(readback_bev.notes_this_blocks, vec![]);
  }

  #[test]
  #[traced_test]
  fn remove_deps_loop_allows_adding_dep() {
    let mut s = RusqliteStorage::open(None).unwrap();
    let picard_note = s
      .create_new_note("Picard", "captain of the enterprise".into(), None)
      .unwrap();
    let bev = s
      .create_new_note("Beverly Crusher", "Capable and attractive doc".into(), None)
      .unwrap();

    s.add_blocked_note(&bev.id, &picard_note.id).unwrap();
    let r = s.add_blocked_note(&picard_note.id, &bev.id);
    assert!(matches!(r, Err(_)));

    let wesley = s
      .create_new_note("Wesley Crusher", "Kid".into(), None)
      .unwrap();

    s.add_blocked_note(&wesley.id, &bev.id).unwrap();
    let r = s.add_blocked_note(&picard_note.id, &wesley.id);
    assert!(matches!(r, Err(_)));

    s.remove_blocked_note(&wesley.id, &bev.id).unwrap();
    s.add_blocked_note(&picard_note.id, &wesley.id).unwrap();
  }

  #[test]
  #[traced_test]
  fn undo_create() {
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
    let bev = s
      .create_new_note("Beverly Crusher", "Capable and attractive doc".into(), None)
      .unwrap();
    s.add_blocked_note(&bev.id, &picard_note.id).unwrap();

    let before_undo_picard = s.load_note(&picard_note.id).unwrap();
    let before_undo_riker = s.load_note(&riker.id).unwrap();
    let before_undo_bev = s.load_note(&bev.id).unwrap();

    // Undo dep
    s.undo().unwrap();

    let readback_picard = s.load_note(&picard_note.id).unwrap();
    let expected_picard = Note {
      notes_blocking_this: vec![],
      notes_this_blocks: vec![],
      ..before_undo_picard.clone()
    };
    let readback_riker = s.load_note(&riker.id).unwrap();
    let expected_riker = Note {
      ..before_undo_riker.clone()
    };
    let readback_bev = s.load_note(&bev.id).unwrap();
    let expected_bev = Note {
      notes_this_blocks: vec![],
      ..before_undo_bev.clone()
    };
    assert_eq!(readback_picard, expected_picard);
    assert_eq!(readback_riker, expected_riker);
    assert_eq!(readback_bev, expected_bev);

    // Undo create bev
    s.undo().unwrap();

    let readback_picard = s.load_note(&picard_note.id).unwrap();
    let expected_picard = Note {
      notes_blocking_this: vec![],
      notes_this_blocks: vec![],
      ..before_undo_picard.clone()
    };
    let readback_bev = s.load_note(&bev.id);
    assert_eq!(readback_picard, expected_picard);
    assert!(readback_bev.is_err());

    // Undo create riker
    s.undo().unwrap();

    let readback_picard = s.load_note(&picard_note.id).unwrap();
    let expected_picard = Note {
      notes_blocking_this: vec![],
      notes_this_blocks: vec![],
      children_ids: vec![],
      ..before_undo_picard.clone()
    };
    let readback_riker = s.load_note(&riker.id);
    assert!(readback_riker.is_err());
    assert_eq!(expected_picard, readback_picard);

    // Undo create picard
    s.undo().unwrap();

    let readback_picard = s.load_note(&picard_note.id);
    assert!(readback_picard.is_err());
  }

  // TODO tests for redo
  // TODO tests for update undo.
}
