use rusqlite::{Connection, Result};
use std::path::Path;

const SCHEMA: &str = r#"
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS notes (
  id TEXT PRIMARY KEY,
  title TEXT NOT NULL,
  format TEXT NOT NULL CHECK (format IN ('markdown', 'html')),
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  deleted_at TEXT,
  pinned INTEGER NOT NULL DEFAULT 0,
  metadata_json TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS note_bodies (
  note_id TEXT PRIMARY KEY REFERENCES notes(id) ON DELETE CASCADE,
  content TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  byte_size INTEGER NOT NULL,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE VIRTUAL TABLE IF NOT EXISTS note_fts USING fts5(
  title,
  plain_text,
  tags,
  format,
  content='',
  tokenize='unicode61'
);

INSERT OR IGNORE INTO schema_migrations(version) VALUES (1);
"#;

pub fn open_database(path: &Path) -> Result<Connection> {
    let connection = Connection::open(path)?;
    run_migrations(&connection)?;
    Ok(connection)
}

pub fn open_memory_database() -> Result<Connection> {
    let connection = Connection::open_in_memory()?;
    run_migrations(&connection)?;
    Ok(connection)
}

pub fn run_migrations(connection: &Connection) -> Result<()> {
    connection.execute_batch(SCHEMA)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_create_core_tables() {
        let connection = open_memory_database().expect("database opens");
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('notes', 'note_bodies')",
                [],
                |row| row.get(0),
            )
            .expect("table count query succeeds");

        assert_eq!(count, 2);
    }

    #[test]
    fn migrations_enable_wal_for_file_database() {
        let temp_dir = tempfile::tempdir().expect("temp dir exists");
        let path = temp_dir.path().join("o-note.db");
        let connection = open_database(&path).expect("database opens");
        let journal_mode: String = connection
            .query_row("PRAGMA journal_mode", [], |row| row.get(0))
            .expect("journal mode query succeeds");

        assert_eq!(journal_mode, "wal");
    }
}
