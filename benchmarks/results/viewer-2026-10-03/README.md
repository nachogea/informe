# Native viewer timing pilot — 2026-10-03

Current viewer pass, following GPUI Kit adoption and native Markdown support.
These are runtime measurements on fixed fixtures, not model-generation trials
or a new comparison with HTML. No AI API calls or token usage were measured.

| Document | Scenario | Success | Median (ms) | p95 (ms) |
| --- | --- | ---: | ---: | ---: |
| dashboard | cold-process | 20 / 20 | 180.8 | 218.6 |
| dashboard | warm-repeat-open | 20 / 20 | 232.4 | 250.2 |
| dashboard | reload | 20 / 20 | 250.7 | 266.7 |
| report | cold-process | 20 / 20 | 180.4 | 183.3 |
| report | warm-repeat-open | 20 / 20 | 251.9 | 268.4 |
| report | reload | 20 / 20 | 250.2 | 267.3 |
| plan | cold-process | 20 / 20 | 180.1 | 182.4 |
| plan | warm-repeat-open | 20 / 20 | 283.3 | 299.9 |
| plan | reload | 20 / 20 | 249.2 | 267.0 |

## Workload and environment

Apple M4, arm64, macOS 26.6.2, Rust 1.97.1, release build. One development
machine; 20 samples per document per scenario, run sequentially without concurrent
benchmark harnesses. Small-sample p95 values are exploratory. Other desktop apps
remained running; this is not a controlled dedicated machine or a cold disk cache.
The viewport was 1100 × 760 logical pixels at 2× display scale.

Fixtures: [dashboard.json](dashboard.json) (3,652 bytes,
three metrics, five chart bars, four table rows and prose),
[report.md](../../../examples/viewer/report.md) (2,152 bytes, narrative sections,
lists, a quote and a table), and [plan.md](../../../examples/viewer/plan.md)
(2,355 bytes, headings, task lists, lists, links and a code block). Exact byte
counts and SHA-256 hashes are in [environment.json](environment.json).
The dashboard fixture is archived here because the viewer's current demo has
since changed. These timings describe the archived fixture and build.

Raw observations: [latency.jsonl](latency.jsonl). Machine-readable aggregation:
[summary.json](summary.json). The binary hash identifies this exact build;
the source changes predate the squashed public history. The recorded hashes and
commit IDs describe the historical measured build, not the current viewer.

## Measurement boundaries

- **Cold process:** monotonic time from spawning a new isolated binary with a
  temporary fixture to observing revision 1 in its GPUI next-frame marker. This
  excludes CLI/LaunchServices bootstrap and includes file loading, parsing and layout.
- **Warm repeat open:** CLI subprocess launch through Unix-socket handoff to the
  existing workspace, valid-document frame acknowledgment, and CLI exit. Three
  documents are preloaded in separate tabs; they are opened in rotating order.
  This is repeat opening existing paths, not opening unseen documents. The forced
  load currently waits for the 250 ms polling loop, explaining much of this latency.
- **Reload:** an atomic semantic edit (JSON root ID change, or appended Markdown
  paragraph) through observing revision 2. The 250 ms polling delay is included.
  Edits begin immediately after first readiness, so this is not a randomized
  distribution of edit phases within the polling interval.

Markers indicate GPUI frame readiness, including parsed Markdown content. They
do not measure physical display presentation. Live UI checks independently verified
the visible fixtures, text copying, sorting, section navigation and internal links.
Scrolling and resizing were checked at 660, 1100 and 1400 logical-pixel widths;
no frame-time, FPS or p95 interaction latency claim is made. Much larger documents
and virtualization remain future work.

## Reproduce

Build with `sh scripts/bundle-macos.sh release`. For each fixture, run
`python3 benchmarks/harnesses/native.py --binary target/release/informe
--artifact examples/viewer/report.md --scenario cold-process --trials 20
--output /tmp/viewer.jsonl --environment YOUR_MACHINE` and repeat with
`--scenario reload`. Run each harness sequentially.

Warm opens: `python3 benchmarks/harnesses/warm_native.py
--binary "target/release/Informe.app/Contents/MacOS/informe"
--artifacts benchmarks/results/viewer-2026-10-03/dashboard.json examples/viewer/report.md examples/viewer/plan.md
--trials 20 --output /tmp/viewer.jsonl --environment YOUR_MACHINE`.

Then `python3 benchmarks/summarize.py /tmp/viewer.jsonl`.

## Next performance work

Wake the loader immediately for explicit open requests, measure first opens of
unseen documents separately, and add frame-pacing/large-document workloads.
A matched current HTML runtime and independent generation trials are required
before making a new comparative speed or token-efficiency claim.
