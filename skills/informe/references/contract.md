## Artifact contract v1

```json
{
  "version": 1,
  "root": {
    "type": "column",
    "id": "root",
    "gap": 16,
    "children": [
      { "type": "heading", "id": "title", "level": 1, "text": "Forecast" },
      { "type": "metric", "id": "spend", "label": "Projected", "value": "$412,000", "secondary": "+8.2%" }
    ]
  }
}
```

Every node requires a nonempty ID, unique across the document. Keep IDs stable
across text edits and reordering. IDs identify semantic objects, not array
positions. The viewer does not generate or rewrite them.

| Type | Fields in addition to `type` and `id` |
| --- | --- |
| `column` | `children`, optional `gap` (default 16) |
| `row` | `children`, optional `gap` (default 12) |
| `heading` | `level` (1–6), `text` |
| `text` | `text` |
| `markdown` | `source` (Markdown text) |
| `card` | `children` |
| `metric` | `label`, `value`, optional `secondary` |
| `callout` | `kind`, `body`, optional `title` |
| `divider` | No additional fields |
| `button` | `label`, `action: { "type": "noop" }` |
| `chart` | `title`, `kind: "bar"`, `data: [{ "label": "EC2", "value": 188 }]`, optional `unit` |
| `table` | `title`, `columns: ["Service", "Spend"]`, `rows: [["EC2", "$188,000"]]` |

Callout kinds: `info`, `warning`, `error`, `success`.
Gaps must be finite, nonnegative numbers in GPUI logical pixels.
Unknown versions, types, actions, and fields are rejected. JSON syntax errors
include line/column information; semantic errors include a tree path and node ID.

The renderer owns colors, padding, typography, and borders. Columns and cards
stack their children; rows allocate equal-width slots and stack vertically when
the available content width is below 640 logical pixels. Wide tables scroll
horizontally. Layout adaptation belongs to the renderer. Buttons perform
no further action.

Charts currently support a single series of horizontal bars with a zero baseline
and an automatic maximum. Data must be nonempty, finite, and nonnegative; an
all-zero series is valid. Units are displayed beside values. Tables use string
cells, require at least one column, and validate each row's cell count. Empty
tables show "No rows". In component mode, a chart or table is selected as a whole semantic node;
individual bars and cells are not separate annotation targets yet.

## Identity and provenance (optional)

The top-level `metadata` object supports `id` (nonempty stable string),
`revision` (integer >= 1), optional `generated_at`, and `sources` (default `[]`).
Each source has a nonempty `label`, optional `uri`, and optional `as_of`. Timestamp
strings are author-supplied display values, not proof of freshness or verification.
Preserve identity and increment revision for reviewed edits. Legacy JSON v1
remains valid; its canonical path supplies local identity. Feedback also carries
an authoritative content hash, so repeating a revision label cannot disguise edits.

```json
"metadata": {
  "id": "forecast-dashboard",
  "revision": 1,
  "sources": [{"label": "Illustrative sample; no live AWS connection"}]
}
```

Comments, review baseline and submitted feedback live in a sibling
`artifact.json.review.json`, outside the artifact contract. Agents should update
the artifact through the guarded revision CLI and leave that sidecar to the viewer.


## Native Markdown documents

Open UTF-8 `.md` or `.markdown` files directly, or embed prose in a JSON node:
`{"type":"markdown","id":"summary","source":"**Summary**\n\n- First point"}`.
Headings, emphasis, lists, task lists, blockquotes, fenced code, tables and links
render through GPUI Kit's native TextView. This does not introduce a WebView.
Top-level headings become semantic Heading nodes for Contents navigation.
Reference-style links are carried into the individual blocks during import.
HTTP(S) and mailto links open externally; `#heading-slug` links navigate matching
headings. Other link protocols are ignored. Raw top-level HTML displays literally.
Images, HTML layouts, executable content and arbitrary Markdown extensions are
not a supported authoring contract in this prototype.

Imported IDs derive from block content plus duplicate occurrence. Unchanged
blocks retain IDs when unrelated content moves; editing a block changes its ID.
Use JSON with explicit IDs and metadata for durable review targets. Guarded
`apply-revision` accepts JSON targets only. Search and whole-document copy use
plain text; Markdown PNG export currently flattens formatting into plain text.
