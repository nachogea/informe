---
name: informe
description: Generate and update semantic JSON dashboards and native Markdown reports/plans for the native Informe viewer.
---

Use `informe --version`, `informe capabilities`, and
`informe schema` to discover the installed contract. If the CLI is
unavailable, use this repository's built binary or explain that it needs installing.
Use Markdown for prose reports and plans; use semantic JSON for dashboards with
metrics/charts/sortable tables or durable node IDs. Read [the contract](references/contract.md)
when composing JSON or checking Markdown support.

For JSON, create a UTF-8 document with `version: 1` and `root`. Prefer semantic
metrics, callouts, charts, and tables over generic card nests. The renderer owns
typography and colors. Use descriptive IDs that remain stable through edits;
IDs must be globally unique. Label invented data as illustrative. Do not generate
buttons: the supported `noop` legacy action is displayed as unavailable.

Write to a sibling temporary file, validate with
`informe --check temporary.json` (or `temporary.md`; preserve the format extension), then atomically rename it to the
intended artifact path. Repair validation errors before publishing the update.
For a new preview, run `informe open /absolute/path/artifact.json`.
For an already-open artifact, update its existing path; the viewer polls every
250 ms and retains the last valid artifact on an error. `open` reuses the app
and a tab for the canonical path, brings it forward, and waits for a valid
rendered frame before acknowledging success. Report launch errors; do not claim
the report is visible just because JSON was generated. Updates usually need no
new open command; reopen to bring an existing report forward if requested.

People can drag/double-click text and copy with Command-C; Command-A selects
the whole document. Tables offer Copy TSV and Save CSV directly. More contains
optional whole-document PNG copy/export; headless `informe export-png
input.json output.png` is also available. Find and Contents help navigate
documents. Markdown PNG export currently flattens formatting to plain text. Charts support one
nonnegative horizontal bar series. Tables contain string cells with equal row
widths. Keep content readable at the default 1100 × 760 window size.

Only when the user explicitly requests the experimental review workflow, supply
`metadata.id` and `metadata.revision`
(start at 1). Preserve the artifact identity and existing node IDs, increment
revision on edits, and include only sources/freshness information actually known.
Legacy artifacts still work, using their canonical path as local identity.

When asked to revise a reviewed artifact, read [the review adapter](references/review.md).
The CLI can wait for explicitly submitted feedback and apply a revision guarded
by its content hash. Do not modify review sidecars or resolve comments on the
user’s behalf unless asked.
