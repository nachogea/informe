# Native JSON versus HTML benchmark

Status: paired renderer fixtures, local timing harnesses, saved-file tokenizer,
and results summarizer. Fixed-fixture pilot results are separate from agent
generation benchmarks. The viewer footer measures read/parse/validation,
not time until content is visibly rendered.

The hypothesis is that shared native components reduce generated output and
improve the review loop. It is not assumed to be true.

## Compare three approaches

1. Native semantic JSON rendered by the release-mode GPUI viewer.
2. Self-contained HTML/CSS rendered by a pinned browser-based viewer.
3. HTML using a shared theme/component vocabulary, so the HTML agent does not
   need to reinvent styling for every artifact.

The third arm tests whether savings come from semantic JSON or simply from
reusing a renderer. Separate ordinary browser opening from a controlled WebView
host comparison. Do not label Chrome startup as universal WebView performance.

## Matched tasks

Start with three briefs: the AWS dashboard, the chart/table cost breakdown, and
a longer project review containing repeated cards and callouts. The existing
JSON artifacts supply the content briefs, not privileged input to just one arm.
Add prescribed edits: change a metric, append a table row, and rewrite a callout.

Give each approach the same facts, interaction requirements, and visual quality
target. Include node selection and copy behavior in both renderers if these are
part of the task. HTML may attach stable data IDs for equivalent selection.
Do not compare static HTML against an interactive native view.

First create fixed paired fixtures to isolate renderer performance. Separately
run agent-generated artifacts to measure generation, repairs, and quality.

## Measurements

| Metric | Definition |
| --- | --- |
| Input tokens | Provider-reported input usage, including schema/skill instructions and repairs |
| Cached input tokens | Record separately; already included in input usage |
| Output tokens | Provider-reported output across all attempts; document whether reasoning is included |
| Reasoning tokens | Separate provider-reported detail when available; never add twice |
| First token latency | Request dispatch until the first generated output token |
| Generation latency | Request dispatch until a valid, complete artifact is available, including repairs |
| Open latency | Local launch/navigation request until the complete artifact reaches a documented renderer readiness marker |
| Reload latency | Atomic edit completion until the updated content reaches that marker |
| Repair count | Failed attempts before a valid artifact |
| Output bytes | UTF-8 payload size; not a token estimate |
| Success rate | Trials meeting validation, content, interaction, and visual criteria |

Also record total request-to-ready wall time for the full workflow. Do not infer
it by summing overlapping streamed stages. If generation is not streamed, the
sequential stages may be reported separately as well.

Use the same exact model/version, inference settings, tool environment, and
concurrency for every arm. Count the skill and schema input cost, all repair
calls, and tool output that enters model context. Keep structured-output and
ordinary-text generation experiments separately labeled. Model billing usage is
the primary token measurement; tokenizer counts on saved files are secondary
and must name their encoding. A characters-per-token estimate is insufficient.

## Timing protocol

- Build GPUI in release mode, bundle all resources locally, and pin the HTML
  viewer/browser version. HTML fixtures should work offline with no CDN fonts
  or chart libraries fetched during timing.
- Use a monotonic clock. Declare exactly where every measurement starts/ends.
- Run cold-process launches, warm-runtime document opens, and warm reloads as
  separate scenarios. A new process is not necessarily a cold filesystem cache.
- Randomize/interleave native and HTML trials on the same machine. Record OS,
  CPU, app revisions, build profile, display/window size, and relevant settings.
- For GPUI, add an explicit content-ready marker after loading and submitting
  the first artifact frame. For HTML, instrument equivalent content readiness
  after fonts/layout and a rendering opportunity. Neither marker proves physical
  screen presentation; verify against captures and label the approximation.
- Compare the current 250 ms polling loop fairly: include the polling delay in
  native reload results. Disclose the HTML reload trigger and cadence. A
  matched-polling arm can help isolate rendering from change detection.
- Aim for 20 generation trials per task/arm and 50 opening/reload trials once
  instrumentation exists. Record failures/timeouts rather than dropping them.
- Report sample count, median, p95, and success rate. Treat small-sample tails
  cautiously. Retain raw records, prompts, outputs, and screenshots.

Use a content/layout rubric: all requested facts, readable text, complete chart
labels, aligned table cells, correct selection, and successful copy behavior.
Fast generation is not a win if the artifact is materially less useful.

## Repository layout to grow into

