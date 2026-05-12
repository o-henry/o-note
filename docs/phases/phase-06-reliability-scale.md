# Phase 6: Reliability And Scale Design

## Goal

Harden o-note so it remains fast and trustworthy as the note database grows, artifacts become richer, and the user depends on it daily.

## Scope

- Large synthetic vault benchmarks.
- Database backup and restore.
- Index repair.
- Crash-safe writes.
- Render cache management.
- Performance regression gates.
- Optional advanced search evaluation.

## Reliability Features

- Automatic database integrity check.
- Manual backup and restore.
- Safe shutdown for background jobs.
- Index rebuild command.
- Render cache cleanup.
- Revision pruning policy.
- Local audit log for import, export, bridge, and index events.

## Scale Benchmarks

Required benchmark suites:

- Startup with 10,000 and 100,000 notes.
- Search latency with 10,000 and 100,000 notes.
- Indexing throughput for Markdown and HTML.
- Note switching latency.
- HTML preview mount latency.
- Import throughput.
- Memory usage during import and artifact preview.

## Performance Gates

Phase 6 should make performance harder to regress:

- Store benchmark baselines.
- Fail or warn when key paths regress beyond threshold.
- Track bundle size.
- Track SQLite query plans for hot queries.
- Track webview memory during HTML preview stress tests.

## Search Engine Decision Point

Evaluate Tantivy only if SQLite FTS5 misses measured requirements:

- Search latency too high after query/schema tuning.
- Ranking quality too weak for real use.
- Snippet quality insufficient.
- Field weighting becomes too limited.

Do not add Tantivy just because it sounds faster.

## Data Safety

- Backups must be portable.
- Exports must not require o-note to be readable.
- Crashes during autosave or import must not corrupt existing notes.
- Render cache and index data must be rebuildable.

## Verification

- Database corruption simulation where practical.
- Backup/restore integration test.
- Index rebuild test.
- Benchmark report checked into release artifacts.
- Long-running import/index stress test.

## Done Criteria

- Medium scale target is comfortably met.
- Large scale behavior is understood and documented.
- Backup, restore, and index repair work.
- Performance regressions have visible gates.
