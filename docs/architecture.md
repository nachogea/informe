[← README](../README.md)

# Architecture

- `src/artifact.rs`: GPUI-independent types, parsing, validation, node lookup.
- `src/loader.rs`: content comparison, file reads, parsing, reload timing.
- `src/workspace.rs`: document tabs, canonical-path reuse, recent documents.
- `src/ipc.rs`: private Unix socket, process lock, bounded open handoff.
- `src/preview.rs`: per-document reload state, reading/selection UI, optional inspector.
- `src/export.rs`: full-content semantic PNG rendering and CSV/TSV encoding.
- `src/render.rs`: recursive native rendering and the opinionated theme.
- `src/data_visuals.rs`: native bar charts and simple tables.
- `src/review.rs`: renderer-independent comments, revision snapshots, semantic diffs, locked atomic sidecars.
- `src/review_panel.rs`: contextual comment editor, human-facing review panel,
  and a GPUI Kit root decoration that captures text before toolbar clicks clear selection.
- `src/review_cli.rs`: local feedback/wait protocol and guarded revision application.
- `src/main.rs`: CLI, validation-only mode, window setup.

Loads are serialized and only applied on the UI thread. The task uses a weak
entity handle and is owned by the preview, so it does not keep a closed view
alive. The typed AST is the IR; no second representation is needed yet.

To add a primitive: extend `ArtifactNode`, its identity/type helpers and
validation if needed, implement its renderer arm, and add it to the examples.

## Loading and rendering

```text
artifact.json → serde + validation → ArtifactNode → native GPUI components
```

`ArtifactNode` is both the typed AST and the IR. Its contract has no GPUI
dependency. Component appearance belongs to the renderer; JSON describes
semantics and a small amount of layout, rather than serializing styling APIs.

The background loader compares contents every 250 ms and detects writes,
rename-over saves, deletion, and recreation. Reads and parsing happen off the UI
thread. Successful updates reconcile selection against the new IDs. Invalid
updates show an error banner while preserving the last valid artifact; a later
valid update clears the error.

GPUI Kit’s `gpui-base` foundation supplies `Root`, the window-wide selection
layer, and `SelectableText`. GPUI/GPUI platform use the matching published
`gpui-pre` 0.3.7 snapshot, pinned with `gpui-base` 0.7.0. We retain our semantic
components and theme. Normal dragging/double-clicking selects text across
regions. Command-A is a viewer-level whole-document selection, with a visual
highlight and copy fallback that includes text outside the viewport. Reloads
clear text selection; semantic IDs still reconcile against the new tree.

With `INFORME_EXPERIMENTAL_TOOLS=1`, component mode (or Option-click)
selects a semantic node. Nested
clicks stop propagation so the deepest node wins. The inspector is hidden by
default; copy/export commands are available in the document UI.

`open` validates the input and hands its canonical path to one per-user app via
a private socket under `~/Library/Application Support/Informe`. A file
lock prevents competing servers; the owning process cleans stale sockets. The
server queues one request at a time. UI acknowledgment occurs in GPUI’s next-frame
callback after rendering a valid document. The window is activated before waiting
for that frame so minimized/hidden windows can draw. Cold launch supplies the
requested path immediately instead of bootstrapping an empty document service. The
caller has a 20-second bound. This is framework readiness, not physical screen
presentation. Recent paths (at most ten) are persisted as workspace metadata. Comments and
review baselines are persisted beside each artifact; see [review storage](review.md).

PNG export walks the semantic tree into an internal SVG surface and rasterizes
with resvg, using system fonts. It never executes HTML or a browser. The fixed
960-point layout includes all content and excludes app chrome; wrapping can
differ from the live viewport. Export normally uses 2x pixels, falls back to 1x
for long reports, and rejects heights above 20,000 points. Rendering and file
writes happen off the UI thread, after a native save dialog. The export font
database excludes macOS LastResort placeholders; color emoji remain an export
limitation. CSV/quoted TSV
preserve separators, quotes and multiline cells.

## Extending the contract

Add a Rust variant, validation, identity/type helpers, text export, and native
renderer. Update the [schema](../schema/artifact-v1.json), capabilities output,
[format reference](artifact-format.md), generation skill reference, examples,
and benchmark fixture builder together. Add tests for behavior or invariants
that could fail, rather than duplicating implementation details.

Keep new components semantic. Avoid arbitrary styling fields, browser
integration, databases, and generic event systems until a concrete viewer workflow
requires them. See the [roadmap](roadmap.md) for planned stages.

## Verification

Tests cover parsing and semantic validation, ID lookup/reconciliation, ordinary
and atomic file edits, deletion/recreation, last-valid-preview recovery, nested
selection, chart/table rendering, and clipboard export. Check visual behavior in
the running app when changing components or the pinned GPUI revision.

`--ready-file marker.json` writes an atomic revision marker after GPUI renders an
artifact frame. It is benchmark instrumentation, not a physical-display timing
claim. This mode remains isolated from the singleton and is used for renderer
benchmarks. The
[benchmark protocol](../benchmarks/README.md) defines the measurement boundaries.

## Review transactions

Optional metadata identifies an artifact across revisions. Without it, the canonical
file path is its local identity. SHA-256 of the serialized typed artifact binds
each comment and submitted batch to its content. The review baseline stores the
last accepted semantic tree. Diffs compare each node’s own fields and direct child
IDs, so descendant edits do not highlight every ancestor. Source dates are supplied
metadata, never verified freshness claims.

A per-artifact OS file lock serializes sidecar edits; read-modify-write merges
concurrent comments. Atomic writes use a same-directory temporary file. Comment
drafts retain the starting hash; stale writes are rejected and the draft remains
visible. Applying an agent candidate requires that exact current hash, the same
identity, and an increasing metadata revision. Submitted batches are immutable
snapshots; resolving a comment does not change previously submitted feedback.


## Viewer-first document layer

`Artifact::parse_document` dispatches by extension: strict semantic JSON, or GFM
Markdown imported into Heading/Markdown/Text blocks with content-derived IDs.
This parser stays independent of GPUI. `viewer.rs` owns outline anchors, cached
plain-text search, table sorting state and managed GPUI Kit TextView entities.
Text entities are reused until their source changes, and observed so asynchronous
parsing redraws the preview. Readiness waits for a committed Markdown parse,
including blocks with no visible text, rather than a virtualized list counter:
this viewer renders TextView without internal virtualization. Very large
documents are not virtualized yet.

Navigation and search scroll the same document handle. Layout width selects row
stacking, while narrative documents have a narrower reading column. Review state
is loaded/created only when its workflow is opened; ordinary viewing does not
write review sidecars. The existing semantic PNG renderer flattens Markdown to
plain text; it does not capture the native TextView layout.
