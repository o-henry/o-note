# Phase 3: Search And Indexing Design

## Goal

Make search instant-feeling at large note counts by using incremental SQLite FTS5 indexing and never scanning note bodies during interactive search.

## Scope

- SQLite FTS5 tables.
- Background indexer.
- Incremental re-index by content hash.
- Search snippets.
- Tag and metadata filters.
- Link extraction for Markdown and HTML.
- Index health status.

## Index Model

Indexes:

- `note_fts`: title, plain_text, tags, format.
- `note_links`: source_note_id, target, link_type, anchor_text.
- `index_jobs`: note_id, content_hash, status, attempts, updated_at.
- `index_state`: key, value for checkpoints and schema version.

HTML text extraction should strip scripts/styles and index readable text plus selected metadata.

## Background Worker

- Runs in Rust.
- Processes bounded batches.
- Yields between batches.
- Emits throttled progress events.
- Cancels cleanly on app shutdown.
- Resumes from `index_jobs`.

## Search UI

- Command-palette style search.
- Results show title, snippet, format, updated time.
- Keyboard navigation is required.
- Filters are compact and rectangular.
- No full-page modal that hides context.

## Performance Requirements

- Query response: <= 100 ms at 10,000 notes after warm index.
- Index unchanged note: skipped by content hash.
- Progress events: throttled to avoid UI event floods.
- Import/index pipeline should not block typing or note open.

## Verification

- Unit tests for Markdown text extraction.
- Unit tests for HTML text extraction.
- Rust tests for FTS queries and ranking basics.
- Benchmark search over 10,000-note fixture.
- Test that unchanged content is not re-indexed.

## Done Criteria

- Search is index-backed.
- Snippets work.
- Background indexing is resumable.
- UI remains responsive during indexing.
