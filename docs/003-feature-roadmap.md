# Feature Roadmap

## Phase 0: Foundation

- Confirm product brief and technical stack.
- Create app scaffold.
- Establish lint, format, test, and CI.
- Create SQLite schema and migration system.
- Create basic shell layout.

## Phase 1: Core Notes

- Create, rename, delete, and update notes.
- Support note formats: `markdown`, `html`.
- Store notes in SQLite.
- Show recent notes and note list.
- Add keyboard-first navigation.

## Phase 2: Rendering

- Markdown editor and preview.
- HTML source editor and sandboxed preview.
- Split editor/preview mode.
- Full preview mode.
- Copy/export source.

## Phase 3: Search And Indexing

- SQLite FTS5 indexing.
- Incremental background indexer.
- Search result snippets.
- Tag and metadata filtering.
- Link extraction for Markdown and HTML anchors.

## Phase 4: Import And Export

- Import Markdown folder or Obsidian vault.
- Preserve folder-derived metadata where useful.
- Export notes to `.md`, `.html`, and portable bundles.
- Detect duplicate attachments by content hash.

## Phase 5: HTML Artifact Workflow

- Create HTML artifact from template.
- Store artifact metadata.
- Preview generated diagrams and interactive sections.
- Provide safe artifact templates for reports, plans, reviews, dashboards, and explainers.

## Phase 6: Reliability And Scale

- Performance benchmark with large synthetic vaults.
- Index repair flow.
- Database backup and restore.
- Crash-safe writes.
- Optional advanced search engine evaluation.

## Open Product Questions

- Should Markdown and HTML be separate note types, or should one note support both source formats at once?
- Should the first app expose raw HTML editing only, or also a structured artifact builder?
- How much Obsidian compatibility is required for the first import?
- Should links use Obsidian-style wikilinks, standard Markdown links, or both?
