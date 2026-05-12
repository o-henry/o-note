use rusqlite::{params, Connection, OptionalExtension, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;
use uuid::Uuid;

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

CREATE TABLE IF NOT EXISTS note_revisions (
  id TEXT PRIMARY KEY,
  note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
  content_hash TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  summary TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS note_events (
  id TEXT PRIMARY KEY,
  note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
  event_type TEXT NOT NULL,
  payload_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_notes_recent
  ON notes(deleted_at, updated_at DESC);

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateNoteInput {
    pub title: String,
    pub format: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListNotesQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateNoteInput {
    pub id: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteSummary {
    pub id: String,
    pub title: String,
    pub format: String,
    pub updated_at: String,
    pub byte_size: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteDetail {
    pub id: String,
    pub title: String,
    pub format: String,
    pub content: String,
    pub content_hash: String,
    pub byte_size: i64,
    pub created_at: String,
    pub updated_at: String,
}

pub fn create_note(connection: &mut Connection, input: CreateNoteInput) -> Result<NoteDetail> {
    validate_format(&input.format)?;
    let id = Uuid::new_v4().to_string();
    let title = normalize_title(&input.title);
    let content_hash = content_hash(&input.content);
    let byte_size = input.content.len() as i64;
    let tx = connection.transaction()?;

    tx.execute(
        "INSERT INTO notes(id, title, format) VALUES (?1, ?2, ?3)",
        params![id, title, input.format],
    )?;
    tx.execute(
        "INSERT INTO note_bodies(note_id, content, content_hash, byte_size)
         VALUES (?1, ?2, ?3, ?4)",
        params![id, input.content, content_hash, byte_size],
    )?;
    insert_revision(&tx, &id, &content_hash, "created")?;
    insert_event(&tx, &id, "created", "{}")?;
    tx.commit()?;

    get_note(connection, &id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn list_notes(connection: &Connection, query: ListNotesQuery) -> Result<Vec<NoteSummary>> {
    let limit = query.limit.unwrap_or(100).clamp(1, 500);
    let offset = query.offset.unwrap_or(0).max(0);
    let mut statement = connection.prepare(
        "SELECT notes.id, notes.title, notes.format, notes.updated_at, note_bodies.byte_size
         FROM notes
         JOIN note_bodies ON note_bodies.note_id = notes.id
         WHERE notes.deleted_at IS NULL
         ORDER BY notes.updated_at DESC, notes.id DESC
         LIMIT ?1 OFFSET ?2",
    )?;

    let rows = statement.query_map(params![limit, offset], |row| {
        Ok(NoteSummary {
            id: row.get(0)?,
            title: row.get(1)?,
            format: row.get(2)?,
            updated_at: row.get(3)?,
            byte_size: row.get(4)?,
        })
    })?;

    rows.collect()
}

pub fn get_note(connection: &Connection, id: &str) -> Result<Option<NoteDetail>> {
    connection
        .query_row(
            "SELECT notes.id, notes.title, notes.format, note_bodies.content,
                    note_bodies.content_hash, note_bodies.byte_size,
                    notes.created_at, notes.updated_at
             FROM notes
             JOIN note_bodies ON note_bodies.note_id = notes.id
             WHERE notes.id = ?1 AND notes.deleted_at IS NULL",
            params![id],
            |row| {
                Ok(NoteDetail {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    format: row.get(2)?,
                    content: row.get(3)?,
                    content_hash: row.get(4)?,
                    byte_size: row.get(5)?,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            },
        )
        .optional()
}

pub fn update_note(connection: &mut Connection, input: UpdateNoteInput) -> Result<NoteDetail> {
    let content_hash = content_hash(&input.content);
    let byte_size = input.content.len() as i64;
    let tx = connection.transaction()?;

    tx.execute(
        "UPDATE note_bodies
         SET content = ?1, content_hash = ?2, byte_size = ?3, updated_at = CURRENT_TIMESTAMP
         WHERE note_id = ?4",
        params![input.content, content_hash, byte_size, input.id],
    )?;
    tx.execute(
        "UPDATE notes SET updated_at = CURRENT_TIMESTAMP WHERE id = ?1 AND deleted_at IS NULL",
        params![input.id],
    )?;
    insert_revision(&tx, &input.id, &content_hash, "updated")?;
    insert_event(&tx, &input.id, "updated", "{}")?;
    tx.commit()?;

    get_note(connection, &input.id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn rename_note(connection: &Connection, id: &str, title: &str) -> Result<NoteSummary> {
    connection.execute(
        "UPDATE notes SET title = ?1, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?2 AND deleted_at IS NULL",
        params![normalize_title(title), id],
    )?;
    insert_event(connection, id, "renamed", "{}")?;
    get_note_summary(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn delete_note(connection: &Connection, id: &str) -> Result<()> {
    connection.execute(
        "UPDATE notes SET deleted_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?1 AND deleted_at IS NULL",
        params![id],
    )?;
    insert_event(connection, id, "deleted", "{}")?;
    Ok(())
}

fn get_note_summary(connection: &Connection, id: &str) -> Result<Option<NoteSummary>> {
    connection
        .query_row(
            "SELECT notes.id, notes.title, notes.format, notes.updated_at, note_bodies.byte_size
             FROM notes
             JOIN note_bodies ON note_bodies.note_id = notes.id
             WHERE notes.id = ?1 AND notes.deleted_at IS NULL",
            params![id],
            |row| {
                Ok(NoteSummary {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    format: row.get(2)?,
                    updated_at: row.get(3)?,
                    byte_size: row.get(4)?,
                })
            },
        )
        .optional()
}

fn insert_revision(
    connection: &Connection,
    note_id: &str,
    content_hash: &str,
    summary: &str,
) -> Result<()> {
    connection.execute(
        "INSERT INTO note_revisions(id, note_id, content_hash, summary) VALUES (?1, ?2, ?3, ?4)",
        params![Uuid::new_v4().to_string(), note_id, content_hash, summary],
    )?;
    Ok(())
}

fn insert_event(
    connection: &Connection,
    note_id: &str,
    event_type: &str,
    payload: &str,
) -> Result<()> {
    connection.execute(
        "INSERT INTO note_events(id, note_id, event_type, payload_json) VALUES (?1, ?2, ?3, ?4)",
        params![Uuid::new_v4().to_string(), note_id, event_type, payload],
    )?;
    Ok(())
}

fn content_hash(content: &str) -> String {
    format!("{:x}", Sha256::digest(content.as_bytes()))
}

fn normalize_title(title: &str) -> String {
    let trimmed = title.trim();

    if trimmed.is_empty() {
        "Untitled".to_string()
    } else {
        trimmed.to_string()
    }
}

fn validate_format(format: &str) -> Result<()> {
    match format {
        "markdown" | "html" => Ok(()),
        _ => Err(rusqlite::Error::InvalidParameterName("format".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_create_core_tables() {
        let connection = open_memory_database().expect("database opens");
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'table'
                 AND name IN ('notes', 'note_bodies', 'note_revisions', 'note_events')",
                [],
                |row| row.get(0),
            )
            .expect("table count query succeeds");

        assert_eq!(count, 4);
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

    #[test]
    fn note_crud_keeps_metadata_and_body_paths_separate() {
        let mut connection = open_memory_database().expect("database opens");
        let note = create_note(
            &mut connection,
            CreateNoteInput {
                title: "  First note  ".to_string(),
                format: "markdown".to_string(),
                content: "# Hello".to_string(),
            },
        )
        .expect("note is created");

        assert_eq!(note.title, "First note");
        assert_eq!(note.content, "# Hello");

        let summaries = list_notes(
            &connection,
            ListNotesQuery {
                limit: Some(100),
                offset: Some(0),
            },
        )
        .expect("notes list");

        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].byte_size, 7);

        let renamed = rename_note(&connection, &note.id, "Renamed").expect("note renamed");
        assert_eq!(renamed.title, "Renamed");

        let updated = update_note(
            &mut connection,
            UpdateNoteInput {
                id: note.id.clone(),
                content: "<h1>Hello</h1>".to_string(),
            },
        )
        .expect("note updated");
        assert_eq!(updated.byte_size, 14);

        delete_note(&connection, &note.id).expect("note deleted");
        let after_delete = list_notes(
            &connection,
            ListNotesQuery {
                limit: Some(100),
                offset: Some(0),
            },
        )
        .expect("notes list after delete");
        assert!(after_delete.is_empty());
    }

    #[test]
    fn list_notes_stays_bounded_with_large_vault_metadata() {
        let mut connection = open_memory_database().expect("database opens");
        let mut target_id = String::new();

        for index in 0..10_000 {
            let note = create_note(
                &mut connection,
                CreateNoteInput {
                    title: format!("Note {index}"),
                    format: "markdown".to_string(),
                    content: "body".to_string(),
                },
            )
            .expect("note inserted");

            if index == 9_999 {
                target_id = note.id;
            }
        }

        let started = std::time::Instant::now();
        let summaries = list_notes(
            &connection,
            ListNotesQuery {
                limit: Some(100),
                offset: Some(0),
            },
        )
        .expect("notes list");

        assert_eq!(summaries.len(), 100);
        assert!(
            started.elapsed().as_millis() <= 50,
            "metadata list exceeded 50ms budget: {:?}",
            started.elapsed()
        );

        let body_started = std::time::Instant::now();
        let note = get_note(&connection, &target_id)
            .expect("note lookup succeeds")
            .expect("note exists");
        assert_eq!(note.content, "body");
        assert!(
            body_started.elapsed().as_millis() <= 100,
            "body load exceeded 100ms budget: {:?}",
            body_started.elapsed()
        );
    }
}
