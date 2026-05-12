use rusqlite::{params, Connection, OptionalExtension, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
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

INSERT OR IGNORE INTO schema_migrations(version) VALUES (1);
"#;

const SEARCH_SCHEMA: &str = r#"
DROP TABLE IF EXISTS note_fts;

CREATE VIRTUAL TABLE note_fts USING fts5(
  note_id UNINDEXED,
  title,
  plain_text,
  tags,
  format,
  tokenize='unicode61'
);

CREATE TABLE IF NOT EXISTS note_links (
  id TEXT PRIMARY KEY,
  source_note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
  target TEXT NOT NULL,
  link_type TEXT NOT NULL,
  anchor_text TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS index_jobs (
  note_id TEXT PRIMARY KEY REFERENCES notes(id) ON DELETE CASCADE,
  content_hash TEXT NOT NULL,
  status TEXT NOT NULL,
  attempts INTEGER NOT NULL DEFAULT 0,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS index_state (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

PRAGMA user_version = 2;
INSERT OR IGNORE INTO schema_migrations(version) VALUES (2);
"#;

const IMPORT_EXPORT_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS attachments (
  id TEXT PRIMARY KEY,
  content_hash TEXT NOT NULL UNIQUE,
  byte_size INTEGER NOT NULL,
  original_path TEXT NOT NULL,
  storage_path TEXT NOT NULL,
  ref_count INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS note_attachments (
  note_id TEXT NOT NULL REFERENCES notes(id) ON DELETE CASCADE,
  attachment_id TEXT NOT NULL REFERENCES attachments(id) ON DELETE CASCADE,
  relative_path TEXT NOT NULL,
  PRIMARY KEY(note_id, attachment_id, relative_path)
);

CREATE TABLE IF NOT EXISTS import_runs (
  id TEXT PRIMARY KEY,
  root_path TEXT NOT NULL,
  status TEXT NOT NULL,
  imported_notes INTEGER NOT NULL DEFAULT 0,
  imported_attachments INTEGER NOT NULL DEFAULT 0,
  skipped_files INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

PRAGMA user_version = 3;
INSERT OR IGNORE INTO schema_migrations(version) VALUES (3);
"#;

const RELIABILITY_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS audit_log (
  id TEXT PRIMARY KEY,
  event_type TEXT NOT NULL,
  subject_id TEXT NOT NULL,
  payload_json TEXT NOT NULL DEFAULT '{}',
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_audit_recent
  ON audit_log(created_at DESC, event_type);

PRAGMA user_version = 4;
INSERT OR IGNORE INTO schema_migrations(version) VALUES (4);
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
    let mut user_version: i64 =
        connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    if user_version < 2 {
        connection.execute_batch(SEARCH_SCHEMA)?;
        rebuild_search_index(connection)?;
        user_version = 2;
    }

    if user_version < 3 {
        connection.execute_batch(IMPORT_EXPORT_SCHEMA)?;
        user_version = 3;
    }

    if user_version < 4 {
        connection.execute_batch(RELIABILITY_SCHEMA)?;
    }

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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchNotesQuery {
    pub query: String,
    pub limit: Option<i64>,
    pub format: Option<String>,
    pub tag: Option<String>,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub id: String,
    pub title: String,
    pub format: String,
    pub snippet: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexHealth {
    pub pending: i64,
    pub indexed: i64,
    pub failed: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPathInput {
    pub path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportReport {
    pub run_id: String,
    pub imported_notes: i64,
    pub imported_attachments: i64,
    pub skipped_files: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportNoteInput {
    pub id: String,
    pub path: String,
    pub bundle: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportReport {
    pub output_path: String,
    pub files_written: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInput {
    pub path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReliabilityReport {
    pub status: String,
    pub detail: String,
}

struct NoteLink {
    target: String,
    link_type: String,
    anchor_text: String,
}

struct ImportedNote {
    id: String,
    source_path: PathBuf,
    format: String,
    title: String,
    content: String,
    content_hash: String,
    frontmatter: Vec<(String, String)>,
    links: Vec<NoteLink>,
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
    queue_index_job(&tx, &id, &content_hash)?;
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
    let previous_hash = get_note(connection, &input.id)?
        .ok_or(rusqlite::Error::QueryReturnedNoRows)?
        .content_hash;

    if previous_hash == content_hash {
        return get_note(connection, &input.id)?.ok_or(rusqlite::Error::QueryReturnedNoRows);
    }

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
    queue_index_job(&tx, &input.id, &content_hash)?;
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
    let content_hash: String = connection.query_row(
        "SELECT content_hash FROM note_bodies WHERE note_id = ?1",
        params![id],
        |row| row.get(0),
    )?;
    queue_index_job(connection, id, &content_hash)?;
    get_note_summary(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)
}

pub fn delete_note(connection: &Connection, id: &str) -> Result<()> {
    connection.execute(
        "UPDATE notes SET deleted_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?1 AND deleted_at IS NULL",
        params![id],
    )?;
    insert_event(connection, id, "deleted", "{}")?;
    remove_from_index(connection, id)?;
    connection.execute("DELETE FROM index_jobs WHERE note_id = ?1", params![id])?;
    Ok(())
}

pub fn search_notes(connection: &Connection, query: SearchNotesQuery) -> Result<Vec<SearchResult>> {
    let normalized = normalize_search_query(&query.query);
    let format_filter = query
        .format
        .filter(|format| matches!(format.as_str(), "markdown" | "html"));
    let tag_filter = query
        .tag
        .map(|tag| tag.trim().trim_start_matches('#').to_string());

    if normalized.is_empty() {
        return Ok(Vec::new());
    }

    let limit = query.limit.unwrap_or(20).clamp(1, 100);
    let mut statement = connection.prepare(
        "SELECT notes.id, notes.title, notes.format,
                snippet(note_fts, 2, '<mark>', '</mark>', '...', 12) AS snippet,
                notes.updated_at
         FROM note_fts
         JOIN notes ON notes.id = note_fts.note_id
         WHERE note_fts MATCH ?1
           AND notes.deleted_at IS NULL
           AND (?3 IS NULL OR notes.format = ?3)
           AND (?4 IS NULL OR instr(note_fts.tags, ?4) > 0)
         ORDER BY rank
         LIMIT ?2",
    )?;
    let rows = statement.query_map(
        params![normalized, limit, format_filter, tag_filter],
        |row| {
            Ok(SearchResult {
                id: row.get(0)?,
                title: row.get(1)?,
                format: row.get(2)?,
                snippet: row.get(3)?,
                updated_at: row.get(4)?,
            })
        },
    )?;

    rows.collect()
}

pub fn rebuild_search_index(connection: &Connection) -> Result<()> {
    connection.execute("DELETE FROM note_fts", [])?;
    connection.execute("DELETE FROM note_links", [])?;
    let mut statement = connection.prepare(
        "SELECT notes.id
         FROM notes
         JOIN note_bodies ON note_bodies.note_id = notes.id
         WHERE notes.deleted_at IS NULL",
    )?;
    let ids = statement
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>>>()?;

    for id in ids {
        index_note(connection, &id)?;
    }

    Ok(())
}

pub fn process_index_jobs(connection: &Connection, batch_size: i64) -> Result<usize> {
    let limit = batch_size.clamp(1, 250);
    let mut statement = connection.prepare(
        "SELECT note_id
         FROM index_jobs
         WHERE status IN ('queued', 'failed')
         ORDER BY updated_at ASC, note_id ASC
         LIMIT ?1",
    )?;
    let note_ids = statement
        .query_map(params![limit], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>>>()?;
    let count = note_ids.len();

    for note_id in note_ids {
        match index_note(connection, &note_id) {
            Ok(()) => {}
            Err(error) => {
                connection.execute(
                    "UPDATE index_jobs
                     SET status = 'failed', attempts = attempts + 1, updated_at = CURRENT_TIMESTAMP
                     WHERE note_id = ?1",
                    params![note_id],
                )?;
                return Err(error);
            }
        }
    }

    if count > 0 {
        write_audit_log(
            connection,
            "index_batch",
            "note_fts",
            &format!("{{\"processed\":{count}}}"),
        )?;
    }

    Ok(count)
}

pub fn index_health(connection: &Connection) -> Result<IndexHealth> {
    let pending = count_index_jobs(connection, "queued")?;
    let indexed = count_index_jobs(connection, "indexed")?;
    let failed = count_index_jobs(connection, "failed")?;

    Ok(IndexHealth {
        pending,
        indexed,
        failed,
    })
}

pub fn import_path(
    connection: &mut Connection,
    root_path: &Path,
    attachments_dir: &Path,
) -> Result<ImportReport> {
    let run_id = Uuid::new_v4().to_string();
    let manifest = scan_import_manifest(root_path)?;
    fs::create_dir_all(attachments_dir).map_err(io_error)?;

    connection.execute(
        "INSERT INTO import_runs(id, root_path, status) VALUES (?1, ?2, 'running')",
        params![run_id, root_path.display().to_string()],
    )?;

    let imported_notes = manifest
        .iter()
        .filter(|path| is_note_path(path))
        .map(|path| parse_imported_note(root_path, path))
        .collect::<Result<Vec<_>>>()?;
    let tx = connection.transaction()?;

    for note in &imported_notes {
        tx.execute(
            "INSERT INTO notes(id, title, format, metadata_json)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                note.id,
                note.title,
                note.format,
                import_metadata_json(root_path, note)
            ],
        )?;
    }

    for note in &imported_notes {
        tx.execute(
            "INSERT INTO note_bodies(note_id, content, content_hash, byte_size)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                note.id,
                note.content,
                note.content_hash,
                note.content.len() as i64
            ],
        )?;
        insert_revision(&tx, &note.id, &note.content_hash, "imported")?;
        insert_event(&tx, &note.id, "imported", "{}")?;
        queue_index_job(&tx, &note.id, &note.content_hash)?;
    }

    let attachment_paths = manifest
        .iter()
        .filter(|path| !is_note_path(path))
        .cloned()
        .collect::<Vec<_>>();
    let mut imported_attachments = 0;

    for attachment_path in &attachment_paths {
        if attachment_path.is_file() {
            import_attachment(&tx, root_path, attachment_path, attachments_dir)?;
            imported_attachments += 1;
        }
    }

    for note in &imported_notes {
        for link in &note.links {
            let target_path = note
                .source_path
                .parent()
                .unwrap_or(root_path)
                .join(&link.target);
            if target_path.is_file() && !is_note_path(&target_path) {
                let attachment_id =
                    import_attachment(&tx, root_path, &target_path, attachments_dir)?;
                tx.execute(
                    "INSERT OR IGNORE INTO note_attachments(note_id, attachment_id, relative_path)
                     VALUES (?1, ?2, ?3)",
                    params![note.id, attachment_id, link.target],
                )?;
            }
        }
    }

    let skipped_files = manifest.len() as i64 - imported_notes.len() as i64 - imported_attachments;
    tx.execute(
        "UPDATE import_runs
         SET status = 'complete',
             imported_notes = ?1,
             imported_attachments = ?2,
             skipped_files = ?3,
             updated_at = CURRENT_TIMESTAMP
         WHERE id = ?4",
        params![
            imported_notes.len() as i64,
            imported_attachments,
            skipped_files.max(0),
            run_id
        ],
    )?;
    write_audit_log(
        &tx,
        "import",
        &run_id,
        &format!(
            "{{\"notes\":{},\"attachments\":{}}}",
            imported_notes.len(),
            imported_attachments
        ),
    )?;
    tx.commit()?;

    Ok(ImportReport {
        run_id,
        imported_notes: imported_notes.len() as i64,
        imported_attachments,
        skipped_files: skipped_files.max(0),
    })
}

pub fn export_note(connection: &Connection, input: ExportNoteInput) -> Result<ExportReport> {
    let note = get_note(connection, &input.id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    let output_root = PathBuf::from(input.path);
    fs::create_dir_all(&output_root).map_err(io_error)?;
    let extension = if note.format == "html" { "html" } else { "md" };
    let file_name = format!("{}.{}", sanitize_file_name(&note.title), extension);

    if input.bundle {
        let bundle_dir = output_root.join(format!("{}.bundle", sanitize_file_name(&note.title)));
        fs::create_dir_all(&bundle_dir).map_err(io_error)?;
        let source_path = bundle_dir.join(&file_name);
        fs::write(&source_path, &note.content).map_err(io_error)?;
        fs::write(
            bundle_dir.join("metadata.json"),
            note_metadata_json(connection, &note.id)?,
        )
        .map_err(io_error)?;
        fs::write(
            bundle_dir.join("manifest.json"),
            format!(
                "{{\"noteId\":\"{}\",\"format\":\"{}\",\"source\":\"{}\"}}\n",
                json_escape(&note.id),
                json_escape(&note.format),
                json_escape(&file_name)
            ),
        )
        .map_err(io_error)?;
        write_audit_log(connection, "export", &note.id, "{\"bundle\":true}")?;
        return Ok(ExportReport {
            output_path: bundle_dir.display().to_string(),
            files_written: 3,
        });
    }

    let output_path = output_root.join(file_name);
    fs::write(&output_path, &note.content).map_err(io_error)?;
    write_audit_log(connection, "export", &note.id, "{\"bundle\":false}")?;
    Ok(ExportReport {
        output_path: output_path.display().to_string(),
        files_written: 1,
    })
}

pub fn database_integrity(connection: &Connection) -> Result<ReliabilityReport> {
    let detail: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    let status = if detail == "ok" { "ok" } else { "failed" }.to_string();

    Ok(ReliabilityReport { status, detail })
}

pub fn backup_database(connection: &Connection, path: &Path) -> Result<ReliabilityReport> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(io_error)?;
    }
    if path.exists() {
        fs::remove_file(path).map_err(io_error)?;
    }
    let escaped_path = path.display().to_string().replace('\'', "''");
    connection.execute_batch(&format!("VACUUM INTO '{escaped_path}'"))?;
    write_audit_log(
        connection,
        "backup",
        &path.display().to_string(),
        "{\"status\":\"complete\"}",
    )?;

    Ok(ReliabilityReport {
        status: "ok".to_string(),
        detail: path.display().to_string(),
    })
}

pub fn repair_search_index(connection: &Connection) -> Result<ReliabilityReport> {
    rebuild_search_index(connection)?;
    write_audit_log(
        connection,
        "index_repair",
        "note_fts",
        "{\"status\":\"complete\"}",
    )?;

    Ok(ReliabilityReport {
        status: "ok".to_string(),
        detail: "search index rebuilt".to_string(),
    })
}

pub fn prune_revisions(connection: &Connection, keep_per_note: i64) -> Result<ReliabilityReport> {
    let keep = keep_per_note.max(1);
    let removed = connection.execute(
        "DELETE FROM note_revisions
         WHERE id IN (
           SELECT id FROM (
             SELECT id,
                    ROW_NUMBER() OVER (
                      PARTITION BY note_id
                      ORDER BY created_at DESC, id DESC
                    ) AS revision_rank
             FROM note_revisions
           )
           WHERE revision_rank > ?1
         )",
        params![keep],
    )?;
    write_audit_log(
        connection,
        "revision_prune",
        "note_revisions",
        &format!("{{\"removed\":{removed}}}"),
    )?;

    Ok(ReliabilityReport {
        status: "ok".to_string(),
        detail: format!("{removed} revisions removed"),
    })
}

pub fn extract_search_text(format: &str, content: &str) -> String {
    match format {
        "html" => extract_html_text(content),
        _ => extract_markdown_text(content),
    }
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

fn index_note(connection: &Connection, id: &str) -> Result<()> {
    let note = get_note(connection, id)?.ok_or(rusqlite::Error::QueryReturnedNoRows)?;
    let plain_text = extract_search_text(&note.format, &note.content);
    let tags = extract_tags(&note.content).join(" ");

    remove_from_index(connection, id)?;
    connection.execute(
        "INSERT INTO note_fts(note_id, title, plain_text, tags, format)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![note.id, note.title, plain_text, tags, note.format],
    )?;
    replace_note_links(connection, id, &note.format, &note.content)?;
    connection.execute(
        "INSERT INTO index_jobs(note_id, content_hash, status, attempts, updated_at)
         VALUES (?1, ?2, 'indexed', 0, CURRENT_TIMESTAMP)
         ON CONFLICT(note_id) DO UPDATE SET
           content_hash = excluded.content_hash,
           status = 'indexed',
           updated_at = CURRENT_TIMESTAMP",
        params![id, note.content_hash],
    )?;
    Ok(())
}

fn queue_index_job(connection: &Connection, id: &str, content_hash: &str) -> Result<()> {
    connection.execute(
        "INSERT INTO index_jobs(note_id, content_hash, status, attempts, updated_at)
         VALUES (?1, ?2, 'queued', 0, CURRENT_TIMESTAMP)
         ON CONFLICT(note_id) DO UPDATE SET
           content_hash = excluded.content_hash,
           status = 'queued',
           updated_at = CURRENT_TIMESTAMP",
        params![id, content_hash],
    )?;
    Ok(())
}

fn remove_from_index(connection: &Connection, id: &str) -> Result<()> {
    connection.execute("DELETE FROM note_fts WHERE note_id = ?1", params![id])?;
    connection.execute(
        "DELETE FROM note_links WHERE source_note_id = ?1",
        params![id],
    )?;
    Ok(())
}

fn replace_note_links(
    connection: &Connection,
    note_id: &str,
    format: &str,
    content: &str,
) -> Result<()> {
    connection.execute(
        "DELETE FROM note_links WHERE source_note_id = ?1",
        params![note_id],
    )?;

    for link in extract_links(format, content) {
        connection.execute(
            "INSERT INTO note_links(id, source_note_id, target, link_type, anchor_text)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                Uuid::new_v4().to_string(),
                note_id,
                link.target,
                link.link_type,
                link.anchor_text
            ],
        )?;
    }

    Ok(())
}

fn count_index_jobs(connection: &Connection, status: &str) -> Result<i64> {
    connection.query_row(
        "SELECT COUNT(*) FROM index_jobs WHERE status = ?1",
        params![status],
        |row| row.get(0),
    )
}

fn write_audit_log(
    connection: &Connection,
    event_type: &str,
    subject_id: &str,
    payload_json: &str,
) -> Result<()> {
    connection.execute(
        "INSERT INTO audit_log(id, event_type, subject_id, payload_json)
         VALUES (?1, ?2, ?3, ?4)",
        params![
            Uuid::new_v4().to_string(),
            event_type,
            subject_id,
            payload_json
        ],
    )?;
    Ok(())
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

fn normalize_search_query(query: &str) -> String {
    query
        .split_whitespace()
        .map(escape_fts_token)
        .filter(|part| !part.is_empty())
        .map(|part| format!("{part}*"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn escape_fts_token(token: &str) -> String {
    token
        .chars()
        .filter(|character| character.is_alphanumeric() || *character == '_' || *character == '-')
        .collect()
}

fn extract_markdown_text(content: &str) -> String {
    content
        .replace(['#', '*', '`', '>', '[', ']', '(', ')'], " ")
        .lines()
        .filter(|line| !line.trim_start().starts_with("```"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_html_text(content: &str) -> String {
    let without_scripts = strip_tag_block(content, "script");
    let without_styles = strip_tag_block(&without_scripts, "style");
    let mut output = String::with_capacity(without_styles.len());
    let mut in_tag = false;

    for character in without_styles.chars() {
        match character {
            '<' => {
                in_tag = true;
                output.push(' ');
            }
            '>' => {
                in_tag = false;
                output.push(' ');
            }
            _ if !in_tag => output.push(character),
            _ => {}
        }
    }

    decode_basic_entities(&output)
}

fn strip_tag_block(content: &str, tag: &str) -> String {
    let mut remaining = content.to_string();
    let open = format!("<{tag}");
    let close = format!("</{tag}>");

    loop {
        let lower = remaining.to_lowercase();
        let Some(start) = lower.find(&open) else {
            return remaining;
        };
        let Some(relative_end) = lower[start..].find(&close) else {
            remaining.replace_range(start.., "");
            return remaining;
        };
        let end = start + relative_end + close.len();
        remaining.replace_range(start..end, " ");
    }
}

fn decode_basic_entities(content: &str) -> String {
    content
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
}

fn extract_tags(content: &str) -> Vec<String> {
    let mut tags = Vec::new();

    for token in content.split_whitespace() {
        let tag = token
            .trim_matches(|character: char| {
                !character.is_alphanumeric()
                    && character != '#'
                    && character != '_'
                    && character != '-'
            })
            .trim_start_matches('#');

        if tag.len() >= 2
            && tag.chars().all(|character| {
                character.is_alphanumeric() || character == '_' || character == '-'
            })
            && token.contains('#')
            && !tags.iter().any(|existing| existing == tag)
        {
            tags.push(tag.to_string());
        }
    }

    tags
}

fn extract_links(format: &str, content: &str) -> Vec<NoteLink> {
    match format {
        "html" => extract_html_links(content),
        _ => extract_markdown_links(content),
    }
}

fn extract_markdown_links(content: &str) -> Vec<NoteLink> {
    let mut links = Vec::new();
    let bytes = content.as_bytes();
    let mut index = 0;

    while index < bytes.len() {
        let Some(label_start) = content[index..].find('[').map(|offset| index + offset) else {
            break;
        };
        let Some(label_end) = content[label_start..]
            .find(']')
            .map(|offset| label_start + offset)
        else {
            break;
        };
        let target_start = label_end + 1;

        if !content[target_start..].starts_with('(') {
            index = label_end + 1;
            continue;
        }

        let Some(target_end) = content[target_start + 1..]
            .find(')')
            .map(|offset| target_start + 1 + offset)
        else {
            break;
        };

        let anchor_text = content[label_start + 1..label_end].trim();
        let target = content[target_start + 1..target_end].trim();
        if !target.is_empty() {
            links.push(NoteLink {
                target: target.to_string(),
                link_type: classify_link(target),
                anchor_text: anchor_text.to_string(),
            });
        }
        index = target_end + 1;
    }

    links
}

fn extract_html_links(content: &str) -> Vec<NoteLink> {
    let mut links = Vec::new();
    let mut index = 0;
    let lower = content.to_lowercase();

    while let Some(anchor_start_offset) = lower[index..].find("<a") {
        let anchor_start = index + anchor_start_offset;
        let Some(tag_end) = lower[anchor_start..]
            .find('>')
            .map(|offset| anchor_start + offset)
        else {
            break;
        };
        let tag = &content[anchor_start..=tag_end];
        let Some(target) = extract_attribute(tag, "href") else {
            index = tag_end + 1;
            continue;
        };
        let close_start = lower[tag_end + 1..]
            .find("</a>")
            .map(|offset| tag_end + 1 + offset)
            .unwrap_or(tag_end + 1);
        let anchor_text = extract_html_text(&content[tag_end + 1..close_start])
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");

        links.push(NoteLink {
            link_type: classify_link(&target),
            target,
            anchor_text,
        });
        index = close_start.saturating_add(4);
    }

    links
}

fn extract_attribute(tag: &str, name: &str) -> Option<String> {
    let lower = tag.to_lowercase();
    let pattern = format!("{name}=");
    let start = lower.find(&pattern)? + pattern.len();
    let quote = tag[start..].chars().next()?;

    if quote != '"' && quote != '\'' {
        return None;
    }

    let value_start = start + quote.len_utf8();
    let value_end = tag[value_start..].find(quote)? + value_start;
    Some(decode_basic_entities(&tag[value_start..value_end]))
}

fn classify_link(target: &str) -> String {
    if target.starts_with("http://") || target.starts_with("https://") {
        "external".to_string()
    } else if target.starts_with('#') {
        "anchor".to_string()
    } else {
        "local".to_string()
    }
}

fn scan_import_manifest(root_path: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![root_path.to_path_buf()];

    while let Some(path) = stack.pop() {
        if path.is_dir() {
            let entries = fs::read_dir(&path).map_err(io_error)?;
            for entry in entries {
                let entry = entry.map_err(io_error)?;
                let entry_path = entry.path();
                if entry_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with('.'))
                {
                    continue;
                }
                stack.push(entry_path);
            }
        } else {
            files.push(path);
        }
    }

    files.sort();
    Ok(files)
}

fn parse_imported_note(_root_path: &Path, path: &Path) -> Result<ImportedNote> {
    let raw_content = fs::read_to_string(path).map_err(io_error)?;
    let (frontmatter, content) = parse_frontmatter(&raw_content);
    let title = frontmatter
        .iter()
        .find(|(key, _)| key == "title")
        .map(|(_, value)| value.to_string())
        .or_else(|| {
            path.file_stem()
                .and_then(|stem| stem.to_str())
                .map(|stem| stem.to_string())
        })
        .unwrap_or_else(|| "Untitled".to_string());
    let format = if path.extension().and_then(|ext| ext.to_str()) == Some("html") {
        "html"
    } else {
        "markdown"
    };
    let links = extract_links(format, &content);
    let content_hash = content_hash(&content);

    Ok(ImportedNote {
        id: Uuid::new_v4().to_string(),
        source_path: path.to_path_buf(),
        format: format.to_string(),
        title,
        content,
        content_hash,
        frontmatter,
        links: links
            .into_iter()
            .filter(|link| link.link_type == "local")
            .collect(),
    })
}

fn parse_frontmatter(content: &str) -> (Vec<(String, String)>, String) {
    if !content.starts_with("---\n") {
        return (Vec::new(), content.to_string());
    }

    let Some(end) = content[4..].find("\n---") else {
        return (Vec::new(), content.to_string());
    };
    let frontmatter_raw = &content[4..4 + end];
    let body_start = 4 + end + "\n---".len();
    let body = content[body_start..].trim_start_matches('\n').to_string();
    let frontmatter = frontmatter_raw
        .lines()
        .filter_map(|line| {
            let (key, value) = line.split_once(':')?;
            Some((
                key.trim().to_string(),
                value.trim().trim_matches('"').to_string(),
            ))
        })
        .collect();

    (frontmatter, body)
}

fn import_attachment(
    connection: &Connection,
    root_path: &Path,
    attachment_path: &Path,
    attachments_dir: &Path,
) -> Result<String> {
    let bytes = fs::read(attachment_path).map_err(io_error)?;
    let hash = format!("{:x}", Sha256::digest(&bytes));
    let relative_path = attachment_path
        .strip_prefix(root_path)
        .unwrap_or(attachment_path)
        .display()
        .to_string();
    let storage_path = attachments_dir.join(&hash[0..2]).join(&hash);
    fs::create_dir_all(storage_path.parent().unwrap_or(attachments_dir)).map_err(io_error)?;

    if !storage_path.exists() {
        fs::write(&storage_path, &bytes).map_err(io_error)?;
    }

    let existing_id = connection
        .query_row(
            "SELECT id FROM attachments WHERE content_hash = ?1",
            params![hash],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    if let Some(id) = existing_id {
        connection.execute(
            "UPDATE attachments SET ref_count = ref_count + 1 WHERE id = ?1",
            params![id],
        )?;
        return Ok(id);
    }

    let id = Uuid::new_v4().to_string();
    connection.execute(
        "INSERT INTO attachments(id, content_hash, byte_size, original_path, storage_path)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            id,
            hash,
            bytes.len() as i64,
            relative_path,
            storage_path.display().to_string()
        ],
    )?;
    Ok(id)
}

fn note_metadata_json(connection: &Connection, note_id: &str) -> Result<String> {
    connection.query_row(
        "SELECT metadata_json FROM notes WHERE id = ?1",
        params![note_id],
        |row| row.get(0),
    )
}

fn import_metadata_json(root_path: &Path, note: &ImportedNote) -> String {
    let relative_path = note
        .source_path
        .strip_prefix(root_path)
        .unwrap_or(&note.source_path)
        .display()
        .to_string();
    let frontmatter = note
        .frontmatter
        .iter()
        .map(|(key, value)| format!("\"{}\":\"{}\"", json_escape(key), json_escape(value)))
        .collect::<Vec<_>>()
        .join(",");

    format!(
        "{{\"importPath\":\"{}\",\"frontmatter\":{{{}}}}}",
        json_escape(&relative_path),
        frontmatter
    )
}

fn is_note_path(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|extension| extension.to_str()),
        Some("md" | "markdown" | "html")
    )
}

fn sanitize_file_name(value: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string();

    if sanitized.is_empty() {
        "untitled".to_string()
    } else {
        sanitized
    }
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn io_error(error: std::io::Error) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(error))
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
                 AND name IN (
                   'notes',
                   'note_bodies',
                   'note_revisions',
                   'note_events',
                   'note_links',
                   'index_jobs',
                   'index_state',
                   'attachments',
                   'note_attachments',
                   'import_runs',
                   'audit_log'
                 )",
                [],
                |row| row.get(0),
            )
            .expect("table count query succeeds");

        assert_eq!(count, 11);
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

        connection.execute_batch("BEGIN").expect("begin seed");
        for index in 0..10_000 {
            seed_note_metadata(
                &mut connection,
                &format!("note-{index}"),
                "markdown",
                "body",
            );
        }
        connection.execute_batch("COMMIT").expect("commit seed");

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
        let note = get_note(&connection, "note-9999")
            .expect("note lookup succeeds")
            .expect("note exists");
        assert_eq!(note.content, "body");
        assert!(
            body_started.elapsed().as_millis() <= 100,
            "body load exceeded 100ms budget: {:?}",
            body_started.elapsed()
        );
    }

    #[test]
    fn extracts_markdown_and_html_text_for_search() {
        let markdown = extract_search_text("markdown", "# Title\n\n[Link](target) **bold**");
        assert!(markdown.contains("Title"));
        assert!(markdown.contains("bold"));

        let html = extract_search_text(
            "html",
            "<h1>Report</h1><style>.x{}</style><script>bad()</script><p>Alpha&nbsp;Beta</p>",
        );
        assert!(html.contains("Report"));
        assert!(html.contains("Alpha Beta"));
        assert!(!html.contains("bad"));
    }

    #[test]
    fn search_uses_fts_and_updates_incrementally() {
        let mut connection = open_memory_database().expect("database opens");
        let note = create_note(
            &mut connection,
            CreateNoteInput {
                title: "Rate limiter explainer".to_string(),
                format: "markdown".to_string(),
                content: "Token bucket refill behavior #infra [Spec](note://spec)".to_string(),
            },
        )
        .expect("note created");
        assert_eq!(
            process_index_jobs(&connection, 10).expect("index processed"),
            1
        );

        let results = search_notes(
            &connection,
            SearchNotesQuery {
                query: "bucket".to_string(),
                limit: Some(10),
                format: None,
                tag: Some("infra".to_string()),
            },
        )
        .expect("search succeeds");
        assert_eq!(results[0].id, note.id);
        assert!(results[0].snippet.contains("<mark>bucket</mark>"));
        let link_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM note_links WHERE source_note_id = ?1",
                params![note.id],
                |row| row.get(0),
            )
            .expect("link count");
        assert_eq!(link_count, 1);

        update_note(
            &mut connection,
            UpdateNoteInput {
                id: note.id.clone(),
                content: "Leaky queue behavior".to_string(),
            },
        )
        .expect("note updated");
        assert_eq!(
            process_index_jobs(&connection, 10).expect("index processed"),
            1
        );

        let old_results = search_notes(
            &connection,
            SearchNotesQuery {
                query: "bucket".to_string(),
                limit: Some(10),
                format: None,
                tag: None,
            },
        )
        .expect("old search succeeds");
        assert!(old_results.is_empty());

        update_note(
            &mut connection,
            UpdateNoteInput {
                id: note.id.clone(),
                content: "Leaky queue behavior".to_string(),
            },
        )
        .expect("unchanged update returns note");
        assert_eq!(process_index_jobs(&connection, 10).expect("no reindex"), 0);

        let job_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM index_jobs WHERE status = 'indexed'",
                [],
                |row| row.get(0),
            )
            .expect("index job count");
        assert_eq!(job_count, 1);
    }

    #[test]
    fn search_stays_bounded_with_large_vault() {
        let mut connection = open_memory_database().expect("database opens");

        connection
            .execute_batch("BEGIN")
            .expect("begin search seed");
        for index in 0..10_000 {
            seed_indexed_note(
                &mut connection,
                &format!("search-note-{index}"),
                "markdown",
                &format!("common text phase3unique{index}"),
            );
        }
        connection
            .execute_batch("COMMIT")
            .expect("commit search seed");

        let started = std::time::Instant::now();
        let results = search_notes(
            &connection,
            SearchNotesQuery {
                query: "phase3unique9999".to_string(),
                limit: Some(20),
                format: Some("markdown".to_string()),
                tag: None,
            },
        )
        .expect("search succeeds");

        assert_eq!(results.len(), 1);
        assert!(
            started.elapsed().as_millis() <= 100,
            "search exceeded 100ms budget: {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn imports_obsidian_style_vault_and_deduplicates_attachments() {
        let mut connection = open_memory_database().expect("database opens");
        let temp_dir = tempfile::tempdir().expect("temp dir exists");
        let vault = temp_dir.path().join("vault");
        let assets = vault.join("assets");
        let attachment_store = temp_dir.path().join("attachments");
        fs::create_dir_all(&assets).expect("assets dir");
        fs::write(
            vault.join("daily.md"),
            "---\ntitle: Daily Note\n---\n# Daily\n\n![Image](assets/photo.png)\n#journal",
        )
        .expect("daily note");
        fs::write(
            vault.join("artifact.html"),
            "<h1>Artifact</h1><a href=\"assets/photo-copy.png\">Copy</a>",
        )
        .expect("html artifact");
        fs::write(assets.join("photo.png"), b"same image").expect("photo");
        fs::write(assets.join("photo-copy.png"), b"same image").expect("photo copy");

        let report = import_path(&mut connection, &vault, &attachment_store).expect("vault import");
        assert_eq!(report.imported_notes, 2);
        assert_eq!(report.imported_attachments, 2);

        let attachment_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM attachments", [], |row| row.get(0))
            .expect("attachment count");
        assert_eq!(attachment_count, 1);

        assert_eq!(
            process_index_jobs(&connection, 10).expect("index import"),
            2
        );
        let results = search_notes(
            &connection,
            SearchNotesQuery {
                query: "Daily".to_string(),
                limit: Some(10),
                format: Some("markdown".to_string()),
                tag: Some("journal".to_string()),
            },
        )
        .expect("search imported note");
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn exports_note_source_and_portable_bundle() {
        let mut connection = open_memory_database().expect("database opens");
        let note = create_note(
            &mut connection,
            CreateNoteInput {
                title: "Export Me".to_string(),
                format: "markdown".to_string(),
                content: "# Export Me".to_string(),
            },
        )
        .expect("note created");
        let temp_dir = tempfile::tempdir().expect("temp dir exists");

        let source_report = export_note(
            &connection,
            ExportNoteInput {
                id: note.id.clone(),
                path: temp_dir.path().display().to_string(),
                bundle: false,
            },
        )
        .expect("source export");
        assert_eq!(source_report.files_written, 1);
        assert!(PathBuf::from(source_report.output_path).exists());

        let bundle_report = export_note(
            &connection,
            ExportNoteInput {
                id: note.id,
                path: temp_dir.path().display().to_string(),
                bundle: true,
            },
        )
        .expect("bundle export");
        let bundle_path = PathBuf::from(bundle_report.output_path);
        assert!(bundle_path.join("Export_Me.md").exists());
        assert!(bundle_path.join("metadata.json").exists());
        assert!(bundle_path.join("manifest.json").exists());
    }

    #[test]
    fn integrity_backup_and_index_repair_work() {
        let temp_dir = tempfile::tempdir().expect("temp dir exists");
        let database_path = temp_dir.path().join("o-note.db");
        let backup_path = temp_dir.path().join("backup.db");
        let mut connection = open_database(&database_path).expect("database opens");
        let note = create_note(
            &mut connection,
            CreateNoteInput {
                title: "Repair target".to_string(),
                format: "markdown".to_string(),
                content: "searchable repair content".to_string(),
            },
        )
        .expect("note created");
        assert_eq!(
            process_index_jobs(&connection, 10).expect("index processed"),
            1
        );
        connection
            .execute("DELETE FROM note_fts WHERE note_id = ?1", params![note.id])
            .expect("delete fts row");

        let repair = repair_search_index(&connection).expect("index repair");
        assert_eq!(repair.status, "ok");
        let results = search_notes(
            &connection,
            SearchNotesQuery {
                query: "repair".to_string(),
                limit: Some(10),
                format: None,
                tag: None,
            },
        )
        .expect("search after repair");
        assert_eq!(results.len(), 1);

        let integrity = database_integrity(&connection).expect("integrity check");
        assert_eq!(integrity.status, "ok");
        let backup = backup_database(&connection, &backup_path).expect("backup database");
        assert_eq!(backup.status, "ok");
        assert!(backup_path.exists());

        let audit_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM audit_log", [], |row| row.get(0))
            .expect("audit count");
        assert!(audit_count >= 2);
    }

    fn seed_note_metadata(connection: &mut Connection, id: &str, format: &str, content: &str) {
        connection
            .execute(
                "INSERT INTO notes(id, title, format) VALUES (?1, ?2, ?3)",
                params![id, id, format],
            )
            .expect("seed note metadata");
        connection
            .execute(
                "INSERT INTO note_bodies(note_id, content, content_hash, byte_size)
                 VALUES (?1, ?2, ?3, ?4)",
                params![id, content, content_hash(content), content.len() as i64],
            )
            .expect("seed note body");
    }

    fn seed_indexed_note(connection: &mut Connection, id: &str, format: &str, content: &str) {
        seed_note_metadata(connection, id, format, content);
        connection
            .execute(
                "INSERT INTO note_fts(note_id, title, plain_text, tags, format)
                 VALUES (?1, ?2, ?3, '', ?4)",
                params![id, id, extract_search_text(format, content), format],
            )
            .expect("seed search index");
    }
}
