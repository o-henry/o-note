# UI/UX Direction

o-note should use a Bullpen-inspired UI direction, governed by the user's selected o-rade skills:

- `$orade`
- `$orade-bullpen-faithful-ui`
- `$orade-axi-precision-board`
- `$orade-open-design-reference`
- `$orade-open-design-terminal`

## Priority Order

1. User's latest explicit direction.
2. Actual Bullpen source files and screenshots.
3. Bullpen faithful style contract.
4. o-rade anti-slop rules.
5. Axi precision constraints.
6. Open Design references only as supporting critique, not as a replacement style.

## Current Reference

The target style is `https://bullpen.sh/`.

The local Bullpen source files are not present in this repository yet. Before production UI implementation, inspect actual Bullpen source files or screenshots and write a concrete style contract.

Required source-grounded files when available:

- `.local/bullpen-source/frontend/src/styles.css`
- `.local/bullpen-source/frontend/src/app/App.tsx`
- `.local/bullpen-source/frontend/src/app/AppSidebar.tsx`
- `.local/bullpen-source/frontend/src/features/analysis/AnalysisPage.tsx`
- `.local/bullpen-source/frontend/src/features/portfolio/PortfolioPage.tsx`

## Style Contract

Preserve:

- Sidebar shell first, not generic top navigation.
- Inset content area with dense report surfaces.
- Zero-radius or nearly zero-radius UI.
- Flat backgrounds and bordered surfaces.
- Monochrome or dark restrained tokens.
- Mono uppercase microcopy for metadata, tabs, counters, status, and controls.
- Rectangular or underline tab states, not rounded pills.
- Report/source/agent-like information organization.
- Dense but quiet hierarchy.
- Tables, source panels, and right rails with strong alignment.

Avoid:

- Fake logo or fake product branding.
- Purple, blue, or pink AI gradients.
- Decorative blobs, bokeh, glassmorphism, or SVG hero filler.
- Generic SaaS dashboard cards.
- Shadow-heavy hierarchy.
- Rounded pill controls for primary navigation.
- Text clipping or overlap.
- Internal implementation jargon in user-facing UI.

## o-note Mapping

Map Bullpen-style sections into o-note product areas:

- Notes: local note list, recent artifacts, folders/tags.
- Artifact report: rendered Markdown or HTML note with source/check panels.
- Sources: backlinks, attachments, generated context, imported files.
- Agent workspace: Codex-generated plans, reviews, reports, and custom editors.
- Index status: local indexing, import progress, search health.
- Settings: local storage, export, sandbox policy, trusted paths.

## Interaction Principles

- Keyboard-first navigation is required.
- Search should feel like a command cockpit, not a modal afterthought.
- HTML artifacts should have clear preview/source modes.
- Artifact controls should expose safe actions such as copy prompt, copy JSON, export HTML, and export Markdown.
- HTML artifact JavaScript cannot call privileged app APIs directly.
- Local-only status should be visible where storage, indexing, and generated artifacts are discussed.

## Candidate Workflow

For UI ideation:

- Work inside an isolated `ui-lab` or planning folder.
- Create only a small number of genuinely distinct candidates.
- Do not touch production Tauri code until a direction is selected.
- Verify candidates in a real browser.

For production:

- Implement from shell outward: sidebar, content inset, report header, tabs, right rail/source panels.
- Verify at 13-inch and 24-inch desktop widths.
- Run forbidden-pattern checks before claiming visual quality.
- Do not claim Bullpen fidelity without browser evidence.
