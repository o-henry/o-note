# o-note

o-note is a local-first note app for Markdown notes and rich, self-contained HTML artifacts.

The project starts from a practical problem with large Obsidian vaults: Markdown files are portable and pleasant, but file-count-heavy vaults can make indexing and UI responsiveness feel slow. o-note keeps the portability spirit while using a database-first, incremental-indexing architecture built for fast search, fast navigation, and safe rendering of richer `.html` notes.

## Product Direction

- Markdown remains a first-class note format.
- HTML artifacts are first-class notes, not attachments.
- Notes should open instantly, search should feel live, and indexing should never block writing.
- HTML rendering must be sandboxed and safe by default.
- Storage should be local-first, exportable, and resilient to future sync strategies.

## Current Status

Planning phase. No application code has been implemented yet.

See:

- [Product Brief](docs/000-product-brief.md)
- [Technical Stack](docs/001-tech-stack.md)
- [Development Rules](docs/002-development-rules.md)
- [Feature Roadmap](docs/003-feature-roadmap.md)
- [HTML Artifact Requirements](docs/004-html-artifact-requirements.md)
- [UI/UX Direction](docs/005-ui-ux-direction.md)
- [Performance Contract](docs/006-performance-contract.md)

Phase designs:

- [Phase 0: Foundation](docs/phases/phase-00-foundation.md)
- [Phase 1: Core Notes](docs/phases/phase-01-core-notes.md)
- [Phase 2: Rendering](docs/phases/phase-02-rendering.md)
- [Phase 3: Search And Indexing](docs/phases/phase-03-search-indexing.md)
- [Phase 4: Import And Export](docs/phases/phase-04-import-export.md)
- [Phase 5: HTML Artifact Workflow](docs/phases/phase-05-html-artifact-workflow.md)
- [Phase 6: Reliability And Scale](docs/phases/phase-06-reliability-scale.md)
