# Phase 0: Foundation Design

## Goal

Create a project foundation that prevents slow architecture from entering the codebase. This phase sets the app scaffold, style contract, database migration path, CI, and performance harness before product features begin.

## Scope

- Tauri 2 + Rust + React + TypeScript + Vite scaffold.
- Strict TypeScript and Rust linting.
- SQLite migration system.
- Basic Bullpen-inspired shell skeleton.
- Performance benchmark harness.
- Browser verification path.
- CI for format, lint, unit tests, and build.

## Architecture

- `src-tauri/`: Rust commands, SQLite access, background tasks, migrations.
- `src/`: React UI, routing, shell, editor/viewer placeholders.
- `src/shared/`: TypeScript domain types mirrored from Rust where needed.
- `tests/`: Playwright flows and fixture vaults.
- `bench/`: synthetic vault generator and startup/search/open benchmarks.

## Performance Requirements

- Empty app cold launch to shell: <= 1.5 s.
- Shell JavaScript bundle must stay small enough to load before note features.
- No database work may run on the UI thread.
- Benchmark fixtures must be generated before Phase 1 feature work.

## UI Requirements

- Bullpen-style sidebar shell first.
- Zero-radius tokens.
- Flat bordered surfaces.
- Mono uppercase metadata labels.
- No generic hero page, fake logo, gradient, or rounded SaaS cards.

## Implementation Order

1. Scaffold Tauri, Vite, React, TypeScript.
2. Add lint, format, unit test, and build commands.
3. Add SQLite migration runner.
4. Add app shell route with sidebar/content inset placeholders.
5. Add benchmark fixture generator.
6. Add CI.

## Verification

- `npm run build`
- `npm test`
- `cargo test`
- `cargo check`
- Playwright smoke test for shell render.
- Startup benchmark with recorded result.

## Done Criteria

- App opens to the shell.
- CI passes.
- Migrations can run on a fresh local database.
- Performance harness exists and records launch baseline.
- UI shell follows the documented Bullpen direction.
