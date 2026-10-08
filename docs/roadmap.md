[← README](../README.md)

# Product roadmap

Product direction: a fast, polished native viewer for agent-generated reports,
dashboards, and plans. The reading experience is the immediate product priority.
Structured review is implemented but is secondary; it must not dominate the UI.
The semantic artifact contract supplies consistent content and GPUI supplies the
desktop experience.

This checklist is the persistent implementation plan. Update completion and
verification evidence as changes land. Milestones are acceptance criteria, not dates.

Latest assessment: [product QA and release cuts — 2026-10-04](qa-2026-10-04.md).
The follow-up pass fixed sorting, copy/search fidelity and narrowed the normal
viewer surface. Live keyboard/VoiceOver and clean-install acceptance remain open;
the integration pilot and matched generation proof below stay deferred experiments.

Public documentation pass (2026-10-04): simplified the README, added a user guide
and docs index, corrected experimental review setup, and replaced the obsolete
private-pilot distribution proposal with current release gates. Added an original
SVG mark and macOS icon with a reproducible build helper. Local documentation
links, skill validation, formatting/Clippy, and archive assets passed checks.
The README product screenshot was refreshed with the Informe identity and an
illustrative release report; see [distribution](distribution.md).

## Immediate priority — A convincing viewer

User feedback after milestone 2: the review workflow is functional, but the UX
is not convincing compared with Plannotator. The immediate priority is a good,
solid, performant report/dashboard/plan viewer. Completed checklists below record
functionality delivered; they do not establish product quality or user acceptance.
Defer further review features and broad distribution work until the viewer passes
this acceptance gate. Keep existing review available as a secondary workflow.

Acceptance: three realistic documents—a narrative report, a data dashboard, and
a development plan—open directly into an attractive, readable experience at the
default window size. Routine reading, navigation, copying and resizing feel
natural. Measure the current runtime before claiming a performance advantage.

- [x] Replace stacked toolbars with compact document chrome and an overflow menu.
- [x] Make review/diff controls optional; hide developer tools and change outlines in normal reading.
- [x] Improve typography, content density, spacing and responsive metric/chart/table layouts.
- [x] Add document search, section navigation, working links and readable long-document behavior.
- [x] Evaluate native Markdown ingestion for plans; support semantic lists and code blocks as needed.
- [x] Make dashboards useful to inspect: readable labels/units, data hover details, and practical table behavior.
- [x] Create three realistic fixtures and inspect them at narrow, default and wide window sizes.
- [x] Measure cold/warm open and reload with workload, raw trials and percentile results.
- [x] Exercise scrolling and resizing on the three fixtures at 660, 1100 and 1400 logical-pixel widths.
- [ ] Instrument frame pacing and stress-test much larger documents before claiming sustained rendering performance.
- [ ] Agree on the visual direction from a working viewer example before expanding features again.

GPUI remains the current renderer. Native rendering alone is not the product
advantage; consistent presentation, easy generation, reliable launch and a smooth
reading experience must earn it. Rich arbitrary HTML layouts remain a separate
tradeoff and are not a promise of this viewer.

Implementation: compact reading chrome and More menu, optional Contents sidebar
(or temporary drawer at narrow widths), cached block search, native GFM Markdown,
responsive metric rows, chart units/details and numeric-aware table sorting.
Ordinary viewing no longer creates review sidecars. Cold `open` launches into
its requested document; existing windows activate before the render acknowledgment,
including when minimized. JSON IDs remain explicit; imported Markdown IDs derive
from unchanged block content.

Verification: 28 Rust tests, Clippy with warnings denied, benchmark-tool tests and
skill validation pass. Live checks covered three tabs, Markdown word copy into Find,
block search, section jumps, internal links, numeric sorting and responsive layouts.
Screenshots from that pass remain in the corresponding Git revision. Runtime
measurements and boundaries
are in [the viewer timing report](../benchmarks/results/viewer-2026-10-03/README.md).
PNG currently flattens Markdown formatting. Very large documents are not virtualized.
This pass is available for visual evaluation; user acceptance remains unchecked.

## Queued proof — Matched agent generation

Requested on 2026-10-04; retain this experiment for a later run. The current
renderer timings and saved-file token counts do not establish an end-to-end
advantage over generated HTML. Follow the [benchmark protocol](../benchmarks/README.md).

- [ ] Freeze three identical briefs (report, dashboard, plan), source facts,
  interaction requirements and a reading-quality rubric before generation.
