# Phase 4: Import And Export Design

## Goal

Move existing Markdown vaults and generated HTML artifacts into and out of o-note without losing ownership, portability, or responsiveness.

## Scope

- Import Markdown folders and Obsidian-style vaults.
- Import standalone HTML artifacts.
- Preserve useful folder, tag, frontmatter, and link metadata.
- Content-addressed attachment storage.
- Export `.md`, `.html`, and portable bundles.
- Duplicate attachment detection.

## Import Pipeline

1. Scan file tree in Rust background task.
2. Build manifest with paths, sizes, mtimes, hashes.
3. Parse frontmatter and links in bounded batches.
4. Insert metadata first.
5. Insert bodies after metadata.
6. Schedule indexing jobs.
7. Report progress at a throttled cadence.

## Export Pipeline

- Markdown note: `.md` plus attachment folder when needed.
- HTML artifact: standalone `.html` when possible.
- Bundle: note source, metadata JSON, attachments, and manifest.
- Preserve original import path metadata without requiring it for operation.

## Obsidian Compatibility

Support first:

- Markdown files.
- YAML frontmatter.
- Wikilinks.
- Markdown links.
- Attachments referenced by relative path.

Defer:

- Plugin-specific block syntax.
- Canvas files.
- Dataview execution.
- Full theme/plugin reproduction.

## Performance Requirements

- Import must be cancellable.
- Import must not freeze UI.
- Large vault scan must stream progress.
- Attachment hashing must run off the UI thread.
- Export should stream large bundles instead of building entire archives in memory.

## Verification

- Fixture vault import test.
- Duplicate attachment test.
- Round-trip export/import smoke test.
- Cancel/resume import test.
- Import benchmark for 10,000 Markdown files.

## Done Criteria

- User can import an Obsidian-style Markdown folder.
- User can import standalone HTML artifacts.
- User can export notes and bundles.
- Import and export operations remain responsive.
