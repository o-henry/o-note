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
    let user_version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    if user_version < 2 {
        connection.execute_batch(SEARCH_SCHEMA)?;
        rebuild_search_index(connection)?;
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

struct NoteLink {
    target: String,
    link_type: String,
    anchor_text: String,
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
                   'index_state'
                 )",
                [],
                |row| row.get(0),
            )
            .expect("table count query succeeds");

        assert_eq!(count, 7);
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