- [ ] Compare semantic JSON, self-contained HTML, and HTML with a supplied shared
  component/theme library. Give every arm comparable documentation and capabilities;
  record schema/skill/library input overhead separately.
- [ ] Start with five independent generations per brief/arm (45 runs) using the
  existing subscription models, fresh contexts, fixed model/settings and randomized
  order. Record the exact prompts, outputs, validation failures and repair attempts.
  Expand the sample only if the pilot supports a useful comparison.
- [ ] Capture observed model usage when exposed; otherwise mark it unavailable.
  Saved-file token counts with a named tokenizer remain a separate proxy. Include
  repair turns and input overhead; never label estimated counts as billed usage.
- [ ] Measure request-to-visible-document including generation, repairs, file writing
  and launch. Report cold and warm host conditions separately, with equivalent HTML
  browser reuse. Keep renderer frame acknowledgments distinct from physical presentation.
- [ ] Assess factual completeness, readability, copyability, chart/table usefulness
  and human preference alongside latency and tokens. Retain raw trials and screenshots;
  report limitations instead of declaring a winner from speed alone.

## Proposed next product pilot — Workspace-linked living reports

Candidate direction, not implemented or accepted: make reports a useful part of
an agent workspace rather than another temporary preview window. Keep review secondary.

Acceptance scenario: ask “Show me what changed on this branch”; a report opens,
belongs to the originating workspace, links findings to source files, and can be
regenerated into the same document without losing reading position. Reopen it from
the workspace and return to the originating agent. Show the source revision and
generation time so an older report is recognizable; do not invent provenance.

- [ ] Pilot one local Herdr workflow: publish/open, list/reopen workspace artifacts,
  and return to the originating pane. Reuse the existing viewer instance and tabs.
- [ ] Add explicit source references with file/line targets; launch the configured
  editor on deliberate clicks. Keep document data accessible through CSV/text export.
- [ ] Define a small host-independent publish contract carrying artifact path,
  title, workspace and origin identifiers; acknowledge readiness or return an error.
- [ ] Test repeated generation, workspace switching and missing origin/source targets
  with a real branch report before introducing another host or new primitives.
- [ ] If this workflow earns repeated use, expose the same contract through a small
  MCP adapter for Zed agents, then package it as a Zed extension.

