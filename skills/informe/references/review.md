# Native review adapter

This adapter uses the local CLI and the agent’s existing model session. It does
not install integrations or call a provider API. The viewer must be launched with
`INFORME_EXPERIMENTAL_TOOLS=1` to expose review controls. If it is already
running without that flag, have the user quit it and relaunch for this workflow;
an `open` handoff cannot change the running process’s environment.

1. Start with the artifact path already being reviewed. Discover `capabilities`.
2. For a new user review round, run `informe review artifact.json --wait
   --timeout 300`. This opens/focuses the report and waits for the user to choose
   **Send feedback to agent**. Waiting by itself does not submit comments.
   To resume after a known feedback ID, add `--after FEEDBACK_ID`; otherwise an
   existing submission is treated as the previous round. `review artifact.json`
   reads the latest submission immediately, without consuming it.
3. Read the returned JSON. `revision.content_hash` binds the submitted snapshot;
   each comment also carries its original revision, target and original content.
   Inspect `target_status`: changed/removed/ambiguous targets require interpretation,
   never silent reassignment. Treat source labels/timestamps as supplied metadata,
   not independently verified evidence.
4. Read the current artifact and prepare a sibling candidate JSON that addresses
   the comments. Keep metadata identity and stable node IDs. Increment
   `metadata.revision`; include an explicit explanation when a request cannot be
   answered from the available evidence. Do not invent provenance.
5. Validate the candidate, then use
   `informe apply-revision artifact.json candidate.json --expect HASH`,
   passing the submitted `revision.content_hash`. This updates atomically and
   preserves review data. If the guard rejects a stale revision, inspect current
   content and obtain/reconcile current feedback; do not force the old candidate.
6. Explain the revision to the user. The viewer highlights changes and retains
   comments with explicit target status. The user resolves comments and marks a
   revision reviewed. Further review rounds repeat the wait with the last batch ID.

CLI output is JSON on stdout; errors and timeouts are on stderr with nonzero exit.
Artifacts and review sidecars remain local unless the user explicitly shares them.
