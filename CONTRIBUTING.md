# Contributing

Informe is an experimental native viewer. Focus changes on making
agent-generated reports, dashboards, and plans useful to read and explore. Discuss
larger changes in an issue before introducing a new subsystem.

## Development setup

Source builds are verified on Apple Silicon with macOS 26. Install Rust through
rustup and Xcode Command Line Tools, clone the repository, then run:

```sh
cargo run --locked -- examples/viewer/dashboard.json
cargo run --locked -- artifacts/cost-breakdown.json
```

Rust and GPUI revisions are pinned. Avoid updating the UI dependency set as part
of an unrelated feature. See [architecture](docs/architecture.md) for component
boundaries and the [artifact format](docs/artifact-format.md) for the contract.

## Design constraints

- Keep the artifact contract independent of GPUI and describe semantic objects.
- Keep node IDs stable; preserve selection and last-valid-preview recovery.
- Prefer one opinionated native theme over a general styling system.
- Keep parsing/file reads off the UI thread and loads ordered.
- Add primitives when they improve a concrete reading or exploration task. Avoid generic webpage
  builders, premature plugin systems, and complex application state.

When changing a primitive, update the Rust model, validation, renderer, text
export, schema, capabilities, format docs, skill reference, and relevant examples.
If benchmarks support that node, update all three fixture arms consistently.

## Checks

From the repository root:

```sh
cargo fmt --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
python3 -m unittest discover -s benchmarks -p test_tools.py
cargo run --locked -- --check examples/viewer/dashboard.json
cargo run --locked -- --check artifacts/cost-breakdown.json
```

Use tests for meaningful behavior and invariants. For UI changes, also run the
app and check selection, copy, scrolling, and reload/error recovery. Include a
screenshot when it helps explain a visual change. Documentation-only changes
need accurate commands, working links, and consistent descriptions of scope.
The macOS CI workflow checks Rust/Python behavior and creates an unsigned
internal release archive; it does not establish signed-installation readiness.

## Issues and pull requests

Report the app revision, macOS version, architecture, steps to reproduce, and
expected/actual behavior. Attach a minimal artifact when possible; use synthetic
data and remove credentials or private information. For benchmark reports,
include environment metadata, raw records, timing boundaries, and sample counts.

Keep pull requests focused. Describe the problem, resulting behavior, validation,
and material limitations. Contract changes should call out compatibility impact.
Performance claims should distinguish saved payload tokens from actual model
usage and process launches from warm opens.

## License

The project uses the [MIT license](LICENSE). Contributions should be compatible
with it; preserve the license and attribution of any third-party material added.