```text
benchmarks/
  README.md            Protocol and limits
  summarize.py         Aggregates measured trial records
  tasks/               Identical briefs and prescribed edits
  fixtures/            Paired JSON and HTML artifacts
  harnesses/           Native and HTML readiness instrumentation
  results/             Raw JSONL records and reports
```

Benchmark dependencies stay outside the production viewer. The three paired
fixtures are deterministically built by `build_fixtures.py`; they measure renderer
behavior and are not independently generated model outputs. Native and both
HTML arms implement node selection and selected-node copy. The shared HTML arm
references bundled CSS/JavaScript; its payload token count excludes those
preinstalled assets, analogous to excluding the native renderer binary.

Use your existing ChatGPT/Codex subscription for initial generation trials.
No API key or separately billed API call is required by this repository.
Record the exact selected model/settings, prompts, saved outputs, measured
wall time, and observed usage when exposed by the client. Leave unavailable
usage fields empty; file token counts do not substitute for provider input,
reasoning, repair, or billed-token usage. Existing conversation fixtures cannot
establish independent generation latency or model efficiency.

## Run local renderer trials

```sh
python3 -m venv /tmp/artifact-bench-venv
/tmp/artifact-bench-venv/bin/pip install -r benchmarks/requirements.txt
/tmp/artifact-bench-venv/bin/playwright install chromium
sh scripts/package-macos.sh
/tmp/artifact-bench-venv/bin/python benchmarks/run_local.py \
  --binary target/release/informe \
  --environment my-immutable-environment-id \
  --output /tmp/artifact-trials.jsonl --trials 5
python3 benchmarks/summarize.py /tmp/artifact-trials.jsonl
/tmp/artifact-bench-venv/bin/python benchmarks/count_tokens.py benchmarks/fixtures/*
```

Create an environment manifest before running, following the checked-in pilot
example. Chromium uses an isolated profile and headed windows; it does not use
your personal browser. Both harnesses measure process start through framework
readiness. Browser timing includes browser/context creation and local navigation;
native timing includes its app initialization and file load. The local HTTP
server and Playwright driver are initialized before timing. This compares these
particular launch workflows, not GPUI versus every browser/WebView host.
The suite randomizes task/arm/trial order with a recorded seed. Browser selection
and clipboard checks occur after timing; native interactions are covered by GPUI
tests. Visual validation remains a separate check. Screenshot files remain local
and are gitignored. `--ready-file` writes an atomic revision marker after a GPUI
artifact frame renders. Native marker observation polls every 2 ms.

Native reloads can be measured with `benchmarks/harnesses/native.py --scenario
reload` and the same binary/artifact/output/environment options. This changes
the root ID via atomic rename and includes the 250 ms polling delay. HTML reload
and warm-runtime opens are not yet instrumented; do not infer them from cold
process numbers.

## Results format and summarizer

Record one completed trial per JSONL line. Required fields:
`task`, `format` (`native_json`, `html_standalone`, `html_shared`), `model`,
`environment`, `scenario`, and `success` (boolean).
`environment` identifies an immutable metadata manifest with machine, OS,
renderer revisions, build, and inference configuration. `scenario` distinguishes
cold-process, warm-open, reload, and generation experiments.

Optional measured numeric fields: `input_tokens`, `cached_input_tokens`,
`output_tokens`, `reasoning_tokens`, `first_token_ms`, `generation_ms`, `open_ms`,
`reload_ms`, `request_to_ready_ms`, `repair_count`, `output_bytes`.
Use null or omit unmeasured fields. Successful repair trials include total
usage/latency across attempts. Failed trials must retain any observed usage and
latency; successful-trial latency aggregates are reported separately from
all-attempt usage so failures cannot disappear from token accounting.

```sh
python3 benchmarks/summarize.py benchmarks/results/run.jsonl
```

The summarizer validates records and groups by task/format/model/environment/
scenario. It prints counts, success rate, token totals across all trials, and
median/p95 for measured fields among successful trials. It does not call an AI
API, start a browser, estimate token counts, or fabricate missing measurements.

See the [first local fixed-fixture pilot](results/pilot/README.md) for measured
results and their limitations.


## Current native viewer measurements

[Viewer pass measurements](results/viewer-2026-10-03/README.md) cover the three
new reading fixtures, JSON and native Markdown, on the current release build.
`harnesses/native.py` measures isolated cold processes and atomic polling reloads;
`harnesses/warm_native.py` measures repeat opens in the existing workspace.
These are runtime measurements, separate from generation tokens and the historical
HTML comparison. No API calls or model charges are involved.