Integration feasibility checked 2026-10-04: installed Herdr 0.9.3 exposes plugin
commands. [Herdr plugin v1](https://herdr.chefgroep.nl/docs/plugins/) supports commands,
context and terminal panes, explicitly excluding native non-terminal plugin UI.
The first pilot therefore opens the companion GPUI app; an embedded native tab
would require host work. [Zed's documented extension surface](https://zed.dev/docs/extensions/developing-extensions)
does not expose custom GPUI panels; [MCP server integration](https://zed.dev/docs/ai/mcp)
is the practical initial route. Plannotator already offers
[Herdr Annotate](https://github.com/backnotprop/plannotator), so host integration
alone is not differentiation. Evaluate recurring report usefulness, not novelty of
the rendering technology. HTML could implement the same workflow.

## Milestone 1 — A useful native document app (complete — 2026-10-03)

Acceptance: a natural agent request produces a visible document; ordinary text
can be selected/copied; the canvas is useful without developer metadata; people
can copy images and table data for use elsewhere.

### Dependable opening

- [x] Reuse a single local app instance and one tab per canonical artifact path.
- [x] Focus the requested document and bring its window forward.
- [x] Acknowledge `open` only after a valid document frame renders; report failures/timeouts.
- [x] Keep isolated process/readiness mode for reproducible renderer benchmarks.
- [x] Support switching/closing documents and a small recent-document list.
- [x] Verify cold open, repeat open, two documents, invalid input, and recovery.

### Reading and selection

- [x] Evaluate/adopt GPUI Kit's foundation selection with a compatible pinned GPUI set.
- [x] Make headings, paragraphs, metrics, chart labels, and table cells selectable.
- [x] Support dragging, double-click selection, Command-C, and Command-A.
- [x] Separate normal text interaction from deliberate semantic component selection.
- [x] Hide the developer inspector by default; expose it through a developer toggle.
- [x] Use a document title and quiet status instead of a full path/debug timing toolbar.
- [x] Preserve reload recovery and stable semantic selection; clear stale text selection on edits.

### Copy and export

- [x] Copy selected text and whole-document text without opening the inspector.
- [x] Copy a selected semantic component as a PNG image, excluding app chrome.
- [x] Save the whole document as PNG, including content below the viewport.
- [x] Copy/export rectangular tables as TSV/CSV with proper escaping.
- [x] Use a native save dialog and report export completion or errors.
- [x] Check exports visually, including Unicode, long text, zero-valued charts, and long tables.

### Delivery

- [x] Update README, capabilities, agent skill, and architecture docs to match actual behavior.
- [x] Run Rust checks, meaningful interaction/IPC/export tests, and live macOS smoke checks.
- [x] Rebuild the release bundle, demonstrate the new report experience, and push the milestone.

Decisions: retain semantic JSON v1. No annotation UI, PDF export, hosted accounts,
Herdr rendering backend, or broader artifact action system in this milestone.
PNG is an export renderer, not a new primary artifact format. Signing can proceed
in parallel when credentials become available.

Verification: 15 Rust tests, Clippy with warnings denied, three benchmark-tool
tests, and generation-skill validation passed. Release checks covered cold and
repeat opens, two document tabs, invalid CLI input, deletion/recreation, atomic
reload, and a relocated packaged app. Native component-image copy and the PNG/CSV
save dialogs were exercised; saved CSV matched every source cell. Full-content
PNG exports were visually checked with long paragraphs, Latin/CJK text, a zero
chart and a 31-row table. Color emoji are a documented PNG limitation.

The README screenshot shows the actual document UI. Historical benchmark timings
remain labeled as belonging to the earlier runtime; this milestone makes no new
performance claim. The local archive remains unsigned pending milestone 3.

## Milestone 2 — A complete review loop (complete — 2026-10-03)

Acceptance: comment → agent revision → highlighted changes, with comments
preserved or explicitly flagged when their target changes.

- [x] Add artifact identity/revision metadata and node/text-targeted comments.
- [x] Show contextual commenting and a collapsible human-oriented comments panel.
- [x] Export revision-bound feedback; provide a CLI review/wait protocol and one agent adapter.
- [x] Highlight meaningful changes and identify removed/changed annotation targets.
- [x] Add explicit source/freshness information without inventing provenance.
- [x] Run the demo: cost report → comment on a metric → revision → highlighted change.

Implementation decisions: optional metadata preserves JSON v1 compatibility;
legacy documents use their canonical path as local identity. A content hash binds
feedback to the exact parsed snapshot. A sibling `.review.json` stores comments,
review baseline and submitted feedback, with atomic writes and a process lock.
Targets retain their original text/content and revision; updates never silently
retarget comments. The agent adapter is the existing generation skill plus CLI
`review --wait` and guarded `apply-revision`, with no built-in model calls.

Verification: 23 Rust tests include target changes/removal/ambiguity, immutable
feedback, concurrent comment saves, guarded revisions, corrupt sidecar retention,
quoted-text selection, and stale native drafts. The live macOS demo added a metric
comment through the native editor, returned submitted feedback to a waiting CLI,
applied revision 2, and showed the added comparison and retained changed-target
comment. Restarting the viewer preserved review state. A second native comment retained
exactly the selected quote. Isolated benchmark mode rendered a readiness marker
without creating review data; release archives exclude local review sidecars. Clippy with warnings denied,
benchmark-tool checks and skill validation passed. The release bundle is rebuilt;
it remains unsigned until milestone 3. See [the review guide](review.md) for the
reproducible demo and local storage boundaries.

## Milestone 3 — Something people can adopt and share

Acceptance: signed installation, easy agent setup, useful PDF export, and
successful trials with outside users.

- [ ] Sign/notarize Apple Silicon releases and verify a clean Mac without Rust.
- [ ] Offer a direct app download and a Homebrew cask for the same runtime.
- [ ] Verify installation, launch, update, reload, and uninstall; determine actual OS support.
- [ ] Provide explicit agent setup with a working sample-report check.
- [ ] Export searchable, paginated PDFs with readable tables and sensible page breaks.
- [ ] Pilot with developers/technical leads using actual reports; record friction and failures.
- [ ] Run independent subscription-model generation trials with observed usage and quality scoring.
- [ ] Measure request-to-visible-document, warm opens, and matched reload workflows.

## Later experiments

- [ ] Prototype a Herdr launch/focus adapter before considering embedded rendering/input.
- [ ] Explore portable sharing/readers without changing the semantic source format.
- [ ] Add primitives, history, and simple state/actions only for demonstrated review needs.

The current contract has twelve primitives, JSON/Markdown loading, reload/error recovery,
and stable node IDs. [Distribution](distribution.md) and
[benchmark protocol](../benchmarks/README.md) retain detailed experiments and limitations.
