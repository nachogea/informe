[← README](../README.md)

# Reviewing a report

Review is an experimental workflow. Quit any running viewer first, then launch
with `INFORME_EXPERIMENTAL_TOOLS=1` (for example,
`INFORME_EXPERIMENTAL_TOOLS=1 informe open report.json`).
Choose **More → Review / comments** to open the panel. **More → Select
components** enables semantic node selection. An open handoff to an existing
process does not change its environment.

Select text, or choose **Select components** and click a metric, chart, table,
or other component. Click **Comment…** (Command-Shift-M), write what should
change, and choose **Add comment**. With no selection, the comment targets the
whole report. The collapsible **Comments** panel shows context, target status,
and resolve/reopen controls; developer IDs stay in the optional inspector.

**Send feedback to agent** publishes a local feedback snapshot for a waiting
agent. It does not contact a model or remote service. The agent integration is
the bundled [generation/review skill](../skills/informe/SKILL.md) and CLI.

## Agent round trip

From your existing agent session:

```sh
informe review /tmp/report.json --wait --timeout 300
```

This opens/focuses the report and waits for a new submission. If an earlier
submission exists, it is treated as the previous round. To resume explicitly,
pass `--after FEEDBACK_ID`. Without `--wait`, the command reads the latest
submission immediately. Reading never consumes feedback. Output is JSON;
errors/timeouts go to stderr with a nonzero exit status.

Feedback includes the artifact identity, exact content hash, supplied source
metadata, and unresolved comments. Each comment retains its original node or
quoted text, original content, and original revision. The batch records target
status at submission; later edits and resolution do not mutate a sent batch.

After reading feedback, generate and validate a sibling candidate. Preserve
artifact identity and stable IDs; increment `metadata.revision` when present:

```sh
informe --check /tmp/report.next.json
informe apply-revision /tmp/report.json /tmp/report.next.json --expect HASH
```

Use the submitted `revision.content_hash` for `HASH`. A concurrent edit makes
this guard fail; inspect/reconcile the current report rather than overwriting it.
The preview reloads automatically. Added nodes appear in green, changed nodes
in amber, and the panel describes additions, removals, and before/after values.
Selecting a component uses its usual selection outline. Resolve comments when
satisfied and choose **Mark this revision reviewed** to move the diff baseline.
Those are separate decisions; neither silently resolves the other.

The CLI also supports adding and submitting comments for scripted workflows:

```sh
informe comment /tmp/report.json --node projected-spend --text "Compare against last month."
informe comment /tmp/report.json --quote "Projected spend" --text "Explain this estimate."
informe submit /tmp/report.json
```

Repeated text requires `--node ID` and, if needed, `--offset BYTES` within that
node's plain-text export. The native editor rejects ambiguous text anchors
rather than guessing. Selections across multiple components may target their
smallest matching container. A removed/retyped node, edited quote, ambiguous
quote, or changed component is explicitly flagged for review.

## Storage and provenance

Review data lives beside the artifact as `report.json.review.json`. It stores
comments, the accepted semantic baseline, and the latest submitted batch.
Atomic writes and a per-file lock merge concurrent comment saves. A draft is
bound to its starting revision; if it becomes stale, saving fails and leaves
the draft visible. Corrupt or mismatched review files are reported and preserved.

Artifacts without metadata remain supported; their canonical path is their
local identity. Give new reports an explicit `metadata.id` to keep identity
stable. Metadata source labels, URIs, `as_of`, and `generated_at` are author
supplied information, not verified claims. The viewer says when sources are
absent. See the [format](artifact-format.md).

Keep the sidecar with its artifact when moving a report with explicit identity.
Legacy path identities require the same canonical path. Sidecars can contain
the full baseline and comment context; share them deliberately. PNG exports
contain the report, not private comments. There is no cloud synchronization,
full revision history, or built-in model execution. Benchmark readiness mode
does not create or read review sidecars.

## Reproduce the demonstration

```sh
mkdir -p /tmp/artifact-review-demo
cp examples/review/cost-v1.json /tmp/artifact-review-demo/report.json
informe review /tmp/artifact-review-demo/report.json --wait --timeout 300 > /tmp/artifact-review-demo/feedback.json
```

In the viewer, comment **“Compare this against last month.”** on **Projected**,
then send feedback. Use the hash from that JSON with `apply-revision`, passing
`examples/review/cost-v2.json` as the candidate. It adds an illustrative
$356,000 comparison baseline and explains the $56,000 / 15.7% difference.
The comment remains present with **Target changed — review context**. These
fixtures exercise the protocol; they are sample figures, not live AWS data.
