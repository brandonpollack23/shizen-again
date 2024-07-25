//! Sqlite storage engine using rusqlite.
use rusqlite::Connection;
use tracing::info;

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
  use tracing_subscriber::fmt::try_init;

  use super::*;

  fn init_tracing() {
    tracing_subscriber::fmt::init();
  }

  #[test]
  fn database_init() {
    init_tracing();
    RusqliteStorage::new(None).unwrap();
  }
}
