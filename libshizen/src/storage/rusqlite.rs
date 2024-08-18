//! Sqlite storage engine using rusqlite.
use std::cell::RefCell;
use std::net::{SocketAddr, ToSocketAddrs};
use std::rc::Rc;

use lexorank::{Bucket, LexoRank, Rank};
use rusqlite::Error::QueryReturnedNoRows;
use rusqlite::{Connection, Row};
use tracing::{info, trace, warn};
use uuid::Uuid;

use crate::entities::{Action, Note, NoteId, PeerId, PeerInfo};
use crate::storage::TodoStorage;
use crate::sync::{SyncConnection, SyncResults};
use crate::{ShizenError, ShizenResult};

macro_rules! sqlite_str {
  ($path:expr) => {
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), $path))
  };
}

#[derive(Clone)]
pub struct RusqliteStorage {
  conn: Rc<RefCell<Connection>>,
}

impl RusqliteStorage {
  pub fn open(db_path: Option<&std::path::PathBuf>) -> ShizenResult<RusqliteStorage> {
    Self::open_create(db_path, false)
  }

  pub fn open_create(
    db_path: Option<&std::path::PathBuf>,
    create: bool,
  ) -> ShizenResult<RusqliteStorage> {
    if create {
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

    let mut conn = if let Some(p) = db_path {
      if (p.parent().is_some() && !p.parent().unwrap().exists())
        && !p.to_str().unwrap().starts_with("file::memory:")
        && !p.to_str().unwrap().contains("mode=memory")
      {
        return Err(ShizenError::ErrorOpeningDb(format!(
          "No such path to file: {}",
          p.parent().unwrap().to_string_lossy()
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
      conn.execute(
        "INSERT INTO LocalSettings (clock, peer_id) VALUES (?, ?)",
        (0, Uuid::new_v4().to_string()),
      )?;
    }

    run_schema_migrations(&mut conn)?;

    info!("Database running at version {}", get_schema_version(&conn)?);

    Ok(RusqliteStorage {
      conn: Rc::new(RefCell::new(conn)),
    })
  }

  fn load_note_txn(conn: &Connection, note_id: &NoteId) -> ShizenResult<Note> {
    conn.query_row_and_then::<Note, ShizenError, _, _>(
      "SELECT uuid, title, description, parent, blocks, blocked, children, completed FROM FullyQualifiedNotes WHERE uuid = ?",
      [note_id.0.to_string()],
      Self::row_to_note
    )
  }

  fn note_exists_txn(conn: &Connection, note_id: &NoteId) -> ShizenResult<bool> {
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
      .query_and_then([note_id.0.to_string()], Self::row_to_note)?
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
        .query_and_then([note_id.0.to_string()], Self::row_to_note)?
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
      .query_and_then([note_id.0.to_string()], Self::row_to_note)?
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
      completed: row.get(7)?,
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
    conn
      .prepare("SELECT blockee FROM Dependencies where blocker = ?")?
      .query_and_then([note_id.0.to_string()], |r| {
        Ok(NoteId(Uuid::parse_str(&r.get::<_, String>(0)?)?))
      })?
      .collect::<ShizenResult<Vec<_>>>()
  }

  fn notes_blocking_note(conn: &Connection, note_id: &NoteId) -> Result<Vec<NoteId>, ShizenError> {
    conn
      .prepare("SELECT blocker FROM Dependencies where blockee = ?")?
      .query_and_then([note_id.0.to_string()], |r| {
        Ok(NoteId(Uuid::parse_str(&r.get::<_, String>(0)?)?))
      })?
      .collect::<ShizenResult<Vec<_>>>()
  }

  fn insert_mutations(conn: &Connection, actions: &[Action]) -> ShizenResult<()> {
    let mut clock: usize = Self::get_clock_txn(conn)?;

    for action in actions {
      let action_json = serde_json::to_string(&action).unwrap();
      conn.execute(
        "INSERT INTO Mutations (action_json, clock) VALUES (?, ?)",
        (action_json, clock),
      )?;
      clock += 1;
    }

    conn.execute("UPDATE LocalSettings SET clock = ?", [clock])?;

    Ok(())
  }

  fn insert_redo_mutations_json(conn: &Connection, actions: &[String]) -> ShizenResult<()> {
    let mut clock: usize = Self::get_clock_txn(conn)?;

    for action_json in actions {
      conn.execute(
        "INSERT INTO RedoMutations (action_json) VALUES (?)",
        [action_json],
      )?;
      clock += 1;
    }

    conn.execute("UPDATE LocalSettings SET clock = ?", [clock])?;

    Ok(())
  }

  fn get_rank_for_new_note(conn: &Connection) -> ShizenResult<LexoRank> {
    let has_note: bool =
      conn.query_row_and_then("SELECT COUNT(*) > 0 FROM Notes", [], |r| r.get(0))?;
    if !has_note {
      return Ok(LexoRank::new(
        Bucket::new(0).unwrap(),
        Rank::new("a").unwrap(),
      ));
    }
    let last_rank_str: String = conn.query_row_and_then(
      "SELECT rank FROM Notes ORDER BY rank DESC LIMIT 1",
      [],
      |r| -> ShizenResult<_> { Ok(r.get(0)?) },
    )?;

    let last_rank = LexoRank::from_string(&last_rank_str)?;

    // TODO configure taking next or last (change above to asc vs desc and use prev or next here).
    Ok(last_rank.next())
  }

  fn create_new_note_txn(
    txn: &Connection,
    id: Option<&NoteId>,
    title: &str,
    description: Option<&str>,
    parent_id: Option<&NoteId>,
  ) -> ShizenResult<Note> {
    if let Some(pid) = parent_id {
      if !Self::note_exists_txn(txn, pid)? {
        return Err(ShizenError::NoSuchNote(pid.clone()));
      }
    }

    let rank = Self::get_rank_for_new_note(txn)?;

    let uuid = id.cloned().unwrap_or_else(|| NoteId(Uuid::new_v4()));
    trace!("Creating note: {title} {description:?} with rank {rank:?}");
    txn.execute(
      "INSERT INTO Notes (uuid, title, description, completed, rank) VALUES (?, ?, ?, ?, ?)",
      (
        uuid.to_string(),
        title,
        description,
        false,
        rank.to_string(),
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

    Self::insert_mutations(
      txn,
      &[Action::CreateNote {
        id: uuid.clone(),
        parent: parent_id.cloned(),
        title: title.to_string(),
        description: description.map(|s| s.to_string()),
      }],
    )?;

    Ok(Note {
      id: uuid,
      title: title.to_string(),
      description: description.map(|s| s.to_string()),
      completed: false,
      parent_id: parent_id.cloned(),
      children_ids: Vec::new(),
      notes_this_blocks: Vec::new(),
      notes_blocking_this: Vec::new(),
    })
  }

  fn get_clock_txn(txn: &Connection) -> ShizenResult<usize> {
    let clock: usize =
      txn.query_row("SELECT clock FROM LocalSettings LIMIT 1", (), |r| r.get(0))?;
    Ok(clock)
  }

  fn set_completed_txn(
    txn: &Connection,
    note_id: &NoteId,
    completed: bool,
    old_completed: bool,
  ) -> ShizenResult<()> {
    txn.execute(
      r#"UPDATE Notes SET completed = ? WHERE uuid = ?"#,
      (completed, &note_id.0.to_string()),
    )?;

    Self::insert_mutations(
      txn,
      &[Action::SetCompleted {
        id: note_id.clone(),
        new_completed: completed,
        old_completed,
      }],
    )?;

    Ok(())
  }

  fn update_title_txn(
    txn: &Connection,
    note_id: &NoteId,
    title: &str,
    old_title: &str,
  ) -> ShizenResult<()> {
    txn.execute(
      r#"UPDATE Notes SET title = ? WHERE uuid = ?"#,
      [title, &note_id.0.to_string()],
    )?;

    Self::insert_mutations(
      txn,
      &[Action::UpdateTitle {
        id: note_id.clone(),
        new_title: title.to_string(),
        old_title: old_title.to_string(),
      }],
    )?;

    Ok(())
  }

  fn update_description_txn(
    txn: &Connection,
    note_id: &NoteId,
    description: Option<&str>,
    old_description: Option<&str>,
  ) -> ShizenResult<()> {
    Self::insert_mutations(
      txn,
      &[Action::UpdateDescription {
        id: note_id.clone(),
        old_description: old_description.map(|d| d.to_string()),
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
    old_parent: Option<&NoteId>,
  ) -> ShizenResult<()> {
    if !Self::note_exists_txn(txn, note_id)? {
      return Err(ShizenError::NoSuchNote(note_id.clone()));
    }

    Self::insert_mutations(
      txn,
      &[Action::ChangeParent {
        id: note_id.clone(),
        old_parent: old_parent.cloned(),
        new_parent: parent.cloned(),
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
      if !Self::note_exists_txn(txn, parent.unwrap())? {
        return Err(ShizenError::NoSuchNote(parent.unwrap().clone()));
      }

      if Self::is_descendent_of(txn, note_id, parent.unwrap())? {
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
    if !Self::note_exists_txn(txn, note_id)? {
      return Err(ShizenError::NoSuchNote(note_id.clone()));
    }

    if !Self::note_exists_txn(txn, blocked_note)? {
      return Err(ShizenError::NoSuchNote(blocked_note.clone()));
    }

    Self::insert_mutations(
      txn,
      &[Action::AddDependency {
        blocker: note_id.clone(),
        blockee: blocked_note.clone(),
      }],
    )?;

    if Self::note_blocks_other_note(txn, blocked_note, note_id)? {
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
    if !Self::note_exists_txn(txn, note_id)? {
      return Err(ShizenError::NoSuchNote(note_id.clone()));
    }

    if !Self::note_exists_txn(txn, blocked_note)? {
      return Err(ShizenError::NoSuchNote(blocked_note.clone()));
    }

    Self::insert_mutations(
      txn,
      &[Action::RemoveDependency {
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
    let old_note = Self::load_note_txn(txn, note_id)?;

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

    Self::insert_mutations(txn, &[Action::DeleteNote { note: old_note }])?;

    Ok(())
  }

  fn adjust_rank_between_txn(
    txn: &Connection,
    note_id: &NoteId,
    after: Option<&NoteId>,
    before: Option<&NoteId>,
  ) -> ShizenResult<()> {
    trace!("Adjusting rank of {note_id:#?} between {after:#?} and before {before:#?}");

    // TODO if after or before are none then infer (if not end of list).

    let after_rank: Option<ShizenResult<LexoRank>> = after.and_then(|n| {
      Some(
        txn
          .query_row(
            "SELECT rank FROM Notes WHERE uuid = ?",
            [n.0.to_string()],
            |r: &Row| r.get::<_, String>(0),
          )
          .map(|s| LexoRank::from_string(&s).unwrap())
          .map_err(|e| e.into()),
      )
    });
    let before_rank: Option<ShizenResult<LexoRank>> = before.and_then(|n| {
      Some(
        txn
          .query_row(
            "SELECT rank FROM Notes WHERE uuid = ?",
            [n.0.to_string()],
            |r: &Row| r.get::<_, String>(0),
          )
          .map(|s| LexoRank::from_string(&s).unwrap())
          .map_err(|e| e.into()),
      )
    });

    let (after_rank, before_rank) = (after_rank.transpose()?, before_rank.transpose()?);

    trace!("Rank to be moved after {after_rank:?} and before {before_rank:?}");

    let new_rank = match (after_rank, before_rank) {
      (None, Some(r)) => r.prev(),
      (Some(r), None) => r.next(),
      (Some(a), Some(b)) => a.between(&b).unwrap(),
      (None, None) => unreachable!(),
    };

    let old_rank: String = txn.query_row(
      "SELECT rank FROM Notes WHERE uuid = ?",
      [note_id.0.to_string()],
      |r| r.get(0),
    )?;

    trace!("changing rank from {old_rank:?} to {new_rank:?}");

    txn.execute(
      "UPDATE Notes SET rank = ? WHERE uuid = ?",
      (new_rank.to_string(), note_id.0.to_string()),
    )?;

    Self::insert_mutations(
      &txn,
      &[Action::ReorderNote {
        id: note_id.clone(),
        before: before.cloned(),
        after: after.cloned(),
        old_rank,
      }],
    )?;

    Ok(())
  }
}

impl TodoStorage for RusqliteStorage {
  fn create_new_note(
    &self,
    title: &str,
    description: Option<&str>,
    parent_id: Option<&NoteId>,
  ) -> ShizenResult<Note> {
    let mut conn = self.conn.borrow_mut();
    let txn = conn.transaction()?;
    let note = Self::create_new_note_txn(&txn, None, title, description, parent_id)?;

    txn.commit()?;
    Ok(note)
  }

  fn add_peer(
    &self,
    addr: &SocketAddr,
    local_server_addr: Option<&SocketAddr>,
  ) -> ShizenResult<PeerId> {
    let this_peer_id = self.get_peer_id()?;
    let socket_addr = addr.to_socket_addrs().unwrap().next().unwrap();
    let addr_json = serde_json::to_string(&socket_addr)?;

    let mut sync = SyncConnection::new(&this_peer_id, &addr, local_server_addr.as_ref())?;
    let peer_id = sync.peer_id_handshake()?;

    if peer_id == this_peer_id {
      return Err(ShizenError::CannotBePeerOfSelf);
    }

    if self.get_peer(&peer_id).is_ok() {
      return Err(ShizenError::PeerAlreadyExists(peer_id.clone()));
    }

    let conn = self.conn.borrow_mut();

    conn.execute(
      "INSERT INTO Peers (peer_id, clock, addr) VALUES (?, ?, ?)",
      (peer_id.0.to_string(), 0, addr_json),
    )?;
    Ok(peer_id)
  }

  fn add_connected_peer(
    &self,
    peer_id: PeerId,
    other_clock: usize,
    addr: &SocketAddr,
  ) -> ShizenResult<()> {
    let conn = self.conn.borrow_mut();

    let socket_addr = addr.to_socket_addrs().unwrap().next().unwrap();
    let addr_json = serde_json::to_string(&socket_addr)?;
    conn.execute(
      "INSERT INTO Peers (peer_id, clock, addr) VALUES (?, ?, ?)",
      (peer_id.0.to_string(), other_clock, addr_json),
    )?;

    Ok(())
  }

  fn load_all_notes(&self) -> ShizenResult<Vec<Note>> {
    let conn = self.conn.borrow();
    let mut stmt = conn.prepare(
      "SELECT uuid, title, description, parent, blocks, blocked, children, completed FROM FullyQualifiedNotes",
    )?;

    let result: ShizenResult<Vec<_>> = stmt
      .query_and_then((), Self::row_to_note)?
      .map(|v: ShizenResult<_>| v.map_err(Into::into))
      .collect();

    result
  }

  fn load_all_incomplete_notes(&self) -> ShizenResult<Vec<Note>> {
    let conn = self.conn.borrow();
    let mut stmt = conn.prepare(
      "SELECT uuid, title, description, parent, blocks, blocked, children, completed FROM FullyQualifiedNotes WHERE completed = 0",
    )?;

    let result: ShizenResult<Vec<_>> = stmt
      .query_and_then((), Self::row_to_note)?
      .map(|v: ShizenResult<_>| v.map_err(Into::into))
      .collect();

    result
  }

  fn load_all_complete_notes(&self) -> ShizenResult<Vec<Note>> {
    let conn = self.conn.borrow();
    let mut stmt = conn.prepare(
      "SELECT uuid, title, description, parent, blocks, blocked, children, completed FROM FullyQualifiedNotes WHERE completed = 1",
    )?;

    let result: ShizenResult<Vec<_>> = stmt
      .query_and_then((), Self::row_to_note)?
      .map(|v: ShizenResult<_>| v.map_err(Into::into))
      .collect();

    result
  }

  fn load_all_unblocked_notes(&self, load_completed: bool) -> ShizenResult<Vec<Note>> {
    let conn = self.conn.borrow();
    let sql = if load_completed {
      "SELECT uuid, title, description, parent, blocks, blocked, children, completed FROM FullyQualifiedNotes WHERE is_blocked = 0 OR blocked = NULL"
    } else {
      "SELECT uuid, title, description, parent, blocks, blocked, children, completed FROM FullyQualifiedNotes WHERE completed = 0 AND is_blocked = 0"
    };
    let mut stmt = conn.prepare(sql)?;

    let result: ShizenResult<Vec<_>> = stmt
      .query_and_then([], Self::row_to_note)?
      .map(|v: ShizenResult<_>| v.map_err(Into::into))
      .collect();

    result
  }

  fn load_all_changes_since_clock(&self, clock: usize) -> ShizenResult<Vec<Action>> {
    let mut conn = self.conn.borrow_mut();
    let txn = conn.transaction()?;

    let actions: ShizenResult<Vec<_>> = txn
      .prepare("SELECT action_json FROM Mutations WHERE clock >= ? ORDER BY clock ASC")?
      .query_and_then([clock], |r| -> ShizenResult<_> {
        let action_json: String = r.get(0)?;
        let action: Action = serde_json::from_str(&action_json)?;
        Ok(action)
      })?
      .collect();

    actions
  }

  fn load_redo_queue(&self) -> ShizenResult<Vec<Action>> {
    let mut conn = self.conn.borrow_mut();
    let txn = conn.transaction()?;

    let actions: ShizenResult<Vec<_>> = txn
      .prepare("SELECT action_json FROM RedoMutations ORDER BY id DESC")?
      .query_and_then([], |r| -> ShizenResult<_> {
        let action_json: String = r.get(0)?;
        let action: Action = serde_json::from_str(&action_json)?;
        Ok(action)
      })?
      .collect();

    actions
  }

  fn load_note(&self, note_id: &NoteId) -> ShizenResult<Note> {
    let conn = self.conn.borrow();
    Self::load_note_txn(&conn, note_id)
  }

  fn note_exists(&self, note_id: &NoteId) -> ShizenResult<bool> {
    let conn = self.conn.borrow();
    Self::note_exists_txn(&conn, note_id)
  }

  fn get_all_descendents(&self, note_id: &NoteId) -> ShizenResult<Vec<Note>> {
    let conn = self.conn.borrow();
    Self::get_all_descendents(&conn, note_id)
  }

  fn get_all_blocked(&self, note_id: &NoteId, recursive: bool) -> ShizenResult<Vec<Note>> {
    let conn = self.conn.borrow();
    Self::get_all_blocked(&conn, note_id, recursive)
  }

  fn get_peer_id(&self) -> ShizenResult<PeerId> {
    let conn = self.conn.borrow();

    let peer_id_str: String =
      conn.query_row("SELECT peer_id FROM LocalSettings LIMIT 1", (), |r| {
        r.get(0)
      })?;

    Ok(PeerId(Uuid::parse_str(&peer_id_str)?))
  }

  fn get_clock(&self) -> ShizenResult<usize> {
    let conn = self.conn.borrow();

    let clock = Self::get_clock_txn(&conn)?;

    Ok(clock)
  }

  fn get_peers(&self) -> ShizenResult<Vec<PeerInfo>> {
    let conn = self.conn.borrow();

    let mut stmt = conn.prepare("SELECT peer_id, clock, addr FROM Peers")?;
    let peer_infos: ShizenResult<Vec<PeerInfo>> = stmt
      .query_and_then([], |r| -> ShizenResult<_> {
        let peer_id_str: String = r.get(0)?;
        let peer_id = PeerId(Uuid::parse_str(&peer_id_str)?);
        let clock: usize = r.get(1)?;
        let addr_json: String = r.get(2)?;
        let addr: SocketAddr = serde_json::from_str(&addr_json)?;

        Ok(PeerInfo {
          peer_id,
          clock,
          addr,
        })
      })?
      .collect();

    peer_infos
  }

  fn get_peer(&self, peer_id: &PeerId) -> ShizenResult<PeerInfo> {
    let conn = self.conn.borrow();

    let peer_info = conn.query_row_and_then(
      "SELECT peer_id, clock, addr FROM Peers WHERE peer_id = ?",
      [peer_id.0.to_string()],
      |r| -> ShizenResult<_> {
        let peer_id_str: String = r.get(0)?;
        let peer_id = PeerId(Uuid::parse_str(&peer_id_str)?);
        let clock: usize = r.get(1)?;
        let addr_json: String = r.get(2)?;
        let addr: SocketAddr = serde_json::from_str(&addr_json)?;

        Ok(PeerInfo {
          peer_id,
          clock,
          addr,
        })
      },
    )?;

    Ok(peer_info)
  }

  fn set_completed(&self, note_id: &NoteId, completed: bool) -> ShizenResult<()> {
    let mut conn = self.conn.borrow_mut();
    let txn = conn.transaction()?;

    let old_completed = Self::load_note_txn(&txn, note_id)?.completed;
    Self::set_completed_txn(&txn, note_id, completed, old_completed)?;

    txn.commit()?;

    Ok(())
  }

  fn update_title(&self, note_id: &NoteId, title: &str) -> ShizenResult<()> {
    let mut conn = self.conn.borrow_mut();
    let txn = conn.transaction()?;

    let old_title = Self::load_note_txn(&txn, note_id)?.title;
    Self::update_title_txn(&txn, note_id, title, &old_title)?;

    txn.commit()?;

    Ok(())
  }

  fn update_description(&self, note_id: &NoteId, description: Option<&str>) -> ShizenResult<()> {
    let mut conn = self.conn.borrow_mut();
    let txn = conn.transaction()?;

    let old_description = Self::load_note_txn(&txn, note_id)?.description;
    Self::update_description_txn(&txn, note_id, description, old_description.as_deref())?;
    txn.commit()?;

    Ok(())
  }

  fn update_parent(&self, note_id: &NoteId, parent: Option<&NoteId>) -> ShizenResult<()> {
    let mut conn = self.conn.borrow_mut();
    let txn = conn.transaction()?;

    let old_parent = Self::load_note_txn(&txn, note_id)?.parent_id;
    Self::change_parent_txn(&txn, note_id, parent, old_parent.as_ref())?;

    txn.commit()?;
    Ok(())
  }

  fn add_blocked_note(&self, note_id: &NoteId, blocked_note: &NoteId) -> ShizenResult<()> {
    let mut conn = self.conn.borrow_mut();
    let txn = conn.transaction()?;

    Self::add_dep_txn(&txn, note_id, blocked_note)?;

    txn.commit()?;
    Ok(())
  }

  fn remove_blocked_note(&self, note_id: &NoteId, blocked_note: &NoteId) -> ShizenResult<()> {
    let mut conn = self.conn.borrow_mut();
    let txn = conn.transaction()?;

    Self::remove_dep_txn(&txn, note_id, blocked_note)?;

    txn.commit()?;
    Ok(())
  }

  fn set_peer_clock(&self, peer_id: &PeerId, clock: usize) -> ShizenResult<()> {
    let conn = self.conn.borrow_mut();

    conn.execute(
      "UPDATE Peers SET clock = ? WHERE peer_id = ?",
      (clock, peer_id.0.to_string()),
    )?;

    Ok(())
  }

  fn adjust_rank_between(
    &self,
    note_id: &NoteId,
    after: Option<&NoteId>,
    before: Option<&NoteId>,
  ) -> ShizenResult<()> {
    if after.is_none() && before.is_none() {
      return Err(ShizenError::InvalidRankAdjustment);
    }

    let mut conn = self.conn.borrow_mut();
    let txn = conn.transaction()?;

    Self::adjust_rank_between_txn(&txn, note_id, after, before)?;

    txn.commit()?;
    Ok(())
  }

  fn undo(&self) -> ShizenResult<usize> {
    let mut conn = self.conn.borrow_mut();
    let txn = conn.transaction()?;

    let start_clock = Self::get_clock_txn(&txn)?;

    let q = txn.query_row_and_then(
      "SELECT id, action_json, clock FROM Mutations ORDER BY id DESC LIMIT 1",
      [],
      |r| -> ShizenResult<_> {
        let id_to_remove: u32 = r.get(0)?;
        let undo_action_json: String = r.get(1)?;
        let clock: usize = r.get(2)?;

        let undo_action: Action = serde_json::from_str(&undo_action_json)?;

        Ok((id_to_remove, undo_action, clock, undo_action_json))
      },
    );

    if let Err(ShizenError::RusqliteError(QueryReturnedNoRows)) = q {
      return Ok(start_clock);
    }

    let (id_to_remove, undo_action, new_clock, undo_action_json) = q?;

    trace!("Executing undo action: {:#?}", undo_action);

    match undo_action {
      Action::CreateNote { id, parent, .. } => {
        txn.execute("DELETE FROM Notes WHERE uuid = ?", [id.0.to_string()])?;
        if parent.is_some() {
          txn.execute("DELETE FROM Children WHERE child = ?", [id.0.to_string()])?;
        }
      }
      Action::SetCompleted {
        id, old_completed, ..
      } => {
        txn.execute(
          "UPDATE Notes SET completed = ? WHERE uuid = ?",
          (old_completed, id.0.to_string()),
        )?;
      }
      Action::UpdateTitle { id, old_title, .. } => {
        txn.execute(
          "UPDATE Notes SET title = ? WHERE uuid = ?",
          [old_title, id.0.to_string()],
        )?;
      }
      Action::UpdateDescription {
        id,
        old_description,
        ..
      } => {
        txn.execute(
          "UPDATE Notes SET description = ? WHERE uuid = ?",
          (old_description, id.0.to_string()),
        )?;
      }
      Action::ChangeParent {
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
      Action::AddDependency { blocker, blockee } => {
        txn.execute(
          "DELETE FROM Dependencies WHERE blocker = ? and blockee = ?",
          (blocker.0.to_string(), blockee.0.to_string()),
        )?;
      }
      Action::RemoveDependency { blocker, blockee } => {
        txn.execute(
          "INSERT INTO Dependencies (blocker, blockee) VALUES (?, ?)",
          (blocker.0.to_string(), blockee.0.to_string()),
        )?;
      }
      Action::DeleteNote { note } => {
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
      Action::ReorderNote { id, old_rank, .. } => {
        txn.execute(
          "UPDATE Notes rank = ? WHERE uuid = ?",
          (old_rank, id.0.to_string()),
        )?;
      }
    }

    txn.execute("DELETE FROM Mutations WHERE id = ?", [id_to_remove])?;
    Self::insert_redo_mutations_json(&txn, &[undo_action_json])?;

    txn.execute("UPDATE LocalSettings SET clock = ?", [new_clock])?;

    txn.commit()?;
    Ok(new_clock)
  }

  fn redo(&self) -> ShizenResult<usize> {
    let mut conn = self.conn.borrow_mut();
    let txn = conn.transaction()?;

    let start_clock = Self::get_clock_txn(&txn)?;

    let q = txn.query_row_and_then(
      "SELECT id, action_json FROM RedoMutations ORDER BY id DESC LIMIT 1",
      [],
      |r| -> ShizenResult<_> {
        let id_to_remove: u32 = r.get(0)?;
        let redo_action_json: String = r.get(1)?;
        let redo_action: Action = serde_json::from_str(&redo_action_json)?;

        Ok((id_to_remove, redo_action))
      },
    );

    if let Err(ShizenError::RusqliteError(QueryReturnedNoRows)) = q {
      return Ok(start_clock);
    }

    let (id_to_remove, redo_action) = q?;

    trace!("Executing redo action: {redo_action:#?}");

    match redo_action {
      Action::CreateNote {
        id,
        parent,
        title,
        description,
      } => {
        Self::create_new_note_txn(
          &txn,
          Some(&id),
          &title,
          description.as_deref(),
          parent.as_ref(),
        )?;
      }
      Action::SetCompleted {
        id,
        new_completed,
        old_completed,
      } => {
        Self::set_completed_txn(&txn, &id, new_completed, old_completed)?;
      }
      Action::UpdateTitle {
        id,
        old_title,
        new_title,
      } => {
        Self::update_title_txn(&txn, &id, &new_title, &old_title)?;
      }
      Action::UpdateDescription {
        id,
        old_description,
        new_description,
      } => {
        Self::update_description_txn(
          &txn,
          &id,
          new_description.as_deref(),
          old_description.as_deref(),
        )?;
      }
      Action::ChangeParent { id, old_parent, .. } => {
        Self::change_parent_txn(&txn, &id, old_parent.as_ref(), old_parent.as_ref())?;
      }
      Action::AddDependency { blocker, blockee } => {
        Self::add_dep_txn(&txn, &blocker, &blockee)?;
      }
      Action::RemoveDependency { blocker, blockee } => {
        Self::remove_dep_txn(&txn, &blocker, &blockee)?;
      }
      Action::DeleteNote { note } => {
        Self::delete_note_txn(&txn, &note.id)?;
      }
      Action::ReorderNote {
        id, before, after, ..
      } => {
        Self::adjust_rank_between_txn(&txn, &id, after.as_ref(), before.as_ref())?;
      }
    }

    txn.execute("DELETE FROM RedoMutations WHERE id = ?", [id_to_remove])?;
    let new_clock = Self::get_clock_txn(&txn)?;

    txn.commit()?;
    Ok(new_clock)
  }

  fn delete_note(&self, note_id: &NoteId) -> ShizenResult<()> {
    let mut conn = self.conn.borrow_mut();
    let txn = conn.transaction()?;

    Self::delete_note_txn(&txn, note_id)?;

    txn.commit()?;

    Ok(())
  }

  fn remove_peer(&self, peer_id: &PeerId) -> ShizenResult<()> {
    let conn = self.conn.borrow_mut();
    conn.execute(
      "DELETE FROM Peers WHERE peer_id = ?",
      [peer_id.0.to_string()],
    )?;
    Ok(())
  }

  fn sync_with_peer(&self, peer: &PeerInfo) -> ShizenResult<SyncResults> {
    info!("Beginning sync with peer: {peer:#?}");

    let this_peer_id = self.get_peer_id()?;
    let peer_addr = self.get_peer(&peer.peer_id)?.addr;
    let mut sync_conn = SyncConnection::new(&this_peer_id, peer_addr, None)?;

    // Check that peer is added to peers table, if not handshake it.
    let peer = {
      let stored_peer_info = self.get_peer(&peer.peer_id);
      if let Err(_) = stored_peer_info {
        warn!("Peer is not in the database, handshaking...");

        let peer_id = sync_conn.peer_id_handshake()?;
        self.get_peer(&peer_id)?
      } else {
        stored_peer_info.unwrap()
      }
    };

    sync_conn.sync_with_peer(self, &peer)
  }

  fn apply_action(&self, action: &Action) -> ShizenResult<()> {
    let mut conn = self.conn.borrow_mut();
    let txn = conn.transaction()?;

    trace!("Applying action {action:#?}");

    match action {
      Action::CreateNote {
        id,
        title,
        description,
        parent,
      } => {
        Self::create_new_note_txn(
          &txn,
          Some(id),
          title,
          description.as_ref().map(|d| d.as_str()),
          parent.as_ref(),
        )?;
      }
      Action::SetCompleted {
        id,
        new_completed,
        old_completed,
      } => {
        Self::set_completed_txn(&txn, id, *new_completed, *old_completed)?;
      }
      Action::UpdateTitle {
        id,
        old_title,
        new_title,
      } => {
        Self::update_title_txn(&txn, id, new_title, old_title)?;
      }
      Action::UpdateDescription {
        id,
        old_description,
        new_description,
      } => {
        Self::update_description_txn(
          &txn,
          id,
          new_description.as_ref().map(|d| d.as_str()),
          old_description.as_ref().map(|d| d.as_str()),
        )?;
      }
      Action::ChangeParent {
        id,
        old_parent,
        new_parent,
      } => {
        Self::change_parent_txn(&txn, id, new_parent.as_ref(), old_parent.as_ref())?;
      }
      Action::AddDependency { blocker, blockee } => {
        Self::add_dep_txn(&txn, blocker, blockee)?;
      }
      Action::RemoveDependency { blocker, blockee } => {
        Self::remove_dep_txn(&txn, blocker, blockee)?;
      }
      Action::DeleteNote { note } => {
        Self::delete_note_txn(&txn, &note.id)?;
      }
      Action::ReorderNote {
        id, before, after, ..
      } => {
        Self::adjust_rank_between_txn(&txn, id, before.as_ref(), after.as_ref())?;
      }
    }

    txn.commit()?;
    Ok(())
  }
}

fn database_is_initialized(conn: &Connection) -> ShizenResult<bool> {
  Ok(conn.query_row(
    "SELECT count(name) FROM sqlite_master WHERE type='table' AND name='SchemaVersion'",
    (),
    |row| row.get(0),
  )?)
}

pub fn libshizen_sql_init_str() -> &'static str {
  sqlite_str!("/sql/sqlite/init.sql")
}

const MIGRATE_DB_TO_VERSION: usize = 2;
/// Array of tuples of sql text to run along with functional alterations to get
/// the database migrated into a good state (eg assigning ranks).
const SCHEMA_MIGRATIONS: [(&str, Option<fn(conn: &Connection) -> ShizenResult<()>>); 1] = [(
  sqlite_str!("/sql/sqlite/migrations/1_add_user_order.sql"),
  Some(schema_migration_add_user_order),
)];

fn schema_migration_add_user_order(conn: &Connection) -> ShizenResult<()> {
  let mut lexorank = LexoRank::new(Bucket::new(0).unwrap(), Rank::new("a").unwrap());
  let mut stmnt = conn.prepare("SELECT uuid FROM Notes")?;
  let mut rows = stmnt.query([])?;

  while let Some(row) = rows.next()? {
    let uuid: String = row.get(0)?;
    conn.execute(
      "UPDATE Notes SET rank = ? WHERE UUID = ?",
      (lexorank.to_string(), uuid),
    )?;
    lexorank = lexorank.next();
  }

  Ok(())
}

fn run_schema_migrations(conn: &mut Connection) -> ShizenResult<()> {
  let version = get_schema_version(conn)?;

  for v in version..MIGRATE_DB_TO_VERSION {
    let (sql, alteration) = &SCHEMA_MIGRATIONS[v - 1];
    info!("Running schema migration:\n{sql}");

    let txn = conn.transaction()?;
    txn
      .execute_batch(sql)
      .map_err(|e| ShizenError::MigrationError(v, e))?;

    if let Some(f) = alteration {
      info!("running associated update logic...");
      f(&txn)?;
    }

    set_schema_version(&txn, v + 1)?;

    txn.commit()?;
    info!("Done with migration from {v}!");
  }

  Ok(())
}

fn get_schema_version(conn: &Connection) -> ShizenResult<usize> {
  Ok(conn.query_row("SELECT version FROM SchemaVersion", (), |r| r.get(0))?)
}

fn set_schema_version(conn: &Connection, version: usize) -> ShizenResult<()> {
  conn.execute("UPDATE SchemaVersion SET version = ?", [version])?;
  Ok(())
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
    let s = RusqliteStorage::open(None).unwrap();

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
    let s = RusqliteStorage::open(None).unwrap();
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
    let s = RusqliteStorage::open(None).unwrap();
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
    let s = RusqliteStorage::open(None).unwrap();
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

    let conn = s.conn.borrow();
    assert!(RusqliteStorage::is_descendent_of(&conn, &picard_note.id, &riker.id).unwrap());
    assert!(RusqliteStorage::is_descendent_of(&conn, &picard_note.id, &worf.id).unwrap());
    assert!(RusqliteStorage::is_descendent_of(&conn, &riker.id, &worf.id).unwrap());
  }

  #[test]
  #[traced_test]
  fn update_title() {
    let s = RusqliteStorage::open(None).unwrap();
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
    let s = RusqliteStorage::open(None).unwrap();
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
    let s = RusqliteStorage::open(None).unwrap();
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
    let s = RusqliteStorage::open(None).unwrap();
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
    let s = RusqliteStorage::open(None).unwrap();
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
    let s = RusqliteStorage::open(None).unwrap();
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

    s.remove_blocked_note(&wesley.id, &bev.id).unwrap();
    s.add_blocked_note(&picard_note.id, &wesley.id).unwrap();
  }

  #[test]
  #[traced_test]
  fn undo_create() {
    let s = RusqliteStorage::open(None).unwrap();

    let clock = s.get_clock().unwrap();
    assert_eq!(0, clock);

    let picard_note = s
      .create_new_note("Picard", "captain of the enterprise".into(), None)
      .unwrap();
    let clock = s.get_clock().unwrap();
    assert_eq!(1, clock);

    let riker = s
      .create_new_note(
        "Riker",
        "Number One of the enterprise".into(),
        Some(&picard_note.id),
      )
      .unwrap();
    let clock = s.get_clock().unwrap();
    assert_eq!(2, clock);

    let bev = s
      .create_new_note("Beverly Crusher", "Capable and attractive doc".into(), None)
      .unwrap();
    let clock = s.get_clock().unwrap();
    assert_eq!(3, clock);

    s.add_blocked_note(&bev.id, &picard_note.id).unwrap();
    let clock = s.get_clock().unwrap();
    assert_eq!(4, clock);

    let before_undo_picard = s.load_note(&picard_note.id).unwrap();
    let before_undo_riker = s.load_note(&riker.id).unwrap();
    let before_undo_bev = s.load_note(&bev.id).unwrap();

    // Undo dep
    s.undo().unwrap();

    let clock = s.get_clock().unwrap();
    assert_eq!(3, clock);

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

    let clock = s.get_clock().unwrap();
    assert_eq!(2, clock);

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

    let clock = s.get_clock().unwrap();
    assert_eq!(1, clock);

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

    let clock = s.get_clock().unwrap();
    assert_eq!(0, clock);

    let readback_picard = s.load_note(&picard_note.id);
    assert!(readback_picard.is_err());
  }

  #[test]
  #[traced_test]
  fn initialized_local_settings() {
    let s = RusqliteStorage::open(None).unwrap();
    s.get_peer_id().unwrap();
    let c = s.get_clock().unwrap();
    assert_eq!(c, 0);
  }

  #[test]
  #[traced_test]
  fn clock_increments() {
    let s = RusqliteStorage::open(None).unwrap();

    let clock = s.get_clock().unwrap();
    assert_eq!(clock, 0);

    let picard_note = s
      .create_new_note("Picard", "captain of the enterprise".into(), None)
      .unwrap();

    let clock = s.get_clock().unwrap();
    assert_eq!(clock, 1);

    let _riker = s
      .create_new_note(
        "Riker",
        "Number One of the enterprise".into(),
        Some(&picard_note.id),
      )
      .unwrap();

    let clock = s.get_clock().unwrap();
    assert_eq!(clock, 2);
  }

  #[test]
  #[traced_test]
  fn adjust_rank_between_test() {
    let s = RusqliteStorage::open(None).unwrap();

    let a = s.create_new_note("a", None, None).unwrap();
    let b = s.create_new_note("b", None, None).unwrap();
    let c = s.create_new_note("c", None, None).unwrap();

    let notes = s.load_all_notes().unwrap();
    assert_eq!(notes, vec![a.clone(), b.clone(), c.clone()]);

    s.adjust_rank_between(&a.id, Some(&b.id), Some(&c.id))
      .unwrap();

    let notes = s.load_all_notes().unwrap();
    assert_eq!(notes, vec![b.clone(), a.clone(), c.clone()]);

    s.adjust_rank_between(&c.id, None, Some(&b.id)).unwrap();
    let notes = s.load_all_notes().unwrap();
    assert_eq!(notes, vec![c.clone(), b.clone(), a.clone()]);

    s.adjust_rank_between(&c.id, Some(&a.id), None).unwrap();
    let notes = s.load_all_notes().unwrap();
    assert_eq!(notes, vec![b.clone(), a.clone(), c.clone()]);
  }

  // TODO cannot add self as peer test.
  // TODO cannot add peer twice test.
  // TODO tests for redo
  // TODO tests for update undo.
}
