# Technical Stack

## Decision

Build o-note as a desktop-first, local-first app with a web UI architecture:

- Shell: Tauri 2
- Core backend: Rust
- UI: React, TypeScript, Vite
- Editor: CodeMirror 6
- Database: SQLite
- Search: SQLite FTS5 first, with Tantivy reserved for later advanced indexing
- Markdown rendering: markdown-it or unified/remark on the UI side, with sanitization
- HTML rendering: sandboxed iframe-based preview
- Artifact interaction: postMessage bridge with explicit allowlisted commands
- Tests: Vitest, Testing Library, Playwright, Rust unit tests
- Package management: pnpm
- CI: GitHub Actions

## Why This Stack

### Tauri 2 + Rust

Tauri gives a native-feeling desktop app without shipping a full Electron runtime. Rust is a good fit for filesystem watching, indexing, attachment handling, SQLite integration, and future performance-sensitive search work.

### React + TypeScript + Vite

React and TypeScript keep the UI iteration speed high while preserving type safety. Vite keeps local development fast.

### CodeMirror 6

CodeMirror is a strong fit for editing Markdown and raw HTML. It supports syntax highlighting, extensions, keymaps, decorations, and custom panels without forcing a rich-text model too early.

### SQLite + FTS5

SQLite avoids the many-small-files indexing problem by making the primary working set database-backed. FTS5 gives fast local full-text search with low operational complexity.

Tantivy should be considered later if ranking, snippets, field weighting, or large-vault performance exceed what SQLite FTS5 can comfortably handle.

## Storage Model

Primary data should live in SQLite:

- `notes`: identity, title, format, timestamps, metadata.
- `note_revisions`: content snapshots or deltas.
- `search_index`: FTS-backed searchable text.
- `attachments`: content-addressed local files.
- `links`: parsed wikilinks, Markdown links, and HTML anchors.
- `tasks`: optional extracted tasks.

Large binary attachments should live in a content-addressed attachment directory. SQLite stores metadata and hashes, not large blobs by default.

## Rendering Model

Markdown:

- Parse Markdown.
- Sanitize generated HTML.
- Render in the normal app document surface.

HTML:

- Store source HTML as note content.
- Render in an iframe with `sandbox`.
- Apply a strict Content Security Policy.
- Default to no network access.
- Provide an explicit trusted mode later, if needed.

Interactive artifacts:

- Use a narrow `postMessage` bridge for copy/export events.
- Allow artifact-to-app messages only for explicit, typed commands such as `copy_text`, `export_json`, or `request_asset`.
- Never let artifact JavaScript call privileged Tauri commands directly.
- Preserve a static fallback view when JavaScript is disabled.

## Performance Model

- App startup should load shell UI and recent note metadata first.
- Note bodies should be lazy-loaded.
- Indexing should run incrementally in the background.
- File imports should be chunked and cancellable.
- Search should query FTS indexes, not scan documents.
- UI lists should use virtualization when item counts grow.
- Artifact previews should mount lazily and avoid re-rendering the iframe unless the source changed.

## Security Model

- Treat generated HTML as untrusted.
- Sanitize Markdown output.
- Sandbox HTML previews.
- Block local file access from rendered artifacts.
- Avoid remote resource loading by default.
- Validate every artifact-to-app message against a strict schema.
- Keep secrets out of note content, logs, and crash reports.
- Require tests for any code that changes rendering trust boundaries.

## Deferred Decisions

- Sync provider: not needed for the first local-first version.
- CRDT support: defer until collaboration or multi-device live sync exists.
- Plugin API: defer until the core app shape is stable.
- Mobile: possible later, not part of the first implementation plan.
