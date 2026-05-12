# AGENTS.md

## Completion Integrity & Termination Policy

Treat completion state as a hard contract in this project.

1. Do not claim completion unless every explicit acceptance criterion is satisfied and verified.
2. Once an item is resolved, closed, or verified complete, do not list it again as remaining work unless fresh evidence proves a regression. Label that case `regression`, not `remaining`.
3. Separate follow-up state into these buckets: `verified_open`, `validation_only`, `regression`, `out_of_scope`, and `resolved`.
4. If the requested scope is complete, stop. Do not invent extra improvement work, continue a new task chain, or repeat "now do X next" unless the user asked for recommendations.
5. "Next steps" are allowed only when verification failed, the task is blocked, the user requested options, or the step is explicitly optional and labeled optional.
6. Completion responses must state `done` or `not done`, the verification evidence, and any residual risk. Then stop.
7. Do not recycle a previous improvement list after completing it. Re-open an item only with a new run, new logs, failing tests, or a concrete diff that contradicts the resolved state.

## Project Goal

o-note is a fast, local-first note app for Markdown and rich HTML artifacts.

The app exists because large Markdown-file vaults can become slow to index and navigate. o-note should keep notes portable while using a database-first architecture, incremental indexing, and sandboxed HTML rendering.

## Technical Direction

- Desktop shell: Tauri 2.
- Backend: Rust.
- Frontend: React, TypeScript, Vite.
- Editor: CodeMirror 6.
- Database: SQLite.
- Search: SQLite FTS5 first; evaluate Tantivy only when measurements justify it.
- Rendering: sanitized Markdown and sandboxed HTML iframe previews.
- Tests: Rust tests, Vitest, Testing Library, Playwright.

## Engineering Rules

- Prefer boring durable data models.
- Keep hot paths fast: startup, note open, search, render, index.
- Treat generated HTML as untrusted.
- Keep feature work small and verifiable.
- Use SOLID and DRY pragmatically.
- Avoid broad refactors during feature work unless required for correctness.

## Size Guidelines

- Function: 40 lines or fewer.
- React component: 220 lines or fewer.
- Rust module: 400 lines or fewer.
- TypeScript module: 350 lines or fewer.

Exceeding a guideline is allowed when the code is clearer that way, but large files should trigger refactoring pressure.

## Required Verification

- Run focused tests for changed behavior.
- For UI changes, use browser or Playwright verification when possible.
- For rendering/security changes, include tests around sanitizer, CSP, or sandbox behavior.
- For performance claims, provide a measured command or benchmark.

## Git Diff Display

When inspecting or showing diffs in Codex CLI, prefer delta explicitly because Git pagers are not reliably used when stdout is not a TTY.

Use `git d` for unstaged diffs and `git dc` for staged diffs.

Equivalent commands:

```sh
git diff --color=always ... | delta --syntax-theme GitHub --light --line-numbers
git diff --cached --color=always ... | delta --syntax-theme GitHub --light --line-numbers
```
