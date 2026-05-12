# Phase 5: HTML Artifact Workflow Design

## Goal

Make o-note excellent for the workflow described in "The Unreasonable Effectiveness of HTML": agent-generated specs, plans, code reviews, reports, prototypes, and custom editors that the user can read, interact with, and feed back into Codex.

## Scope

- Artifact templates.
- Artifact categories.
- Side-by-side comparison grids.
- Copy/export controls.
- Artifact metadata.
- Annotation and review status.
- Safe interaction bridge.

## Artifact Categories

- Planning.
- Exploration.
- Implementation plan.
- Code review.
- PR explainer.
- Design prototype.
- Research report.
- Incident report.
- Custom editor.

## Artifact Metadata

Fields:

- category.
- generated_by.
- source_context.
- related_notes.
- related_files.
- artifact_version.
- safety_policy.
- export_capabilities.

## UI Model

- Left sidebar: note/artifact navigation.
- Main report area: rendered artifact.
- Top tabs: preview, source, exports, annotations.
- Right rail: sources, related notes, generated context, safety status.
- Comparison mode: grid of artifacts with tradeoff labels.

## Two-Way Interaction

Artifacts can ask the app to:

- Copy prompt text.
- Copy Markdown.
- Copy JSON.
- Copy diff.
- Export standalone HTML.
- Save an annotation.

Artifacts cannot:

- Read arbitrary files.
- Run shell commands.
- Call Tauri commands directly.
- Access secrets.
- Load remote resources without policy support.

## Performance Requirements

- Only active artifact iframe mounts by default.
- Comparison grids use static previews or thumbnails unless opened.
- Export actions must be async.
- Large artifacts must not block note navigation.
- Annotation state must update without remounting the artifact.

## Verification

- Playwright tests for preview/source/export tabs.
- Security tests for bridge allowlist.
- Performance check for switching among multiple artifacts.
- Static forbidden-pattern UI check for Bullpen direction.

## Done Criteria

- User can create, view, compare, annotate, and export HTML artifacts.
- Copy/export bridge is safe and tested.
- Artifact-heavy workspaces stay responsive.
