> Historical baseline: these results predate the GPUI Kit/document-workspace migration. Rerun before making claims about the current build.

# Local renderer pilot

Fixed, deterministic fixtures; no independent generation trial and no AI API
calls. macOS 26.6.2, Apple Silicon, release GPUI; headed Chromium 145.0.7632.6
via Playwright 1.58.0. Five samples per task/arm, interleaved with seed 42.
All 45 cold-process readiness checks and 30 HTML selection/clipboard checks
passed. Native node selection/copy is covered by GPUI tests, not checked by the
latency harness. Visual parity is not a fully scored quality evaluation.

## Cold-process opening

Median milliseconds from process launch to framework marker:

| Fixture | Native GPUI | Standalone HTML | Shared HTML |
| --- | ---: | ---: | ---: |
| dashboard | 179.4 | 1432.8 | 1407.2 |
| cost-breakdown | 195.2 | 1418.6 | 1391.3 |
| long-review | 185.3 | 1461.5 | 1443.6 |

Five native chart/table reloads passed; median 250.5 ms.
The edit happens immediately after initial readiness and includes the next
250 ms file poll. It is not a random-phase reload benchmark. No HTML reload
measurements exist yet.

These are process-cold, filesystem-cache-warm pilot measurements on one active
development machine with other prototype windows open. Smoke runs preceded the
suite. GPUI uses its after-frame callback; HTML waits for fonts and two animation
frames. Browser launch includes context creation and navigation; the local HTTP
server and Playwright driver start before timing. Native has different framework
initialization costs. This demonstrates these two workflows, not a universal
renderer speed comparison. Five samples do not establish stable p95 claims.

## Saved-file tokens

`tiktoken 0.12.0`, `o200k_base`; counts include the exact stored whitespace.
These are payload sizes, **not** model usage, billing, input cost, reasoning cost,
or generation latency. Shared assets are installed once and counted separately.

| Fixture | Native JSON | Standalone HTML | Shared HTML payload |
| --- | ---: | ---: | ---: |
| dashboard | 350 | 1355 | 609 |
| cost-breakdown | 561 | 2080 | 1334 |
| long-review | 2042 | 4273 | 3527 |

Shared theme: 605 tokens; shared interaction script:
151 tokens. Both HTML arms store copy text in data
attributes; this duplicates some visible content. JSON is pretty-printed while
HTML/CSS/JS are compact. These fixture choices affect counts. Further trials
should include payload normalization and alternative copy implementations.

The next generation experiment can use subscription models with the same task
brief and independent fresh sessions for each arm. Record observed usage when
available, retain repairs, and leave unobserved fields empty.

Raw data: [cold-process.jsonl](cold-process.jsonl), [reload.jsonl](reload.jsonl),
[file-tokens.jsonl](file-tokens.jsonl), [summary.json](summary.json),
[environment.json](environment.json), and the recorded trial schedule.
