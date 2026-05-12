# Performance Contract

Performance is a product requirement for o-note. A feature is not complete if it works functionally but makes note opening, search, typing, scrolling, indexing, or HTML preview feel slow.

## Hard Rules

- The UI thread must never perform vault-wide scans, full-text indexing, import parsing, or attachment hashing.
- React state must not hold all note bodies.
- Search must query an index, not scan note content.
- Note lists must virtualize when the visible set can exceed 500 rows.
- HTML previews must mount lazily and remount only when source content changes.
- Indexing must be incremental, cancellable, and resumable.
- Markdown and HTML render output should be cached by content hash when useful.
- SQLite must use WAL mode for normal app operation.

## Initial Budgets

Budgets are measured on the user's development machine unless otherwise noted.

| Area | Budget |
| --- | ---: |
| Cold launch to interactive shell | <= 1.5 s |
| Warm launch to interactive shell | <= 700 ms |
| Open recent note metadata | <= 50 ms |
| Open note body after metadata load | <= 100 ms |
| Keystroke-to-editor paint | <= 16 ms target, <= 32 ms max |
| Search response after index warm | <= 100 ms at 10,000 notes |
| Switch Markdown preview for current note | <= 100 ms for normal notes |
| Mount current HTML preview | <= 250 ms for normal artifacts |
| Import progress update cadence | >= 4 updates/s while active |
| UI responsiveness during indexing | no visible typing or scroll stalls |

## Scale Targets

Phase benchmarks should use synthetic vaults:

- Small: 100 notes, 10 HTML artifacts.
- Medium: 10,000 notes, 1,000 HTML artifacts.
- Large: 100,000 notes, 10,000 HTML artifacts.

The first production-quality target is Medium. Large is a design pressure and should not block the first usable build unless the Medium path is already compromised.

## Measurement Requirements

Every phase that touches performance-sensitive paths must include:

- A repeatable benchmark or profiling command.
- A written baseline result.
- A threshold that fails CI or local verification when practical.
- A note explaining any budget miss.

## Likely Bottlenecks

- Rendering too many note rows in React.
- Loading note bodies for every list row.
- Recreating iframe previews while typing.
- Running Markdown parsing on every keypress without debounce or worker boundaries.
- Scanning the filesystem instead of reading SQLite metadata.
- Re-indexing unchanged notes.
- Storing large attachments directly in hot SQLite rows.
- Emitting too many progress events from Rust to the webview.

## Escalation Path

Start with SQLite FTS5 and simple caches. Add complexity only after measurements show need:

1. Tune schema, indexes, query plans, and WAL settings.
2. Add content-hash render caches.
3. Move heavier parse/render work into workers or Rust tasks.
4. Add Tantivy only if SQLite FTS5 cannot satisfy search quality or latency.
5. Add DOM-aware or rendered visual diffing only after HTML artifact versioning exists.
