# Phase 1: Core Notes Design

## Goal

Implement fast local notes: create, rename, update, delete, list, and open Markdown or HTML notes without loading the whole vault into the UI.

## Scope

- `markdown` and `html` note formats.
- SQLite-backed note metadata and content.
- Recent notes.
- Keyboard-first navigation.
- Note list virtualization.
- Autosave with crash-safe writes.

## Data Model

Core tables:

- `notes`: id, title, format, created_at, updated_at, deleted_at, pinned, metadata_json.
- `note_bodies`: note_id, content, content_hash, byte_size, updated_at.
- `note_revisions`: id, note_id, content_hash, created_at, summary.
- `note_events`: id, note_id, event_type, payload_json, created_at.

Keep metadata queries separate from body loading. List views should read `notes`, not `note_bodies`.

## Rust Boundary

Commands:

- `create_note(input)`
- `list_notes(query)`
- `get_note(id)`
- `update_note(input)`
- `rename_note(id, title)`
- `delete_note(id)`

Every command returns typed results and bounded payloads.

## UI Design

- Sidebar: note spaces, recent notes, tags later.
- Main inset: current note editor/viewer.
- Right rail: note metadata, backlinks placeholder, render/index status.
- Command palette: create note, switch note, search placeholder.

## Performance Requirements

- List first 100 note metadata rows: <= 50 ms.
- Open selected note body: <= 100 ms after metadata exists.
- Autosave debounce: 300-800 ms, never on every keystroke.
- Editor typing must not wait for SQLite writes.
- Render only visible list rows.

## Verification

- Unit tests for note CRUD.
- Rust tests for migration and repository behavior.
- UI tests for create, edit, rename, delete.
- Benchmark opening notes in a 10,000-note fixture.

## Done Criteria

- Markdown and HTML notes can be created and edited.
- The note list stays responsive with 10,000 metadata rows.
- Note bodies are lazy-loaded.
- Autosave survives app restart.
