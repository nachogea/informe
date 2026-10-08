[← README](../README.md)

# Agent workflow

An agent with local file/process access can create an artifact, validate it, and
launch the native viewer. No model provider or AI API is built into the runtime.
Generation happens in the agent environment you already use.

## Make the CLI available

From the repository root, build once and copy the CLI to a directory on PATH:

```sh
cargo build --release --locked
mkdir -p "$HOME/.local/bin"
install -m 755 target/release/informe "$HOME/.local/bin/informe"
export PATH="$HOME/.local/bin:$PATH"
informe --version
informe capabilities
informe schema
```

Add `~/.local/bin` to PATH in your shell profile for future sessions, and make
sure the agent process inherits it. Reinstall after rebuilding. Signed installation
is still pending; the skill does not install the viewer for you.

## Install the generation skill

The source skill lives at `skills/informe`. The repository exposes a
symlink at `.agents/skills/informe` for Codex discovery in this checkout.
To use it across projects, copy it to your personal skills directory:

```sh
mkdir -p ~/.agents/skills
cp -R skills/informe ~/.agents/skills/
```

Run this from the repository root. Check for an existing installation before
copying so you do not overwrite local customizations. Load or refresh skills as
required by your agent client. Other clients may use a different discovery path;
see the [official Codex skill
documentation](https://learn.chatgpt.com/docs/build-skills) for Codex locations.
The skill itself is plain Markdown with a contract reference. Homebrew packaging
does not modify agent settings.

Example request:

> Show me a dashboard of this project’s progress.

## Write, validate, publish, preview

1. Discover the installed capabilities and schema.
2. Use Markdown for prose reports/plans, or semantic JSON for dashboards and
   durable semantic targets. For JSON, use stable, globally unique IDs. Label
   invented data as illustrative. Do not generate `noop` buttons.
3. Write a sibling temporary file and run `informe --check` on it.
4. Repair errors, then rename the valid file atomically to the preview path.
5. Run `informe open /absolute/path/artifact.json` for a new preview.
   Further edits go to that same path; do not reopen the window for every edit.

For example, after an agent writes `/tmp/artifact.next.json`:

```sh
informe --check /tmp/artifact.next.json && \
  mv /tmp/artifact.next.json /tmp/artifact.json
informe open /tmp/artifact.json
```

Atomic renames should stay on the same filesystem. The viewer polls every 250 ms
and keeps the last valid artifact visible if an invalid update slips through.
A missing or invalid initial artifact leaves the viewer waiting for recovery;
`open` validates before handoff and exits with an error for invalid input. It
reuses the app and tab and waits for a valid rendered frame before reporting
success. Reopening an existing path brings that report forward.

## Reading and export

Drag/double-click text and use Command-C. Command-A selects the entire document;
Command-C then copies it.
Tables expose **Copy TSV** and **Save CSV…** directly. **More → Export PNG…**
includes the full document, even below the viewport; `export-png input.json
output.png` also works without a window. PNG flattens rich Markdown formatting.

The optional review workflow is documented in [reviewing reports](review.md) and
the [agent adapter](../skills/informe/references/review.md). Launch the
viewer with `INFORME_EXPERIMENTAL_TOOLS=1` to show its review and
developer controls in the menu. Quit an already-running viewer first; an open
handoff does not change that process’s environment. Existing local review data remains usable.
