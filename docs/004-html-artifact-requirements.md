# HTML Artifact Requirements

This document captures the primary source direction from "Using Claude Code: The Unreasonable Effectiveness of HTML" and translates it into o-note requirements.

## Core Thesis

Markdown is portable and easy to edit, but it becomes restrictive when agents produce large specs, plans, reports, diagrams, prototypes, and review artifacts. HTML is more expressive and often more readable because it can combine structure, visual hierarchy, SVG, CSS, tables, images, interactions, and export controls in one self-contained document.

o-note should treat this as a first-class workflow:

- Capture agent-generated HTML artifacts.
- Render them safely.
- Make them searchable.
- Let users compare, annotate, and revise them.
- Let interactive artifacts export results back into prompts or structured files.

## Required Artifact Capabilities

### Information Density

HTML notes should support:

- Tables for structured data.
- CSS for visual hierarchy and design data.
- SVG for diagrams, flowcharts, and illustrations.
- Images for visual references.
- Code snippets with syntax-aware styling.
- Absolute or canvas-based layouts for spatial information.
- Workflow diagrams that combine HTML and SVG.

### Visual Clarity

The reader should be able to navigate artifacts through:

- Sections and persistent headings.
- Tabs.
- Jump links.
- Responsive layouts.
- Comparison grids.
- Collapsible details.
- Severity colors and labels for reviews.

### Sharing And Portability

The app should support:

- Export as a standalone `.html` file.
- Export as a portable bundle when attachments are needed.
- Copy link/path for local sharing.
- Future upload/share integrations without making the core app cloud-dependent.

### Two-Way Interaction

Interactive artifacts should support:

- Sliders and knobs for tuning values.
- Drag-and-drop bucketing and prioritization.
- Form-based editing for structured config.
- Live previews for prompts, templates, and copy.
- Copy/export buttons for Markdown, JSON, diffs, prompt text, and selected rows.

All interaction with the app must go through a narrow, validated bridge. Artifacts must not receive direct privileged access.

## First-Class Use Cases

### Specs, Planning, And Exploration

The app should support webs of related HTML artifacts, not just one linear plan. A user may generate several exploration artifacts, compare directions, expand one into mockups, and then produce an implementation plan.

### Code Review And Understanding

The app should make it natural to store:

- PR writeups.
- Annotated diffs.
- Module maps.
- Flowcharts.
- Severity-coded findings.
- Review guides for collaborators.

### Design And Prototypes

The app should preserve artifacts that demonstrate:

- Design system references.
- Component variants.
- Motion prototypes.
- Parameter-tuning playgrounds.
- Clickable flows.

### Reports, Research, And Learning

The app should be good at long-form artifacts synthesized from code, git history, web research, MCP context, Slack, Linear, and other sources.

Examples:

- Feature explainers.
- Concept explainers.
- Weekly status reports.
- Incident reports.
- Leadership summaries.
- Technical diagrams and slide decks.

### Custom Editing Interfaces

The app should support one-off editors that exist only to help the user manipulate a specific dataset and export the result.

Examples:

- Ticket prioritization board.
- Feature flag editor.
- Prompt tuning workspace.
- Dataset curation table.
- Transcript or diff annotation tool.
- Color, easing, crop, cron, or regex picker.

## Product Constraints

- HTML generation can be slower than Markdown; o-note should optimize reading, retrieval, and reuse rather than generation time.
- HTML diffs can be noisy; o-note should preserve source history and may later add rendered visual diffs or DOM-aware diffs.
- Markdown remains supported because existing notes and imports matter.
- HTML artifacts are untrusted by default.
