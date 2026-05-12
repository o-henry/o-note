# Phase 2: Rendering Design

## Goal

Render Markdown and HTML artifacts safely and quickly while keeping editing responsive.

## Scope

- Markdown editor and preview.
- HTML source editor and sandboxed preview.
- Preview/source/split modes.
- Static fallback for HTML artifacts.
- Copy/export source.
- Render cache by content hash.

## Markdown Rendering

- Use a maintained Markdown parser.
- Sanitize rendered HTML.
- Debounce preview updates while typing.
- Cache rendered output by content hash.
- Avoid reparsing if source did not change.

## HTML Rendering

- Render HTML in a sandboxed iframe.
- Default to no network access.
- Use strict CSP.
- Disable privileged app access.
- Use a typed `postMessage` bridge only for allowlisted export/copy events.
- Remount iframe only when content hash changes.

## Artifact Bridge

Allowed commands:

- `copy_text`
- `copy_markdown`
- `copy_json`
- `copy_diff`
- `export_html`
- `request_local_asset` only after explicit policy exists

All bridge messages require schema validation, note id, artifact id, and a command version.

## Performance Requirements

- Normal Markdown preview update: <= 100 ms after debounce.
- Normal HTML preview mount: <= 250 ms.
- Editing source must not remount iframe until preview commit.
- Large HTML artifacts should show preview loading state instead of blocking.
- Only the active artifact preview is mounted.

## Security Tests

- Markdown script injection is removed.
- HTML cannot call Tauri commands.
- HTML cannot read local files.
- Disallowed bridge message is ignored and logged as a safe audit event.
- Remote resource loading follows the configured policy.

## Done Criteria

- Markdown and HTML render paths work.
- HTML artifacts are sandboxed by default.
- Preview does not stall typing.
- Security boundary tests pass.
