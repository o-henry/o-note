# Development Rules

These rules define how o-note should be built before feature work begins.

## Engineering Principles

- Prefer simple, local-first architecture over premature distributed design.
- Keep the core data model boring and durable.
- Optimize the hot paths: startup, note switch, search, render, index.
- Make unsafe rendering impossible by default.
- Use SOLID as a design pressure, not ceremony.
- Use DRY when duplication expresses the same concept; allow local duplication when it keeps code readable.

## Code Size Guidelines

These are defaults, not hard physics. Exceed them only with a short comment or refactor plan.

- Function: 40 lines or fewer.
- React component: 220 lines or fewer.
- Rust module: 400 lines or fewer.
- TypeScript module: 350 lines or fewer.
- Test file: may exceed limits when it improves clarity.

## Style

- TypeScript strict mode is required.
- Rust warnings should be treated seriously; avoid `unwrap()` in production paths.
- Prefer explicit domain types over loose strings.
- Keep UI state local until it must be shared.
- Keep database access behind small repository/service APIs.
- Avoid broad utility modules that become junk drawers.

## Testing

Minimum expectations:

- Unit tests for parsers, storage, indexing, and sanitization.
- Component tests for editor and viewer behavior.
- Playwright tests for critical user flows.
- Regression tests for every fixed bug that can reasonably be reproduced.

Critical flows:

- Create Markdown note.
- Create HTML note.
- Render Markdown.
- Render sandboxed HTML.
- Search notes.
- Import many notes without blocking the UI.

## Security

- All HTML artifacts are untrusted unless explicitly marked otherwise.
- Markdown rendering must be sanitized.
- HTML preview must use iframe sandboxing.
- Any relaxation of CSP or sandbox rules needs a test and a written reason.
- Do not log note body content by default.
- Do not send note content to external services without explicit user action.

## Performance Budgets

Initial budgets for the first usable build:

- Cold launch to interactive shell: under 1.5 seconds on the user's machine.
- Open recent note: under 100 ms after metadata load.
- Search query response: under 100 ms for 10,000 notes after indexing.
- UI remains responsive during import and indexing.

Budgets can be revised only with measurement.

See [Performance Contract](006-performance-contract.md) for hard rules, scale targets, and benchmark requirements.

## Codex Workflow

- Read local context before changing code.
- Use relevant skills, plugins, and MCP tools when they clearly fit the task.
- Keep changes scoped to the requested feature or bug.
- Before finalizing, run the narrowest meaningful verification.
- Completion responses must say `done` or `not done`, include verification evidence, and name residual risk.
- Do not list resolved work as remaining unless fresh evidence shows a regression.
